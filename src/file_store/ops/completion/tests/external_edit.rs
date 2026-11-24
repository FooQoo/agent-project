use super::*;

#[test]
fn direct_task_file_updates_are_rechecked_and_existing_done_is_not_reopened() {
    let f = Fixture::new();
    let id = f.task("Edited externally", Assignee::Ai, OPEN, false);
    f.replace_description(id, COMPLETE);
    assert!(f.store.auto_complete_all(at(2)).unwrap());

    f.replace_description(id, &OPEN.replace("Work.", "Work, rewritten."));
    assert!(f.store.auto_complete_all(at(3)).unwrap());
    let task = f.store.task(&f.project, id).unwrap();
    assert_eq!(task.task_status, Column::Done);
    assert!(task.external_edit);
    assert_eq!(
        task.history
            .iter()
            .filter(|entry| entry.reason.contains("updated directly"))
            .count(),
        1
    );
    let confirmed = f.store.confirm_external_edit(&f.project, id).unwrap();
    assert!(!confirmed.external_edit);
}

#[test]
fn direct_edit_in_doing_is_recorded_and_blocks_automatic_completion() {
    let f = Fixture::new();
    let id = f.task("Direct edit", Assignee::Ai, OPEN, false);
    f.store
        .move_work_task(
            &f.project,
            id,
            Column::Doing,
            ChangeContext {
                actor: Actor::Ai,
                reason: "Start".into(),
            },
            at(2),
        )
        .unwrap();
    // The checkboxes are ticked, but the body changed in the same edit.
    f.replace_description(id, &COMPLETE.replace("Work.", "Work, rewritten."));
    assert!(f.store.auto_complete_all(at(3)).unwrap());
    assert!(!f.store.auto_complete_all(at(4)).unwrap());
    let task = f.store.task(&f.project, id).unwrap();
    assert_eq!(task.task_status, Column::Doing);
    assert_eq!(
        task.history
            .iter()
            .filter(|entry| entry.reason.contains("updated directly"))
            .count(),
        1
    );
}

#[test]
fn direct_edit_in_todo_is_accepted_and_can_complete() {
    let f = Fixture::new();
    let id = f.task("Planning edit", Assignee::Ai, OPEN, false);
    f.replace_description(id, COMPLETE);
    assert!(f.store.auto_complete_all(at(2)).unwrap());
    assert_eq!(
        f.store.task(&f.project, id).unwrap().task_status,
        Column::Done
    );
}

#[test]
fn a_direct_edit_of_only_checkboxes_in_doing_completes_the_card_without_review() {
    let f = Fixture::new();
    let id = f.task("Checkbox edit", Assignee::Ai, OPEN, false);
    f.store
        .move_work_task(
            &f.project,
            id,
            Column::Doing,
            ChangeContext {
                actor: Actor::Ai,
                reason: "Start".into(),
            },
            at(2),
        )
        .unwrap();
    f.replace_description(id, COMPLETE);
    assert!(f.store.auto_complete_all(at(3)).unwrap());
    let task = f.store.task(&f.project, id).unwrap();
    assert_eq!(task.task_status, Column::Done);
    assert!(!task.external_edit);
    assert!(
        !task
            .history
            .iter()
            .any(|entry| entry.reason.contains("updated directly"))
    );
}

#[test]
fn the_shape_of_a_task_file_ignores_only_acceptance_checkboxes() {
    use super::super::super::save::shape_digest;
    let shape = |text: &str| shape_digest(text.as_bytes());
    assert_eq!(shape(OPEN), shape(COMPLETE));
    let crlf_open = OPEN.replace('\n', "\r\n");
    let crlf_complete = COMPLETE.replace('\n', "\r\n");
    assert_eq!(shape(&crlf_open), shape(&crlf_complete));
    assert_ne!(shape(OPEN), shape(&OPEN.replace("Work.", "Other.")));
    assert_ne!(shape(OPEN), shape(&OPEN.replace("Second", "Third")));
    assert_ne!(
        shape(OPEN),
        shape(&OPEN.replace("    text: First", "    text: Changed"))
    );
    assert_ne!(
        shape(OPEN),
        shape(&OPEN.replace("    done: false", "    done: \"false\""))
    );
    assert_ne!(shape(OPEN), shape(&format!("{OPEN}metadata: changed\n")));
    // A `done:` line in the body is not a checkbox.
    assert_ne!(
        shape(&format!("{OPEN}\ndone: true\n")),
        shape(&format!("{OPEN}\ndone: false\n"))
    );
}

#[test]
fn watcher_reconciliation_upgrades_old_shape_digest_without_touching_task_markdown() {
    let f = Fixture::new();
    let id = f.task("Unchanged", Assignee::Ai, OPEN, false);
    let markdown_path = f.path(id);
    let dir = markdown_path.parent().unwrap();
    let metadata_path = dir.join("task.json");
    let markdown = std::fs::read(&markdown_path).unwrap();
    let mut metadata: crate::file_store::model::TaskMetadata =
        serde_json::from_slice(&std::fs::read(&metadata_path).unwrap()).unwrap();
    metadata.shape_digest = Some("legacy-shape-digest".into());
    let mut json = serde_json::to_vec_pretty(&metadata).unwrap();
    json.push(b'\n');
    std::fs::write(&metadata_path, json).unwrap();

    assert!(f.store.auto_complete_all(at(2)).unwrap());
    let upgraded: crate::file_store::model::TaskMetadata =
        serde_json::from_slice(&std::fs::read(&metadata_path).unwrap()).unwrap();
    assert_eq!(
        upgraded.content_digest.as_deref(),
        Some(super::super::super::save::content_digest(&markdown).as_str())
    );
    assert_eq!(
        upgraded.shape_digest.as_deref(),
        Some(super::super::super::save::shape_digest(&markdown).as_str())
    );
    assert_eq!(std::fs::read(&markdown_path).unwrap(), markdown);
}

#[test]
fn watcher_reconciliation_bootstraps_an_ordinary_task_once() {
    let f = Fixture::new();
    let markdown = "---\ntitle: Ordinary task\n---\n\nNo acceptance conditions.\n";
    let id = f.task("Ordinary task", Assignee::Ai, markdown, false);
    let markdown_path = f.path(id);
    let metadata_path = markdown_path.parent().unwrap().join("task.json");
    let mut metadata: crate::file_store::model::TaskMetadata =
        serde_json::from_slice(&std::fs::read(&metadata_path).unwrap()).unwrap();
    metadata.content_digest = None;
    metadata.shape_digest = None;
    let mut json = serde_json::to_vec_pretty(&metadata).unwrap();
    json.push(b'\n');
    std::fs::write(&metadata_path, json).unwrap();

    assert!(f.store.auto_complete_all(at(2)).unwrap());
    assert!(!f.store.auto_complete_all(at(3)).unwrap());
    let saved: crate::file_store::model::TaskMetadata =
        serde_json::from_slice(&std::fs::read(&metadata_path).unwrap()).unwrap();
    assert_eq!(
        saved.content_digest.as_deref(),
        Some(super::super::super::save::content_digest(markdown.as_bytes()).as_str())
    );
    assert_eq!(
        saved.shape_digest.as_deref(),
        Some(super::super::super::save::shape_digest(markdown.as_bytes()).as_str())
    );
    assert_eq!(std::fs::read(&markdown_path).unwrap(), markdown.as_bytes());
}
