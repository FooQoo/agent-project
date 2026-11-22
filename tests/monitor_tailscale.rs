//! The Tailscale entrance over its whole life, with a fake `tailscale` tool standing in for the
//! real one. No test runs `tailscale serve` for real or touches the tailnet.
//!
//! The backend is spoken to as `tailscale serve` would: with the machine's `ts.net` Host. The
//! protections are the ngrok entrance's, so these tests show that they hold for this address
//! and that the Tailscale-specific parts (the command lines, the origin, the refusals) are right.
#![cfg(unix)]

#[path = "support/ngrok_harness.rs"]
mod harness;
#[path = "support/mobile_monitor.rs"]
mod support;

use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use agent_project::server::monitor_connection::tailscale::TailscaleConnector;
use harness::*;
use serde_json::json;

const HOST: &str = "mac.tail1234.ts.net";
const RUNNING: &str = r#"{"BackendState":"Running","Self":{"DNSName":"mac.tail1234.ts.net."},"CertDomains":["mac.tail1234.ts.net"]}"#;

/// A fake `tailscale` that prints `status.json` for `status --json` and logs every other call.
fn fake_cli(dir: &Path, status: &str) -> PathBuf {
    std::fs::write(dir.join("status.json"), status).unwrap();
    let script = dir.join("tailscale");
    std::fs::write(
        &script,
        format!(
            "#!/bin/sh\nD='{}'\nif [ \"$1\" = status ]; then cat \"$D/status.json\"; exit 0; fi\nif [ \"$1 $2\" = \"serve status\" ]; then cat \"$D/serve.json\" 2>/dev/null || echo '{{}}'; exit 0; fi\necho \"$@\" >> \"$D/calls.log\"\n",
            dir.display()
        ),
    )
    .unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script
}

fn calls(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("calls.log"))
        .unwrap_or_default()
        .lines()
        .map(str::to_string)
        .collect()
}

fn server(home: &Path, cli: Option<PathBuf>) -> Server {
    let connector: Arc<dyn agent_project::server::monitor_connection::lifecycle::Connector> =
        Arc::new(match cli {
            Some(cli) => TailscaleConnector::with_cli(cli),
            None => TailscaleConnector::with_cli(PathBuf::from("/nonexistent/tailscale")),
        });
    Server::start(
        home,
        Opts {
            connector: Some(connector),
            ..Opts::default()
        },
    )
}

fn choose_tailscale(s: &Server) {
    let r = s.write(
        "PUT",
        "/api/monitor-connections/settings/ngrok",
        &json!({"enabled": true, "revision": 0, "provider": "tailscale"}),
    );
    assert_eq!(r.status, 200, "{}", r.body);
    assert_eq!(r.json()["settings"]["provider"], "tailscale");
}

