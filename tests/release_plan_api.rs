//! The web API for planned versions: the project's versions and release version, one task's
//! planned version, and an epic-wide change with its preview. Every test uses a temporary data
//! area; servers listen on port 0.

mod common;

use std::path::Path;

use common::{Running, agpj, json_ok};
use serde_json::{Value, json};

fn run(home: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--project", "Demo"];
    all.extend(args);
    all.push("--json");
    json_ok(home, &all)
}

struct Board {
    _home: tempfile::TempDir,
    server: Running,
    base: String,
}

impl Board {
    /// A project with the epic PLAN holding tasks 1 to 4, all Todo.
    fn new() -> Board {
        let home = tempfile::tempdir().unwrap();
        let (code, _, err) = agpj(home.path(), &["project", "create", "Demo"]);
        assert_eq!(code, 0, "{err}");
        run(
            home.path(),
            &["epic", "add", "PLAN", "--name", "Plan", "--actor", "ai"],
        );
        run(
            home.path(),
            &["epic", "add", "OTHER", "--name", "Other", "--actor", "ai"],
        );
        for title in ["A", "B", "C", "D"] {
            run(
                home.path(),
                &[
                    "add",
                    title,
                    "--assignee",
                    "ai",
                    "--actor",
                    "ai",
                    "--epic",
                    "PLAN",
                ],
            );
        }
        run(
            home.path(),
            &[
                "add",
                "E",
                "--assignee",
                "ai",
                "--actor",
                "ai",
                "--epic",
                "OTHER",
            ],
        );
        let pid = json_ok(home.path(), &["project", "ls", "--json"])["projects"][0]["id"]
            .as_str()
            .unwrap()
            .to_string();
        let server = Running::start(home.path());
        Board {
            _home: home,
            server,
            base: format!("/api/projects/{pid}"),
        }
    }

    fn call(&self, method: &str, path: &str, body: Value) -> common::Reply {
        self.server.write(
            method,
            &format!("{}{path}", self.base),
            Some(&body.to_string()),
        )
    }

    fn summary_task(&self, id: i64) -> Value {
        let summary = self.server.get(&format!("{}/summary", self.base)).json();
        summary["tasks"]
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["id"] == id)
            .cloned()
            .unwrap_or_else(|| panic!("task {id} not in the summary"))
    }

    fn plan(&self, id: i64, version: &str) {
        let reply = self.call(
            "PUT",
            &format!("/tasks/{id}/planned-version"),
            json!({"choice": "named", "version": version}),
        );
        assert_eq!(reply.status, 200, "{}", reply.body);
    }
}

#[test]
fn versions_are_listed_added_and_set_current_with_non_ascii_names() {
    let board = Board::new();
    let empty = board.server.get(&format!("{}/versions", board.base));
    assert_eq!(empty.status, 200);
    assert_eq!(
        empty.json(),
        json!({"versions": [], "releaseVersion": null})
    );

    let added = board.call("POST", "/versions", json!({"name": "v1.8.0"}));
    assert_eq!(added.status, 200, "{}", added.body);
    assert_eq!(added.json()["versions"], json!(["v1.8.0"]));
    assert_eq!(added.json()["changed"], true);
    let again = board.call("POST", "/versions", json!({"name": "v1.8.0"}));
    assert_eq!(
        again.json()["changed"],
        false,
        "listing twice changes nothing"
    );
    let japanese = board.call("POST", "/versions", json!({"name": "第2版"}));
    assert_eq!(japanese.status, 200, "{}", japanese.body);

    let release = board.call("PUT", "/versions/release", json!({"version": "第2版"}));
    assert_eq!(release.status, 200, "{}", release.body);
    assert_eq!(release.json()["releaseVersion"], "第2版");
    let read = board.server.get(&format!("{}/versions", board.base)).json();
    assert_eq!(read["versions"], json!(["第2版", "v1.8.0"]));
    assert_eq!(read["releaseVersion"], "第2版");

    // A version that is not listed is listed when it becomes the release version.
    let unlisted = board.call("PUT", "/versions/release", json!({"version": "v2.0.0"}));
    assert_eq!(unlisted.json()["versions"].as_array().unwrap().len(), 3);
    let cleared = board.call("PUT", "/versions/release", json!({"version": null}));
    assert_eq!(cleared.json()["releaseVersion"], Value::Null);

    for bad in [json!({"name": "has space"}), json!({"name": ""})] {
        let reply = board.call("POST", "/versions", bad);
        assert_eq!(reply.status, 422, "{}", reply.body);
        assert_eq!(reply.code(), "invalid_value");
    }
    let unknown = board.call("POST", "/versions", json!({"name": "v3", "x": 1}));
    assert_eq!(unknown.status, 400, "{}", unknown.body);
}

