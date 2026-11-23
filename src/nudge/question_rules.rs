//! Turn rules about questions put on the board with `message ask`: images,
//! the copy in the chat, and waiting for the answer.

use super::commands::{ends_with_question, runs_agpj, tokens};
use super::rules::produced_image;
use super::{Call, Facts, Finding};

fn is_ask(call: &Call) -> bool {
    runs_agpj(&call.command, &[&["message", "ask"]])
}

/// A real ask: not a call that only prints the help text.
fn is_real_ask(call: &Call) -> bool {
    is_ask(call)
        && !tokens(&call.command)
            .iter()
            .any(|t| matches!(*t, "--help" | "-h"))
}

/// The ask carries an image: an `images` field or an `attachment:` reference.
fn carries_image(call: &Call) -> bool {
    let lower = call.command.to_ascii_lowercase();
    lower.contains("images\"") || lower.contains("attachment:")
}

/// The turn produced an image and asked a question that carries none.
pub fn question_images(facts: &Facts) -> Vec<Finding> {
    let calls = facts.calls();
    if !calls.iter().any(is_real_ask)
        || !calls.iter().any(produced_image)
        || calls
            .iter()
            .any(|c| carries_image(c) || runs_agpj(&c.command, &[&["attachment", "add"]]))
    {
        return Vec::new();
    }
    vec![Finding::new(facts.lang.pick(
        "You produced an image and asked a question with no image. If the question is about something visual, attach the images (`agpj attachment add`, then `images` or `![](attachment:<id>)`).",
        "画像を生成したのに、画像のない質問を起票しました。見た目についての質問なら、画像を添付してください(`agpj attachment add` のあと、`images` または `![](attachment:<id>)` で指定します)。",
    ))]
}

/// The turn asked a question on the board and the reply never shows its Q-id.
pub fn question_in_chat(facts: &Facts) -> Vec<Finding> {
    let Some(reply) = facts.reply() else {
        return Vec::new();
    };
    if !facts.calls().iter().any(is_real_ask) || mentions_question_id(reply) {
        return Vec::new();
    }
    vec![Finding::new(facts.lang.pick(
        "You asked a question on the board but the reply shows no Q-id. Show the question with its Q-id in the chat too.",
        "ボードに質問を起票しましたが、返答に Q-id がありません。質問を Q-id つきでチャットにも載せてください。",
    ))]
}

/// The turn asked with `--no-wait` and never waited for the answer.
pub fn question_wait(facts: &Facts) -> Vec<Finding> {
    let calls = facts.calls();
    let Some(last_ask) = calls
        .iter()
        .rposition(|c| is_real_ask(c) && tokens(&c.command).contains(&"--no-wait"))
    else {
        return Vec::new();
    };
    if calls[last_ask..]
        .iter()
        .any(|c| runs_agpj(&c.command, &[&["message", "wait"]]))
    {
        return Vec::new();
    }
    vec![Finding::new(facts.lang.pick(
        "You asked with `--no-wait` and never waited. Wait for the answer with `agpj message wait <Q-id>` in the background, or explain why you do not.",
        "`--no-wait` で質問しましたが、回答を待っていません。バックグラウンドで `agpj message wait <Q-id>` を実行して回答を待つか、待たない理由を説明してください。",
    ))]
}

/// Whether `text` holds a `Q-<n>` reference.
fn mentions_question_id(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.windows(3).enumerate().any(|(i, w)| {
        w[0] == b'Q'
            && w[1] == b'-'
            && w[2].is_ascii_digit()
            && (i == 0 || !bytes[i - 1].is_ascii_alphanumeric())
    })
}

/// The last reply is a question to the user and none was put on the board.
pub fn question(facts: &Facts) -> Vec<Finding> {
    let Some(reply) = facts.reply() else {
        return Vec::new();
    };
    // Without a readable transcript the turn cannot be checked.
    if !ends_with_question(reply)
        || facts.turn.is_none()
        || facts
            .calls()
            .iter()
            .any(|c| runs_agpj(&c.command, &[&["message", "ask"], &["message", "answer"]]))
    {
        return Vec::new();
    }
    vec![
        Finding::new(facts.lang.pick(
            "Does your question wait on the user's answer? If so, put it on the board with `agpj message ask` and show it in the chat too.",
            "この質問はユーザーの回答を待つものですか。待つ場合は `agpj message ask` でボードに起票し、チャットにも載せてください。",
        )),
        Finding::new(facts.lang.pick(
            "Does it ask about something visual (a screen, a mock, a design)? Attach the images to the question.",
            "見た目(画面、モック、デザイン)についての質問ですか。その場合は質問に画像を添付してください。",
        )),
    ]
}
