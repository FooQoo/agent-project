//! `finalResultMissing` of the progress table, and the late final result on a Done task.
//! Runs the real CLI on a temporary data area.

mod common;
#[path = "support/connected_demo.rs"]
mod connected_demo;

use connected_demo::ConnectedDemo;

use std::path::PathBuf;

use serde_json::{Value, json};

const P: [&str; 2] = ["--project", "Demo"];

fn run(home: &ConnectedDemo, rest: &[&str]) -> Value {
    home.json_ok(&[&P[..], rest, &["--json"]].concat())
}

fn missing(home: &ConnectedDemo) -> Value {
    run(home, &["progress"])["finalResultMissing"].clone()
}

fn task_md(home: &ConnectedDemo) -> PathBuf {
    let table = run(home, &["progress"]);
    PathBuf::from(table["unassigned"][0]["taskPath"].as_str().unwrap())
}

/// Give task 1 conditions, all met or not.
fn set_conditions(home: &ConnectedDemo, all_done: bool) {
    let text = format!(
        "---\nacceptance:\n  - id: AC-1\n    text: One.\n    done: {all_done}\nhistory: []\n---\n\nBody\n"
    );
    std::fs::write(task_md(home), text).unwrap();
}

fn final_result(home: &ConnectedDemo) -> Value {
    run(
        home,
        &[
            "result",
            "add",
            "1",
            "--summary",
            "Shipped",
            "--final",
            "--actor",
            "ai",
        ],
    )
}

#[test]
fn the_field_is_absent_until_every_condition_is_met_and_clears_with_a_final_result() {
    let home = ConnectedDemo::new();
    set_conditions(&home, false);
    assert_eq!(missing(&home), Value::Null);

    set_conditions(&home, true);
    let listed = missing(&home);
    assert_eq!(listed[0]["id"], 1);
    assert_eq!(listed[0]["title"], "Implement sharing");
    assert!(listed[0]["taskPath"].as_str().unwrap().ends_with("TASK.md"));

    // A progress result does not clear it.
    run(
        &home,
        &["result", "add", "1", "--summary", "Half", "--actor", "ai"],
    );
    assert_ne!(missing(&home), Value::Null);

    final_result(&home);
    assert_eq!(missing(&home), Value::Null);
}

#[test]
fn a_done_task_accepts_one_late_final_result_and_nothing_else() {
    let home = ConnectedDemo::new();
    set_conditions(&home, true);
    run(
        &home,
        &["mv", "1", "done", "--actor", "ai", "--reason", "All met."],
    );
    // Done tasks leave the table but are still listed in the field.
    let table = run(&home, &["progress"]);
    assert_eq!(table["finalResultMissing"][0]["id"], 1);

    let (_, code) = home.json_error(&[
        "--project",
        "Demo",
        "result",
        "add",
        "1",
        "--summary",
        "Late",
        "--actor",
        "ai",
    ]);
    assert_eq!(code, "task_is_done");

    let recorded = final_result(&home);
    assert_eq!(recorded["created"], true);
    assert_eq!(missing(&home), Value::Null);
    assert_eq!(
        run(&home, &["show", "1"])["task"]["taskStatus"],
        json!("done")
    );

    // A second, different final result is refused on the Done task.
    let (_, code) = home.json_error(&[
        "--project",
        "Demo",
        "result",
        "add",
        "1",
        "--summary",
        "Again",
        "--final",
        "--actor",
        "ai",
    ]);
    assert_eq!(code, "task_is_done");
}
