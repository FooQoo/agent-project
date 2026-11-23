//! `mode`: show or switch the interaction mode of the project, and set its daily sleep window.

use serde_json::{Value, json};

use crate::cli::args::{ModeArg, ModeArgs};
use crate::cli::{Context, Outcome};
use crate::error::AgpjError;
use crate::mode::{self, ModeKind, Schedule, Source};
use crate::mode_schedule::{format_time, parse_time};

fn write_failed(error: std::io::Error) -> AgpjError {
    AgpjError::FileError {
        action: crate::error::FileAction::Write,
        file: "modes.json".to_string(),
        reason: error.to_string(),
    }
}

fn invalid(field: &str, reason: &str) -> AgpjError {
    AgpjError::InvalidValue {
        field: field.into(),
        reason: reason.into(),
    }
}

/// The window after `--start`, `--end`, or `--off` are applied to the saved one.
fn next_schedule(args: &ModeArgs, saved: Option<Schedule>) -> Result<Schedule, AgpjError> {
    let time = |field: &str, text: &Option<String>| match text {
        None => Ok(None),
        Some(text) => parse_time(text).map(Some).ok_or_else(|| {
            invalid(
                field,
                "Give the time as HH:MM in 24 hours, for example 23:00.",
            )
        }),
    };
    let start = time("start", &args.start)?.or(saved.map(|s| s.start));
    let end = time("end", &args.end)?.or(saved.map(|s| s.end));
    let (Some(start), Some(end)) = (start, end) else {
        return Err(invalid(
            "schedule",
            "Give both --start HH:MM and --end HH:MM the first time.",
        ));
    };
    if start == end {
        return Err(invalid("schedule", "The start and the end must differ."));
    }
    Ok(Schedule {
        enabled: !args.off,
        start,
        end,
    })
}

pub fn run(ctx: &mut Context, args: &ModeArgs) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?.to_string();
    let data_dir = mode::data_dir_of(ctx.files.shared_dir());
    let flags = args.start.is_some() || args.end.is_some() || args.off;
    if flags && args.mode != Some(ModeArg::Schedule) {
        return Err(invalid(
            "mode",
            "--start, --end, and --off go with \"mode schedule\".",
        ));
    }
    let changed = args.mode.is_some();
    match args.mode {
        None => {}
        Some(ModeArg::Sparring) | Some(ModeArg::Sleep) => {
            let kind = if args.mode == Some(ModeArg::Sleep) {
                ModeKind::Sleep
            } else {
                ModeKind::Sparring
            };
            mode::set(&data_dir, &project, kind, ctx.clock.now()).map_err(write_failed)?;
        }
        Some(ModeArg::Auto) => mode::clear_manual(&data_dir, &project).map_err(write_failed)?,
        Some(ModeArg::Schedule) => {
            mode::change_schedule(&data_dir, &project, |saved| {
                // "--off" with no window saved has nothing to turn off.
                if args.off && saved.is_none() && args.start.is_none() && args.end.is_none() {
                    return Ok(None);
                }
                next_schedule(args, saved).map(Some)
            })
            .map_err(write_failed)??;
        }
    }
    let state = mode::state(&data_dir, &project);
    let now = chrono::Local::now();
    let effective = state.effective_at(now);
    let mut human = match (effective.kind, effective.source) {
        (ModeKind::Sparring, _) => {
            "Mode: sparring. Listen with \"agpj wait\" and ask the user with \"message ask\".\n"
                .to_string()
        }
        (ModeKind::Sleep, Source::Manual) => {
            "Mode: sleep. Questions to the user are refused; have a subagent decide and record it with \"agpj decision add\".\n"
                .to_string()
        }
        (ModeKind::Sleep, Source::Schedule) => {
            let until = effective
                .until
                .map(|time| format_time(time.with_timezone(&chrono::Local).time()))
                .unwrap_or_default();
            format!(
                "Mode: sleep, by the schedule until {until}. Questions to the user are refused; have a subagent decide and record it with \"agpj decision add\".\n"
            )
        }
    };
    if let Some(schedule) = state.schedule {
        human.push_str(&format!(
            "Schedule: {}-{} ({}).\n",
            format_time(schedule.start),
            format_time(schedule.end),
            if schedule.enabled { "on" } else { "off" }
        ));
    }
    let mut mode_json =
        json!({"kind": effective.kind.as_str(), "source": effective.source.as_str()});
    let api = state.api_json(now);
    for key in ["since", "until"] {
        if let Some(value) = api.get(key) {
            mode_json[key] = value.clone();
        }
    }
    Ok(Outcome {
        changed,
        human,
        json: json!({
            "mode": mode_json,
            "schedule": state.schedule.map_or(Value::Null, |s| s.to_json()),
        }),
    })
}
