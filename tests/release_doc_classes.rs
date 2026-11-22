//! The classes of a version's release document: done without acceptance conditions, and tasks
//! moved to another version. Each test runs the real binary against a temporary data area.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{agpj, json_ok};
use serde_json::Value;

fn run(home: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--project", "Demo"];
    all.extend(args);
    all.push("--json");
    json_ok(home, &all)
}

fn demo() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().to_path_buf();
    let (code, _, err) = agpj(&home, &["project", "create", "Demo"]);
    assert_eq!(code, 0, "{err}");
    (root, home)
}

fn project_dir(home: &Path) -> PathBuf {
    fs::read_dir(home.join("shared/projects"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
}

fn release(home: &Path, version: &str) -> PathBuf {
    project_dir(home).join(format!("docs/release/{version}.md"))
}

fn add(home: &Path, title: &str, version: &str, conditions: &[(&str, bool)]) -> i64 {
    let mut desc = String::from("---\nacceptance:\n");
    for (i, (text, done)) in conditions.iter().enumerate() {
        desc.push_str(&format!(
            "  - id: AC-{}\n    text: {text}\n    done: {done}\n",
            i + 1
        ));
    }
    desc.push_str("---\n\nBody.\n");
    let file = home.join(format!("desc-{}.md", title.replace(' ', "-")));
    fs::write(&file, desc).unwrap();
    run(
        home,
        &[
            "add",
            title,
            "--assignee",
            "ai",
            "--actor",
            "human",
            "--planned-version",
            version,
            "--desc-file",
            file.to_str().unwrap(),
        ],
    )["task"]["id"]
        .as_i64()
        .unwrap()
}

fn generate(home: &Path, version: &str) -> Value {
    run(home, &["release", "doc", "--version", version])
}

fn section(text: &str, title: &str) -> String {
    let from = text.find(&format!("### {title}\n")).unwrap();
    text[from..].split("\n### ").next().unwrap().to_owned()
}

fn finish(home: &Path, id: i64) {
    let id = id.to_string();
    for to in ["doing", "done"] {
        run(
            home,
            &["mv", &id, to, "--actor", "human", "--reason", "Step"],
        );
    }
}

#[test]
fn a_done_task_without_conditions_has_its_own_class() {
    let (_root, home) = demo();
    run(&home, &["version", "add", "v1"]);
    let bare = add(&home, "No conditions", "v1", &[]);
    let checked = add(&home, "All met", "v1", &[("Works", true)]);
    let partial = add(&home, "Part met", "v1", &[("Works", true), ("Docs", false)]);
    let todo = add(&home, "Not started", "v1", &[]);
    for id in [bare, checked, partial] {
        finish(&home, id);
    }
    let out = generate(&home, "v1");
    assert_eq!(out["planned"], 5);
    assert_eq!(out["verified"], 1);
    assert_eq!(out["doneWithoutAcceptance"], 1);
    assert_eq!(out["remaining"], 3);
    let text = fs::read_to_string(release(&home, "v1")).unwrap();
    assert!(text.contains("- Done without acceptance conditions: 1"));
    let unchecked = section(&text, "Done without acceptance conditions");
    assert!(unchecked.contains(&format!("#{bare} No conditions")));
    let remaining = section(&text, "Remaining");
    assert!(!remaining.contains(&format!("#{bare} ")));
    assert!(remaining.contains(&format!("#{partial} Part met")));
    assert!(remaining.contains(&format!("#{todo} Not started")));
    assert!(section(&text, "Verified").contains(&format!("#{checked} All met")));
    assert!(!section(&text, "Verified").contains(&format!("#{bare} ")));
    let (_, human, _) = agpj(
        &home,
        &["--project", "Demo", "release", "doc", "--version", "v1"],
    );
    assert!(human.contains("1 verified, 1 done without acceptance conditions, 3 remaining"));
}

#[test]
fn a_task_moved_back_to_an_earlier_version_names_where_it_went() {
    let (_root, home) = demo();
    run(&home, &["version", "add", "v1"]);
    run(&home, &["version", "add", "v2"]);
    let back = add(&home, "Moved back", "v2", &[("Works", false)]);
    let id = back.to_string();
    run(&home, &["edit", &id, "--planned-version", "v1"]);
    generate(&home, "v2");
    let text = fs::read_to_string(release(&home, "v2")).unwrap();
    assert!(text.contains("- Moved to another version: 1"));
    assert!(!text.contains("next version"));
    let moved = section(&text, "Moved to another version");
    assert!(moved.contains(&format!("#{back} Moved back")));
    assert!(moved.contains("Moved to: v1"));
}
