//! One-line, length-limited text for the progress table.

/// Longest question text in the table, in characters.
const QUESTION_LIMIT: usize = 200;

/// Longest request title in the table, in characters.
pub(super) const TITLE_LIMIT: usize = 80;

/// The question on one line, cut to `QUESTION_LIMIT` characters with an
/// ellipsis when it was longer.
pub(super) fn short_question(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    cut(&flat, QUESTION_LIMIT)
}

/// Longest plan content or condition in the table, in characters.
const PLAN_LIMIT: usize = 300;

/// A plan's content or condition on one line, cut to `PLAN_LIMIT` characters.
pub(super) fn short_plan(text: &str) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    cut(&flat, PLAN_LIMIT)
}

/// The first non-empty line as plain text, cut to `TITLE_LIMIT` characters with an
/// ellipsis when it was longer.
pub(super) fn request_title(body: &str) -> String {
    cut(&crate::domain::first_line_plain(body), TITLE_LIMIT)
}

fn cut(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        return text.to_string();
    }
    let kept: String = text.chars().take(limit).collect();
    format!("{kept}…")
}
