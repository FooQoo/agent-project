//! A publication record is the one result a Done task still takes, from the CLI and the web API.

mod common;

use std::fs;
use std::path::Path;

use common::{Running, demo_project, json_error, json_ok};
use serde_json::{Value, json};

const URL: &str = "https://example.com/releases/v1";

fn run(home: &Path, rest: &[&str]) -> Value {
    json_ok(
        home,
        &[&["--project", "Demo"][..], rest, &["--json"]].concat(),
    )
}

fn fail(home: &Path, rest: &[&str]) -> String {
    json_error(home, &[&["--project", "Demo"][..], rest].concat()).1
}

/// Task #1 planned for `v1`, with a final result, moved to Done.
fn done_task(home: &Path) {
    demo_project(home);
    run(home, &["version", "add", "v1"]);
    run(home, &["edit", "1", "--planned-version", "v1"]);
    run(
        home,
        &[
            "result",
            "add",
            "1",
            "--summary",
            "Finished",
            "--final",
            "--actor",
            "ai",
        ],
    );
    run(
        home,
        &["mv", "1", "done", "--actor", "ai", "--reason", "Finish"],
    );
}

fn publish(home: &Path, actor: &str, extra: &[&str]) -> Value {
    run(
        home,
        &[
            &[
                "result",
                "add",
                "1",
                "--published",
                "v1",
                "--summary",
                "Released",
                "--actor",
                actor,
            ][..],
            extra,
        ]
        .concat(),
    )
}

#[test]
fn ai_and_human_publication_on_a_done_card_with_a_final_result_is_counted() {
    let home = tempfile::tempdir().unwrap();
    done_task(home.path());
    let before = run(home.path(), &["release", "doc", "--version", "v1"]);
    assert_eq!(before["published"], 0);

    let ai = publish(home.path(), "ai", &["--link", URL]);
    assert_eq!(ai["created"], true);
    assert_eq!(ai["result"]["kind"], "progress");
    assert!(
        ai["result"]["summary"]
            .as_str()
            .unwrap()
            .starts_with("Published: v1\n")
    );
    let human = publish(
        home.path(),
        "human",
        &["--link", "https://example.com/other", "--key", "second"],
    );
    assert_eq!(human["created"], true);

    let task = &run(home.path(), &["show", "1"])["task"];
    assert_eq!(task["taskStatus"], "done");
    assert_eq!(task["results"].as_array().unwrap().len(), 3);

    let after = run(home.path(), &["release", "doc", "--version", "v1"]);
    assert_eq!(after["published"], 1);
    let dir = fs::read_dir(home.path().join("shared/projects"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let text = fs::read_to_string(dir.join("docs/release/v1.md")).unwrap();
    assert!(text.contains("https://example.com/other"));
}

#[test]
fn a_publication_needs_a_link_and_a_valid_version_and_is_not_final() {
    let home = tempfile::tempdir().unwrap();
    done_task(home.path());
    // clap requires the link, so the CLI refuses before the store is reached.
    let (code, _, _) = common::agboard(
        home.path(),
        &[
            "--project",
            "Demo",
            "result",
            "add",
            "1",
            "--published",
            "v1",
            "--summary",
            "x",
            "--actor",
            "ai",
        ],
    );
    assert_ne!(code, 0);
    assert_eq!(
        fail(
            home.path(),
            &[
                "result",
                "add",
                "1",
                "--published",
                "a/b",
                "--link",
                URL,
                "--summary",
                "x",
                "--actor",
                "ai"
            ]
        ),
        "invalid_value"
    );
    let (code, _, _) = common::agboard(
        home.path(),
        &[
            "--project",
            "Demo",
            "result",
            "add",
            "1",
            "--published",
            "v1",
            "--link",
            URL,
            "--final",
            "--summary",
            "x",
            "--actor",
            "ai",
        ],
    );
    assert_ne!(code, 0);
    let out = run(home.path(), &["release", "doc", "--version", "v1"]);
    assert_eq!(out["published"], 0);
}

#[test]
fn everything_else_on_a_done_card_is_still_refused() {
    let home = tempfile::tempdir().unwrap();
    done_task(home.path());
    assert_eq!(
        fail(
            home.path(),
            &["result", "add", "1", "--summary", "More", "--actor", "ai"]
        ),
        "task_is_done"
    );
    // A plain summary line is an ordinary result, not a publication record.
    assert_eq!(
        fail(
            home.path(),
            &[
                "result",
                "add",
                "1",
                "--summary",
                "Published: v1",
                "--link",
                URL,
                "--actor",
                "ai"
            ]
        ),
        "task_is_done"
    );
    assert_eq!(
        fail(home.path(), &["edit", "1", "--title", "New"]),
        "task_is_done"
    );
    assert_eq!(
        fail(
            home.path(),
            &["mv", "1", "doing", "--actor", "ai", "--reason", "Back"]
        ),
        "task_is_done"
    );
    let task = &run(home.path(), &["show", "1"])["task"];
    assert_eq!(task["taskStatus"], "done");
    assert_eq!(task["results"].as_array().unwrap().len(), 1);
}

#[test]
fn the_web_api_records_a_publication_on_a_done_card_and_refuses_a_missing_link() {
    let home = tempfile::tempdir().unwrap();
    done_task(home.path());
    let pid = json_ok(home.path(), &["project", "ls", "--json"])["projects"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();
    let server = Running::start(home.path());
    let path = format!("/api/projects/{pid}/tasks/1/results");

    let body = json!({"published": "v1", "link": URL, "summary": "Released"});
    let created = server.write("POST", &path, Some(&body.to_string()));
    assert_eq!(created.status, 201, "{}", created.body);
    assert_eq!(created.json()["created"], true);
    assert_eq!(created.json()["task"]["taskStatus"], "done");
    let again = server.write("POST", &path, Some(&body.to_string()));
    assert_eq!(again.status, 200, "{}", again.body);
    assert_eq!(again.json()["created"], false);

    let no_link = json!({"published": "v1", "link": " ", "summary": "Released"});
    let refused = server.write("POST", &path, Some(&no_link.to_string()));
    assert_eq!(refused.status, 422, "{}", refused.body);
    let unknown = json!({"published": "v1", "link": URL, "summary": "x", "kind": "final"});
    assert_eq!(
        server
            .write("POST", &path, Some(&unknown.to_string()))
            .status,
        400
    );

    let listed = server.get(&path).json();
    assert_eq!(listed["results"].as_array().unwrap().len(), 2);
    server.stop();
    let out = run(home.path(), &["release", "doc", "--version", "v1"]);
    assert_eq!(out["published"], 1);
}
