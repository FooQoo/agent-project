use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use rusqlite::{Connection, Transaction, params};
use serde::Serialize;
use sha2::{Digest, Sha256};

use super::{RapError, RapStore, WorkId, fetch_work};
use crate::file_store::io::sync_dir;

const STAGING_PREFIX: &str = ".stage-";

/// An immutable object name formed from the SHA-256 digest of its bytes.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(transparent)]
pub struct DocumentHash(String);

impl DocumentHash {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn from_bytes(bytes: &[u8]) -> Self {
        Self(format!("{:x}", Sha256::digest(bytes)))
    }
}

impl TryFrom<&str> for DocumentHash {
    type Error = RapError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        if is_document_hash(value) {
            Ok(Self(value.to_owned()))
        } else {
            Err(RapError::InvalidInput("invalid document hash"))
        }
    }
}

impl RapStore {
    /// Stores a document or attachment and associates it with one work item.
    pub fn put_document(&self, work: WorkId, bytes: &[u8]) -> Result<DocumentHash, RapError> {
        self.write(|tx| {
            fetch_work(tx, work)?;
            self.persist_document(tx, work, bytes)
        })
    }

    /// Reads an associated document and verifies the exact stored bytes.
    pub fn read_document(&self, work: WorkId, hash: &DocumentHash) -> Result<Vec<u8>, RapError> {
        self.read(|connection| self.read_work_document(connection, work, hash))
    }

    /// Verifies that every document referenced by a work item is present and intact.
    pub fn verify_work_documents(&self, work: WorkId) -> Result<(), RapError> {
        self.read(|connection| {
            fetch_work(connection, work)?;
            self.verify_work_documents_in(connection, work)
        })
    }

    pub(crate) fn verify_work_documents_in(
        &self,
        connection: &Connection,
        work: WorkId,
    ) -> Result<(), RapError> {
        let mut statement = connection
            .prepare("SELECT hash FROM work_documents WHERE work_id = ?1 ORDER BY hash")?;
        let hashes: Vec<String> = statement
            .query_map([work.0], |row| row.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        if hashes.is_empty() {
            return Ok(());
        }
        let directory = self.document_directory();
        require_directory(&directory)
            .map_err(|_| RapError::DocumentIntegrity(hashes[0].clone()))?;
        for raw in hashes {
            let hash = DocumentHash::try_from(raw.as_str())
                .map_err(|_| RapError::DocumentIntegrity(raw.clone()))?;
            verify_object(&directory.join(hash.as_str()), &hash)?;
        }
        Ok(())
    }

    pub(crate) fn persist_document(
        &self,
        tx: &Transaction<'_>,
        work: WorkId,
        bytes: &[u8],
    ) -> Result<DocumentHash, RapError> {
        let hash = DocumentHash::from_bytes(bytes);
        let directory = self.document_directory();
        ensure_document_directory(&directory)?;
        let path = directory.join(hash.as_str());

        // All writers and recovery hold the same SQLite write transaction.
        // Publish the durable file before inserting its database reference.
        match fs::symlink_metadata(&path) {
            Ok(_) => verify_object(&path, &hash)?,
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                let mut pending = tempfile::Builder::new()
                    .prefix(STAGING_PREFIX)
                    .tempfile_in(&directory)?;
                pending.write_all(bytes)?;
                pending.as_file().sync_all()?;
                verify_object(pending.path(), &hash)?;
                pending
                    .persist_noclobber(path)
                    .map_err(|error| error.error)?;
            }
            Err(error) => return Err(error.into()),
        }
        sync_dir(&directory)?;
        tx.execute(
            "INSERT INTO work_documents (work_id, hash) VALUES (?1, ?2)
             ON CONFLICT (work_id, hash) DO NOTHING",
            params![work.0, hash.as_str()],
        )?;
        Ok(hash)
    }

    pub(crate) fn read_work_document(
        &self,
        connection: &Connection,
        work: WorkId,
        hash: &DocumentHash,
    ) -> Result<Vec<u8>, RapError> {
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM work_documents WHERE work_id = ?1 AND hash = ?2)",
            params![work.0, hash.as_str()],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(RapError::NotFound("document"));
        }

        let directory = self.document_directory();
        require_directory(&directory).map_err(|_| RapError::DocumentIntegrity(hash.0.clone()))?;
        read_object(&directory.join(hash.as_str()), hash)
    }

    pub(super) fn recover_documents(&self, tx: &Transaction<'_>) -> Result<(), RapError> {
        let directory = self.document_directory();
        match require_directory(&directory) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(error) => return Err(error.into()),
        }

        let mut removed = false;
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let name = entry.file_name();
            let Some(name) = name.to_str() else { continue };
            if name.starts_with(STAGING_PREFIX)
                || (is_document_hash(name)
                    && !tx.query_row(
                        "SELECT EXISTS(SELECT 1 FROM work_documents WHERE hash = ?1)",
                        [name],
                        |row| row.get::<_, bool>(0),
                    )?)
            {
                fs::remove_file(entry.path())?;
                removed = true;
            }
        }
        if removed {
            sync_dir(&directory)?;
        }
        Ok(())
    }

    fn document_directory(&self) -> PathBuf {
        let mut name = self.database.as_os_str().to_owned();
        name.push(".objects");
        PathBuf::from(name)
    }
}

