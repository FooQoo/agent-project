//! One module per subcommand. Each exposes
//! `pub fn run(ctx: &mut Context, args: &XxxArgs) -> Result<Outcome, AgboardError>`.

pub mod add;
pub mod attachment;
pub mod edit;
pub mod epic;
pub mod epic_plan;
pub mod init;
pub mod ls;
pub mod message;
pub mod mv;
pub mod open;
pub mod progress;
pub mod project;
pub mod release;
pub mod remote;
pub mod result;
pub mod rm;
pub mod serve;
pub mod serve_auto;
pub mod setup;
pub mod setup_remote;
pub mod show;
pub mod snapshot;
pub mod state;
pub mod stop;
pub mod version;
pub mod workspace;

pub mod history;
