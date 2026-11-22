//! What to do next after `agpj setup remote` saves the Tailscale provider.
//!
//! The check is advice only: it never blocks or fails the save, and it changes nothing in
//! Tailscale. It reuses the probe that the connection itself runs before it opens the entrance.

use std::path::Path;

use serde_json::{Value, json};

use crate::server::monitor_connection::lifecycle::FailureCode;
use crate::server::monitor_connection::tailscale;

const ADMIN_DNS: &str = "https://login.tailscale.com/admin/dns";

/// The state of Tailscale on this computer and what the user should do about it.
pub struct Advice {
    status: &'static str,
    domain: Option<String>,
    human: String,
}

/// Check Tailscale now, with the tool that the connection would use.
pub fn check_now() -> Advice {
    check(tailscale::find_cli().as_deref())
}

/// Check Tailscale with `cli`, or as not installed when there is none.
pub fn check(cli: Option<&Path>) -> Advice {
    let (status, domain, human) = match tailscale::probe(cli) {
        Ok(host) => (
            "ready",
            Some(host.clone()),
            format!("Tailscale is ready. The phone will open https://{host}\n"),
        ),
        Err(FailureCode::TailscaleNotInstalled) => ("not_installed", None, NOT_INSTALLED.into()),
        Err(FailureCode::TailscaleHttpsDisabled) => (
            "https_disabled",
            None,
            format!(
                "Tailscale is signed in, but HTTPS certificates are off. In the Tailscale admin console, turn on MagicDNS and HTTPS Certificates: {ADMIN_DNS}\n"
            ),
        ),
        Err(_) => (
            "not_logged_in",
            None,
            "Tailscale is not signed in or not running. Open the Tailscale app and sign in, or run \"tailscale up\" if you use the Homebrew command-line tool.\n".into(),
        ),
    };
    Advice {
        status,
        domain,
        human,
    }
}

const NOT_INSTALLED: &str = "Tailscale is not installed. Install it one of two ways, then sign in:\n  - The Tailscale Mac app: \"brew install --cask tailscale-app\", the App Store, or the download on tailscale.com.\n  - The Homebrew command-line tool: \"brew install tailscale\", then \"sudo brew services start tailscale\" and \"tailscale up\".\nThe app and the Homebrew tool cannot be used together; pick one.\n";

impl Advice {
    pub fn to_json(&self) -> Value {
        json!({ "status": self.status, "domain": self.domain })
    }

    /// Add the advice to the output of a save.
    pub fn add_to(&self, json: &mut Value, human: &mut String) {
        json["tailscale"] = self.to_json();
        human.push_str(&self.human);
    }

    /// Show the advice on the terminal during the questions.
    pub fn show(&self) {
        let _ = cliclack::note("Tailscale", self.human.trim_end());
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    fn fake(dir: &Path, status: &str) -> std::path::PathBuf {
        let script = dir.join("tailscale");
        std::fs::write(&script, format!("#!/bin/sh\ncat <<'EOF'\n{status}\nEOF\n")).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    fn state(status: &str) -> (String, Value) {
        let dir = tempfile::tempdir().unwrap();
        let advice = check(Some(&fake(dir.path(), status)));
        (advice.human.clone(), advice.to_json())
    }

    #[test]
    fn each_state_has_its_own_advice() {
        let (human, json) = check_text(None);
        assert_eq!(json["status"], "not_installed");
        assert!(
            human.contains("brew install --cask tailscale-app"),
            "{human}"
        );
        assert!(
            human.contains("sudo brew services start tailscale"),
            "{human}"
        );
        assert!(human.contains("cannot be used together"), "{human}");

        let (human, json) = state(r#"{"BackendState":"NeedsLogin"}"#);
        assert_eq!(json["status"], "not_logged_in");
        assert!(human.contains("tailscale up"), "{human}");

        let (human, json) =
            state(r#"{"BackendState":"Running","Self":{"DNSName":"a.ts.net."},"CertDomains":[]}"#);
        assert_eq!(json["status"], "https_disabled");
        assert!(human.contains(ADMIN_DNS), "{human}");

        let (human, json) = state(
            r#"{"BackendState":"Running","Self":{"DNSName":"A.ts.net."},"CertDomains":["a.ts.net"]}"#,
        );
        assert_eq!(
            (json["status"].clone(), json["domain"].clone()),
            ("ready".into(), "a.ts.net".into())
        );
        assert!(human.contains("https://a.ts.net"), "{human}");
    }

    fn check_text(cli: Option<&Path>) -> (String, Value) {
        let advice = check(cli);
        (advice.human.clone(), advice.to_json())
    }
}