fn is_document_hash(name: &str) -> bool {
    name.len() == 64
        && name
            .bytes()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
}

fn ensure_document_directory(path: &Path) -> io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    match builder.create(path) {
        Ok(()) => {
            // Persist the new directory entry before any database references
            // can point into the directory.
            if let Some(parent) = path
                .parent()
                .filter(|parent| !parent.as_os_str().is_empty())
            {
                sync_dir(parent)?;
            }
        }
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
        Err(error) => return Err(error),
    }
    require_directory(path)
}

fn require_directory(path: &Path) -> io::Result<()> {
    if fs::symlink_metadata(path)?.is_dir() {
        return Ok(());
    }
    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        "RAP document directory must be a physical directory",
    ))
}

fn open_object(path: &Path) -> io::Result<File> {
    if !fs::symlink_metadata(path)?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "RAP document must be a regular file",
        ));
    }
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options.open(path)?;
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "RAP document must be a regular file",
        ));
    }
    Ok(file)
}

fn verify_object(path: &Path, hash: &DocumentHash) -> Result<(), RapError> {
    let actual = (|| -> io::Result<_> {
        let mut actual = Sha256::new();
        io::copy(&mut open_object(path)?, &mut actual)?;
        Ok(format!("{:x}", actual.finalize()))
    })();
    if actual.is_ok_and(|actual| actual == hash.0) {
        return Ok(());
    }
    Err(RapError::DocumentIntegrity(hash.0.clone()))
}

fn read_object(path: &Path, hash: &DocumentHash) -> Result<Vec<u8>, RapError> {
    let mut bytes = Vec::new();
    let read = open_object(path).and_then(|mut file| file.read_to_end(&mut bytes));
    if read.is_ok() && DocumentHash::from_bytes(&bytes) == *hash {
        return Ok(bytes);
    }
    Err(RapError::DocumentIntegrity(hash.0.clone()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rap::{NewWork, WorkKind};

    #[test]
    fn publication_survives_reopen_and_recovery_removes_only_unreferenced_objects() {
        let root = tempfile::tempdir().unwrap();
        let db = root.path().join("rap.db");
        let store = RapStore::open(&db).unwrap();
        let create_work = |title| {
            store
                .create_work(NewWork {
                    project_id: "demo",
                    workspace: root.path(),
                    kind: WorkKind::Task,
                    parent_epic_id: None,
                    title,
                    rank: 0,
                })
                .unwrap()
                .id
        };
        let work = create_work("Change");
        let other = create_work("Separate work");
        let original = b"# Approved plan";
        let document = store.put_document(work, original).unwrap();
        assert_eq!(store.put_document(work, original).unwrap(), document);
        assert!(matches!(
            store.read_document(other, &document),
            Err(RapError::NotFound("document"))
        ));
        let directory = store.document_directory();
        let staged = directory.join(".stage-abandoned");
        fs::write(&staged, b"partial").unwrap();
        let orphan = DocumentHash::from_bytes(b"reference transaction rolled back");
        fs::write(
            directory.join(orphan.as_str()),
            b"reference transaction rolled back",
        )
        .unwrap();

        let reopened = RapStore::open(&db).unwrap();
        assert!(!staged.exists());
        assert!(!directory.join(orphan.as_str()).exists());
        assert_eq!(reopened.read_document(work, &document).unwrap(), original);
        reopened.verify_work_documents(work).unwrap();

        fs::write(directory.join(document.as_str()), b"wrong bytes").unwrap();
        assert!(matches!(
            reopened.read_document(work, &document),
            Err(RapError::DocumentIntegrity(_))
        ));
        assert!(matches!(
            reopened.verify_work_documents(work),
            Err(RapError::DocumentIntegrity(_))
        ));
        assert!(matches!(
            reopened.put_document(work, original),
            Err(RapError::DocumentIntegrity(_))
        ));
    }
}
