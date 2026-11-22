use super::*;

#[test]
fn detects_a_final_question() {
    assert!(ends_with_question("Done.\n\nShould I go ahead?"));
    assert!(ends_with_question("進めてよいですか？\n"));
    assert!(ends_with_question("Shall I continue? **\n\n"));
    assert!(ends_with_question("(Is that right?)"));
    assert!(!ends_with_question("Why? Because it is faster."));
    assert!(!ends_with_question("All done."));
    assert!(!ends_with_question("What next?\nDone."));
    assert!(!ends_with_question("```\nx?\n```"));
    assert!(!ends_with_question("  \n"));
}

#[test]
fn agpj_commands_are_recognized_by_tokens() {
    let ask: &[&[&str]] = &[&["message", "ask"], &["message", "answer"]];
    assert!(runs_agpj(
        "agpj --project \"x\" message ask --actor ai",
        ask
    ));
    assert!(runs_agpj("/usr/bin/agpj message answer Q-1", ask));
    assert!(runs_agpj(
        r#"tools.exec_command({cmd: "agpj message ask"})"#,
        ask
    ));
    assert!(!runs_agpj("agpj message show Q-1", ask));
    assert!(!runs_agpj("echo message ask", ask));
}
