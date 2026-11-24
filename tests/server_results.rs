//! `GET /api/projects/{id}/tasks/{task}/results`: the recorded results of a task.
//! The server listens on port 0.

mod common;

use common::{Running, demo_project, json_ok};
use serde_json::json;

#[test]
fn results_route_lists_what_the_cli_recorded() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let pid = json_ok(home.path(), &["project", "ls", "--json"])["projects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let server = Running::start(home.path());

    let empty = server.get(&format!("/api/projects/{pid}/tasks/1/results"));
    assert_eq!(empty.status, 200);
    assert_eq!(empty.json()["results"], json!([]));

    json_ok(
        home.path(),
        &[
            "--project",
            "Demo",
            "result",
            "add",
            "1",
            "--summary",
            "Ready",
            "--link",
            "out/a.mp4",
            "--actor",
            "ai",
            "--json",
        ],
    );
    let listed = server
        .get(&format!("/api/projects/{pid}/tasks/1/results"))
        .json();
    assert_eq!(listed["taskId"], 1);
    assert_eq!(listed["results"][0]["version"], 1);
    assert_eq!(listed["results"][0]["summary"], "Ready");
    assert_eq!(listed["results"][0]["link"], "out/a.mp4");
    assert_eq!(listed["results"][0]["kind"], "progress");

    assert_eq!(
        server
            .get(&format!("/api/projects/{pid}/tasks/9/results"))
            .status,
        404
    );
    server.stop();
}

#[test]
fn results_route_returns_the_plans_with_their_delivered_state() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let file = home.path().join("desc.md");
    std::fs::write(
        &file,
        "---\nresultPlans:\n  - content: A screenshot\n    condition: When fixed\n  - content: The report\n    condition: When done\n    final: true\n---\n\nBody.\n",
    )
    .unwrap();
    let (code, _, err) = common::agpj(
        home.path(),
        &[
            "--project",
            "Demo",
            "add",
            "Planned",
            "--assignee",
            "ai",
            "--actor",
            "human",
            "--desc-file",
            file.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    let pid = json_ok(home.path(), &["project", "ls", "--json"])["projects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let task = json_ok(home.path(), &["--project", "Demo", "ls", "--json"])["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["id"].as_i64().unwrap())
        .max()
        .unwrap();
    let server = Running::start(home.path());
    let path = format!("/api/projects/{pid}/tasks/{task}/results");
    let before = server.get(&path).json();
    assert_eq!(before["resultPlans"].as_array().unwrap().len(), 2);
    assert_eq!(
        before["resultPlans"][0]["deliveredResultVersion"],
        serde_json::Value::Null
    );

    json_ok(
        home.path(),
        &[
            "--project",
            "Demo",
            "result",
            "add",
            &task.to_string(),
            "--plan",
            "RP-1",
            "--summary",
            "Fixed",
            "--actor",
            "ai",
            "--json",
        ],
    );
    let after = server.get(&path).json();
    assert_eq!(after["resultPlans"][0]["deliveredResultVersion"], 1);
    assert_eq!(after["results"][0]["planId"], "RP-1");
}
