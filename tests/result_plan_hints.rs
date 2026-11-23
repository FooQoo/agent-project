//! The undelivered result plans an agent sees when it starts or resumes a task: the progress
//! table (CLI and hook) lists them on the task entry, hides the delivered ones, and leaves the
//! field out for a task with no plans. Runs the real CLI on a temporary data area.

mod common;

use common::{agpj, demo_project, json_ok};
use serde_json::{Value, json};
use std::io::Cursor;
use std::path::Path;

const PLANS: &str = "---\nacceptance:\n  - id: AC-1\n    text: The report is complete\n    done: false\nresultPlans:\n  - content: A screenshot of the layout\n    condition: When the layout is fixed\n  - content: The working endpoint\n    condition: When the endpoint answers its first request\n  - content: The finished report\n    condition: When every condition is met\n    final: true\n---\n\nBody.\n";

/// Demo task #1 has no plans; task #2 has three (RP-1, RP-2, and the final RP-3).
fn board_with_plans(home: &Path) {
    demo_project(home);
    let file = home.join("desc.md");
    std::fs::write(&file, PLANS).unwrap();
    let (code, _, err) = agpj(
        home,
        &[
            "--project",
            "Demo",
            "add",
            "Planned task",
            "--assignee",
            "ai",
            "--actor",
            "human",
            "--desc-file",
            file.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{err}");
}

fn task_entry(table: &Value, id: i64) -> Value {
    table["epics"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|epic| epic["tasks"].as_array().unwrap().iter())
        .chain(table["unassigned"].as_array().unwrap().iter())
        .find(|task| task["id"] == id)
        .unwrap_or_else(|| panic!("task {id} is not in {table}"))
        .clone()
}

fn progress(home: &Path) -> Value {
    json_ok(home, &["--project", "Demo", "progress", "--json"])
}

fn plan_ids(entry: &Value) -> Vec<String> {
    entry["resultPlans"]
        .as_array()
        .map(|plans| {
            plans
                .iter()
                .map(|p| p["id"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

fn deliver(home: &Path, plan: &str) {
    let (code, _, err) = agpj(
        home,
        &[
            "--project",
            "Demo",
            "result",
            "add",
            "2",
            "--plan",
            plan,
            "--actor",
            "ai",
        ],
    );
    assert_eq!(code, 0, "{err}");
}

#[test]
fn progress_shows_the_undelivered_plans_with_their_conditions() {
    let home = tempfile::tempdir().unwrap();
    board_with_plans(home.path());
    let entry = task_entry(&progress(home.path()), 2);
    assert_eq!(
        entry["resultPlans"],
        json!([
            {"id": "RP-1", "content": "A screenshot of the layout", "condition": "When the layout is fixed", "final": false},
            {"id": "RP-2", "content": "The working endpoint", "condition": "When the endpoint answers its first request", "final": false},
            {"id": "RP-3", "content": "The finished report", "condition": "When every condition is met", "final": true},
        ])
    );
}

#[test]
fn progress_hides_a_delivered_plan_and_keeps_the_rest_across_calls() {
    let home = tempfile::tempdir().unwrap();
    board_with_plans(home.path());
    deliver(home.path(), "RP-1");
    // Each call is a fresh process, like a resume from another session.
    for _ in 0..2 {
        let entry = task_entry(&progress(home.path()), 2);
        assert_eq!(plan_ids(&entry), ["RP-2", "RP-3"]);
    }
    deliver(home.path(), "RP-3");
    assert_eq!(plan_ids(&task_entry(&progress(home.path()), 2)), ["RP-2"]);
    deliver(home.path(), "RP-2");
    let entry = task_entry(&progress(home.path()), 2);
    assert!(entry.get("resultPlans").is_none(), "{entry}");
}

#[test]
fn the_field_is_absent_for_a_task_with_no_plans() {
    let home = tempfile::tempdir().unwrap();
    board_with_plans(home.path());
    let entry = task_entry(&progress(home.path()), 1);
    assert!(entry.get("resultPlans").is_none(), "{entry}");
}

#[test]
fn the_hook_carries_the_undelivered_plans() {
    let home = tempfile::tempdir().unwrap();
    board_with_plans(home.path());
    deliver(home.path(), "RP-2");
    let out = agent_project::agent::hook::run(
        "claude",
        &mut Cursor::new(r#"{"hook_event_name":"SessionStart"}"#),
        &home.path().join("shared"),
        Some("Demo"),
    )
    .unwrap();
    let parsed: Value = serde_json::from_str(&out).unwrap();
    let context = parsed["hookSpecificOutput"]["additionalContext"]
        .as_str()
        .unwrap();
    let table: Value =
        serde_json::from_str(context.strip_prefix("AGPJ_PROGRESS\n").unwrap()).unwrap();
    let entry = task_entry(&table, 2);
    assert_eq!(plan_ids(&entry), ["RP-1", "RP-3"]);
    assert_eq!(entry["resultPlans"][1]["final"], true);
    assert!(task_entry(&table, 1).get("resultPlans").is_none());
}

#[test]
fn the_text_table_lists_what_to_deliver() {
    let home = tempfile::tempdir().unwrap();
    board_with_plans(home.path());
    let (code, out, err) = agpj(home.path(), &["--project", "Demo", "progress"]);
    assert_eq!(code, 0, "{err}");
    assert!(
        out.contains("undelivered RP-2: The working endpoint - deliver when: When the endpoint answers its first request"),
        "{out}"
    );
    assert!(out.contains("undelivered RP-3 (final)"), "{out}");
}

#[test]
fn a_done_or_human_task_shows_no_plans() {
    let home = tempfile::tempdir().unwrap();
    board_with_plans(home.path());
    let file = home.path().join("human.md");
    std::fs::write(&file, PLANS).unwrap();
    let (code, _, err) = agpj(
        home.path(),
        &[
            "--project",
            "Demo",
            "add",
            "Human task",
            "--assignee",
            "human",
            "--actor",
            "human",
            "--desc-file",
            file.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    let entry = task_entry(&progress(home.path()), 3);
    assert!(entry.get("resultPlans").is_none(), "{entry}");
}
