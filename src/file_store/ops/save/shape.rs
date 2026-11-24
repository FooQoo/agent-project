use crate::file_store::frontmatter;

/// Whether `markdown` differs from the accepted file (`digest`, `shape`) only in acceptance
/// checkboxes.
pub(in crate::file_store::ops) fn is_checkbox_only_edit(
    markdown: &[u8],
    digest: Option<&str>,
    shape: Option<&str>,
) -> bool {
    let Some(digest) = digest else { return false };
    if digest == content_digest(markdown) {
        return false;
    }
    let Some(shape) = shape else { return false };
    shape == shape_digest(markdown)
}

/// Digest the Markdown with every `done: true|false` of the acceptance block of the front matter
/// read as `done: false`, so two files that differ only in checkbox states have one shape.
pub(in crate::file_store::ops) fn shape_digest(bytes: &[u8]) -> String {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return shape_bytes_digest(bytes);
    };
    let parsed = frontmatter::parse_document(text);
    let split = frontmatter::split(text);
    let frontmatter::Split::Found { yaml, body } = split else {
        return shape_bytes_digest(bytes);
    };
    let Ok(mut value) = frontmatter::read_yaml(yaml) else {
        return shape_bytes_digest(bytes);
    };
    use serde_yaml_ng::Value;
    let key = Value::String("acceptance".into());
    let Some(Value::Sequence(items)) = value.as_mapping_mut().and_then(|map| map.get_mut(&key))
    else {
        return shape_bytes_digest(bytes);
    };
    for item in items {
        let Some(map) = item.as_mapping_mut() else {
            return shape_bytes_digest(bytes);
        };
        let done = Value::String("done".into());
        match map.get_mut(&done) {
            Some(slot @ Value::Bool(_)) => *slot = Value::Bool(false),
            _ => return shape_bytes_digest(bytes),
        }
    }
    let Ok(normalized) = serde_yaml_ng::to_string(&value) else {
        return shape_bytes_digest(bytes);
    };
    if !parsed.defects.is_empty() {
        return shape_bytes_digest(bytes);
    }
    shape_digest_in_domain(
        b"agent-project/task-shape/normalized-acceptance/v2\0",
        format!("{normalized}\n---\n\n{body}").as_bytes(),
    )
}

fn shape_bytes_digest(bytes: &[u8]) -> String {
    shape_digest_in_domain(b"agent-project/task-shape/raw/v2\0", bytes)
}

fn shape_digest_in_domain(domain: &[u8], bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(domain);
    hasher.update(bytes);
    format!("v2:{:x}", hasher.finalize())
}

/// Digest the exact persisted Markdown bytes.
pub(in crate::file_store::ops) fn content_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}
