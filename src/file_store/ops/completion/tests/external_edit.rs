use super::*;

#[test]
fn direct_task_file_updates_are_rechecked_and_existing_done_is_not_reopened() {
    let f = Fixture::new();
    let id = f.task("Edited externally", Assignee::Ai, OPEN, false);
    f.replace_description(id, COMPLETE);
    assert!(f.store.auto_complete_all(at(2)).unwrap());

    f.replace_description(id, OPEN);
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
    f.replace_description(id, COMPLETE);
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
