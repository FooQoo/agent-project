//! The Tailscale entrance: `tailscale serve` forwards HTTPS on the tailnet to the loopback
//! backend, and the board's address is the machine's own `ts.net` name.
//!
//! Only the connector differs from ngrok. The backend, the Host and Origin checks, the
//! passcode, the sessions, the registered devices and the CSRF checks are the ones the ngrok
//! entrance uses. The `Tailscale-User-Login` header that Serve adds is never read: the backend
//! port is also reachable by other programs on this computer, so a header proves nothing.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::contracts::PublicOrigin;
use super::types::{Connector, FailureCode, OpenFailure, OpenFuture, OpenRequest, OpenedTunnel};

/// Where the macOS app keeps its command-line tool when `tailscale` is not on the `PATH`.
const MACOS_APP_CLI: &str = "/Applications/Tailscale.app/Contents/MacOS/Tailscale";

/// The `tailscale` executable: the one on the `PATH`, else the macOS app's.
pub fn find_cli() -> Option<PathBuf> {
    find_cli_in(
        std::env::var_os("PATH").as_deref(),
        Path::new(MACOS_APP_CLI),
    )
}

fn find_cli_in(path: Option<&std::ffi::OsStr>, app: &Path) -> Option<PathBuf> {
    let on_path = path
        .into_iter()
        .flat_map(std::env::split_paths)
        .find_map(|dir| {
            let candidate = dir.join("tailscale");
            candidate.is_file().then_some(candidate)
        });
    on_path.or_else(|| app.is_file().then(|| app.to_path_buf()))
}

/// Opens the entrance with `tailscale serve`.
pub struct TailscaleConnector {
    /// A fixed tool to run instead of looking one up; tests use a fake.
    cli: Option<PathBuf>,
}

impl TailscaleConnector {
    pub fn new() -> Self {
        Self { cli: None }
    }

    /// Always run `cli`, whatever is installed.
    pub fn with_cli(cli: PathBuf) -> Self {
        Self { cli: Some(cli) }
    }
}

impl Default for TailscaleConnector {
    fn default() -> Self {
        Self::new()
    }
}

/// Turns the entrance off when dropped, which is when the connection stops.
struct ServeGuard {
    cli: PathBuf,
}

impl Drop for ServeGuard {
    fn drop(&mut self) {
        let _ = Command::new(&self.cli)
            .args(["serve", "--https=443", "off"])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}

impl Connector for TailscaleConnector {
    fn open(&self, request: OpenRequest) -> OpenFuture {
        let cli = self.cli.clone().or_else(find_cli);
        Box::pin(async move {
            let fail = |code| OpenFailure { code };
            let cli = cli.ok_or_else(|| {
                eprintln!("agboard: the tailscale command was not found");
                fail(FailureCode::TailscaleNotInstalled)
            })?;
            let port = request.backend.port().ok_or(fail(FailureCode::Internal))?;
            tokio::task::spawn_blocking(move || start(cli, port))
                .await
                .unwrap_or(Err(FailureCode::Internal))
                .map_err(fail)
        })
    }
}

fn start(cli: PathBuf, port: u16) -> Result<OpenedTunnel, FailureCode> {
    let status = run(&cli, &["status", "--json"]).map_err(|e| {
        eprintln!("agboard: tailscale status failed: {e}");
        match e {
            RunError::Missing => FailureCode::TailscaleNotInstalled,
            RunError::Failed(_) => FailureCode::TailscaleNotLoggedIn,
        }
    })?;
    let host = host_of_status(&status)?;
    let public = PublicOrigin::parse(&format!("https://{host}"))
        .map_err(|_| FailureCode::PublicUrlInvalid)?;
    let target = format!("http://127.0.0.1:{port}");
    run(&cli, &["serve", "--bg", "--https=443", &target]).map_err(|e| {
        eprintln!("agboard: tailscale serve failed: {e}");
        FailureCode::TailscaleServeFailed
    })?;
    Ok(OpenedTunnel {
        public,
        keep_alive: Box::new(ServeGuard { cli }),
    })
}

/// The machine's `ts.net` host name from `tailscale status --json`, or why there is none.
pub(super) fn host_of_status(json: &str) -> Result<String, FailureCode> {
    let value: serde_json::Value =
        serde_json::from_str(json).map_err(|_| FailureCode::TailscaleNotLoggedIn)?;
    if value["BackendState"].as_str() != Some("Running") {
        return Err(FailureCode::TailscaleNotLoggedIn);
    }
    let host = value["Self"]["DNSName"]
        .as_str()
        .map(|name| name.trim_end_matches('.'))
        .filter(|name| !name.is_empty())
        .ok_or(FailureCode::TailscaleNotLoggedIn)?;
    // HTTPS certificates must be enabled for the tailnet; without them Serve cannot terminate TLS.
    let has_certs = value["CertDomains"]
        .as_array()
        .is_some_and(|domains| !domains.is_empty());
    if !has_certs {
        return Err(FailureCode::TailscaleHttpsDisabled);
    }
    Ok(host.to_ascii_lowercase())
}

enum RunError {
    Missing,
    Failed(String),
}

impl std::fmt::Display for RunError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Missing => f.write_str("the tool could not be run"),
            Self::Failed(text) => f.write_str(text),
        }
    }
}

