use crate::{
    cli::{Context, Outcome},
    clock::format_utc,
    error::AgpjError,
    file_store::reader::FetchedSnapshot,
};

pub fn run(ctx: &mut Context) -> Result<Outcome, AgpjError> {
    let fetched_at = format_utc(ctx.clock.now());
    let snapshot = ctx.files.board(ctx.project_id()?)?;
    let json = serde_json::to_value(FetchedSnapshot {
        fetched_at,
        snapshot: &snapshot,
    })
    .expect("snapshot serializes");
    Ok(Outcome {
        changed: false,
        human: String::new(),
        json,
    })
}
