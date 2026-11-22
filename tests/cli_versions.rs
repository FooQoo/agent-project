//! The version commands: the current version, a task's planned version, an epic's bulk change,
//! and the version filter of `ls` and `progress`. Each test runs the real binary against a
//! temporary data area.

mod common;

use std::path::Path;

use common::{agpj, json_error, json_ok};
use serde_json::Value;

fn run(home: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--project", "Demo"];
    all.extend(args);
    all.push("--json");
    json_ok(home, &all)
}

fn fail(home: &Path, args: &[&str]) -> (i32, String) {
    let mut all = vec!["--project", "Demo"];
    all.extend(args);
    json_error(home, &all)
}

fn plain(home: &Path, args: &[&str]) -> String {
    let mut all = vec!["--project", "Demo"];
    all.extend(args);
    let (code, out, err) = agpj(home, &all);
    assert_eq!(code, 0, "{args:?} failed: {err}");
    out
}

fn add(home: &Path, title: &str, extra: &[&str]) -> i64 {
    let mut args = vec!["add", title, "--assignee", "ai", "--actor", "human"];
    args.extend(extra);
    run(home, &args)["task"]["id"].as_i64().unwrap()
}

fn demo(home: &Path) {
    let (code, _, err) = agpj(home, &["project", "create", "Demo"]);
    assert_eq!(code, 0, "{err}");
}

/// List versions the way a person can, by creating their files in the project's `docs/release/`,
/// so that no release-notes card takes a task number from the cards a test counts.
fn list_versions(home: &Path, names: &[&str]) {
    let projects = home.join("shared/projects");
    let project = std::fs::read_dir(&projects)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let dir = project.join("docs/release");
    std::fs::create_dir_all(&dir).unwrap();
    for name in names {
        std::fs::write(dir.join(format!("{name}.md")), "").unwrap();
    }
}

fn ids(value: &Value) -> Vec<i64> {
    value
        .as_array()
        .unwrap()
        .iter()
        .map(|task| task["id"].as_i64().unwrap())
        .collect()
}

fn ls_ids(home: &Path, args: &[&str]) -> Vec<i64> {
    let mut all = vec!["ls"];
    all.extend(args);
    ids(&run(home, &all)["tasks"])
}

/// An epic `WAVES` whose tasks run in two waves: #1 (planned v1.8.0) and #2 (planned v1.9.0)
/// first, #3 (v1.9.0) after them, then #4 with no plan and no wave. #5 has no epic.
fn waves_board(home: &Path) {
    demo(home);
    list_versions(home, &["v1.8.0", "v1.9.0"]);
    run(home, &["version", "current", "v1.9.0"]);
    let (code, _, err) = agpj(
        home,
        &[
            "--project",
            "Demo",
            "epic",
            "add",
            "waves",
            "--name",
            "Waves",
            "--actor",
            "human",
        ],
    );
    assert_eq!(code, 0, "{err}");
    for (title, version) in [
        ("First", Some("v1.8.0")),
        ("Second", Some("v1.9.0")),
        ("Third", Some("v1.9.0")),
        ("Fourth", None),
    ] {
        let mut extra = vec!["--epic", "WAVES"];
        if let Some(version) = version {
            extra.extend(["--planned-version", version]);
        }
        add(home, title, &extra);
    }
    add(home, "Loose", &["--current-version"]);
    run(home, &["epic", "edit", "WAVES", "--waves", "1,2", "3"]);
}

#[test]
fn the_current_version_is_shown_set_listed_and_cleared() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let shown = run(home.path(), &["version", "current"]);
    assert_eq!(shown["currentVersion"], Value::Null);
    assert_eq!(shown["versions"], serde_json::json!([]));
    assert_eq!(
        plain(home.path(), &["version", "current"]),
        "No current version.\n"
    );

    let set = run(home.path(), &["version", "current", "v1.8.0"]);
    assert_eq!(set["currentVersion"], "v1.8.0");
    assert_eq!(set["versions"], serde_json::json!(["v1.8.0"]));
    run(home.path(), &["version", "add", "v1.9.0"]);
    assert_eq!(
        plain(home.path(), &["version", "ls"]),
        "v1.9.0\nv1.8.0 (current)\n"
    );
    assert_eq!(
        plain(home.path(), &["version", "current"]),
        "Current version: v1.8.0\n"
    );

    let cleared = run(home.path(), &["version", "current", "--clear"]);
    assert_eq!(cleared["currentVersion"], Value::Null);
    assert_eq!(cleared["versions"], serde_json::json!(["v1.9.0", "v1.8.0"]));

    // A value with --clear is a usage error.
    let (code, name) = fail(home.path(), &["version", "current", "v2", "--clear"]);
    assert_eq!((code, name.as_str()), (2, "invalid_argument"));
    let (code, name) = fail(home.path(), &["version", "add", "bad name"]);
    assert_eq!((code, name.as_str()), (2, "invalid_value"));
}