#[test]
fn it_serves_the_backend_on_the_tailnet_and_turns_serve_off_on_stop() {
    let tools = tempfile::tempdir().unwrap();
    let home = support::home();
    let s = server(home.path(), Some(fake_cli(tools.path(), RUNNING)));
    choose_tailscale(&s);
    // No ngrok token is held or asked for.
    assert_eq!(s.get()["tokenConfigured"], false);
    let r = s.start_ngrok(json!({}));
    assert_eq!(r.status, 202, "{}", r.body);
    let ready = s.wait_status("ready");
    assert_eq!(ready["url"], format!("https://{HOST}"));
    let passcode = ready["passcode"].as_str().unwrap().to_string();

    let log = calls(tools.path());
    assert_eq!(log.len(), 1, "{log:?}");
    let target = log[0]
        .strip_prefix("serve --bg --https=443 http://127.0.0.1:")
        .unwrap_or_else(|| panic!("{log:?}"));
    let backend: std::net::SocketAddr = format!("127.0.0.1:{target}").parse().unwrap();

    // The Host must be the ts.net name; anything else is refused before any route runs.
    let origin = format!("https://{HOST}");
    let json_unlock = |host: &str, origin: &str, code: &str| {
        request(
            backend,
            "POST",
            "/api/remote/unlock",
            &[
                ("Host", host),
                ("Origin", origin),
                ("Sec-Fetch-Site", "same-origin"),
                ("Content-Type", "application/json"),
                // Serve adds this header; it must grant nothing.
                ("Tailscale-User-Login", "someone@example.com"),
            ],
            &json!({ "code": code }).to_string(),
        )
    };
    assert_eq!(
        request(
            backend,
            "GET",
            "/api/remote/session",
            &[("Host", "evil.example")],
            ""
        )
        .status,
        403
    );
    let anonymous = request(
        backend,
        "GET",
        "/api/remote/session",
        &[
            ("Host", HOST),
            ("Tailscale-User-Login", "someone@example.com"),
        ],
        "",
    );
    assert_eq!(anonymous.status, 401, "{}", anonymous.body);
    assert_ne!(
        json_unlock(HOST, "https://evil.example", &passcode).status,
        200
    );
    let wrong = if passcode == "0000" { "1111" } else { "0000" };
    assert_eq!(json_unlock(HOST, &origin, wrong).status, 403);
    let ok = json_unlock(HOST, &origin, &passcode);
    assert_eq!(ok.status, 200, "{}", ok.body);
    let cookie = ok.header("set-cookie").unwrap();
    assert!(cookie.starts_with("agboard_remote_session="), "{cookie}");
    assert!(
        cookie.contains("Secure") && cookie.contains("HttpOnly"),
        "{cookie}"
    );

    assert_eq!(s.stop_ngrok().status, 200);
    s.wait_status("disabled");
    // Turning serve off runs on its own thread, so give it a moment.
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while calls(tools.path()).len() < 2 && std::time::Instant::now() < end {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    let log = calls(tools.path());
    assert_eq!(log.last().unwrap(), "serve --https=443 off", "{log:?}");
}

#[test]
fn it_does_not_replace_a_handler_another_program_already_serves_on_443() {
    let tools = tempfile::tempdir().unwrap();
    let home = support::home();
    let cli = fake_cli(tools.path(), RUNNING);
    std::fs::write(
        tools.path().join("serve.json"),
        r#"{"TCP":{"443":{"HTTPS":true}},"Web":{"mac.tail1234.ts.net:443":{"Handlers":{"/":{"Proxy":"http://127.0.0.1:9000"}}}}}"#,
    )
    .unwrap();
    let s = server(home.path(), Some(cli));
    choose_tailscale(&s);
    assert_eq!(s.start_ngrok(json!({})).status, 202);
    assert_eq!(s.wait_status("failed")["failure"], "tailscale_serve_busy");
    // Nothing was changed, and a failed start turns nothing off either.
    assert!(calls(tools.path()).is_empty(), "{:?}", calls(tools.path()));
}

#[test]
fn it_refuses_to_start_when_https_certificates_are_not_enabled() {
    let tools = tempfile::tempdir().unwrap();
    let home = support::home();
    let status =
        r#"{"BackendState":"Running","Self":{"DNSName":"mac.tail1234.ts.net."},"CertDomains":[]}"#;
    let s = server(home.path(), Some(fake_cli(tools.path(), status)));
    choose_tailscale(&s);
    assert_eq!(s.start_ngrok(json!({})).status, 202);
    let v = s.wait_status("failed");
    assert_eq!(v["failure"], "tailscale_https_disabled");
    assert!(calls(tools.path()).is_empty(), "serve must not run");
}

#[test]
fn it_refuses_to_start_when_tailscale_is_not_logged_in() {
    let tools = tempfile::tempdir().unwrap();
    let home = support::home();
    let status = r#"{"BackendState":"NeedsLogin","Self":null,"CertDomains":null}"#;
    let s = server(home.path(), Some(fake_cli(tools.path(), status)));
    choose_tailscale(&s);
    assert_eq!(s.start_ngrok(json!({})).status, 202);
    assert_eq!(
        s.wait_status("failed")["failure"],
        "tailscale_not_logged_in"
    );
    assert!(calls(tools.path()).is_empty());
}

#[test]
fn it_reports_a_missing_tailscale_tool() {
    let home = support::home();
    let s = server(home.path(), None);
    choose_tailscale(&s);
    assert_eq!(s.start_ngrok(json!({})).status, 202);
    assert_eq!(
        s.wait_status("failed")["failure"],
        "tailscale_not_installed"
    );
}

#[test]
fn a_saved_tailscale_setting_comes_back_up_at_startup_without_a_token() {
    let tools = tempfile::tempdir().unwrap();
    let home = support::home();
    {
        let s = server(home.path(), Some(fake_cli(tools.path(), RUNNING)));
        choose_tailscale(&s);
    }
    let s = server(home.path(), Some(fake_cli(tools.path(), RUNNING)));
    let v = s.wait_status("ready");
    assert_eq!(v["url"], format!("https://{HOST}"));
    assert_eq!(v["settings"]["provider"], "tailscale");
}

#[test]
fn the_provider_must_be_ngrok_or_tailscale() {
    let home = support::home();
    let s = server(home.path(), None);
    let r = s.write(
        "PUT",
        "/api/monitor-connections/settings/ngrok",
        &json!({"enabled": false, "revision": 0, "provider": "zerotier"}),
    );
    assert_eq!(r.status, 422, "{}", r.body);
    assert_eq!(r.json()["error"]["code"], "invalid_provider");
}

#[test]
fn the_snapshot_names_the_provider_that_is_running_not_the_one_that_is_saved() {
    let tools = tempfile::tempdir().unwrap();
    let home = support::home();
    let s = server(home.path(), Some(fake_cli(tools.path(), RUNNING)));
    // Nothing runs yet.
    assert_eq!(s.get()["activeProvider"], serde_json::Value::Null);
    choose_tailscale(&s);
    assert_eq!(s.get()["activeProvider"], serde_json::Value::Null);
    assert_eq!(s.start_ngrok(json!({})).status, 202);
    let ready = s.wait_status("ready");
    assert_eq!(ready["activeProvider"], "tailscale");

    // Saving another provider does not touch the live connection, and the snapshot says so.
    let r = s.write(
        "PUT",
        "/api/monitor-connections/settings/ngrok",
        &json!({"enabled": true, "revision": ready["revision"], "provider": "ngrok"}),
    );
    assert_eq!(r.status, 200, "{}", r.body);
    let v = r.json();
    assert_eq!(v["settings"]["provider"], "ngrok");
    assert_eq!(v["status"], "ready");
    assert_eq!(v["activeProvider"], "tailscale");

    s.stop_ngrok();
    assert_eq!(s.get()["activeProvider"], serde_json::Value::Null);
}

#[test]
fn a_start_that_fails_leaves_no_active_provider_and_names_the_failure() {
    let home = support::home();
    let s = server(home.path(), None);
    choose_tailscale(&s);
    assert_eq!(s.start_ngrok(json!({})).status, 202);
    let v = s.wait_status("failed");
    assert_eq!(v["failure"], "tailscale_not_installed");
    assert_eq!(v["activeProvider"], serde_json::Value::Null);
}
