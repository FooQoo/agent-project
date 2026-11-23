//! `message ask` and `message wait`: one event per line, from `registered` to the end of the wait.

use super::events::{Events, ids_of};
use super::{data_dir, input, notify};
use crate::agent::message_wait::{MessageWaitEnd, read_questions, wait_for_questions};
use crate::cli::Context;
use crate::cli::Env;
use crate::cli::args::{MessageAskArgs, MessageWaitArgs};
use crate::cli::run::fail;
use crate::error::AgpjError;
use crate::file_store::qa::MessageState;

/// The exit code of a wait that ran out of time; the questions stay open.
pub const EXIT_TIMEOUT: i32 = 5;
/// The exit code of a wait that ended with a withdrawn question.
pub const EXIT_WITHDRAWN: i32 = 6;

/// A wait needs the server's event stream; it is checked before anything is
/// saved. When it is not running and starting is allowed, it is started now,
/// so a server that cannot start leaves no question behind. A server started
/// now opens the project's board.
fn ensure_serving(ctx: &Context) -> Result<(), AgpjError> {
    let dir = data_dir(ctx.files.shared_dir());
    if crate::autostart::enabled() {
        return crate::autostart::ensure_serving(&dir, Some(ctx.project_id()?)).map(|_| ());
    }
    let running =
        crate::paths::read_serve_info_in(&dir).is_some_and(|info| crate::server::is_serving(&info));
    if running {
        Ok(())
    } else {
        Err(AgpjError::BoardUnavailable {
            reason:
                "A wait needs a running server. Run \"agpj serve\" first, or ask with --no-wait."
                    .into(),
        })
    }
}

fn wait_and_print(
    ctx: &Context,
    env: &mut Env,
    ids: &[String],
    timeout: Option<u64>,
    heartbeat: u64,
) -> i32 {
    let project = match ctx.project_id() {
        Ok(project) => project.clone(),
        Err(error) => return fail(env, &error, ctx.json),
    };
    let mut events = Events {
        out: &mut *env.stdout,
        json: ctx.json,
        project_dir: crate::file_store::layout::project_dir(ctx.files.shared_dir(), &project),
    };
    let result = wait_for_questions(
        ctx.files.shared_dir(),
        project.clone(),
        ids,
        timeout,
        heartbeat,
        |beat| events.waiting(ids, beat),
    );
    let (code, handed) = match result {
        Ok(end) => {
            let handed = !matches!(end, MessageWaitEnd::Timeout(_));
            (events.end(end), handed)
        }
        Err(error) => return events.error(&error, &mut *env.stderr),
    };
    // The answers just printed are in the agent's hands: they leave the progress table and the
    // hook. A failure here loses nothing; the answers stay listed until they are accepted.
    if handed
        && let Ok(written) = ctx.files.accept_answers(&project, ids, ctx.clock.now())
        && !written.is_empty()
    {
        notify(ctx, env);
    }
    code
}

pub(super) fn ask(ctx: &mut Context, env: &mut Env, args: &MessageAskArgs) -> i32 {
    let prepared = (|| {
        let project = ctx.project_id()?.clone();
        // In sleep mode nothing reaches the board, the notifications, or the chat.
        let (brainstorm, held) =
            super::brainstorm::target_for_ask(ctx.files.shared_dir(), &project, args.brainstorm)?;
        let mut input = input::ask_input(&args.questions_json, args.actor.actor(), &args.tasks)?;
        input.brainstorm = brainstorm;
        input.held = held;
        let timeout = input::timeout(args.timeout.as_deref())?;
        // A held question is never waited for, so no server is needed.
        if !args.no_wait && !held {
            ensure_serving(ctx)?;
        }
        let questions = ctx.files.ask_questions(&project, input, ctx.clock.now())?;
        Ok::<_, AgpjError>((questions, timeout, held))
    })();
    let (questions, timeout, held) = match prepared {
        Ok(found) => found,
        Err(error) => return fail(env, &error, ctx.json),
    };
    let mut events = Events {
        out: &mut *env.stdout,
        json: ctx.json,
        project_dir: std::path::PathBuf::new(),
    };
    if held {
        events.held(
            &ids_of(&questions)
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>(),
        );
        // The board counts the held questions; nothing else learns of them.
        notify(ctx, env);
        return 0;
    }
    events.registered(&questions, false);
    // The page learns of the questions now, not after the answers.
    notify(ctx, env);
    if args.no_wait {
        return 0;
    }
    let ids: Vec<String> = questions.iter().map(|q| q.id.clone()).collect();
    wait_and_print(ctx, env, &ids, timeout, args.heartbeat)
}

pub(super) fn wait(ctx: &mut Context, env: &mut Env, args: &MessageWaitArgs) -> i32 {
    let prepared = (|| {
        let timeout = input::timeout(args.timeout.as_deref())?;
        let project = ctx.project_id()?.clone();
        let _ = crate::mode::release_held(ctx.files.shared_dir(), &project);
        let questions = read_questions(ctx.files.shared_dir(), &project, &args.ids)?;
        // Questions that have ended need no server.
        if questions.iter().any(|q| q.state() == MessageState::Open) {
            ensure_serving(ctx)?;
        }
        Ok::<_, AgpjError>((questions, timeout))
    })();
    let (questions, timeout) = match prepared {
        Ok(found) => found,
        Err(error) => return fail(env, &error, ctx.json),
    };
    // A held question cannot be waited for: it is not open until sleep ends.
    let held: Vec<String> = questions
        .iter()
        .filter(|q| q.state() == MessageState::Held)
        .map(|q| q.id.clone())
        .collect();
    let mut events = Events {
        out: &mut *env.stdout,
        json: ctx.json,
        project_dir: std::path::PathBuf::new(),
    };
    if !held.is_empty() {
        events.held(&held);
        return 0;
    }
    events.registered(&questions, true);
    wait_and_print(ctx, env, &args.ids, timeout, args.heartbeat)
}
