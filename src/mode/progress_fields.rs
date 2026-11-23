//! The mode fields of the progress table.

use serde::Serialize;

use super::{ModeKind, Target};

/// The project's mode when it is not the default sparring: `mode`, `modeSource`, `modeUntil`, and,
/// in brainstorm, `modeTarget` (`{"type", "id"}`). Each is left out when it has no value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct ProgressMode {
    #[serde(rename = "mode", skip_serializing_if = "Option::is_none")]
    pub kind: Option<ModeKind>,
    #[serde(rename = "modeSource", skip_serializing_if = "Option::is_none")]
    pub source: Option<&'static str>,
    #[serde(rename = "modeUntil", skip_serializing_if = "Option::is_none")]
    pub until: Option<String>,
    #[serde(rename = "modeTarget", skip_serializing_if = "Option::is_none")]
    pub target: Option<Target>,
}
