//! Brainstorm mode: `mode brainstorm --target`, the request it sends, `message ask --brainstorm`,
//! the progress table, and the refusals. Each test runs the real binary on a temporary data area.

mod common;

use common::{agpj, demo_project, json_error, json_ok};
use serde_json::{Value, json};

const QUESTION: &str = r#"{"questions":[{"body":"Who is the reader?"}]}"#;

fn demo(home: &std::path::Path) {
    demo_project(home);
    let (code, _, err) = agpj(
        home,
        &[
            "--project",
            "Demo",
            "epic",
            "add",
            "WALL",
            "--name",
            "Wall",
            "--actor",
            "human",
        ],
    );
    assert_eq!(code, 0, "{err}");
}

fn mode(home: &std::path::Path, args: &[&str]) -> Value {
    let mut all = vec!["--project", "Demo", "mode"];
    all.extend(args);
    all.push("--json");
    json_ok(home, &all)
}

fn ask(home: &std::path::Path, brainstorm: bool) -> (i32, String, String) {
    let mut args = vec![
        "--project",
        "Demo",
        "--json",
        "message",
        "ask",
        "--actor",
        "ai",
        "--no-wait",
        "--questions-json",
        QUESTION,
    ];
    if brainstorm {
        args.push("--brainstorm");
    }
    agpj(home, &args)
}

fn error_code(err: &str) -> String {
    let value: Value = serde_json::from_str(err).unwrap_or_else(|_| panic!("not JSON: {err}"));
    value["error"]["code"].as_str().unwrap().to_string()
}

/// Every non-null `brainstorm` value found anywhere in `value`.
fn brainstorms(value: &Value, found: &mut Vec<Value>) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map {
                if key == "brainstorm" && !inner.is_null() {
                    found.push(inner.clone());
                }
                brainstorms(inner, found);
            }
        }
        Value::Array(items) => items.iter().for_each(|item| brainstorms(item, found)),
        _ => {}
    }
}

#[test]
fn brainstorm_on_an_epic_shows_in_mode_and_progress_and_sends_a_request() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let shown = mode(home.path(), &["brainstorm", "--target", "wall"]);
    assert_eq!(shown["mode"]["kind"], "brainstorm");
    assert_eq!(
        shown["mode"]["target"],
        json!({"type": "epic", "id": "WALL"})
    );
    assert_eq!(
        mode(home.path(), &[])["mode"]["target"],
        json!({"type": "epic", "id": "WALL"})
    );
    let (code, out, _) = agpj(
        home.path(),
        &["--project", "Demo", "mode", "brainstorm", "--target", "1"],
    );
    assert_eq!(code, 0);
    assert!(out.contains("Mode: brainstorm on task #1."), "{out}");
    assert!(out.contains("agpj skills brainstorm"), "{out}");

    let progress = json_ok(home.path(), &["--project", "Demo", "progress", "--json"]);
    assert_eq!(progress["mode"], "brainstorm");
    assert_eq!(progress["modeTarget"], json!({"type": "task", "id": "1"}));

    let messages = json_ok(
        home.path(),
        &["--project", "Demo", "message", "ls", "--json"],
    );
    let text = messages.to_string();
    assert!(text.contains("Start brainstorming on epic WALL"), "{text}");
    assert!(text.contains("Start brainstorming on task #1"), "{text}");

    // Back to sparring: the target goes with it.
    let back = mode(home.path(), &["sparring"]);
    assert!(back["mode"].get("target").is_none());
    let progress = json_ok(home.path(), &["--project", "Demo", "progress", "--json"]);
    assert!(progress.get("modeTarget").is_none());
}

#[test]
fn the_target_is_validated_and_only_goes_with_brainstorm() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let run = |args: &[&str]| {
        json_error(
            home.path(),
            &[&["--project", "Demo", "mode"], args].concat(),
        )
    };
    assert_eq!(run(&["brainstorm", "--target", "99"]).1, "task_not_found");
    assert_eq!(run(&["brainstorm", "--target", "NOPE"]).1, "epic_not_found");
    assert_eq!(run(&["brainstorm"]).1, "invalid_value");
    assert_eq!(run(&["sleep", "--target", "1"]).1, "invalid_value");
    assert_eq!(run(&["sparring", "--target", "WALL"]).1, "invalid_value");
    // Nothing was saved.
    assert_eq!(mode(home.path(), &[])["mode"]["kind"], "sparring");
}

#[test]
fn brainstorm_questions_are_stored_with_the_target_and_refused_outside_brainstorm() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());

    // Sparring: a brainstorm question is refused, an ordinary one is accepted.
    let (code, _, err) = ask(home.path(), true);
    assert_ne!(code, 0);
    assert_eq!(error_code(&err), "brainstorm_not_active");
    assert_eq!(ask(home.path(), false).0, 0);

    mode(home.path(), &["brainstorm", "--target", "WALL"]);
    let (code, out, err) = ask(home.path(), true);
    assert_eq!(code, 0, "{err}");
    assert!(out.contains("registered"), "{out}");
    let messages = json_ok(
        home.path(),
        &["--project", "Demo", "message", "ls", "--json"],
    );
    let mut found = Vec::new();
    brainstorms(&messages, &mut found);
    assert_eq!(found, vec![json!({"type": "epic", "id": "WALL"})]);
    // A brainstorm question is not linked to a task.
    let shown = json_ok(home.path(), &["--project", "Demo", "show", "1", "--json"]);
    assert!(!shown.to_string().contains("Who is the reader?"));

    // An ordinary question is still accepted, and carries no target.
    assert_eq!(ask(home.path(), false).0, 0);
    let messages = json_ok(
        home.path(),
        &["--project", "Demo", "message", "ls", "--json"],
    );
    let mut found = Vec::new();
    brainstorms(&messages, &mut found);
    assert_eq!(found.len(), 1);

    // Sleep wins.
    mode(home.path(), &["sleep"]);
    let (_, _, err) = ask(home.path(), true);
    assert_eq!(error_code(&err), "sleep_mode");
    mode(home.path(), &["sparring"]);
    let (_, _, err) = ask(home.path(), true);
    assert_eq!(error_code(&err), "brainstorm_not_active");
}

#[test]
fn the_skill_topic_is_served() {
    let home = tempfile::tempdir().unwrap();
    let (code, out, _) = agpj(home.path(), &["skills", "brainstorm"]);
    assert_eq!(code, 0);
    assert!(out.contains("modeTarget"), "{out}");
}