#[test]
fn add_stores_the_concrete_current_version_and_refuses_without_one() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let (code, name) = fail(
        home.path(),
        &[
            "add",
            "T",
            "--assignee",
            "ai",
            "--actor",
            "human",
            "--current-version",
        ],
    );
    assert_eq!((code, name.as_str()), (1, "no_current_version"));
    assert_eq!(
        ls_ids(home.path(), &[]),
        Vec::<i64>::new(),
        "nothing was created"
    );

    run(home.path(), &["version", "current", "v1.8.0"]);
    let id = add(home.path(), "T", &["--current-version"]);
    let task = &run(home.path(), &["show", &id.to_string()])["task"];
    assert_eq!(task["plannedVersion"], "v1.8.0");

    // Switching the current version later does not move the task.
    run(home.path(), &["version", "current", "v1.9.0"]);
    let task = &run(home.path(), &["show", &id.to_string()])["task"];
    assert_eq!(task["plannedVersion"], "v1.8.0");
}

#[test]
fn edit_plans_a_task_for_a_version_the_current_one_or_none() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let id = add(home.path(), "T", &[]).to_string();
    run(home.path(), &["version", "add", "v1.8.0"]);

    let task = &run(home.path(), &["edit", &id, "--planned-version", "v1.8.0"])["task"];
    assert_eq!(task["plannedVersion"], "v1.8.0");
    let history = task["history"].as_array().unwrap().len();
    // The same value again changes nothing.
    let task = &run(home.path(), &["edit", &id, "--planned-version", "v1.8.0"])["task"];
    assert_eq!(task["history"].as_array().unwrap().len(), history);

    let task = &run(home.path(), &["edit", &id, "--unplanned"])["task"];
    assert!(task.get("plannedVersion").is_none_or(Value::is_null));

    let (code, name) = fail(home.path(), &["edit", &id, "--current-version"]);
    assert_eq!((code, name.as_str()), (1, "no_current_version"));
    run(home.path(), &["version", "current", "v1.8.0"]);
    let task = &run(home.path(), &["edit", &id, "--current-version"])["task"];
    assert_eq!(task["plannedVersion"], "v1.8.0");

    // A version the project does not list, and options that exclude each other.
    let (code, name) = fail(home.path(), &["edit", &id, "--planned-version", "v9"]);
    assert_eq!((code, name.as_str()), (2, "invalid_argument"));
    let (code, name) = fail(
        home.path(),
        &["edit", &id, "--unplanned", "--current-version"],
    );
    assert_eq!((code, name.as_str()), (2, "invalid_argument"));
}

#[test]
fn a_description_file_that_changes_the_plan_records_one_history_entry() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let id = add(home.path(), "T", &[]).to_string();
    run(home.path(), &["version", "add", "v1.8.0"]);
    run(home.path(), &["version", "add", "v1.9.0"]);
    run(home.path(), &["edit", &id, "--planned-version", "v1.8.0"]);
    let file = home.path().join("desc.md");
    let edit = |text: &str| {
        std::fs::write(&file, text).unwrap();
        run(
            home.path(),
            &["edit", &id, "--desc-file", file.to_str().unwrap()],
        )["task"]
            .clone()
    };
    let entries = |task: &Value| -> Vec<String> {
        task["history"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|e| e["reason"].as_str())
            .filter(|r| r.contains("planned version"))
            .map(str::to_owned)
            .collect()
    };
    let before = entries(&run(home.path(), &["show", &id])["task"]);
    assert_eq!(before.len(), 1);

    // A different version is kept and recorded once.
    let task = edit("---\nplannedVersion: v1.9.0\n---\nBody one\n");
    assert_eq!(task["plannedVersion"], "v1.9.0");
    let after = entries(&task);
    assert_eq!(after.len(), 2);
    assert_eq!(after[1], "Changed planned version from v1.8.0 to v1.9.0.");

    // The same version adds nothing.
    let task = edit("---\nplannedVersion: v1.9.0\n---\nBody two\n");
    assert_eq!(task["plannedVersion"], "v1.9.0");
    assert_eq!(entries(&task).len(), 2);

    // Omitting the key keeps the stored plan and adds nothing.
    let task = edit("Body three\n");
    assert_eq!(task["plannedVersion"], "v1.9.0");
    assert_eq!(entries(&task).len(), 2);
}

