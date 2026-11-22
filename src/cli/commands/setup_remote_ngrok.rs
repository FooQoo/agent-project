//! The ngrok-only questions of `agboard setup remote`.

use super::setup_remote::{Answers, cancel_false, confirm};
use crate::error::AgboardError;
use crate::remote_settings::{self as saved, Saved};

/// The ngrok-only questions: the fixed domain and the token.
/// Returns `false` when a question was cancelled, so the caller saves nothing.
pub(super) fn ask_ngrok(now: &Saved, answers: &mut Answers) -> Result<bool, AgboardError> {
    let change_domain = match &now.domain {
        Some(_) => match confirm("Change the fixed domain?") {
            Some(answer) => answer,
            None => return cancel_false(),
        },
        None => true,
    };
    if change_domain {
        let prompt = if now.domain.is_some() {
            "New fixed domain (empty removes it)"
        } else {
            "Fixed ngrok domain, such as example.ngrok-free.dev (empty skips)"
        };
        let typed: Result<String, _> = cliclack::input(prompt)
            .required(false)
            .validate(|text: &String| {
                if text.trim().is_empty() || saved::domain_of(text).is_ok() {
                    Ok(())
                } else {
                    Err("Enter a host name only, without https://, a path, or a port.")
                }
            })
            .interact();
        match typed {
            // Empty means "skip" when nothing is saved, and "remove" when something is.
            Ok(text) if text.trim().is_empty() && now.domain.is_none() => {}
            Ok(text) => answers.domain = Some(text),
            Err(_) => return cancel_false(),
        }
    }

    let change_token = if now.token_saved == Some(true) {
        match confirm("Replace the saved token?") {
            Some(answer) => answer,
            None => return cancel_false(),
        }
    } else {
        true
    };
    if change_token {
        let typed = cliclack::password("ngrok authtoken (typed hidden; empty skips)")
            .mask('*')
            .allow_empty()
            .validate(|text: &String| {
                if text.trim().is_empty() || saved::token_of(text).is_ok() {
                    Ok(())
                } else {
                    Err("The token must not contain spaces.")
                }
            })
            .interact();
        match typed {
            Ok(text) => answers.token = Some(text),
            Err(_) => return cancel_false(),
        }
    }

    Ok(true)
}
