//! The events of `message ask` and `message wait`: one line each, as JSON Lines or as English
//! sentences.

use std::io::Write;

use serde_json::{Value, json};

use super::answer_text;
use super::stream::{EXIT_TIMEOUT, EXIT_WITHDRAWN};
use crate::agent::message_wait::{Heartbeat, MessageWaitEnd};
use crate::cli::output;
use crate::error::AgpjError;
use crate::file_store::qa::{MessageState, Question, QuestionView};

/// Writes the events, as JSON Lines or as English sentences.
pub(super) struct Events<'a> {
    pub(super) out: &'a mut dyn Write,
    pub(super) json: bool,
    /// The project folder, where the images of an answer are found. It is empty for the
    /// events that print no answer.
    pub(super) project_dir: std::path::PathBuf,
}

pub(super) fn ids_of(questions: &[Question]) -> Vec<&str> {
    questions.iter().map(|q| q.id.as_str()).collect()
}

/// The answers as the end events print them.
fn answers_of(questions: &[Question], project_dir: &std::path::Path) -> (Vec<Value>, Vec<String>) {
    let mut values = Vec::new();
    let mut lines = Vec::new();
    for question in questions {
        let view = QuestionView::of(question, Vec::new()).with_image_paths(project_dir);
        let Some(reply) = &view.reply else { continue };
        values.push(json!({
            "id": view.id,
            "choiceId": reply.choice_id,
            "choiceLabel": reply.choice_label,
            "text": reply.text,
            "images": reply.images,
            "imagePaths": reply.image_paths,
            "via": reply.via,
            "at": crate::clock::format_utc(reply.at),
        }));
        let mut line = format!("Answered {}: {}", view.id, answer_text(reply));
        for path in &reply.image_paths {
            line.push_str(&format!("\nImage: {}", path.display()));
        }
        lines.push(line);
    }
    (values, lines)
}

impl Events<'_> {
    pub(super) fn line(&mut self, value: Value, human: String) {
        let text = if self.json {
            format!("{}\n", output::render_json(&value))
        } else {
            format!("{human}\n")
        };
        let _ = self.out.write_all(text.as_bytes());
        let _ = self.out.flush();
    }

    pub(super) fn registered(&mut self, questions: &[Question], resumed: bool) {
        let ids = ids_of(questions);
        let listed: Vec<Value> = ids.iter().map(|id| json!({"id": id})).collect();
        let count = ids.len();
        let noun = if count == 1 { "question" } else { "questions" };
        let human = if resumed {
            format!("Waiting again for {}.", ids.join(", "))
        } else {
            format!("Asked {count} {noun}: {}.", ids.join(", "))
        };
        self.line(
            json!({"type": "registered", "questions": listed, "resumed": resumed}),
            human,
        );
    }

    /// The questions were kept back (sleep mode): nothing waits and nothing reaches the person.
    pub(super) fn held(&mut self, ids: &[String]) {
        let listed: Vec<Value> = ids.iter().map(|id| json!({"id": id})).collect();
        let count = ids.len();
        let noun = if count == 1 { "question" } else { "questions" };
        self.line(
            json!({"type": "held", "questions": listed}),
            format!(
                "Held {count} {noun}: {}. Sleep mode keeps them back and they reach the user all at once when sleep ends. Do not wait; continue with other work. Withdraw one you no longer need with message withdraw <id>.",
                ids.join(", ")
            ),
        );
    }

    pub(super) fn waiting(&mut self, ids: &[String], beat: Heartbeat) {
        let left = match beat.remaining {
            Some(seconds) if seconds >= 60 => format!(", {}m left", seconds / 60),
            Some(seconds) => format!(", {seconds}s left"),
            None => String::new(),
        };
        self.line(
            json!({"type": "waiting", "ids": ids, "elapsed": beat.elapsed, "remaining": beat.remaining, "answered": beat.answered, "total": beat.total}),
            format!(
                "Waiting for answers: {} of {} answered{left}.",
                beat.answered, beat.total
            ),
        );
    }

    /// The last line; returns the exit code.
    pub(super) fn end(&mut self, end: MessageWaitEnd) -> i32 {
        match end {
            MessageWaitEnd::Answered(questions) => {
                let (answers, lines) = answers_of(&questions, &self.project_dir);
                self.line(
                    json!({"type": "answered", "ids": ids_of(&questions), "answers": answers}),
                    lines.join("\n"),
                );
                0
            }
            MessageWaitEnd::Timeout(questions) => {
                let answered = questions
                    .iter()
                    .filter(|q| q.state() == MessageState::Closed)
                    .count();
                let total = questions.len();
                let open: Vec<&str> = questions
                    .iter()
                    .filter(|q| q.state() == MessageState::Open)
                    .map(|q| q.id.as_str())
                    .collect();
                self.line(
                    json!({"type": "timeout", "ids": ids_of(&questions), "answered": answered, "total": total}),
                    format!(
                        "Timed out: {answered} of {total} answered. The questions stay open; wait again with message wait {}.",
                        open.join(" ")
                    ),
                );
                EXIT_TIMEOUT
            }
            MessageWaitEnd::Withdrawn(questions) => {
                let withdrawn: Vec<&str> = questions
                    .iter()
                    .filter(|q| q.state() == MessageState::Withdrawn)
                    .map(|q| q.id.as_str())
                    .collect();
                let (answers, lines) = answers_of(&questions, &self.project_dir);
                let mut human = lines;
                human.push(format!("Withdrawn: {}.", withdrawn.join(", ")));
                self.line(
                    json!({"type": "withdrawn", "ids": withdrawn, "answers": answers}),
                    human.join("\n"),
                );
                EXIT_WITHDRAWN
            }
        }
    }

    /// A failure after the questions were asked: they stay.
    pub(super) fn error(&mut self, error: &AgpjError, stderr: &mut dyn Write) -> i32 {
        if self.json {
            self.line(
                json!({"type": "error", "error": {"code": error.code(), "message": error.message(), "reason": error.reason()}}),
                String::new(),
            );
        } else {
            let _ = stderr.write_all(crate::cli::errors::human(error).as_bytes());
        }
        error.exit_code()
    }
}