/// Run the tool and return its standard output. No secret is ever passed or printed.
fn run(cli: &Path, args: &[&str]) -> Result<String, RunError> {
    let output = Command::new(cli)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|_| RunError::Missing)?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let first = stderr.lines().next().unwrap_or("").trim();
        return Err(RunError::Failed(format!("{}: {first}", output.status)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status(state: &str, dns: &str, certs: &str) -> String {
        format!(
            r#"{{"BackendState":"{state}","Self":{{"DNSName":"{dns}"}},"CertDomains":{certs}}}"#
        )
    }

    #[test]
    fn the_host_is_the_self_name_without_the_trailing_dot() {
        let json = status(
            "Running",
            "Mac.Tail1234.ts.net.",
            r#"["mac.tail1234.ts.net"]"#,
        );
        assert_eq!(host_of_status(&json).as_deref(), Ok("mac.tail1234.ts.net"));
    }

    #[test]
    fn it_refuses_without_a_login_or_https_certificates() {
        assert_eq!(
            host_of_status(&status("NeedsLogin", "a.ts.net.", r#"["a.ts.net"]"#)),
            Err(FailureCode::TailscaleNotLoggedIn)
        );
        assert_eq!(
            host_of_status(&status("Running", "a.ts.net.", "[]")),
            Err(FailureCode::TailscaleHttpsDisabled)
        );
        assert_eq!(
            host_of_status(
                r#"{"BackendState":"Running","Self":{"DNSName":"a.ts.net."},"CertDomains":null}"#
            ),
            Err(FailureCode::TailscaleHttpsDisabled)
        );
        assert_eq!(
            host_of_status(&status("Running", "", r#"["a.ts.net"]"#)),
            Err(FailureCode::TailscaleNotLoggedIn)
        );
        assert_eq!(
            host_of_status("not json"),
            Err(FailureCode::TailscaleNotLoggedIn)
        );
    }

    #[test]
    fn the_tool_on_the_path_wins_over_the_app() {
        let dir = tempfile::tempdir().unwrap();
        let on_path = dir.path().join("bin");
        std::fs::create_dir(&on_path).unwrap();
        let app = dir.path().join("app-tailscale");
        std::fs::write(&app, "").unwrap();
        assert_eq!(
            find_cli_in(Some(on_path.as_os_str()), &app),
            Some(app.clone())
        );
        std::fs::write(on_path.join("tailscale"), "").unwrap();
        assert_eq!(
            find_cli_in(Some(on_path.as_os_str()), &app),
            Some(on_path.join("tailscale"))
        );
        assert_eq!(find_cli_in(None, &dir.path().join("missing")), None);
    }
}
