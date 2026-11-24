//! Keeping the question of an image when a question body is copied into task history.
//!
//! A question body shows its images as `![](attachments/NAME)`, which only makes sense next to
//! its own `qa/<id>/attachments/` folder. A task history entry has no such folder, and two
//! questions can each have an image of the same name, so the copy names the question:
//! `![](attachments/Q-348/NAME)`. The reference stays relative so stored files stay portable.

const PREFIX: &str = "](attachments/";

/// Whether `name` is a plain file name, by the rule the attachment route applies.
fn is_plain_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.starts_with('.')
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

/// `body` with each `](attachments/NAME)` rewritten to `](attachments/<id>/NAME)`, for the
/// names that are plain file names and that `exists` reports as attachments of the question.
/// Any other reference is left as it is.
pub(super) fn qualify_attachment_refs(
    body: &str,
    id: &str,
    exists: impl Fn(&str) -> bool,
) -> String {
    let mut out = String::with_capacity(body.len());
    let mut rest = body;
    while let Some(at) = rest.find(PREFIX) {
        let (head, tail) = rest.split_at(at + PREFIX.len());
        out.push_str(head);
        let end = tail
            .find(|c: char| c == ')' || c.is_whitespace())
            .unwrap_or(tail.len());
        let name = &tail[..end];
        if is_plain_name(name) && exists(name) {
            out.push_str(id);
            out.push('/');
        }
        rest = tail;
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has(name: &str) -> bool {
        matches!(name, "shot.png" | "a-b_1.webp")
    }

    #[test]
    fn keeps_the_question_in_a_reference() {
        let body = "See ![](attachments/shot.png) and ![x](attachments/a-b_1.webp \"t\").";
        assert_eq!(
            qualify_attachment_refs(body, "Q-348", has),
            "See ![](attachments/Q-348/shot.png) and ![x](attachments/Q-348/a-b_1.webp \"t\")."
        );
    }

    #[test]
    fn two_questions_with_one_name_stay_apart() {
        let a = qualify_attachment_refs("![](attachments/shot.png)", "Q-1", has);
        let b = qualify_attachment_refs("![](attachments/shot.png)", "Q-2", has);
        assert_ne!(a, b);
    }

    #[test]
    fn leaves_other_references_alone() {
        for body in [
            "![](attachments/other.png)",
            "![](attachments/../shot.png)",
            "![](attachments/dir/shot.png)",
            "![](attachments/.hidden)",
            "![](https://example.com/attachments/shot.png)",
            "![](attachment:abc)",
            "![](attachments/)",
        ] {
            assert_eq!(qualify_attachment_refs(body, "Q-9", has), body);
        }
    }
}
