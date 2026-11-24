//! `POST /api/projects/{id}/tasks/{task}/results/multipart`: delivering a result plan from the
//! board with images, and the plans the results route returns. The server listens on port 0.

mod common;

use common::{Running, agpj, demo_project, json_ok, multipart, png};
use serde_json::{Value, json};
use std::path::Path;

const PLANS: &str = "---\nacceptance:\n  - id: AC-1\n    text: Done\n    done: false\nresultPlans:\n  - content: A screenshot of the layout\n    condition: When the layout is fixed\n  - content: The finished report\n    condition: When every condition is met\n    final: true\n---\n\nBody.\n";

fn add_task(home: &Path) {
    let file = home.join("desc.md");
    std::fs::write(&file, PLANS).unwrap();
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

fn project_id(home: &Path) -> String {
    json_ok(home, &["project", "ls", "--json"])["projects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string()
}

#[test]
fn a_plan_is_delivered_with_an_image_and_shows_as_delivered() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    add_task(home.path());
    let pid = project_id(home.path());
    let server = Running::start(home.path());
    let task = json_ok(home.path(), &["--project", "Demo", "ls", "--json"])["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_i64().unwrap())
        .max()
        .unwrap();
    let path = format!("/api/projects/{pid}/tasks/{task}/results");

    let before = server.get(&path).json();
    let plans = before["resultPlans"].as_array().unwrap();
    assert_eq!(plans.len(), 2);
    assert_eq!(plans[0]["deliveredResultVersion"], Value::Null);

    let meta = json!({
        "plan": "RP-1",
        "summary": "Layout is fixed",
        "link": "out/layout.png",
        "attachments": [{"id": "0123456789abcdef0123456789abcdef", "name": "layout.png"}],
    })
    .to_string();
    let body = multipart(
        "b",
        &[("metadata", meta.as_str())],
        &[("files", "layout.png", &png())],
    );
    let reply = server.write_multipart("POST", &format!("{path}/multipart"), body, "b");
    assert_eq!(reply.status, 201, "{}", reply.body);
    let made = reply.json();
    assert_eq!(made["result"]["planId"], "RP-1");
    assert_eq!(
        made["result"]["attachmentId"],
        "0123456789abcdef0123456789abcdef"
    );

    let after = server.get(&path).json();
    assert_eq!(after["resultPlans"][0]["deliveredResultVersion"], 1);
    assert_eq!(
        after["resultPlans"][1]["deliveredResultVersion"],
        Value::Null
    );

    // The same plan cannot be delivered twice.
    let meta = json!({"plan": "RP-1", "summary": "Again"}).to_string();
    let body = multipart("b", &[("metadata", meta.as_str())], &[]);
    let again = server.write_multipart("POST", &format!("{path}/multipart"), body, "b");
    assert_eq!(again.status, 409);
    assert!(again.body.contains("result_plan_delivered"));
}

#[test]
fn an_image_part_without_its_metadata_entry_is_refused() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    add_task(home.path());
    let pid = project_id(home.path());
    let server = Running::start(home.path());
    let meta = json!({"summary": "S"}).to_string();
    let body = multipart(
        "b",
        &[("metadata", meta.as_str())],
        &[("files", "a.png", &png())],
    );
    let reply = server.write_multipart(
        "POST",
        &format!("/api/projects/{pid}/tasks/1/results/multipart"),
        body,
        "b",
    );
    assert_eq!(reply.status, 422);
}

#[test]
fn a_person_can_deliver_a_plan_of_a_done_task_from_the_board() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    add_task(home.path());
    let pid = project_id(home.path());
    let task = json_ok(home.path(), &["--project", "Demo", "ls", "--json"])["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_i64().unwrap())
        .max()
        .unwrap()
        .to_string();
    for step in [
        &["start", task.as_str()][..],
        &["mv", task.as_str(), "done", "--reason", "Finish"],
    ] {
        let mut args = vec!["--project", "Demo"];
        args.extend_from_slice(step);
        args.extend_from_slice(&["--actor", "ai"]);
        let _ = agpj(home.path(), &args);
    }
    let server = Running::start(home.path());
    let meta = json!({"plan": "RP-1", "summary": "Late screenshot"}).to_string();
    let body = multipart("b", &[("metadata", meta.as_str())], &[]);
    let reply = server.write_multipart(
        "POST",
        &format!("/api/projects/{pid}/tasks/{task}/results/multipart"),
        body,
        "b",
    );
    assert_eq!(reply.status, 201, "{}", reply.body);
}