#[test]
fn epic_plan_reports_counts_skips_done_cards_and_can_leave_planned_cards_alone() {
    let home = tempfile::tempdir().unwrap();
    waves_board(home.path());
    // #4 is the only unplanned card of the epic; #2 is finished.
    plain(
        home.path(),
        &["mv", "2", "doing", "--actor", "ai", "--reason", "Go."],
    );
    plain(
        home.path(),
        &["mv", "2", "done", "--actor", "ai", "--reason", "Done."],
    );

    let only = run(
        home.path(),
        &[
            "epic",
            "plan",
            "waves",
            "--planned-version",
            "v1.8.0",
            "--only-unplanned",
        ],
    );
    assert_eq!(only["changed"], serde_json::json!([4]));
    assert_eq!(only["unchanged"], serde_json::json!([]));
    assert_eq!(only["skipped"], serde_json::json!([1, 2, 3]));

    let all = run(
        home.path(),
        &["epic", "plan", "waves", "--planned-version", "v1.8.0"],
    );
    assert_eq!(all["changed"], serde_json::json!([3]));
    assert_eq!(all["unchanged"], serde_json::json!([1, 4]));
    assert_eq!(
        all["skipped"],
        serde_json::json!([2]),
        "a Done card is skipped"
    );
    let text = plain(
        home.path(),
        &["epic", "plan", "waves", "--planned-version", "v1.8.0"],
    );
    assert_eq!(
        text,
        "Epic WAVES: 0 changed, 3 unchanged (#1, #3, #4), 1 skipped (#2).\n"
    );
    let done = &run(home.path(), &["show", "2"])["task"];
    assert_eq!(done["plannedVersion"], "v1.9.0");

    let none = run(home.path(), &["epic", "plan", "waves", "--unplanned"]);
    assert_eq!(none["changed"], serde_json::json!([1, 3, 4]));

    let (code, _) = fail(home.path(), &["epic", "plan", "waves"]);
    assert_eq!(code, 2, "a target is required");
    let (code, _) = fail(
        home.path(),
        &["epic", "plan", "waves", "--unplanned", "--only-unplanned"],
    );
    assert_eq!(code, 2);
    let (code, name) = fail(home.path(), &["epic", "plan", "nope", "--current-version"]);
    assert_eq!(code, 1);
    assert!(
        name == "epic_not_found" || name == "no_current_version",
        "{name}"
    );
}