#[test]
fn the_summary_carries_planned_versions_and_the_project_versions() {
    let board = Board::new();
    assert_eq!(board.summary_task(1)["plannedVersion"], Value::Null);
    board.call("POST", "/versions", json!({"name": "v1.8.0"}));
    board.call("PUT", "/versions/release", json!({"version": "v1.8.0"}));
    board.plan(1, "v1.8.0");

    let summary = board.server.get(&format!("{}/summary", board.base)).json();
    assert_eq!(summary["project"]["versions"], json!(["v1.8.0"]));
    assert_eq!(summary["project"]["releaseVersion"], "v1.8.0");
    assert_eq!(board.summary_task(1)["plannedVersion"], "v1.8.0");
    assert_eq!(board.summary_task(2)["plannedVersion"], Value::Null);
}

#[test]
fn a_change_of_versions_changes_the_summary_so_open_boards_refresh() {
    let board = Board::new();
    let before = board.server.get(&format!("{}/summary", board.base)).json()["version"].clone();
    board.call("POST", "/versions", json!({"name": "v1.8.0"}));
    let listed = board.server.get(&format!("{}/summary", board.base)).json()["version"].clone();
    assert_ne!(
        before, listed,
        "adding a version changes the summary version"
    );
    board.call("PUT", "/versions/release", json!({"version": "v1.8.0"}));
    let release = board.server.get(&format!("{}/summary", board.base)).json()["version"].clone();
    assert_ne!(listed, release, "setting the release version changes it");
    board.plan(1, "v1.8.0");
    let planned = board.server.get(&format!("{}/summary", board.base)).json()["version"].clone();
    assert_ne!(release, planned, "planning a task changes it");
}

#[test]
fn a_task_is_planned_for_a_named_version_the_release_version_or_none() {
    let board = Board::new();
    board.call("POST", "/versions", json!({"name": "v1.8.0"}));

    let no_current = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "release"}),
    );
    assert_eq!(no_current.status, 422, "{}", no_current.body);
    let unknown = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "named", "version": "v9"}),
    );
    assert_eq!(unknown.status, 422, "{}", unknown.body);
    let missing = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "named"}),
    );
    assert_eq!(missing.status, 422, "{}", missing.body);
    let stray = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "unset", "version": "v1.8.0"}),
    );
    assert_eq!(stray.status, 422, "{}", stray.body);
    assert_eq!(board.summary_task(1)["plannedVersion"], Value::Null);

    board.call("PUT", "/versions/release", json!({"version": "v1.8.0"}));
    let release = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "release"}),
    );
    assert_eq!(release.status, 200, "{}", release.body);
    assert_eq!(release.json()["changed"], true);
    assert_eq!(release.json()["task"]["plannedVersion"], "v1.8.0");
    let same = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "named", "version": "v1.8.0"}),
    );
    assert_eq!(same.json()["changed"], false);

    let cleared = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "unset"}),
    );
    assert_eq!(cleared.json()["task"]["plannedVersion"], Value::Null);
    // The creation entry, an earlier plan, then this clearing.
    assert_eq!(
        cleared.json()["task"]["history"].as_array().unwrap().len(),
        4
    );
    let gone = board.call(
        "PUT",
        "/tasks/99/planned-version",
        json!({"choice": "unset"}),
    );
    assert_eq!(gone.status, 404, "{}", gone.body);
}

#[test]
fn a_stale_expectation_is_a_conflict_that_names_the_release_version() {
    let board = Board::new();
    board.call("POST", "/versions", json!({"name": "v1"}));
    board.call("POST", "/versions", json!({"name": "v2"}));
    board.plan(1, "v1");

    // The page showed "no version", but another change set v1 meanwhile.
    let stale = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "named", "version": "v2", "expectedPlannedVersion": null}),
    );
    assert_eq!(stale.status, 409, "{}", stale.body);
    assert_eq!(stale.code(), "planned_version_changed");
    assert_eq!(stale.json()["currentPlannedVersion"], "v1");
    assert_eq!(
        board.summary_task(1)["plannedVersion"],
        "v1",
        "nothing was written"
    );

    // A matching expectation saves; so does a request without one.
    let fresh = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "named", "version": "v2", "expectedPlannedVersion": "v1"}),
    );
    assert_eq!(fresh.status, 200, "{}", fresh.body);
    let unchecked = board.call(
        "PUT",
        "/tasks/1/planned-version",
        json!({"choice": "unset"}),
    );
    assert_eq!(unchecked.status, 200, "{}", unchecked.body);
}

#[test]
fn an_archived_task_can_be_planned() {
    let board = Board::new();
    board.call("POST", "/versions", json!({"name": "v1"}));
    let moved = board.call("POST", "/tasks/4/move", json!({"taskStatus": "done"}));
    assert_eq!(moved.status, 200, "{}", moved.body);
    let archived = board.call("POST", "/tasks/4/archive", json!({}));
    assert_eq!(archived.status, 200, "{}", archived.body);
    let reply = board.call(
        "PUT",
        "/tasks/4/planned-version",
        json!({"choice": "named", "version": "v1"}),
    );
    assert_eq!(reply.status, 200, "{}", reply.body);
    assert_eq!(reply.json()["task"]["plannedVersion"], "v1");
}

