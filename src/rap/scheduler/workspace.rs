//! Identify overlapping workspaces before assigning local agents to them.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The working directory and its nearest enclosing Git worktree, if any.
///
/// A linked worktree has its own `.git` pointer, so it is independent of another
/// worktree of the same repository. Nested directories within one worktree
/// share the same Git index and conflict even if neither contains the other.
#[derive(Debug, PartialEq, Eq)]
pub(super) struct WorkspaceIdentity {
    path: PathBuf,
    ancestors: Vec<DirectoryIdentity>,
    git_root: Option<DirectoryIdentity>,
}

impl WorkspaceIdentity {
    pub(super) fn resolve(path: &Path) -> io::Result<Self> {
        let path = fs::canonicalize(path)?;
        let mut ancestors = Vec::new();
        let mut git_root = None;

        for directory in path.ancestors() {
            let identity = DirectoryIdentity::read(directory)?;
            if git_root.is_none() && has_git_marker(directory)? {
                git_root = Some(identity.clone());
            }
            ancestors.push(identity);
        }

        Ok(Self {
            path,
            ancestors,
            git_root,
        })
    }

    pub(super) fn path(&self) -> &Path {
        &self.path
    }

    pub(super) fn fingerprint(&self) -> String {
        #[cfg(unix)]
        {
            self.ancestors
                .iter()
                .chain(self.git_root.iter())
                .map(|identity| format!("{}:{}", identity.device, identity.inode))
                .collect::<Vec<_>>()
                .join("/")
        }
        #[cfg(not(unix))]
        {
            format!("{:?}", self)
        }
    }

    pub(super) fn overlaps(&self, other: &Self) -> bool {
        self.ancestors.contains(&other.ancestors[0])
            || other.ancestors.contains(&self.ancestors[0])
            || self
                .git_root
                .as_ref()
                .is_some_and(|root| other.git_root.as_ref() == Some(root))
    }
}

fn has_git_marker(directory: &Path) -> io::Result<bool> {
    match fs::symlink_metadata(directory.join(".git")) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectoryIdentity {
    device: u64,
    inode: u64,
}

#[cfg(not(unix))]
#[derive(Clone, Debug, PartialEq, Eq)]
struct DirectoryIdentity(PathBuf);

impl DirectoryIdentity {
    fn read(directory: &Path) -> io::Result<Self> {
        let metadata = fs::metadata(directory)?;
        if !metadata.is_dir() {
            return Err(io::Error::new(
                io::ErrorKind::NotADirectory,
                "workspace path is not a directory",
            ));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            Ok(Self {
                device: metadata.dev(),
                inode: metadata.ino(),
            })
        }
        #[cfg(not(unix))]
        {
            Ok(Self(directory.to_path_buf()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_the_same_directory_and_nested_directories() {
        let home = tempfile::tempdir().unwrap();
        let parent = home.path().join("app");
        let child = parent.join("child");
        let nearby = home.path().join("app-other");
        fs::create_dir_all(&child).unwrap();
        fs::create_dir(&nearby).unwrap();

        let parent = WorkspaceIdentity::resolve(&parent).unwrap();
        let child = WorkspaceIdentity::resolve(&child).unwrap();
        let nearby = WorkspaceIdentity::resolve(&nearby).unwrap();
        assert!(parent.overlaps(&parent));
        assert!(parent.overlaps(&child));
        assert!(child.overlaps(&parent));
        assert!(!parent.overlaps(&nearby));
    }

    #[cfg(unix)]
    #[test]
    fn resolves_a_symlink_to_the_same_workspace() {
        let home = tempfile::tempdir().unwrap();
        let original = home.path().join("original");
        let alias = home.path().join("alias");
        fs::create_dir(&original).unwrap();
        std::os::unix::fs::symlink(&original, &alias).unwrap();

        let original = WorkspaceIdentity::resolve(&original).unwrap();
        let alias = WorkspaceIdentity::resolve(&alias).unwrap();
        assert_eq!(original.path(), alias.path());
        assert_eq!(original, alias);
        assert!(original.overlaps(&alias));
    }

    #[test]
    fn shares_git_worktree_across_siblings_and_excludes_other_worktrees() {
        let home = tempfile::tempdir().unwrap();
        let first_root = home.path().join("first");
        let second_root = home.path().join("second");
        let first = first_root.join("front");
        let second = first_root.join("back");
        let linked = second_root.join("front");
        fs::create_dir_all(&first).unwrap();
        fs::create_dir_all(&second).unwrap();
        fs::create_dir_all(&linked).unwrap();
        fs::create_dir(first_root.join(".git")).unwrap();
        fs::write(
            second_root.join(".git"),
            "gitdir: ../first/.git/worktrees/second",
        )
        .unwrap();

        let first = WorkspaceIdentity::resolve(&first).unwrap();
        let second = WorkspaceIdentity::resolve(&second).unwrap();
        let linked = WorkspaceIdentity::resolve(&linked).unwrap();
        assert!(first.overlaps(&second));
        assert!(!first.overlaps(&linked));
    }

    #[test]
    fn rejects_missing_and_non_directory_paths() {
        let home = tempfile::tempdir().unwrap();
        let missing = home.path().join("missing");
        assert!(WorkspaceIdentity::resolve(&missing).is_err());

        let file = home.path().join("file");
        fs::write(&file, "").unwrap();
        assert_eq!(
            WorkspaceIdentity::resolve(&file).unwrap_err().kind(),
            io::ErrorKind::NotADirectory
        );
    }
}
