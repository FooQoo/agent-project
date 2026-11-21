//! Version names: the releases a project plans its tasks for.

use crate::error::AgboardError;

/// The longest version name, in characters.
pub const MAX_VERSION_CHARS: usize = 64;

/// The most versions one project can list.
pub const MAX_VERSIONS: usize = 200;

/// Check a version name. It is 1 to 64 characters with no whitespace or control
/// characters. The name is kept and compared exactly: `v1.8.0` and `1.8.0` are two
/// versions, and no semantic-version format is required.
pub fn validate_version_name(value: &str) -> Result<String, AgboardError> {
    let count = value.chars().count();
    if count == 0 || count > MAX_VERSION_CHARS {
        return Err(invalid("Version names must be 1 to 64 characters."));
    }
    if value.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(invalid(
            "Version names cannot contain whitespace or control characters.",
        ));
    }
    Ok(value.to_owned())
}

/// Which tasks a version filter keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VersionFilter {
    /// Tasks planned for exactly this version.
    Planned(String),
    /// Tasks with no planned version.
    Unplanned,
}

impl VersionFilter {
    /// Whether a task with this planned version is kept.
    pub fn keeps(&self, planned: Option<&str>) -> bool {
        match self {
            VersionFilter::Planned(version) => planned == Some(version.as_str()),
            VersionFilter::Unplanned => planned.is_none(),
        }
    }
}

fn invalid(reason: &str) -> AgboardError {
    AgboardError::InvalidValue {
        field: "version".into(),
        reason: reason.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_plain_names_and_rejects_blank_long_or_spaced_ones() {
        for ok in ["v1.8.0", "1.8", "2026-Q4", "next", "リリース1"] {
            assert_eq!(validate_version_name(ok).unwrap(), ok);
        }
        assert!(validate_version_name("").is_err());
        assert!(validate_version_name(&"a".repeat(65)).is_err());
        assert!(validate_version_name(&"a".repeat(64)).is_ok());
        for bad in ["v 1", " v1", "v1\n", "v1\u{0}", "v1\u{3000}"] {
            assert!(validate_version_name(bad).is_err(), "{bad:?}");
        }
    }
}
