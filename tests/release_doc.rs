//! Generating a version's release document: `docs/release/<version>.md`. Each test runs the real
//! binary against a temporary data area.

mod common;

use std::fs;
use std::path::{Path, PathBuf};

use common::{agboard, json_error, json_ok};
use serde_json::Value;

const START: &str = "<!-- agboard:generated:start -->";
const END: &str = "<!-- agboard:generated:end -->";

fn run(home: &Path, args: &[&str]) -> Value {
    let mut all = vec!["--project", "Demo"];
    all.extend(args);
    all.push("--json");
    json_ok(home, &all)
}

fn demo() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().to_path_buf();
    let (code, _, err) = agboard(&home, &["project", "create", "Demo"]);
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

#[test]
fn a_new_project_has_a_release_folder() {
    let (_root, home) = demo();
    assert!(project_dir(&home).join("docs/release").is_dir());
}

#[test]
fn generating_groups_tasks_and_keeps_hand_written_text_on_regeneration() {
    let (_root, home) = demo();
    run(&home, &["version", "add", "v1"]);
    run(&home, &["version", "add", "v2"]);
    let done = add(&home, "Verified work", "v1", &[("Works", true)]);
    let open = add(
        &home,
        "Open work",
        "v1",
        &[("Works", true), ("Docs", false)],
    );
    let moved = add(&home, "Moved work", "v1", &[("Works", false)]);
    run(
        &home,
        &["edit", &moved.to_string(), "--planned-version", "v2"],
    );
    let out = generate(&home, "v1");
    assert_eq!(out["path"], "docs/release/v1.md");
    assert_eq!(
        (
            &out["planned"],
            &out["verified"],
            &out["remaining"],
            &out["moved"]
        ),
        (
            &Value::from(3),
            &Value::from(1),
            &Value::from(2),
            &Value::from(1)
        )
    );
    assert_eq!(out["published"], 0);
    let text = fs::read_to_string(release(&home, "v1")).unwrap();
    assert!(text.contains(START) && text.contains(END));
    assert!(text.contains("## Release status of v1"));
    let section = |title: &str| {
        let from = text.find(&format!("### {title}")).unwrap();
        text[from..].split("\n### ").next().unwrap().to_owned()
    };
    assert!(section("Verified").contains(&format!("#{done} Verified work")));
    assert!(section("Remaining").contains(&format!("#{open} Open work")));
    let moved_section = section("Moved to another version");
    assert!(moved_section.contains(&format!("#{moved} Moved work")));
    assert!(moved_section.contains("Moved to: v2"));
    assert!(section("Not confirmed as published").contains(&format!("#{done} Verified work")));
    assert!(section("Published").contains("No task has a publication record."));

    // A hand-written paragraph outside the block survives a regeneration, and the block is
    // not duplicated.
    let edited = text.replacen(START, &format!("My own note.\n\n{START}"), 1);
    fs::write(release(&home, "v1"), edited).unwrap();
    let again = generate(&home, "v1");
    assert_eq!(again["changed"], false);
    let after = fs::read_to_string(release(&home, "v1")).unwrap();
    assert!(after.contains("My own note."));
    assert_eq!(after.matches(START).count(), 1);

    // The board changes: only the block changes.
    run(
        &home,
        &["edit", &open.to_string(), "--planned-version", "v2"],
    );
    let third = generate(&home, "v1");
    assert_eq!(third["changed"], true);
    let after = fs::read_to_string(release(&home, "v1")).unwrap();
    assert!(after.contains("My own note."));
    assert_eq!(after.matches("### Planned").count(), 1);

    // Another version gets its own file; the first is untouched.
    let before = fs::read_to_string(release(&home, "v1")).unwrap();
    let v2 = generate(&home, "v2");
    assert_eq!(v2["planned"], 3);
    assert!(release(&home, "v2").is_file());
    assert_eq!(fs::read_to_string(release(&home, "v1")).unwrap(), before);
}

#[test]
fn an_unmarked_file_keeps_its_text_and_gets_the_block_appended() {
    let (_root, home) = demo();
    run(&home, &["version", "add", "v1"]);
    fs::write(release(&home, "v1"), "My hand-written notes.\n").unwrap();
    generate(&home, "v1");
    let text = fs::read_to_string(release(&home, "v1")).unwrap();
    assert!(text.starts_with("My hand-written notes.\n\n"));
    assert_eq!(text.matches(START).count(), 1);
}

#[test]
fn damaged_markers_are_refused_and_the_file_is_left_as_it_was() {
    let (_root, home) = demo();
    run(&home, &["version", "add", "v1"]);
    let broken = format!("keep me\n{START}\nhalf\n");
    fs::write(release(&home, "v1"), &broken).unwrap();
    let (code, error) = json_error(
        &home,
        &[
            "--project",
            "Demo",
            "release",
            "doc",
            "--version",
            "v1",
            "--json",
        ],
    );
    assert_eq!(code, 1);
    assert!(error.contains("release_doc_refused"), "{error}");
    assert_eq!(fs::read_to_string(release(&home, "v1")).unwrap(), broken);
}

