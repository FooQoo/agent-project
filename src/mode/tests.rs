use super::store::{FILE_NAME, INTERNAL_DIR};
use super::*;
use chrono::TimeZone;

#[test]
fn default_is_sparring_and_sleep_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let now = Utc.with_ymd_and_hms(2026, 11, 24, 1, 2, 3).unwrap();
    assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
    let saved = set(dir.path(), "p1", ModeKind::Sleep, now).unwrap();
    assert_eq!(saved.since, Some(now));
    assert_eq!(load(dir.path(), "p1").kind, ModeKind::Sleep);
    assert_eq!(current(dir.path(), "p2"), ModeKind::Sparring);
    set(dir.path(), "p1", ModeKind::Sparring, now).unwrap();
    assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
}

#[test]
fn unknown_kind_and_broken_file_read_as_sparring() {
    let dir = tempfile::tempdir().unwrap();
    let internal = dir.path().join(INTERNAL_DIR);
    std::fs::create_dir_all(&internal).unwrap();
    std::fs::write(
        internal.join(FILE_NAME),
        r#"{"projects":{"p1":{"kind":"party"}}}"#,
    )
    .unwrap();
    assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
    std::fs::write(internal.join(FILE_NAME), "not json").unwrap();
    assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
}

#[test]
fn set_keeps_an_unreadable_file_intact() {
    let dir = tempfile::tempdir().unwrap();
    let internal = dir.path().join(INTERNAL_DIR);
    std::fs::create_dir_all(&internal).unwrap();
    let file = internal.join(FILE_NAME);
    for broken in ["not json", "[]", r#"{"projects":[]}"#] {
        std::fs::write(&file, broken).unwrap();
        let now = Utc::now();
        assert!(set(dir.path(), "p1", ModeKind::Sleep, now).is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), broken);
    }
    std::fs::remove_file(&file).unwrap();
    std::fs::create_dir(&file).unwrap();
    assert!(set(dir.path(), "p1", ModeKind::Sleep, Utc::now()).is_err());
    assert!(file.is_dir());
}

#[test]
fn concurrent_saves_of_different_projects_both_survive() {
    let dir = tempfile::tempdir().unwrap();
    let now = Utc::now();
    std::thread::scope(|scope| {
        for n in 0..8 {
            let path = dir.path();
            scope.spawn(move || {
                set(path, &format!("p{n}"), ModeKind::Sleep, now).unwrap();
            });
        }
    });
    for n in 0..8 {
        assert_eq!(current(dir.path(), &format!("p{n}")), ModeKind::Sleep);
    }
}

fn epic(id: &str) -> Target {
    Target {
        kind: TargetKind::Epic,
        id: id.into(),
    }
}

#[test]
fn target_parse_tells_tasks_from_epics() {
    assert_eq!(
        Target::parse("12"),
        Target {
            kind: TargetKind::Task,
            id: "12".into()
        }
    );
    assert_eq!(Target::parse("my-epic"), epic("MY-EPIC"));
    assert_eq!(Target::parse("12").label(), "task #12");
    assert_eq!(epic("X").label(), "epic X");
}

#[test]
fn brainstorm_round_trips_with_its_target() {
    let dir = tempfile::tempdir().unwrap();
    let now = Utc::now();
    set_brainstorm(dir.path(), "p1", &epic("E1"), now).unwrap();
    assert_eq!(current(dir.path(), "p1"), ModeKind::Brainstorm);
    assert_eq!(brainstorm_target(dir.path(), "p1"), Some(epic("E1")));
    let json = state(dir.path(), "p1").api_json(Local::now());
    assert_eq!(json["mode"], "brainstorm");
    assert_eq!(json["target"], json!({"type": "epic", "id": "E1"}));
    let raw = std::fs::read_to_string(dir.path().join(INTERNAL_DIR).join(FILE_NAME)).unwrap();
    assert!(raw.contains("\"target\""), "{raw}");
}

#[test]
fn brainstorm_without_a_target_reads_as_sparring() {
    let dir = tempfile::tempdir().unwrap();
    let internal = dir.path().join(INTERNAL_DIR);
    std::fs::create_dir_all(&internal).unwrap();
    for entry in [
        r#"{"kind":"brainstorm"}"#,
        r#"{"kind":"brainstorm","target":{"type":"nope","id":"1"}}"#,
    ] {
        std::fs::write(
            internal.join(FILE_NAME),
            format!(r#"{{"projects":{{"p1":{entry}}}}}"#),
        )
        .unwrap();
        assert_eq!(current(dir.path(), "p1"), ModeKind::Sparring);
        assert_eq!(brainstorm_target(dir.path(), "p1"), None);
    }
}

#[test]
fn other_modes_and_auto_drop_the_target() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(INTERNAL_DIR).join(FILE_NAME);
    let now = Utc::now();
    set_brainstorm(dir.path(), "p1", &epic("E1"), now).unwrap();
    set(dir.path(), "p1", ModeKind::Sleep, now).unwrap();
    assert!(!std::fs::read_to_string(&file).unwrap().contains("target"));
    assert_eq!(brainstorm_target(dir.path(), "p1"), None);
    set_brainstorm(dir.path(), "p1", &epic("E1"), now).unwrap();
    clear_manual(dir.path(), "p1").unwrap();
    assert_eq!(brainstorm_target(dir.path(), "p1"), None);
    assert!(!std::fs::read_to_string(&file).unwrap().contains("target"));
}

#[test]
fn a_sleep_window_ends_a_brainstorm() {
    let state = State {
        manual: Some(ProjectMode {
            kind: ModeKind::Brainstorm,
            since: Some(Utc.with_ymd_and_hms(2026, 11, 24, 12, 0, 0).unwrap()),
        }),
        schedule: Schedule::from_json(&json!({"enabled": true, "start": "23:00", "end": "08:00"})),
        target: Some(epic("E1")),
    };
    let utc = |h, m| Utc.with_ymd_and_hms(2026, 11, 24, h, m, 0).unwrap();
    assert_eq!(state.target_at(utc(20, 0)), Some(epic("E1")));
    assert_eq!(state.effective_at(utc(23, 30)).kind, ModeKind::Sleep);
    assert_eq!(state.target_at(utc(23, 30)), None);
    assert!(state.api_json(utc(23, 30)).get("target").is_none());
}
