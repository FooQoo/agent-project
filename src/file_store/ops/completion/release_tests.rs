use crate::domain::{Actor, ChangeContext, Column};

use super::tests::{COMPLETE, Fixture, OPEN, at};

fn list_version_file(f: &Fixture, name: &str) {
    let dir = f
        .store
        .shared_dir()
        .join("projects")
        .join(f.project.as_str())
        .join("docs/release");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join(format!("{name}.md")), "").unwrap();
}

/// AC-2, AC-3: the automatic Done of a release-work card moves the release version to the next
/// listed version once, and a card of another version leaves the release version alone.
#[test]
fn automatic_done_of_a_release_work_card_advances_the_release_version() {
    let f = Fixture::new();
    f.store.add_version(&f.project, "v1.0.0", at(2)).unwrap();
    f.store.add_version(&f.project, "v1.1.0", at(2)).unwrap();
    list_version_file(&f, "v1.2.0");
    f.store
        .set_release_version(&f.project, Some("v1.0.0"), at(2))
        .unwrap();
    let ids: Vec<i64> = (1..=2).collect();
    for id in &ids {
        f.replace_description(*id, COMPLETE);
        f.store
            .move_work_task_for_user(
                &f.project,
                *id,
                Column::Todo,
                ChangeContext {
                    actor: Actor::Human,
                    reason: "Send".into(),
                },
                at(3),
            )
            .unwrap();
    }
    // The v1.1.0 card is not the release version's card: it completes and changes nothing.
    let other = f
        .store
        .snapshot(&f.project)
        .unwrap()
        .tasks
        .iter()
        .find(|t| t.title.contains("v1.1.0"))
        .unwrap()
        .id;
    let own = ids.iter().copied().find(|id| *id != other).unwrap();
    // Complete only the v1.1.0 card first.
    f.replace_description(own, OPEN);
    assert!(f.store.auto_complete_all(at(4)).unwrap());
    assert_eq!(
        f.store
            .project_versions(&f.project)
            .unwrap()
            .release
            .as_deref(),
        Some("v1.0.0")
    );
    f.replace_description(own, COMPLETE);
    assert!(f.store.auto_complete_all(at(5)).unwrap());
    assert_eq!(
        f.store
            .project_versions(&f.project)
            .unwrap()
            .release
            .as_deref(),
        Some("v1.1.0")
    );
    let history = f.store.task(&f.project, own).unwrap().history;
    assert!(
        history
            .iter()
            .any(|h| h.reason == "Release version moved from v1.0.0 to v1.1.0.")
    );
    // A repeat pass changes nothing.
    assert!(!f.store.auto_complete_all(at(6)).unwrap());
    assert_eq!(
        f.store
            .project_versions(&f.project)
            .unwrap()
            .release
            .as_deref(),
        Some("v1.1.0")
    );
}
