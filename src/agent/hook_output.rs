//! The JSON and text the hook hands back to the agent.

use super::hook::Failure;

pub(super) fn error_context(failure: &Failure) -> String {
    let body = serde_json::json!({"code": failure.code, "reason": failure.reason});
    format!(
        "AGPJ_ERROR\n{body}\nDo not make board-dependent changes until a complete progress table is available."
    )
}

/// The hook's JSON for `event` with `context` as the agent's added context.
pub(crate) fn output(event: &str, context: &str) -> String {
    serde_json::json!({"hookSpecificOutput":{"hookEventName":event,"additionalContext":context}})
        .to_string()
}
