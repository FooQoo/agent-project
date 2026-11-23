mod common;

use common::{agpj, demo_project, json_error, json_ok};

const QUESTION: &str = r#"{"questions":[{"body":"Keep or delete?"}]}"#;

/// Asks during sleep: the question is held, not refused.
fn ask_held(home: &std::path::Path) -> String {
    let held = json_ok(
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
            "--json",
        ],
    );
    assert_eq!(held["type"], "held", "{held}");
    held["questions"][0]["id"].as_str().unwrap().to_string()
}

#[test]
fn sleep_mode_holds_questions_and_shows_in_progress_until_sparring_returns() {
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

    let id = ask_held(home.path());
    let questions = json_ok(
        home.path(),
        &[
            "--project",
            "Demo",
            "message",
            "ls",
            "--status",
            "all",
            "--json",
        ],
    );
    assert_eq!(questions["messages"][0]["id"], id);
    assert_eq!(questions["messages"][0]["status"], "held");
    // The task keeps going: a held question does not make it wait for the user.
    let task = json_ok(home.path(), &["--project", "Demo", "show", "1", "--json"]);
    assert_eq!(task["task"]["awaitingAnswer"], false, "{task}");

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

#[test]
fn a_daily_window_sleeps_by_the_schedule_and_a_manual_mode_overrides_it() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    // The test helper runs the binary with TZ=UTC, so the local time is UTC.
    let now = chrono::Utc::now();
    let hour = chrono::Duration::hours(1);
    let start = (now - hour).format("%H:%M").to_string();
    let end = (now + hour).format("%H:%M").to_string();
    let run = |args: &[&str]| {
        let mut all = vec!["--project", "Demo"];
        all.extend_from_slice(args);
        all.push("--json");
        json_ok(home.path(), &all)
    };

    let scheduled = run(&["mode", "schedule", "--start", &start, "--end", &end]);
    assert_eq!(scheduled["mode"]["kind"], "sleep");
    assert_eq!(scheduled["mode"]["source"], "schedule");
    assert!(scheduled["mode"]["until"].as_str().unwrap().ends_with('Z'));
    assert_eq!(scheduled["schedule"]["start"], start);

    let progress = run(&["progress"]);
    assert_eq!(progress["mode"], "sleep");
    assert_eq!(progress["modeSource"], "schedule");
    assert!(progress["modeUntil"].is_string());
    ask_held(home.path());

    // Sparring set by hand inside the window holds; `auto` gives the window back.
    assert_eq!(run(&["mode", "sparring"])["mode"]["kind"], "sparring");
    assert!(run(&["progress"]).get("mode").is_none());
    assert_eq!(run(&["mode", "auto"])["mode"]["source"], "schedule");

    // `--off` keeps the times; a bad time or a missing one is refused.
    let off = run(&["mode", "schedule", "--off"]);
    assert_eq!(off["mode"]["kind"], "sparring");
    assert_eq!(off["schedule"]["enabled"], false);
    let (_, code) = json_error(
        home.path(),
        &[
            "--project",
            "Demo",
            "mode",
            "schedule",
            "--start",
            "8:00",
            "--json",
        ],
    );
    assert_eq!(code, "invalid_value");
    let (_, code) = json_error(
        home.path(),
        &["--project", "Demo", "mode", "sleep", "--off", "--json"],
    );
    assert_eq!(code, "invalid_value");
}
