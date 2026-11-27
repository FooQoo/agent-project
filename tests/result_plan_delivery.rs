//! Delivering a result against a result plan (`result add --plan`): the link to the plan, the
//! derived delivered state in `show`, idempotent retries, and the guidance that stays in place.
//! Runs the real CLI on a temporary data area.

mod common;
#[path = "support/connected_demo.rs"]
mod connected_demo;

use connected_demo::ConnectedDemo;

use common::agpj;
use serde_json::{Value, json};
use std::path::Path;

const PLANS: &str = "---\nacceptance:\n  - id: AC-1\n    text: The report is complete\n    done: false\nresultPlans:\n  - content: A screenshot of the layout\n    condition: When the layout is fixed\n  - content: The working endpoint\n    condition: When the endpoint answers its first request\n  - content: The finished report\n    condition: When every condition is met\n    final: true\n---\n\nBody.\n";

const FINAL_ONLY: &str = "---\nacceptance:\n  - id: AC-1\n    text: Done\n    done: true\nresultPlans:\n  - content: The finished report\n    condition: When every condition is met\n    final: true\n---\n\nBody.\n";

fn add_task(home: &Path, desc: &str) {
    let file = home.join("desc.md");
    std::fs::write(&file, desc).unwrap();
    let (code, _, err) = agpj(
        home,
        &[
            "--project",
            "Demo",
            "add",
            "Task",
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

fn run(home: &ConnectedDemo, rest: &[&str]) -> Value {
    home.json_ok(&[&["--project", "Demo"][..], rest, &["--json"]].concat())
}

fn deliver(home: &ConnectedDemo, task: &str, rest: &[&str]) -> Value {
    run(
        home,
        &[&["result", "add", task][..], rest, &["--actor", "ai"]].concat(),
    )
}

fn error_code(home: &ConnectedDemo, task: &str, rest: &[&str]) -> String {
    home.json_error(
        &[
            &["--project", "Demo", "result", "add", task][..],
            rest,
            &["--actor", "ai"],
        ]
        .concat(),
    )
    .1
}

fn delivered(home: &ConnectedDemo, task: &str) -> Value {
    let plans = run(home, &["show", task])["task"]["resultPlans"].clone();
    Value::Array(
        plans
            .as_array()
            .unwrap()
            .iter()
            .map(|p| json!([p["id"], p["deliveredResultVersion"]]))
            .collect(),
    )
}

fn history(home: &ConnectedDemo, task: &str) -> Vec<String> {
    run(home, &["show", task])["task"]["history"]
        .as_array()
        .unwrap()
        .iter()
        .map(|h| h["reason"].as_str().unwrap().to_owned())
        .collect()
}

fn setup() -> ConnectedDemo {
    let home = ConnectedDemo::new();
    add_task(home.path(), PLANS);
    home
}

// AC-1
#[test]
fn plans_are_delivered_in_any_order_without_retyping_the_plan() {
    let home = setup();
    let h = &home;
    assert_eq!(
        delivered(h, "2"),
        json!([["RP-1", null], ["RP-2", null], ["RP-3", null]])
    );

    let second = deliver(h, "2", &["--plan", "RP-2", "--summary", "Endpoint is live"]);
    assert_eq!(second["created"], true);
    assert_eq!(second["result"]["planId"], "RP-2");
    assert_eq!(second["result"]["kind"], "progress");
    // The title comes from the plan, the summary from this delivery.
    assert_eq!(second["result"]["title"], "The working endpoint");
    assert_eq!(second["result"]["summary"], "Endpoint is live");
    assert_eq!(
        delivered(h, "2"),
        json!([["RP-1", null], ["RP-2", 1], ["RP-3", null]])
    );

    let first = deliver(h, "2", &["--plan", "RP-1", "--summary", "Layout capture"]);
    assert_eq!(first["result"]["version"], 2);
    assert_eq!(
        delivered(h, "2"),
        json!([["RP-1", 2], ["RP-2", 1], ["RP-3", null]])
    );

    let notes = history(h, "2");
    assert!(
        notes
            .iter()
            .any(|n| n.starts_with("Recorded progress result v1 for plan RP-2:"))
    );
    assert!(
        notes
            .iter()
            .any(|n| n.starts_with("Recorded progress result v2 for plan RP-1:"))
    );
}

#[test]
fn the_plan_content_is_the_default_summary_and_a_final_plan_makes_the_result_final() {
    let home = setup();
    let h = &home;
    let result = deliver(h, "2", &["--plan", "RP-3"]);
    assert_eq!(result["result"]["kind"], "final");
    assert_eq!(result["result"]["summary"], "The finished report");
    assert_eq!(result["result"]["title"], "The finished report");
    // `--final` with the final plan says the same thing, as a retry of the same delivery.
    let again = deliver(h, "2", &["--plan", "RP-3", "--final"]);
    assert_eq!(again["created"], false);
    assert_eq!(again["result"]["version"], 1);
}

#[test]
fn final_marks_only_the_final_plan() {
    let home = setup();
    assert_eq!(
        error_code(&home, "2", &["--plan", "RP-1", "--summary", "S", "--final"]),
        "invalid_value"
    );
    assert_eq!(
        delivered(&home, "2"),
        json!([["RP-1", null], ["RP-2", null], ["RP-3", null]])
    );
}

// AC-2
#[test]
fn a_resend_changes_nothing_and_a_delivered_plan_takes_no_other_result() {
    let home = setup();
    let h = &home;
    let args = ["--plan", "RP-1", "--summary", "Layout capture"];
    let first = deliver(h, "2", &args);
    let before = history(h, "2");
    let again = deliver(h, "2", &args);
    assert_eq!(again["created"], false);
    assert_eq!(again["result"]["version"], first["result"]["version"]);
    assert_eq!(history(h, "2"), before);
    assert_eq!(
        run(h, &["result", "ls", "2"])["results"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // A different delivery against the delivered plan is refused and writes nothing.
    assert_eq!(
        error_code(h, "2", &["--plan", "RP-1", "--summary", "Another capture"]),
        "result_plan_delivered"
    );
    assert_eq!(history(h, "2"), before);

    // The same content without the plan is its own result.
    let unplanned = deliver(h, "2", &["--summary", "Layout capture"]);
    assert_eq!(unplanned["created"], true);
    assert!(unplanned["result"].get("planId").is_none());
}

#[test]
fn an_unknown_plan_or_a_plan_of_another_task_is_refused() {
    let home = setup();
    let h = &home;
    assert_eq!(
        error_code(h, "2", &["--plan", "RP-9", "--summary", "S"]),
        "result_plan_not_found"
    );
    // Task 1 has no plans: RP-1 belongs to task 2.
    assert_eq!(
        error_code(h, "1", &["--plan", "RP-1", "--summary", "S"]),
        "result_plan_not_found"
    );
    assert_eq!(run(h, &["result", "ls", "1"])["results"], json!([]));
    assert_eq!(run(h, &["result", "ls", "2"])["results"], json!([]));
}

#[test]
fn a_failed_save_leaves_the_plan_undelivered_and_a_retry_works() {
    let home = setup();
    let h = &home;
    // An image that is not an image is refused before anything is saved.
    let bad = h.path().join("bad.png");
    std::fs::write(&bad, b"not an image").unwrap();
    let bad = bad.to_str().unwrap();
    let (code, _, _) = h.raw(&[
        "--project",
        "Demo",
        "result",
        "add",
        "2",
        "--plan",
        "RP-1",
        "--summary",
        "S",
        "--file",
        bad,
        "--actor",
        "ai",
    ]);
    assert_ne!(code, 0);
    assert_eq!(
        delivered(h, "2"),
        json!([["RP-1", null], ["RP-2", null], ["RP-3", null]])
    );
    let retry = deliver(h, "2", &["--plan", "RP-1", "--summary", "S"]);
    assert_eq!(retry["created"], true);
}

#[test]
fn a_publication_record_cannot_deliver_a_plan() {
    let home = setup();
    assert_eq!(
        error_code(
            &home,
            "2",
            &[
                "--plan",
                "RP-1",
                "--published",
                "v1.0.0",
                "--link",
                "https://example.com",
                "--summary",
                "S"
            ]
        ),
        // The CLI refuses the combination before it reaches the store.
        "invalid_argument"
    );
}

// AC-3
#[test]
fn an_unplanned_result_keeps_working_and_leaves_every_plan_undelivered() {
    let home = setup();
    let h = &home;
    let result = deliver(h, "2", &["--summary", "Something unplanned"]);
    assert_eq!(result["created"], true);
    assert!(result["result"].get("planId").is_none());
    assert_eq!(
        delivered(h, "2"),
        json!([["RP-1", null], ["RP-2", null], ["RP-3", null]])
    );
    // Work continues with plans undelivered: the task moves and the run state changes.
    run(
        h,
        &["mv", "2", "doing", "--actor", "ai", "--reason", "Starting."],
    );
    assert_eq!(run(h, &["show", "2"])["task"]["taskStatus"], "doing");
}

#[test]
fn a_task_with_only_a_final_plan_delivers_it_and_clears_the_missing_final_result() {
    let home = ConnectedDemo::new();
    let h = &home;
    add_task(h.path(), FINAL_ONLY);
    let listed = run(h, &["progress"])["finalResultMissing"].clone();
    assert!(
        listed.as_array().unwrap().iter().any(|t| t["id"] == 2),
        "{listed}"
    );
    // An intermediate result against nothing does not clear it.
    deliver(h, "2", &["--summary", "Half"]);
    assert!(
        run(h, &["progress"])["finalResultMissing"]
            .as_array()
            .unwrap()
            .iter()
            .any(|t| t["id"] == 2)
    );

    let result = deliver(h, "2", &["--plan", "RP-1", "--summary", "Shipped"]);
    assert_eq!(result["result"]["kind"], "final");
    assert_eq!(delivered(h, "2"), json!([["RP-1", 2]]));
    assert!(
        !run(h, &["progress"])["finalResultMissing"]
            .as_array()
            .map(|a| a.iter().any(|t| t["id"] == 2))
            .unwrap_or(false)
    );
}

#[test]
fn undelivered_intermediate_plans_block_nothing() {
    let home = ConnectedDemo::new();
    let h = &home;
    add_task(
        h.path(),
        &FINAL_ONLY.replace(
            "resultPlans:\n",
            "resultPlans:\n  - content: A first look\n    condition: Early\n",
        ),
    );
    deliver(h, "2", &["--plan", "RP-2", "--summary", "Shipped"]);
    // The final plan is delivered while RP-1 is not, and the task can still finish.
    assert_eq!(delivered(h, "2"), json!([["RP-1", null], ["RP-2", 1]]));
    run(
        h,
        &["mv", "2", "done", "--actor", "ai", "--reason", "All met."],
    );
    assert_eq!(run(h, &["show", "2"])["task"]["taskStatus"], "done");
}
