#[derive(clap::Args, Debug)]
pub struct AcceptanceArgs {
    #[command(subcommand)]
    pub command: AcceptanceCommand,
}

#[derive(clap::Subcommand, Debug)]
pub enum AcceptanceCommand {
    /// Mark one verified condition as met
    Check(AcceptanceCheckArgs),
}

#[derive(clap::Args, Debug)]
pub struct AcceptanceCheckArgs {
    pub id: i64,
    pub condition: String,
}
