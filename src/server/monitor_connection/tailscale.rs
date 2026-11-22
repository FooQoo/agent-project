//! The Tailscale entrance: `tailscale serve` forwards HTTPS on the tailnet to the loopback
//! backend, and the board's address is the machine's own `ts.net` name.
//!
//! Only the connector differs from ngrok. The backend, the Host and Origin checks, the
//! passcode, the sessions, the registered devices and the CSRF checks are the ones the ngrok
//! entrance uses. The `Tailscale-User-Login` header that Serve adds is never read: the backend
//! port is also reachable by other programs on this computer, so a header proves nothing.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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

/// How long one `tailscale` call may run before it is killed.
const CALL_LIMIT: Duration = Duration::from_secs(5);
/// How long a stop waits for the entrance to be turned off before it moves on.
const STOP_WAIT: Duration = Duration::from_secs(2);

/// Serializes every change of the Serve configuration made by this process and remembers which
/// connection owns it, so a late or cancelled call never changes a newer connection's entrance.
static SERVE: Mutex<u64> = Mutex::new(0);
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

fn serve_state() -> std::sync::MutexGuard<'static, u64> {
    SERVE.lock().unwrap_or_else(|e| e.into_inner())
}

/// Turns the entrance off when dropped, but only while this connection still owns it. The call
/// runs on its own thread with a deadline, and a stop waits for it at most [`STOP_WAIT`], so a
/// stalled tool never holds the supervisor for long.
struct ServeGuard {
    cli: PathBuf,
    id: u64,
}

impl Drop for ServeGuard {
    fn drop(&mut self) {
        let (cli, id) = (self.cli.clone(), self.id);
        let (done, finished) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let mut owner = serve_state();
            if *owner == id {
                let _ = run(&cli, &["serve", "--https=443", "off"]);
                *owner = 0;
            }
            let _ = done.send(());
        });
        let _ = finished.recv_timeout(STOP_WAIT);
    }
}

/// Sets the cancel flag when the opening future is dropped (stopped or replaced while connecting).
struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
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
            let cancelled = Arc::new(AtomicBool::new(false));
            let _cancel = CancelOnDrop(cancelled.clone());
            tokio::task::spawn_blocking(move || start(cli, port, &cancelled))
                .await
                .unwrap_or(Err(FailureCode::Internal))
                .map_err(fail)
        })
    }
}

fn start(cli: PathBuf, port: u16, cancelled: &AtomicBool) -> Result<OpenedTunnel, FailureCode> {
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
    // From here on the Serve configuration is changed by one connection at a time, and not at all
    // by one that was stopped meanwhile.
    let mut owner = serve_state();
    if cancelled.load(Ordering::SeqCst) {
        return Err(FailureCode::Internal);
    }
    let existing = run(&cli, &["serve", "status", "--json"]).map_err(|e| {
        eprintln!("agboard: tailscale serve status failed: {e}");
        FailureCode::TailscaleServeFailed
    })?;
    if serve_is_busy(&existing) {
        eprintln!("agboard: tailscale serve already has a handler on port 443; not replacing it");
        return Err(FailureCode::TailscaleServeBusy);
    }
    run(&cli, &["serve", "--bg", "--https=443", &target]).map_err(|e| {
        eprintln!("agboard: tailscale serve failed: {e}");
        FailureCode::TailscaleServeFailed
    })?;
    let id = NEXT_ID.fetch_add(1, Ordering::SeqCst);
    *owner = id;
    Ok(OpenedTunnel {
        public,
        keep_alive: Box::new(ServeGuard { cli, id }),
    })
}

/// Whether `tailscale serve status --json` shows a handler on HTTPS port 443 that is not ours.
/// Empty output and `{}` mean nothing is served.
pub(super) fn serve_is_busy(json: &str) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return !json.trim().is_empty();
    };
    let on_443 = |key: &str| {
        value[key]
            .as_object()
            .is_some_and(|m| m.keys().any(|k| k == "443" || k.ends_with(":443")))
    };
    on_443("TCP") || on_443("Web")
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

/// Run the tool and return its standard output, killing it after [`CALL_LIMIT`]. No secret is
/// ever passed or printed.
fn run(cli: &Path, args: &[&str]) -> Result<String, RunError> {
    let mut child = Command::new(cli)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| RunError::Missing)?;
    let read = |pipe: Option<Box<dyn std::io::Read + Send>>| {
        std::thread::spawn(move || {
            let mut text = String::new();
            if let Some(mut pipe) = pipe {
                let _ = std::io::Read::read_to_string(&mut pipe, &mut text);
            }
            text
        })
    };
    let out = read(child.stdout.take().map(|p| Box::new(p) as _));
    let err = read(child.stderr.take().map(|p| Box::new(p) as _));
    let deadline = Instant::now() + CALL_LIMIT;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(20)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(RunError::Failed("timed out".to_string()));
            }
        }
    };
    let stdout = out.join().unwrap_or_default();
    let stderr = err.join().unwrap_or_default();
    if !status.success() {
        let first = stderr.lines().next().unwrap_or("").trim();
        return Err(RunError::Failed(format!("{status}: {first}")));
    }
    Ok(stdout)
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

    #[test]
    fn a_handler_on_443_makes_serve_busy() {
        assert!(!serve_is_busy(""));
        assert!(!serve_is_busy("{}"));
        assert!(!serve_is_busy(r#"{"TCP":{"8443":{"HTTPS":true}}}"#));
        assert!(serve_is_busy(
            r#"{"TCP":{"443":{"HTTPS":true}},"Web":{"a.ts.net:443":{}}}"#
        ));
        assert!(serve_is_busy(r#"{"Web":{"a.ts.net:443":{"Handlers":{}}}}"#));
        assert!(serve_is_busy("not json"));
    }
}
