//! Result plans in the CLI: what a task will deliver and the condition for delivering it,
//! stored apart from the acceptance conditions, written with the task, edited later, and
//! unchanged once the task is started.

mod common;

use common::{
    agpj, agpj_with_connection, connect_default_agent, demo_project, json_error, json_ok,
};
use serde_json::{Value, json};
use std::path::Path;

const PLANS: &str = "---\nacceptance:\n  - id: AC-1\n    text: The report is complete\n    done: false\nresultPlans:\n  - content: A screenshot of the layout\n    condition: When the layout is fixed\n  - content: The working endpoint\n    condition: When the endpoint answers its first request\n  - content: The finished report\n    condition: When every condition is met\n    final: true\n---\n\nBody.\n";

fn add(home: &Path, desc: &str) -> (i32, String) {
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
    (code, err)
}

fn show(home: &Path, id: &str) -> Value {
    json_ok(home, &["--project", "Demo", "show", id, "--json"])["task"].clone()
}

fn plans(task: &Value) -> Value {
    task["resultPlans"].clone()
}

#[test]
fn plans_get_stable_ids_when_the_task_is_created_and_survive_a_start() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let (code, err) = add(home.path(), PLANS);
    assert_eq!(code, 0, "{err}");

    let expected = json!([
        {"id": "RP-1", "content": "A screenshot of the layout", "condition": "When the layout is fixed", "final": false, "deliveredResultVersion": null},
        {"id": "RP-2", "content": "The working endpoint", "condition": "When the endpoint answers its first request", "final": false, "deliveredResultVersion": null},
        {"id": "RP-3", "content": "The finished report", "condition": "When every condition is met", "final": true, "deliveredResultVersion": null},
    ]);
    let created = show(home.path(), "2");
    assert_eq!(plans(&created), expected);
    assert_eq!(created["resultPlansError"], false);
    // The conditions are a separate list: plans are not conditions.
    assert_eq!(created["acceptance"].as_array().unwrap().len(), 1);

    let secret = connect_default_agent(home.path(), "Demo");
    let (code, _, err) = agpj_with_connection(
        home.path(),
        &secret,
        &[
            "--project",
            "Demo",
            "mv",
            "2",
            "doing",
            "--actor",
            "ai",
            "--reason",
            "Starting.",
        ],
    );
    assert_eq!(code, 0, "{err}");
    let started = show(home.path(), "2");
    assert_eq!(started["taskStatus"], "doing");
    assert_eq!(plans(&started), expected, "starting changes no plan");
}

#[test]
fn a_task_can_have_only_a_final_plan_or_none_at_all() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let only_final = "---\nresultPlans:\n  - id: RP-7\n    content: The report\n    condition: When done\n    final: true\n---\n\nBody.\n";
    assert_eq!(add(home.path(), only_final).0, 0);
    assert_eq!(
        plans(&show(home.path(), "2")),
        json!([{"id": "RP-7", "content": "The report", "condition": "When done", "final": true, "deliveredResultVersion": null}]),
        "a given ID is kept"
    );
    let only_intermediate = "---\nresultPlans:\n  - content: A\n    condition: B\n---\n\nBody.\n";
    assert_eq!(add(home.path(), only_intermediate).0, 0);
    assert_eq!(plans(&show(home.path(), "3"))[0]["final"], false);

    assert_eq!(add(home.path(), "No frontmatter at all.").0, 0);
    let plain = show(home.path(), "4");
    assert_eq!(plans(&plain), json!([]));
    assert_eq!(plain["resultPlansError"], false);
}

#[test]
fn empty_content_or_condition_and_a_second_final_are_input_errors() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    for bad in [
        "---\nresultPlans:\n  - content: ''\n    condition: When done\n---\n",
        "---\nresultPlans:\n  - content: The report\n    condition: '   '\n---\n",
        "---\nresultPlans:\n  - content: The report\n---\n",
        "---\nresultPlans:\n  - {content: a, condition: b, final: true}\n  - {content: c, condition: d, final: true}\n---\n",
        "---\nresultPlans:\n  - {id: RP-1, content: a, condition: b}\n  - {id: RP-1, content: c, condition: d}\n---\n",
    ] {
        let (code, err) = add(home.path(), bad);
        assert_ne!(code, 0, "{bad}");
        assert!(err.contains("resultPlans"), "{err}");
    }
    // Nothing was created by the refused inputs.
    let (code, _) = json_error(home.path(), &["--project", "Demo", "show", "2", "--json"]);
    assert_ne!(code, 0);
}

#[test]
fn an_edit_changes_the_plans_and_keeps_the_ids_that_it_carries() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    assert_eq!(add(home.path(), PLANS).0, 0);

    let edited = "---\nacceptance:\n  - id: AC-1\n    text: The report is complete\n    done: false\nresultPlans:\n  - id: RP-2\n    content: The working endpoint\n    condition: When it answers\n  - content: A new plan\n    condition: After review\n  - id: RP-3\n    content: The finished report\n    condition: When every condition is met\n    final: true\n---\n\nBody.\n";
    let file = home.path().join("edit.md");
    std::fs::write(&file, edited).unwrap();
    let (code, _, err) = agpj(
        home.path(),
        &[
            "--project",
            "Demo",
            "edit",
            "2",
            "--desc-file",
            file.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "{err}");
    let task = show(home.path(), "2");
    let ids: Vec<_> = plans(&task)
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p["id"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(ids, ["RP-2", "RP-4", "RP-3"]);
    assert_eq!(plans(&task)[0]["condition"], "When it answers");

    // An edit without plans removes them; an invalid plan is refused and nothing changes.
    std::fs::write(
        &file,
        "---\nresultPlans:\n  - {content: '', condition: x}\n---\n",
    )
    .unwrap();
    let (code, _, _) = agpj(
        home.path(),
        &[
            "--project",
            "Demo",
            "edit",
            "2",
            "--desc-file",
            file.to_str().unwrap(),
        ],
    );
    assert_ne!(code, 0);
    assert_eq!(
        show(home.path(), "2")["resultPlans"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}

#[test]
fn other_writes_keep_the_plans_and_a_hand_edited_defect_is_reported() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    assert_eq!(add(home.path(), PLANS).0, 0);
    // Editing the body keeps the plans: the file is read, not rebuilt from the input.
    let (code, _, err) = agpj(
        home.path(),
        &["--project", "Demo", "edit", "2", "--title", "Renamed"],
    );
    assert_eq!(code, 0, "{err}");
    assert_eq!(
        show(home.path(), "2")["resultPlans"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let md = home
        .path()
        .join("shared/projects")
        .read_dir()
        .map(|d| d.flatten().next().unwrap().path().join("tasks/2/TASK.md"))
        .unwrap();
    let text = std::fs::read_to_string(&md).unwrap();
    std::fs::write(&md, text.replace("id: RP-2", "id: nope")).unwrap();
    let task = show(home.path(), "2");
    assert_eq!(task["resultPlansError"], true);
    assert_eq!(plans(&task), json!([]));
}
