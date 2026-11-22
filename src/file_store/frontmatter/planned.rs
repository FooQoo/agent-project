//! The `plannedVersion` key of TASK.md: the version a task is planned for.

use serde_yaml_ng::{Mapping, Value};

use super::split::{Split, assemble, defect, read_yaml, split};
use crate::file_store::validate::Defect;

const KEY: &str = "plannedVersion";

/// The `plannedVersion` of a text. Absent, `null` and a blank string all mean no plan. The value
/// is returned as written, whether or not the project lists it: only a string is accepted, so
/// a number or a list is a defect. A frontmatter that cannot be read is not reported here: the
/// acceptance check reports it.
pub fn planned_version_of(text: &str) -> Result<Option<String>, Defect> {
    let text = text.replace("\r\n", "\n");
    let Split::Found { yaml, .. } = split(&text) else {
        return Ok(None);
    };
    let Ok(Value::Mapping(map)) = read_yaml(yaml) else {
        return Ok(None);
    };
    match map.get(KEY) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(name)) if name.trim().is_empty() => Ok(None),
        Some(Value::String(name)) => Ok(Some(name.clone())),
        Some(_) => Err(defect(
            KEY,
            "plannedVersion must be a version name, such as v1.8.0. Quote a name that looks like a number.",
        )),
    }
}

/// Write a text's `plannedVersion`. The other keys and the body stay, and a new key goes in
/// front of `history`. `None` removes the key. A frontmatter that cannot be read is an error, so
/// nothing is written over it.
pub fn compose_planned_version(original: &str, version: Option<&str>) -> Result<String, Defect> {
    let original = original.replace("\r\n", "\n");
    let (map, body) = match split(&original) {
        Split::None => (Mapping::new(), original.clone()),
        Split::Unclosed => {
            return Err(defect(
                "frontmatter",
                "The frontmatter has no closing --- line.",
            ));
        }
        Split::Found { yaml, body } => match read_yaml(yaml)? {
            Value::Mapping(map) => (map, body.to_string()),
            _ => unreachable!("read_yaml returns a mapping"),
        },
    };
    let value = version.map(|name| Value::String(name.to_owned()));
    let map = if map.contains_key(KEY) {
        let mut map = map;
        match value {
            Some(value) => {
                map.insert(Value::String(KEY.into()), value);
            }
            None => {
                map.shift_remove(KEY);
            }
        }
        map
    } else if let Some(value) = value {
        let mut out = Mapping::new();
        let mut placed = false;
        for (key, item) in map {
            if !placed && key.as_str() == Some("history") {
                out.insert(Value::String(KEY.into()), value.clone());
                placed = true;
            }
            out.insert(key, item);
        }
        if !placed {
            out.insert(Value::String(KEY.into()), value);
        }
        out
    } else {
        map
    };
    assemble(map, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_name_as_written() {
        let text = "---\nplannedVersion: v1.8.0\n---\nBody\n";
        assert_eq!(planned_version_of(text).unwrap().as_deref(), Some("v1.8.0"));
        for none in [
            "no frontmatter",
            "---\nacceptance: []\n---\n",
            "---\nplannedVersion:\n---\n",
        ] {
            assert_eq!(planned_version_of(none).unwrap(), None, "{none}");
        }
        for bad in [
            "plannedVersion: 1.8",
            "plannedVersion: [v1]",
            "plannedVersion: {a: b}",
        ] {
            let text = format!("---\n{bad}\n---\n");
            assert_eq!(planned_version_of(&text).unwrap_err().field, KEY, "{bad}");
        }
    }

    #[test]
    fn writes_before_history_and_keeps_everything_else() {
        let text = "---\nacceptance: []\nextra: 1\nhistory: []\n---\n\nBody\n";
        let set = compose_planned_version(text, Some("v2")).unwrap();
        assert!(set.find("plannedVersion").unwrap() < set.find("history").unwrap());
        assert!(set.contains("extra: 1") && set.ends_with("Body\n"));
        let moved = compose_planned_version(&set, Some("v3")).unwrap();
        assert_eq!(planned_version_of(&moved).unwrap().as_deref(), Some("v3"));
        let gone = compose_planned_version(&moved, None).unwrap();
        assert_eq!(gone, text);
    }
}
