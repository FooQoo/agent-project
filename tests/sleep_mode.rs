mod common;

use common::{agpj, demo_project, json_error, json_ok};

const QUESTION: &str = r#"{"questions":[{"body":"Keep or delete?"}]}"#;

fn ask(home: &std::path::Path) -> (i32, String) {
    json_error(
        home,
        &[
            "--project",
            "Demo",
            "message",
            "ask",
            "--actor",
            "ai",
            "--task",
            "1",
            "--no-wait",
            "--questions-json",
            QUESTION,
        ],
    )
}

#[test]
fn sleep_mode_refuses_questions_and_shows_in_progress_until_sparring_returns() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let shown = json_ok(home.path(), &["--project", "Demo", "mode", "--json"]);
    assert_eq!(shown["mode"]["kind"], "sparring");
    let progress = json_ok(home.path(), &["--project", "Demo", "progress", "--json"]);
    assert!(progress.get("mode").is_none());

    let slept = json_ok(
        home.path(),
        &["--project", "Demo", "mode", "sleep", "--json"],
    );
    assert_eq!(slept["mode"]["kind"], "sleep");
    assert!(slept["mode"]["since"].as_str().unwrap().ends_with('Z'));
    let progress = json_ok(home.path(), &["--project", "Demo", "progress", "--json"]);
    assert_eq!(progress["mode"], "sleep");

    let (_, code) = ask(home.path());
    assert_eq!(code, "sleep_mode");
    let questions = json_ok(
        home.path(),
        &["--project", "Demo", "message", "ls", "--json"],
    );
    assert!(questions.to_string().find("Keep or delete?").is_none());

    json_ok(
        home.path(),
        &["--project", "Demo", "mode", "sparring", "--json"],
    );
    let progress = json_ok(home.path(), &["--project", "Demo", "progress", "--json"]);
    assert!(progress.get("mode").is_none());
    let (code, _out, err) = agpj(
        home.path(),
        &[
            "--project",
            "Demo",
            "message",
            "ask",
            "--actor",
            "ai",
            "--task",
            "1",
            "--no-wait",
            "--questions-json",
            QUESTION,
            "--json",
        ],
    );
    assert_eq!(code, 0, "{err}");
}

#[test]
fn the_mode_is_per_project() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let (code, _, err) = agpj(home.path(), &["project", "create", "Other"]);
    assert_eq!(code, 0, "{err}");
    json_ok(
        home.path(),
        &["--project", "Demo", "mode", "sleep", "--json"],
    );
    let other = json_ok(home.path(), &["--project", "Other", "mode", "--json"]);
    assert_eq!(other["mode"]["kind"], "sparring");
}

#[test]
fn a_decision_is_one_history_entry_with_who_what_why_and_basis() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let recorded = json_ok(
        home.path(),
        &[
            "--project",
            "Demo",
            "decision",
            "add",
            "1",
            "--question",
            "Delete or keep the old rows?",
            "--choice",
            "Keep and migrate",
            "--reason",
            "Deleting cannot be undone.",
            "--decided-by",
            "subagent:reviewer",
            "--basis",
            "src/file_store/ops/task.rs",
            "--basis",
            "Q-12",
            "--json",
        ],
    );
    let history = recorded["task"]["history"].as_array().unwrap();
    let reason = history.last().unwrap()["reason"].as_str().unwrap();
    for part in [
        "subagent:reviewer",
        "Delete or keep the old rows?",
        "Keep and migrate",
        "Deleting cannot be undone.",
        "src/file_store/ops/task.rs; Q-12",
    ] {
        assert!(reason.contains(part), "{reason}");
    }
    for bad in [
        vec!["--decided-by", "someone", "--basis", "Q-1"],
        vec!["--decided-by", "subagent:x"],
    ] {
        let args = [
            vec![
                "--project",
                "Demo",
                "decision",
                "add",
                "1",
                "--question",
                "q",
                "--choice",
                "c",
                "--reason",
                "r",
            ],
            bad,
        ]
        .concat();
        let (code, _) = json_error(home.path(), &args);
        assert_ne!(code, 0);
    }
}