#[test]
fn ls_narrows_by_version_and_combines_with_the_other_filters() {
    let home = tempfile::tempdir().unwrap();
    waves_board(home.path());
    run(home.path(), &["version", "current", "v1.9.0"]);

    assert_eq!(ls_ids(home.path(), &[]), vec![1, 2, 3, 4, 5]);
    assert_eq!(ls_ids(home.path(), &["--version", "v1.9.0"]), vec![2, 3, 5]);
    assert_eq!(ls_ids(home.path(), &["--current-version"]), vec![2, 3, 5]);
    assert_eq!(ls_ids(home.path(), &["--version", "v1.8.0"]), vec![1]);
    assert_eq!(ls_ids(home.path(), &["--unplanned"]), vec![4]);
    // Combined with the epic, column, and state filters.
    assert_eq!(
        ls_ids(home.path(), &["--epic", "waves", "--version", "v1.9.0"]),
        vec![2, 3]
    );
    assert_eq!(
        ls_ids(home.path(), &["--epic", "waves", "--unplanned"]),
        vec![4]
    );
    assert_eq!(
        ls_ids(home.path(), &["--column", "doing", "--version", "v1.9.0"]),
        Vec::<i64>::new()
    );
    assert_eq!(
        ls_ids(
            home.path(),
            &["--state", "idle", "--current-version", "--epic", "waves"]
        ),
        vec![2, 3]
    );

    // Every task of the JSON carries its plannedVersion, and the text shows it.
    let tasks = run(home.path(), &["ls"])["tasks"].clone();
    let planned: Vec<Option<&str>> = tasks
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["plannedVersion"].as_str())
        .collect();
    assert_eq!(
        planned,
        vec![
            Some("v1.8.0"),
            Some("v1.9.0"),
            Some("v1.9.0"),
            None,
            Some("v1.9.0")
        ]
    );
    let text = plain(home.path(), &["ls", "--version", "v1.8.0"]);
    assert!(
        text.contains("#1 First [AI] [WAVES] [plan v1.8.0]"),
        "{text}"
    );
    assert!(!text.contains("#2"), "{text}");

    // Another project's current version is not this project's.
    let (code, name) = {
        run(home.path(), &["version", "current", "--clear"]);
        fail(home.path(), &["ls", "--current-version"])
    };
    assert_eq!((code, name.as_str()), (1, "no_current_version"));
    let (code, name) = fail(
        home.path(),
        &["ls", "--current-version", "--version", "v1.8.0"],
    );
    assert_eq!((code, name.as_str()), (2, "invalid_argument"));
    let (code, name) = fail(home.path(), &["ls", "--unplanned", "--version", "v1.8.0"]);
    assert_eq!((code, name.as_str()), (2, "invalid_argument"));
}

#[test]
fn ls_of_every_project_takes_a_named_version_but_not_the_current_one() {
    let home = tempfile::tempdir().unwrap();
    waves_board(home.path());
    let out = json_ok(
        home.path(),
        &["ls", "--all-projects", "--version", "v1.8.0", "--json"],
    );
    assert_eq!(ids(&out["projects"][0]["tasks"]), vec![1]);
    let (code, name) = json_error(home.path(), &["ls", "--all-projects", "--current-version"]);
    assert_eq!((code, name.as_str()), (2, "invalid_argument"));
}

fn progress(home: &Path, args: &[&str]) -> Value {
    let mut all = vec!["progress"];
    all.extend(args);
    run(home, &all)
}

fn epic_task_ids(table: &Value) -> Vec<i64> {
    table["epics"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|epic| ids(&epic["tasks"]))
        .collect()
}

#[test]
fn a_version_filter_in_progress_hides_tasks_but_never_makes_one_ready() {
    let home = tempfile::tempdir().unwrap();
    waves_board(home.path());
    run(home.path(), &["version", "current", "v1.9.0"]);

    // The whole board: wave 1 (#1, #2) is ready, wave 2 (#3) waits for it.
    let all = progress(home.path(), &[]);
    // nextTasks holds at most three tasks.
    assert_eq!(ids(&all["nextTasks"]), vec![1, 2, 4]);
    assert_eq!(epic_task_ids(&all), vec![1, 2, 3, 4]);
    assert_eq!(all["epics"][0]["tasks"][0]["plannedVersion"], "v1.8.0");

    // v1.9.0 holds #2 and #3. Their blocker #1 is hidden, yet #3 stays blocked by it.
    let filtered = progress(home.path(), &["--version", "v1.9.0"]);
    assert_eq!(epic_task_ids(&filtered), vec![2, 3]);
    assert_eq!(ids(&filtered["nextTasks"]), vec![2, 5]);
    assert!(!ids(&filtered["nextTasks"]).contains(&3));
    assert_eq!(progress(home.path(), &["--current-version"]), filtered);

    // Only #3: its blockers are all outside the filter, and it is still not ready.
    run(home.path(), &["edit", "2", "--planned-version", "v1.8.0"]);
    let only_third = progress(home.path(), &["--version", "v1.9.0"]);
    assert_eq!(epic_task_ids(&only_third), vec![3]);
    assert_eq!(ids(&only_third["nextTasks"]), vec![5]);

    // The unplanned tasks, and an epic with nothing left is dropped.
    let unplanned = progress(home.path(), &["--unplanned"]);
    assert_eq!(epic_task_ids(&unplanned), vec![4]);
    assert_eq!(ids(&unplanned["nextTasks"]), vec![4]);
    let other = progress(home.path(), &["--version", "v-none"]);
    assert_eq!(other["epics"], serde_json::json!([]));
    assert_eq!(other["nextTasks"], serde_json::json!([]));

    // Text output shows the plan, and a project with no current version is refused.
    let text = plain(home.path(), &["progress", "--version", "v1.8.0"]);
    assert!(text.contains("plan v1.8.0"), "{text}");
    run(home.path(), &["version", "current", "--clear"]);
    let (code, name) = fail(home.path(), &["progress", "--current-version"]);
    assert_eq!((code, name.as_str()), (1, "no_current_version"));
    let (code, name) = fail(
        home.path(),
        &["progress", "--current-version", "--unplanned"],
    );
    assert_eq!((code, name.as_str()), (2, "invalid_argument"));
}

