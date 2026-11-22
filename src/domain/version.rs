//! Version names: the releases a project plans its tasks for. A version is named by a file,
//! `docs/release/<name>.md` in the project's Docs, so a name must also be a safe file stem.

use crate::error::AgboardError;

/// The longest version name, in characters.
pub const MAX_VERSION_CHARS: usize = 64;

/// The most versions one project can list.
pub const MAX_VERSIONS: usize = 200;

/// Characters a file name cannot hold on some platform, and path separators.
const FORBIDDEN: &[char] = &['/', '\\', ':', '*', '?', '"', '<', '>', '|'];

/// Check a version name. It is 1 to 64 characters with no whitespace, control or
/// file-name characters (`/ \ : * ? " < > |`), and it does not start with a dot. The name is
/// kept and compared exactly: `v1.8.0` and `1.8.0` are two versions, and no semantic-version
/// format is required.
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
    if value.starts_with('.') || value.chars().any(|c| FORBIDDEN.contains(&c)) {
        return Err(invalid(
            "Version names cannot start with a dot or contain / \\ : * ? \" < > |.",
        ));
    }
    Ok(value.to_owned())
}

/// Order two version names naturally: runs of digits compare as numbers and the rest as text,
/// so `v1.9.0` comes before `v1.10.0`. Names that compare equal that way (`v1.08` and `v1.8`)
/// fall back to a plain text comparison, so the order is total.
pub fn compare_versions(a: &str, b: &str) -> std::cmp::Ordering {
    let (ca, cb) = (chunks(a), chunks(b));
    for (x, y) in ca.iter().zip(&cb) {
        let order = match (x.parse::<u128>(), y.parse::<u128>()) {
            (Ok(m), Ok(n)) if x.starts_with(|c: char| c.is_ascii_digit()) => m.cmp(&n),
            _ => x.cmp(y),
        };
        if order.is_ne() {
            return order;
        }
    }
    ca.len().cmp(&cb.len()).then_with(|| a.cmp(b))
}

/// Split a name into runs of ASCII digits and runs of anything else.
fn chunks(value: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut start = 0;
    let mut digit = None;
    for (index, c) in value.char_indices() {
        let is_digit = c.is_ascii_digit();
        if digit.is_some_and(|d| d != is_digit) {
            out.push(&value[start..index]);
            start = index;
        }
        digit = Some(is_digit);
    }
    if start < value.len() {
        out.push(&value[start..]);
    }
    out
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
        for bad in [
            "v 1",
            " v1",
            "v1\n",
            "v1\u{0}",
            "v1\u{3000}",
            ".hidden",
            "..",
            "a/b",
            "a\\b",
            "v1:2",
        ] {
            assert!(validate_version_name(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn versions_sort_naturally_with_a_total_order() {
        let mut names = vec![
            "v1.10.0", "v1.9.0", "v1.8.0", "v1.8", "next", "v1.08.0", "v2",
        ];
        names.sort_by(|a, b| compare_versions(a, b));
        assert_eq!(
            names,
            [
                "next", "v1.8", "v1.08.0", "v1.8.0", "v1.9.0", "v1.10.0", "v2"
            ]
        );
    }
}
