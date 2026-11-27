use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use rusqlite::params;
use serde::Serialize;

use super::{DocumentHash, NewWork, RapError, RapStore, WorkId, WorkKind};
use crate::file_store::{FileStore, layout};

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct MigrationReport {
    pub epics: usize,
    pub tasks: usize,
    pub questions: usize,
    pub answers: usize,
    pub history_entries: usize,
    pub results: usize,
    pub attachments: usize,
    pub auto_run: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LegacyMapping {
    pub project_id: String,
    pub entity_kind: String,
    pub legacy_id: String,
    pub rap_id: i64,
    pub content_hash: String,
}

impl RapStore {
    /// Migrates projects, epics, tasks, history, Q&A, results, and attachments from a legacy
    /// `FileStore` into RAP SQLite. Refuses while any legacy session is active, and forces
    /// `auto_run = false` so the migrated board never starts executing automatically.
    pub fn migrate_from_file_store(
        &self,
        shared_dir: &Path,
        workspace: &Path,
        active_legacy_sessions: usize,
        now: i64,
    ) -> Result<MigrationReport, RapError> {
        if active_legacy_sessions > 0 {
            return Err(RapError::Conflict("stop legacy sessions before migrating"));
        }
        let legacy = FileStore::open(shared_dir);
        let projects = legacy
            .projects()
            .map_err(|_| RapError::InvalidInput("could not read legacy projects"))?;

        let mut report = MigrationReport::default();
        self.write(|tx| {
            tx.execute(
                "UPDATE scheduler_settings SET auto_run = 0 WHERE id = 1",
                [],
            )?;
            Ok(())
        })?;

        for project in projects {
            let snapshot = legacy
                .snapshot(&project.id)
                .map_err(|_| RapError::InvalidInput("could not read legacy project snapshot"))?;
            let project_key = project.id.as_str();
            let mut epic_map: HashMap<String, WorkId> = HashMap::new();
            let mut task_map: HashMap<i64, WorkId> = HashMap::new();

            for epic in &snapshot.epics {
                let item = self.create_work(NewWork {
                    project_id: project_key,
                    workspace,
                    kind: WorkKind::Epic,
                    parent_epic_id: None,
                    title: &epic.name,
                    rank: report.epics as i64,
                })?;
                let hash = self.put_document(item.id, epic.document.as_bytes())?;
                self.record_legacy_map(project_key, "epic", epic.id.as_str(), item.id.0, &hash)?;
                epic_map.insert(epic.id.as_str().to_owned(), item.id);
                report.epics += 1;
            }

            for task in &snapshot.tasks {
                let parent_epic_id = task
                    .epic_id
                    .as_deref()
                    .and_then(|eid| epic_map.get(eid).copied());
                let item = self.create_work(NewWork {
                    project_id: project_key,
                    workspace,
                    kind: WorkKind::Task,
                    parent_epic_id,
                    title: &task.title,
                    rank: task.position,
                })?;
                let desc_hash = self.put_document(item.id, task.description.as_bytes())?;
                self.record_legacy_map(
                    project_key,
                    "task",
                    &task.id.to_string(),
                    item.id.0,
                    &desc_hash,
                )?;
                task_map.insert(task.id, item.id);
                report.tasks += 1;

                for entry in &task.history {
                    self.write(|tx| {
                        tx.execute(
                            "INSERT INTO legacy_history (work_id, recorded_at, reason)
                             VALUES (?1, ?2, ?3)",
                            params![item.id.0, entry.at.to_rfc3339(), entry.reason],
                        )?;
                        Ok(())
                    })?;
                    report.history_entries += 1;
                }

                for result in &task.results {
                    let payload = serde_json::to_vec(result)
                        .map_err(|_| RapError::InvalidInput("cannot serialize legacy result"))?;
                    let hash = self.put_document(item.id, &payload)?;
                    self.record_legacy_map(
                        project_key,
                        "result",
                        &format!("{}:v{}", task.id, result.version),
                        item.id.0,
                        &hash,
                    )?;
                    report.results += 1;
                }

                let project_dir = layout::project_dir(shared_dir, &project.id);
                for att in &task.attachments {
                    if let Some(ext) = layout::asset_extension(&att.media_type)
                        && let Ok(path) =
                            layout::task_asset_path(&project_dir, task.id, &att.id, ext)
                        && let Ok(bytes) = fs::read(&path)
                    {
                        let hash = self.put_document(item.id, &bytes)?;
                        self.record_legacy_map(
                            project_key,
                            "attachment",
                            &format!("{}:{}", task.id, att.id),
                            item.id.0,
                            &hash,
                        )?;
                        report.attachments += 1;
                    }
                }
            }

            for question in &snapshot.questions {
                let target_work = question
                    .tasks
                    .first()
                    .and_then(|tid| task_map.get(tid).copied())
                    .or_else(|| task_map.values().next().copied());
                let Some(work_id) = target_work else { continue };
                let session_id = self.write(|tx| {
                    let id: i64 = tx.query_row(
                        "INSERT INTO sessions (work_id, phase, provider)
                         VALUES (?1, 'execution', 'legacy') RETURNING id",
                        [work_id.0],
                        |row| row.get(0),
                    )?;
                    Ok(super::SessionId(id))
                })?;
                let q = self.ask_question(work_id, session_id, &question.body, now)?;
                let mut digest_input = question.body.clone();
                if let Some(reply) = &question.reply {
                    let answer_text = reply
                        .text
                        .as_deref()
                        .or(reply.choice_label.as_deref())
                        .or(reply.choice_id.as_deref())
                        .unwrap_or("(answered)");
                    self.answer_question(q.id, answer_text, "human", now)?;
                    digest_input.push_str("\n---\n");
                    digest_input.push_str(answer_text);
                    report.answers += 1;
                }
                let hash = DocumentHash::from_bytes(digest_input.as_bytes());
                self.record_legacy_map(project_key, "question", &question.id, q.id.0, &hash)?;
                report.questions += 1;
            }
        }
        Ok(report)
    }

    pub fn legacy_mappings(&self, project_id: &str) -> Result<Vec<LegacyMapping>, RapError> {
        self.read(|connection| {
            let mut stmt = connection.prepare(
                "SELECT project_id, entity_kind, legacy_id, rap_id, content_hash
                 FROM legacy_id_map WHERE project_id = ?1 ORDER BY entity_kind, legacy_id",
            )?;
            let rows = stmt.query_map([project_id], |row| {
                Ok(LegacyMapping {
                    project_id: row.get(0)?,
                    entity_kind: row.get(1)?,
                    legacy_id: row.get(2)?,
                    rap_id: row.get(3)?,
                    content_hash: row.get(4)?,
                })
            })?;
            Ok(rows.collect::<rusqlite::Result<_>>()?)
        })
    }

    fn record_legacy_map(
        &self,
        project_id: &str,
        entity_kind: &str,
        legacy_id: &str,
        rap_id: i64,
        hash: &DocumentHash,
    ) -> Result<(), RapError> {
        self.write(|tx| {
            tx.execute(
                "INSERT INTO legacy_id_map (project_id, entity_kind, legacy_id, rap_id, content_hash)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![project_id, entity_kind, legacy_id, rap_id, hash.as_str()],
            )?;
            Ok(())
        })
    }
}

/// Copies a directory tree and returns every regular file's relative path and SHA-256 digest.
pub fn copy_and_digest_tree(src: &Path, dst: &Path) -> Result<BTreeMap<String, String>, RapError> {
    let mut digests = BTreeMap::new();
    copy_dir_recursive(src, src, dst, &mut digests)?;
    Ok(digests)
}

/// Computes every regular file's relative path and SHA-256 digest for rollback verification.
pub fn digest_tree(root: &Path) -> Result<BTreeMap<String, String>, RapError> {
    let mut digests = BTreeMap::new();
    digest_dir_recursive(root, root, &mut digests)?;
    Ok(digests)
}

fn copy_dir_recursive(
    root: &Path,
    current: &Path,
    dst_root: &Path,
    digests: &mut BTreeMap<String, String>,
) -> Result<(), RapError> {
    fs::create_dir_all(dst_root.join(current.strip_prefix(root).unwrap_or(Path::new(""))))?;
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        if entry.file_type()?.is_dir() {
            copy_dir_recursive(root, &path, dst_root, digests)?;
        } else if entry.file_type()?.is_file() {
            let bytes = fs::read(&path)?;
            let target = dst_root.join(rel);
            fs::write(&target, &bytes)?;
            digests.insert(
                rel.to_string_lossy().into_owned(),
                DocumentHash::from_bytes(&bytes).as_str().to_owned(),
            );
        }
    }
    Ok(())
}