#[test]
fn the_root_version_flag_still_prints_the_binary_version() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let (code, out, _) = agpj(home.path(), &["--version"]);
    assert_eq!(code, 0);
    assert!(out.starts_with("agpj "), "{out}");
    // The option of the subcommands is a different one.
    let (code, _, err) = agpj(
        home.path(),
        &["--project", "Demo", "ls", "--version", "v1.8.0"],
    );
    assert_eq!(code, 0, "{err}");
}

#[test]
fn a_failed_version_change_in_edit_leaves_the_other_changes_unsaved() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let id = add(home.path(), "Old title", &[]).to_string();

    // No current version: the title is not changed.
    let (code, name) = fail(
        home.path(),
        &["edit", &id, "--title", "New title", "--current-version"],
    );
    assert_eq!((code, name.as_str()), (1, "no_current_version"));
    assert_eq!(
        run(home.path(), &["show", &id])["task"]["title"],
        "Old title"
    );

    // A Done task refuses an agent's title change, so a version change in the same edit is
    // not saved either.
    run(home.path(), &["version", "add", "v1.8.0"]);
    plain(
        home.path(),
        &["mv", &id, "doing", "--actor", "ai", "--reason", "Go."],
    );
    plain(
        home.path(),
        &["mv", &id, "done", "--actor", "ai", "--reason", "Done."],
    );
    let (code, name) = fail(
        home.path(),
        &[
            "edit",
            &id,
            "--title",
            "New title",
            "--planned-version",
            "v1.8.0",
        ],
    );
    assert_eq!((code, name.as_str()), (1, "task_is_done"));
    let shown = run(home.path(), &["show", &id]);
    assert_eq!(shown["task"]["title"], "Old title");
    assert_eq!(shown["task"]["plannedVersion"], Value::Null);
}

#[test]
fn a_done_task_takes_a_planned_version_from_an_agent_and_nothing_else() {
    let home = tempfile::tempdir().unwrap();
    demo(home.path());
    let id = add(home.path(), "Shipped", &[]).to_string();
    run(home.path(), &["version", "add", "v1.8.0"]);
    run(home.path(), &["version", "add", "v2.0.0"]);
    plain(
        home.path(),
        &["mv", &id, "doing", "--actor", "ai", "--reason", "Go."],
    );
    plain(
        home.path(),
        &["mv", &id, "done", "--actor", "ai", "--reason", "Done."],
    );

    let set = run(home.path(), &["edit", &id, "--planned-version", "v1.8.0"]);
    assert_eq!(set["task"]["plannedVersion"], "v1.8.0");
    assert_eq!(set["task"]["taskStatus"], "done");
    let moved = run(home.path(), &["edit", &id, "--planned-version", "v2.0.0"]);
    assert_eq!(moved["task"]["plannedVersion"], "v2.0.0");
    let cleared = run(home.path(), &["edit", &id, "--unplanned"]);
    assert_eq!(cleared["task"]["plannedVersion"], Value::Null);
    let history = cleared["task"]["history"].as_array().unwrap();
    assert!(
        history[history.len() - 3..]
            .iter()
            .all(|entry| entry["reason"]
                .as_str()
                .unwrap()
                .starts_with("Changed planned version")),
        "{history:?}"
    );

    // Every other edit of a Done task stays refused.
    let (code, name) = fail(home.path(), &["edit", &id, "--title", "Other"]);
    assert_eq!((code, name.as_str()), (1, "task_is_done"));
}
