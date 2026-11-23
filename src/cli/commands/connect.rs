//! `connect` and `agent ls`: joining this session to a registered agent, and reading the agents.
//! Registering, renaming, and switching agents are a person's actions in the web UI.

use serde_json::json;

use crate::agent::connection_store;
use crate::agent::session_identity::SessionIdentity;
use crate::cli::args::{AgentArgs, AgentCommand, ConnectArgs};
use crate::cli::{Context, Outcome};
use crate::error::AgpjError;
use crate::file_store::ConnectionOrigin;
use crate::server::sessions::Origin;

pub fn connect(ctx: &mut Context, args: &ConnectArgs) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?.clone();
    // The environment secret wins over the stored one, so a new connection would be ignored.
    if std::env::var(crate::agent::connection_store::SECRET_ENV).is_ok_and(|v| !v.trim().is_empty())
    {
        return Err(AgpjError::InvalidArgument {
            message: "AGPJ_CONNECTION_SECRET is set, so this session already has its connection; unset it to connect with a code.".into(),
            command: None,
        });
    }
    let data_dir = ctx
        .files
        .shared_dir()
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| AgpjError::BoardUnavailable {
            reason: "The data folder cannot be found.".into(),
        })?;
    // One session holds one agent: a second connection would replace the stored secret of the
    // first and leave it unreleasable.
    if let Some(held) = connection_store::load_secret(&data_dir, &project)
        && let Ok(verified) = ctx
            .files
            .verify_connection(&project, &held, ctx.clock.now())
    {
        return Err(AgpjError::InvalidArgument {
            message: format!(
                "This session is already connected to agent {}; run agpj disconnect first.",
                verified.agent_id
            ),
            command: None,
        });
    }
    let identity = SessionIdentity::from_env();
    let origin = match identity.origin {
        Origin::Board => ConnectionOrigin::Board,
        Origin::External => ConnectionOrigin::External,
    };
    let connected = ctx.files.exchange_connect_code(
        &project,
        &args.code,
        origin,
        identity.tool.as_str(),
        &identity.key,
        ctx.clock.now(),
    )?;
    if let Err(e) = connection_store::save_secret(&data_dir, &project, &connected.secret) {
        // The connection and its secret stand or fall together: with no secret to use it, the
        // new connection would only keep the agent busy.
        let _ = ctx
            .files
            .disconnect_session(&project, &connected.secret, ctx.clock.now());
        return Err(e);
    }
    Ok(Outcome {
        changed: true,
        human: format!(
            "Connected to agent {} ({}): {}\n",
            connected.agent.id, connected.agent.name, connected.agent.role
        ),
        json: json!({"agent": connected.agent, "connection": connected.connection}),
    })
}

pub fn disconnect(ctx: &mut Context) -> Result<Outcome, AgpjError> {
    let project = ctx.project_id()?.clone();
    let data_dir = ctx
        .files
        .shared_dir()
        .parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| AgpjError::BoardUnavailable {
            reason: "The data folder cannot be found.".into(),
        })?;
    let secret =
        connection_store::load_secret(&data_dir, &project).ok_or(AgpjError::NotConnected)?;
    let paused = ctx
        .files
        .disconnect_session(&project, &secret, ctx.clock.now())?;
    connection_store::delete_secret(&data_dir, &project);
    Ok(Outcome {
        changed: true,
        human: "Disconnected from the agent; its running tasks are waiting.\n".to_string(),
        json: json!({"paused": paused}),
    })
}

pub fn agent(ctx: &mut Context, args: &AgentArgs) -> Result<Outcome, AgpjError> {
    match args.command {
        AgentCommand::Ls => {
            let agents = ctx.files.list_agents(ctx.project_id()?)?;
            let mut human = String::new();
            for agent in &agents {
                let state = match (&agent.archived_at, &agent.connection) {
                    (Some(_), _) => "deleted",
                    (None, Some(_)) => "connected",
                    (None, None) => "not connected",
                };
                human.push_str(&format!(
                    "{}  {}  {}  [{}]\n",
                    agent.id, agent.name, agent.role, state
                ));
            }
            if human.is_empty() {
                human.push_str("No agents are registered.\n");
            }
            Ok(Outcome {
                changed: false,
                human,
                json: json!({"agents": agents}),
            })
        }
    }
}
