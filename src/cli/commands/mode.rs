//! `mode`: show or switch the interaction mode of the project.

use serde_json::json;

use crate::cli::args::{ModeArg, ModeArgs};
use crate::cli::{Context, Outcome};
use crate::error::AgpjError;
use crate::mode::{self, ModeKind};

pub fn run(ctx: &mut Context, args: &ModeArgs) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?.to_string();
    let data_dir = mode::data_dir_of(ctx.files.shared_dir());
    let (saved, changed) = match args.mode {
        None => (mode::load(&data_dir, &project), false),
        Some(arg) => {
            let kind = match arg {
                ModeArg::Sparring => ModeKind::Sparring,
                ModeArg::Sleep => ModeKind::Sleep,
            };
            let saved = mode::set(&data_dir, &project, kind, ctx.clock.now()).map_err(|e| {
                AgpjError::FileError {
                    action: crate::error::FileAction::Write,
                    file: "modes.json".to_string(),
                    reason: e.to_string(),
                }
            })?;
            (saved, true)
        }
    };
    let human = match saved.kind {
        ModeKind::Sparring => {
            "Mode: sparring. Listen with \"agpj wait\" and ask the user with \"message ask\".\n"
        }
        ModeKind::Sleep => {
            "Mode: sleep. Questions to the user are refused; have a subagent decide and record it with \"agpj decision add\".\n"
        }
    };
    Ok(Outcome {
        changed,
        human: human.to_string(),
        json: json!({"mode": saved}),
    })
}