#[test]
fn an_epic_change_previews_then_applies_only_the_checked_tasks() {
    let board = Board::new();
    board.call("POST", "/versions", json!({"name": "v1"}));
    board.call("POST", "/versions", json!({"name": "v2"}));
    board.call("PUT", "/versions/release", json!({"version": "v2"}));
    board.plan(2, "v1");
    // Task 3 is Done: a shipped version is not rewritten.
    let moved = board.call("POST", "/tasks/3/move", json!({"taskStatus": "done"}));
    assert_eq!(moved.status, 200, "{}", moved.body);
    // Task 4 is archived.
    board.call("POST", "/tasks/4/move", json!({"taskStatus": "done"}));
    board.call("POST", "/tasks/4/archive", json!({}));

    let request = |scope: &str, ids: Value, dry_run: bool| {
        json!({
            "choice": "release",
            "scope": scope,
            "taskIds": ids,
            "dryRun": dry_run,
        })
    };
    let preview = board.call(
        "POST",
        "/epics/PLAN/planned-version",
        request("unplanned-only", json!([1, 2, 3, 4]), true),
    );
    assert_eq!(preview.status, 200, "{}", preview.body);
    let body = preview.json();
    assert_eq!(body["dryRun"], true);
    assert_eq!(body["changed"], json!([1]));
    assert_eq!(body["skipped"], json!([2, 3, 4]));
    assert_eq!(
        body["skippedReasons"],
        json!({"2": "planned", "3": "done", "4": "archived"})
    );
    assert_eq!(
        board.summary_task(1)["plannedVersion"],
        Value::Null,
        "a preview writes nothing"
    );

    // The person unchecked task 1 and kept the rest: all scope replaces version 1, still never
    // touching the Done and archived tasks.
    let applied = board.call(
        "POST",
        "/epics/PLAN/planned-version",
        request("all", json!([2, 3, 4]), false),
    );
    let body = applied.json();
    assert_eq!(body["changed"], json!([2]));
    assert_eq!(body["skipped"], json!([3, 4]));
    assert_eq!(body["dryRun"], false);
    assert_eq!(board.summary_task(2)["plannedVersion"], "v2");
    assert_eq!(
        board.summary_task(1)["plannedVersion"],
        Value::Null,
        "an unchecked task is left"
    );
    assert_eq!(
        board.summary_task(3)["plannedVersion"],
        Value::Null,
        "Done is not rewritten"
    );

    // Applying again finds nothing to change.
    let again = board.call(
        "POST",
        "/epics/PLAN/planned-version",
        request("all", json!([1, 2]), false),
    );
    let body = again.json();
    assert_eq!(body["changed"], json!([1]));
    assert_eq!(body["unchanged"], json!([2]));

    // Clearing works for the checked tasks.
    let cleared = board.call(
        "POST",
        "/epics/PLAN/planned-version",
        json!({"choice": "unset", "scope": "all", "taskIds": [1, 2]}),
    );
    assert_eq!(cleared.json()["changed"], json!([1, 2]));
}

#[test]
fn an_epic_change_refuses_bad_requests_before_writing() {
    let board = Board::new();
    board.call("POST", "/versions", json!({"name": "v1"}));
    let named =
        |ids: Value| json!({"choice": "named", "version": "v1", "scope": "all", "taskIds": ids});

    // Task 5 belongs to another epic, task 99 does not exist.
    for ids in [json!([1, 5]), json!([1, 99])] {
        let reply = board.call("POST", "/epics/PLAN/planned-version", named(ids));
        assert_eq!(reply.status, 422, "{}", reply.body);
        assert_eq!(reply.code(), "invalid_value");
    }
    assert_eq!(
        board.summary_task(1)["plannedVersion"],
        Value::Null,
        "nothing was written"
    );

    let no_epic = board.call("POST", "/epics/NOPE/planned-version", named(json!([1])));
    assert_eq!(no_epic.status, 404, "{}", no_epic.body);
    assert_eq!(no_epic.code(), "epic_not_found");
    let no_current = board.call(
        "POST",
        "/epics/PLAN/planned-version",
        json!({"choice": "release", "scope": "all", "taskIds": [1]}),
    );
    assert_eq!(no_current.status, 422, "{}", no_current.body);
    let unknown_version = board.call(
        "POST",
        "/epics/PLAN/planned-version",
        json!({"choice": "named", "version": "v9", "scope": "all", "taskIds": [1]}),
    );
    assert_eq!(unknown_version.status, 422, "{}", unknown_version.body);
    let bad_scope = board.call(
        "POST",
        "/epics/PLAN/planned-version",
        json!({"choice": "unset", "scope": "some", "taskIds": [1]}),
    );
    assert_eq!(bad_scope.status, 422, "{}", bad_scope.body);
}

#[test]
fn writes_need_the_page_headers_like_every_other_write() {
    let board = Board::new();
    let path = format!("{}/versions", board.base);
    let reply = board.server.request(
        "POST",
        &path,
        Some(r#"{"name":"v1"}"#),
        &[],
        Some("application/json"),
    );
    assert!(reply.status == 403 || reply.status == 401, "{}", reply.body);
    let read = board.server.get(&path).json();
    assert_eq!(read["versions"], json!([]));
}
