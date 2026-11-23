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
    let data_dir = ctx
        .files
        .shared_dir()
        .parent()
        .map(std::path::Path::to_path_buf);
    let data_dir = data_dir.ok_or_else(|| AgpjError::BoardUnavailable {
        reason: "The data folder cannot be found.".into(),
    })?;
    connection_store::save_secret(&data_dir, &project, &connected.secret)?;
    Ok(Outcome {
        changed: true,
        human: format!(
            "Connected to agent {} ({}): {}\n",
            connected.agent.id, connected.agent.name, connected.agent.role
        ),
        json: json!({"agent": connected.agent, "connection": connected.connection}),
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
