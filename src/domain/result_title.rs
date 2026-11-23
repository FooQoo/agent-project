//! The heading of a result recorded without a title.

/// The longest result title, in characters.
pub const MAX_RESULT_TITLE_CHARS: usize = 80;

/// The title of a result that was recorded without one: the first sentence of
/// the summary, cut to `MAX_RESULT_TITLE_CHARS` characters (the last one is
/// `…` when it was cut). A sentence ends at the first line break, at `。`, `！`
/// or `？`, or at `.`, `!` or `?` followed by white space or the end.
pub fn derive_result_title(summary: &str) -> String {
    let line = crate::domain::first_line_plain(summary);
    let chars: Vec<char> = line.chars().collect();
    let mut end = chars.len();
    for (i, c) in chars.iter().enumerate() {
        let ideographic = matches!(c, '。' | '！' | '？');
        let latin =
            matches!(c, '.' | '!' | '?') && chars.get(i + 1).is_none_or(|n| n.is_whitespace());
        if ideographic || latin {
            end = i + 1;
            break;
        }
    }
    let sentence: Vec<char> = chars[..end].to_vec();
    if sentence.len() > MAX_RESULT_TITLE_CHARS {
        let mut cut: String = sentence[..MAX_RESULT_TITLE_CHARS - 1].iter().collect();
        cut.push('…');
        cut
    } else {
        sentence.into_iter().collect()
    }
}