#[test]
fn the_current_version_needs_to_be_set_and_writes_nothing_without_it() {
    let (_root, home) = demo();
    let (code, error) = json_error(
        &home,
        &[
            "--project",
            "Demo",
            "release",
            "doc",
            "--current-version",
            "--json",
        ],
    );
    assert_eq!(code, 1);
    assert!(error.contains("no_current_version"), "{error}");
    assert!(
        fs::read_dir(project_dir(&home).join("docs/release"))
            .unwrap()
            .next()
            .is_none()
    );

    run(&home, &["version", "current", "v1.8.0"]);
    let out = run(&home, &["release", "doc", "--current-version"]);
    assert_eq!(out["version"], "v1.8.0");
    assert!(release(&home, "v1.8.0").is_file());
}

#[test]
fn archived_tasks_are_included_and_a_named_unlisted_version_is_listed() {
    let (_root, home) = demo();
    run(&home, &["version", "add", "v1"]);
    let id = add(&home, "Shipped", "v1", &[("Works", true)]);
    run(
        &home,
        &[
            "mv",
            &id.to_string(),
            "done",
            "--actor",
            "ai",
            "--reason",
            "Done",
        ],
    );
    let store = agent_board::file_store::FileStore::open(&home.join("shared"));
    let project = store.project_by_name("Demo").unwrap().id;
    store
        .archive_task(&project, id, chrono::Utc::now())
        .unwrap();
    let out = generate(&home, "v1");
    assert_eq!(out["verified"], 1);
    let text = fs::read_to_string(release(&home, "v1")).unwrap();
    assert!(text.contains("archived"));

    let new = generate(&home, "v9");
    assert_eq!(new["created"], true);
    assert!(release(&home, "v9").is_file());
    // The release-notes card issued with the version is planned for it.
    assert_eq!(new["planned"], 1);
}

#[test]
fn only_a_linked_publication_record_counts_as_published() {
    let (_root, home) = demo();
    run(&home, &["version", "add", "v1"]);
    let linked = add(&home, "Linked", "v1", &[("Works", true)]);
    let unlinked = add(&home, "Unlinked", "v1", &[("Works", true)]);
    let other = add(&home, "Other version", "v1", &[("Works", true)]);
    let bare = add(&home, "Done only", "v1", &[("Works", true)]);
    let result = |id: i64, summary: &str, link: Option<&str>| {
        let id = id.to_string();
        let mut args = vec![
            "result",
            "add",
            &id,
            "--actor",
            "ai",
            "--final",
            "--summary",
            summary,
        ];
        if let Some(link) = link {
            args.extend(["--link", link]);
        }
        run(&home, &args);
    };
    result(
        linked,
        "Shipped.\nPublished: v1",
        Some("https://example.com/releases/v1"),
    );
    result(unlinked, "Published: v1", None);
    result(
        other,
        "Published: v2",
        Some("https://example.com/releases/v2"),
    );
    for id in [linked, unlinked, other, bare] {
        let id = id.to_string();
        run(
            &home,
            &["mv", &id, "doing", "--actor", "ai", "--reason", "Start"],
        );
        run(
            &home,
            &["mv", &id, "done", "--actor", "ai", "--reason", "Finish"],
        );
    }
    let out = generate(&home, "v1");
    assert_eq!(
        (&out["planned"], &out["published"]),
        (&Value::from(5), &Value::from(1))
    );
    let text = fs::read_to_string(release(&home, "v1")).unwrap();
    assert!(text.contains("- Published (with a publication record): 1 / 5"));
    let from = text.find("### Published\n").unwrap();
    let rest = &text[from..];
    let (published, unconfirmed) = rest.split_once("### Not confirmed as published").unwrap();
    assert!(published.contains(&format!("#{linked} Linked")));
    assert!(published.contains("https://example.com/releases/v1"));
    for id in [unlinked, other, bare] {
        assert!(unconfirmed.contains(&format!("#{id} ")), "{id}");
        assert!(!published.contains(&format!("#{id} ")), "{id}");
    }
    assert!(unconfirmed.contains("states the publication but has no link"));
    assert!(unconfirmed.contains("no publication record"));
}

#[cfg(unix)]
#[test]
fn a_linked_release_folder_is_refused_and_the_outside_file_is_left_as_it_was() {
    let (_root, home) = demo();
    run(&home, &["version", "add", "v1"]);
    let outside = home.join("outside");
    fs::create_dir_all(&outside).unwrap();
    fs::write(outside.join("v1.md"), "outside\n").unwrap();
    let folder = project_dir(&home).join("docs/release");
    fs::remove_dir_all(&folder).unwrap();
    std::os::unix::fs::symlink(&outside, &folder).unwrap();
    let (code, error) = json_error(
        &home,
        &[
            "--project",
            "Demo",
            "release",
            "doc",
            "--version",
            "v1",
            "--json",
        ],
    );
    assert_eq!(code, 1);
    assert!(error.contains("release_doc_refused"), "{error}");
    assert_eq!(
        fs::read_to_string(outside.join("v1.md")).unwrap(),
        "outside\n"
    );
}
