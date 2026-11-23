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
        r#"{"projects":{"p1":{"kind":"brainstorm"}}}"#,
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
