//! Checks on the two values a person types for ngrok: the domain and the token.

use super::types::Secret;

/// A bare, lower-case host name, or `None` for anything else: a scheme, a path, a port,
/// a wildcard, or spaces are not accepted, so a mistyped value never reaches ngrok.
pub fn parse_domain(raw: &str) -> Option<String> {
    let domain = raw.trim().to_ascii_lowercase();
    let label_ok = |l: &str| {
        !l.is_empty()
            && l.len() <= 63
            && !l.starts_with('-')
            && !l.ends_with('-')
            && l.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
    };
    (domain.len() <= 253 && domain.contains('.') && domain.split('.').all(label_ok))
        .then_some(domain)
}

/// The longest ngrok token accepted.
const MAX_TOKEN: usize = 512;

/// An ngrok token as typed: trimmed, not empty, not too long, with no space or control
/// character. The board's connection dialog and `agboard setup remote` both use this.
pub fn parse_token(raw: &str) -> Option<Secret> {
    let token = raw.trim();
    (!token.is_empty()
        && token.len() <= MAX_TOKEN
        && !token.chars().any(|c| c.is_control() || c.is_whitespace()))
    .then(|| Secret::new(token))
}

#[cfg(test)]
mod tests {
    use super::parse_domain;

    #[test]
    fn a_domain_is_a_bare_host_name() {
        assert_eq!(
            parse_domain(" My-App.ngrok-free.DEV ").as_deref(),
            Some("my-app.ngrok-free.dev")
        );
        for bad in [
            "",
            "localhost",
            "https://a.ngrok-free.dev",
            "a.ngrok-free.dev/x",
            "a.ngrok-free.dev:443",
            "*.ngrok-free.dev",
            "a..dev",
            "-a.ngrok-free.dev",
            "a b.dev",
        ] {
            assert_eq!(parse_domain(bad), None, "{bad}");
        }
    }
}
