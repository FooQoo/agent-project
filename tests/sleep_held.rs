//! Questions asked during sleep mode are held, and released all at once when sleep ends: by a
//! manual switch (CLI and board) and by the end of the daily window.

mod common;
#[path = "support/connected_demo.rs"]
mod connected_demo;

use connected_demo::ConnectedDemo;

use common::Running;
use serde_json::{Value, json};

fn run(home: &ConnectedDemo, args: &[&str]) -> Value {
    let mut all = vec!["--project", "Demo"];
    all.extend_from_slice(args);
    all.push("--json");
    home.json_ok(&all)
}

fn ask(home: &ConnectedDemo, body: &str, wait: bool) -> Value {
    let questions = json!({"questions": [{"body": body}]}).to_string();
    let mut args = vec![
        "message",
        "ask",
        "--actor",
        "ai",
        "--task",
        "1",
        "--questions-json",
        &questions,
    ];
    if !wait {
        args.push("--no-wait");
    }
    run(home, &args)
}

fn statuses(home: &ConnectedDemo) -> Vec<(String, String)> {
    let listed = run(home, &["message", "ls", "--status", "all"]);
    listed["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["id"].as_str().unwrap().to_string(),
                m["status"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn awaiting(home: &ConnectedDemo) -> bool {
    run(home, &["show", "1"])["task"]["awaitingAnswer"] == true
}

#[test]
fn ac1_an_ask_during_sleep_is_held_and_returns_at_once_even_when_it_would_wait() {
    let home = ConnectedDemo::new();
    run(&home, &["mode", "sleep"]);
    // No --no-wait and no server: a held ask never waits, so it needs neither.
    let first = ask(&home, "Keep the old rows?", true);
    assert_eq!(first, json!({"type": "held", "questions": [{"id": "Q-1"}]}));
    let second = ask(&home, "Which name?", false);
    assert_eq!(second["questions"][0]["id"], "Q-2");
    assert_eq!(
        statuses(&home),
        vec![("Q-1".into(), "held".into()), ("Q-2".into(), "held".into())]
    );
    // Nothing is open for the person, and the task is not waiting.
    let open = run(&home, &["message", "ls"]);
    assert_eq!(open["messages"].as_array().unwrap().len(), 0, "{open}");
    assert!(!awaiting(&home));
    // A wait on a held question returns at once too.
    let waited = run(&home, &["message", "wait", "Q-1"]);
    assert_eq!(waited["type"], "held");
}

#[test]
fn ac2_leaving_sleep_by_hand_releases_every_held_question_in_order() {
    let home = ConnectedDemo::new();
    run(&home, &["mode", "sleep"]);
    ask(&home, "First?", false);
    ask(&home, "Second?", false);
    let back = run(&home, &["mode", "sparring"]);
    assert_eq!(back["mode"]["released"], json!(["Q-1", "Q-2"]));
    assert_eq!(
        statuses(&home),
        vec![("Q-1".into(), "open".into()), ("Q-2".into(), "open".into())]
    );
    // Still tied to the task, which now waits like after an ordinary ask.
    assert!(awaiting(&home));
    let shown = run(&home, &["message", "show", "Q-2"]);
    assert_eq!(shown["question"]["referencedBy"], json!([1]));
    // Released once: another switch finds nothing more.
    let again = run(&home, &["mode", "sparring"]);
    assert!(again["mode"].get("released").is_none());
}

#[test]
fn ac3_a_withdrawn_held_question_is_not_delivered() {
    let home = ConnectedDemo::new();
    run(&home, &["mode", "sleep"]);
    ask(&home, "Not needed any more?", false);
    ask(&home, "Still needed?", false);
    run(&home, &["message", "withdraw", "Q-1", "--actor", "ai"]);
    let back = run(&home, &["mode", "sparring"]);
    assert_eq!(back["mode"]["released"], json!(["Q-2"]));
    assert_eq!(
        statuses(&home),
        vec![
            ("Q-1".into(), "withdrawn".into()),
            ("Q-2".into(), "open".into())
        ]
    );
}

/// A window around now, as `HH:MM` times in the machine's zone (the test binary runs with
/// `TZ=UTC`).
fn window(around: bool) -> (String, String) {
    let now = chrono::Utc::now();
    let hour = chrono::Duration::hours(1);
    if around {
        (
            (now - hour).format("%H:%M").to_string(),
            (now + hour).format("%H:%M").to_string(),
        )
    } else {
        (
            (now + hour * 2).format("%H:%M").to_string(),
            (now + hour * 3).format("%H:%M").to_string(),
        )
    }
}

#[test]
fn ac2_the_end_of_the_daily_window_releases_the_held_questions_when_next_observed() {
    let home = ConnectedDemo::new();
    let (start, end) = window(true);
    let scheduled = run(
        &home,
        &["mode", "schedule", "--start", &start, "--end", &end],
    );
    assert_eq!(scheduled["mode"]["kind"], "sleep");
    ask(&home, "First?", false);
    ask(&home, "Second?", false);
    assert!(!awaiting(&home));

    // The window ends with no command: the saved window simply moves away from now.
    let path = home.path().join(".internal").join("modes.json");
    let mut file: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let (later_start, later_end) = window(false);
    let entry = file["projects"]
        .as_object_mut()
        .unwrap()
        .values_mut()
        .next()
        .unwrap();
    entry["schedule"]["start"] = json!(later_start);
    entry["schedule"]["end"] = json!(later_end);
    std::fs::write(&path, serde_json::to_vec(&file).unwrap()).unwrap();

    // The first look afterwards (here the progress table) delivers them, once.
    let progress = run(&home, &["progress"]);
    assert!(progress.get("mode").is_none());
    assert_eq!(
        statuses(&home),
        vec![("Q-1".into(), "open".into()), ("Q-2".into(), "open".into())]
    );
    assert!(awaiting(&home));
}

#[test]
fn the_board_hides_held_questions_counts_them_and_releases_them_on_a_switch() {
    let home = ConnectedDemo::new();
    let id = std::fs::read_dir(home.path().join("shared").join("projects"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name()
        .to_string_lossy()
        .into_owned();
    let server = Running::start(home.path());
    let mode_path = format!("/api/projects/{id}/mode");
    let put = |body: Value| server.write("PUT", &mode_path, Some(&body.to_string()));
    assert_eq!(put(json!({"mode": "sleep"})).status, 200);
    ask(&home, "Keep the old rows?", false);
    ask(&home, "Which name?", false);

    let mode = server.get(&mode_path).json();
    assert_eq!(mode["heldCount"], 2, "{mode}");
    let all = server
        .get(&format!("/api/projects/{id}/messages?status=all"))
        .json();
    assert_eq!(all["messages"].as_array().unwrap().len(), 0, "{all}");
    assert_eq!(
        server
            .get(&format!("/api/projects/{id}/messages/Q-1"))
            .status,
        404
    );

    let back = put(json!({"mode": "sparring"}));
    assert_eq!(back.status, 200, "{}", back.body);
    assert!(back.json().get("heldCount").is_none());
    let open = server.get(&format!("/api/projects/{id}/messages")).json();
    let ids: Vec<&str> = open["messages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| m["id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 2, "{open}");
    server.stop();
}
