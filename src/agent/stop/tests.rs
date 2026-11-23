use super::*;
use crate::file_store::FileStore;
use crate::nudge::test_support::{Scenario, call, progress_with};

fn decide_for(scenario: &Scenario) -> Option<String> {
    decide(&scenario.facts(), RULES)
}

fn reason(out: &str) -> String {
    let v: serde_json::Value = serde_json::from_str(out).unwrap();
    assert_eq!(v["decision"], "block");
    v["reason"].as_str().unwrap().to_string()
}

#[test]
fn the_checklist_separates_found_items_from_checks() {
    let scenario = Scenario::turn(
        "Shall I merge?",
        vec![call("gh pr create --title x")],
        Some(progress_with(true, &[3])),
    );
    let reason = reason(&decide_for(&scenario).unwrap());
    assert!(reason.starts_with("Agent Project turn-end checklist."));
    let found = reason.find("\nFound:\n").unwrap();
    let check = reason.find("\nCheck:\n").unwrap();
    assert!(found < check);
    let (found_part, check_part) = (&reason[found..check], &reason[check..]);
    assert!(found_part.contains("- NG: Task #3 has all conditions met"));
    assert!(found_part.contains("- NG: You opened a pull request"));
    assert!(!found_part.contains("[ ]"));
    assert!(check_part.contains("- [ ] Does your question wait"));
    assert!(check_part.contains("- [ ] Record a decision that matters later"));
    assert!(!check_part.contains("NG:"));
}

#[test]
fn an_empty_section_is_left_out_and_no_item_means_no_block() {
    let only_checks = Scenario::turn("Shall I?", vec![call("ls")], None);
    let r = reason(&decide_for(&only_checks).unwrap());
    assert!(r.contains("Check:") && !r.contains("Found:"));
    let only_found = Scenario::turn("Done.", vec![], Some(progress_with(false, &[4])));
    let r = reason(&decide_for(&only_found).unwrap());
    assert!(r.contains("Found:") && !r.contains("Check:"));
    let none = Scenario::turn("All done.", vec![], Some(progress_with(true, &[])));
    assert!(decide_for(&none).is_none());
}

#[test]
fn an_active_stop_hook_never_blocks() {
    let home = tempfile::tempdir().unwrap();
    let shared = home.path().join("shared");
    let project = FileStore::open(&shared)
        .create_project("Demo", chrono::Utc::now())
        .unwrap()
        .id;
    // Without a transcript the question rule cannot match, so use a file.
    let path = home.path().join("t.jsonl");
    std::fs::write(
        &path,
        r#"{"type":"user","message":{"role":"user","content":"hi"}}"#,
    )
    .unwrap();
    let input = |active| StopFields {
        stop_hook_active: Some(active),
        last_assistant_message: Some("May I proceed?".into()),
        transcript_path: path.to_str().map(String::from),
        ..Default::default()
    };
    assert!(run(&input(false), &shared, &project).is_some());
    assert!(run(&input(true), &shared, &project).is_none());
}

fn ja(mut progress: crate::progress::Progress) -> crate::progress::Progress {
    progress.locale = Some("ja".into());
    progress
}

#[test]
fn a_japanese_project_gets_the_checklist_in_japanese() {
    let scenario = Scenario::turn(
        "マージしますか?",
        vec![call("gh pr create --title x")],
        Some(ja(progress_with(true, &[3]))),
    );
    let reason = reason(&decide_for(&scenario).unwrap());
    assert!(reason.starts_with("Agent Project のターン終了チェックリスト。"));
    let found = reason.find("\n見つかったこと:\n").unwrap();
    let check = reason.find("\n確認すること:\n").unwrap();
    assert!(found < check);
    let (found_part, check_part) = (&reason[found..check], &reason[check..]);
    assert!(found_part.contains("- NG: タスク #3 は受け入れ条件がすべて満たされています"));
    assert!(found_part.contains("`agpj result add 3 --final ...`"));
    assert!(found_part.contains("- NG: このターンでプルリクエストを作成しました"));
    assert!(check_part.contains("- [ ] この質問はユーザーの回答を待つものですか"));
    assert!(check_part.contains("`agpj message ask`"));
    assert!(!reason.contains("Found:") && !reason.contains("Check:"));
}

#[test]
fn commands_stay_verbatim_in_every_japanese_rule_text() {
    let no_wait = Scenario::turn(
        "Q-1 を起票しました。",
        vec![call("agpj message ask --no-wait --title x")],
        Some(ja(progress_with(false, &[]))),
    );
    let r = reason(&decide_for(&no_wait).unwrap());
    assert!(r.contains("`agpj message wait <Q-id>`"));
    let edited = Scenario::turn(
        "終わりました。",
        vec![crate::nudge::test_support::write_call("/work/src/a.rs")],
        Some(ja(progress_with(false, &[]))),
    );
    let r = reason(&decide_for(&edited).unwrap());
    assert!(r.contains("`agpj mv <id> doing`") && r.contains("`agpj state <id> running`"));
}

#[test]
fn english_stays_the_default_without_a_locale() {
    let scenario = Scenario::turn("Done.", vec![], Some(progress_with(false, &[4])));
    let r = reason(&decide_for(&scenario).unwrap());
    assert!(r.starts_with("Agent Project turn-end checklist."));
    assert!(r.contains("- NG: Task #4 has all conditions met"));
}
