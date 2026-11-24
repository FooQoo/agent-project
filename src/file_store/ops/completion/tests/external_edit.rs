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
    assert_ne!(shape(OPEN), shape(&OPEN.replace("Work.", "Other.")));
    assert_ne!(shape(OPEN), shape(&OPEN.replace("Second", "Third")));
    // A `done:` line in the body is not a checkbox.
    assert_ne!(
        shape(&format!("{OPEN}\ndone: true\n")),
        shape(&format!("{OPEN}\ndone: false\n"))
    );
}