fn digest_dir_recursive(
    root: &Path,
    current: &Path,
    digests: &mut BTreeMap<String, String>,
) -> Result<(), RapError> {
    for entry in fs::read_dir(current)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            digest_dir_recursive(root, &path, digests)?;
        } else if entry.file_type()?.is_file() {
            let rel = path.strip_prefix(root).unwrap_or(&path);
            let bytes = fs::read(&path)?;
            digests.insert(
                rel.to_string_lossy().into_owned(),
                DocumentHash::from_bytes(&bytes).as_str().to_owned(),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Actor, Assignee, NewTask};
    use crate::file_store::model::NewEpic;
    use crate::file_store::qa::AnswerVia;
    use crate::file_store::validate::EpicId;
    use crate::file_store::{AnswerItem, AskInput, AskQuestion};
    use crate::rap::scheduler::SchedulerStore;

    #[test]
    fn migrates_legacy_store_with_hashes_requires_stopped_sessions_and_restores_backup() {
        let root = tempfile::tempdir().unwrap();
        let shared = root.path().join("shared");
        let backup = root.path().join("backup");
        let restored = root.path().join("restored");
        let ws = root.path().join("ws");
        fs::create_dir_all(&ws).unwrap();

        // Populate legacy FileStore with epic, child task, and answered question.
        let legacy = FileStore::open(&shared);
        let now = crate::clock::now();
        let project = legacy.create_project("demo", now).unwrap();
        legacy
            .create_epic(
                &project.id,
                NewEpic {
                    id: "EPIC-1".into(),
                    name: "Core".into(),
                    color: None,
                    draft: None,
                    document: Some("# Core Epic".into()),
                },
                now,
            )
            .unwrap();
        let child = legacy
            .create_task(
                &project.id,
                NewTask {
                    title: "Child task".into(),
                    description: "Child body".into(),
                    assignee: Assignee::Ai,
                    actor: Actor::Human,
                    draft: None,
                },
                now,
            )
            .unwrap();
        let epic_id = EpicId::normalize("EPIC-1").unwrap();
        legacy
            .set_task_epic(&project.id, child.id, Some(&epic_id), Actor::Human, now)
            .unwrap();
        let qs = legacy
            .ask_questions(
                &project.id,
                AskInput {
                    actor: Actor::Ai,
                    asked_by: None,
                    brainstorm: None,
                    held: false,
                    questions: vec![AskQuestion {
                        body: "Ship now?".into(),
                        choices: vec![],
                        recommended_choice_id: None,
                        tasks: vec![child.id],
                        images: vec![],
                    }],
                    tasks: vec![],
                },
                now,
            )
            .unwrap();
        legacy
            .answer_questions(
                &project.id,
                vec![AnswerItem {
                    question_id: qs[0].id.clone(),
                    choice_id: None,
                    text: Some("Yes".into()),
                    images: vec![],
                }],
                AnswerVia::Ui,
                now,
            )
            .unwrap();

        // Backup legacy store before migration.
        let expected_digests = copy_and_digest_tree(&shared, &backup).unwrap();

        let store = RapStore::open(&root.path().join("rap.db")).unwrap();
        // Refuses while legacy sessions are active.
        assert!(matches!(
            store.migrate_from_file_store(&shared, &ws, 1, 100),
            Err(RapError::Conflict(_))
        ));

        // Migrates cleanly with auto_run = false (project seeds BUG + ENHANCEMENT + EPIC-1).
        let report = store.migrate_from_file_store(&shared, &ws, 0, 100).unwrap();
        assert_eq!(report.epics, 3);
        assert_eq!(report.tasks, 1);
        assert_eq!(report.questions, 1);
        assert_eq!(report.answers, 1);
        assert!(report.history_entries >= 1);
        assert!(!report.auto_run);
        assert!(
            !SchedulerStore::new(store.clone())
                .settings()
                .unwrap()
                .auto_run
        );

        let mappings = store.legacy_mappings(project.id.as_str()).unwrap();
        assert_eq!(mappings.len(), 5); // 3 epics + 1 task + 1 question
        let work_items = store.project_work(project.id.as_str()).unwrap();
        assert_eq!(work_items.len(), 4);
        let core_epic = work_items.iter().find(|w| w.title == "Core").unwrap();
        let migrated_child = work_items.iter().find(|w| w.title == "Child task").unwrap();
        assert_eq!(migrated_child.parent_epic_id, Some(core_epic.id));

        // Restore from backup copy and prove byte-for-byte SHA-256 identity.
        copy_and_digest_tree(&backup, &restored).unwrap();
        assert_eq!(digest_tree(&restored).unwrap(), expected_digests);
    }
}
