//! `agboard setup remote`: save the ngrok connection settings.
//!
//! Runs the real `agboard` binary with `AGPJ_HOME` in a temporary directory and
//! `AGBOARD_KEYCHAIN=none`, so the real keychain is never touched. No server is
//! started and port 7420 is never used.

use std::path::Path;

use assert_cmd::Command;
use serde_json::Value;

const TOKEN: &str = "tok_secret_value_123";
const FILE: &str = ".internal/monitor-connection.json";

fn agboard(home: &Path) -> Command {
    let mut cmd = Command::cargo_bin("agboard").unwrap();
    cmd.env("AGBOARD_AUTOSTART", "0")
        .env("AGBOARD_OPEN_BROWSER", "0")
        .env("AGBOARD_KEYCHAIN", "none")
        .env("AGPJ_HOME", home)
        .env("TZ", "UTC")
        .env_remove("NGROK_AUTHTOKEN")
        .current_dir(home);
    cmd
}

fn text(out: &std::process::Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn saving_a_domain_and_enabled_uses_the_boards_file_and_says_when_it_applies() {
    let home = tempfile::tempdir().unwrap();
    let out = agboard(home.path())
        .args([
            "setup",
            "remote",
            "--domain",
            " Example.NGROK-free.dev ",
            "--enabled",
            "true",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["domain"], "example.ngrok-free.dev");
    assert_eq!(json["enabled"], true);
    assert!(
        json["note"]
            .as_str()
            .unwrap()
            .contains("does not replace a running connection")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(home.path().join(FILE)).unwrap()).unwrap();
    assert_eq!(file["formatVersion"], 1);
    assert_eq!(file["revision"], 1);
    assert_eq!(file["domain"], "example.ngrok-free.dev");

    let out = agboard(home.path())
        .args(["setup", "remote", "--clear-domain"])
        .output()
        .unwrap();
    let human = text(&out);
    assert!(human.contains("Domain: none"), "{human}");
    assert!(
        human.contains("applies the next time") || human.contains("apply the next time"),
        "{human}"
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(home.path().join(FILE)).unwrap()).unwrap();
    assert_eq!(
        (file["revision"].as_u64(), file.get("domain")),
        (Some(2), None)
    );
}

#[test]
fn a_bad_domain_exits_2_with_invalid_domain_and_saves_nothing() {
    let home = tempfile::tempdir().unwrap();
    let out = agboard(home.path())
        .args([
            "setup",
            "remote",
            "--domain",
            "https://x.ngrok-free.dev",
            "--json",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    let json: Value = serde_json::from_slice(&out.stderr).unwrap();
    assert_eq!(json["error"]["code"], "invalid_domain");
    assert!(!home.path().join(FILE).exists());
}

#[test]
fn the_token_is_read_only_from_stdin_and_never_appears_anywhere() {
    let home = tempfile::tempdir().unwrap();
    // No argument takes the token.
    let out = agboard(home.path())
        .args(["setup", "remote", "--token", TOKEN])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
    assert!(!text(&out).contains(TOKEN), "{}", text(&out));

    // With no keychain here the save is refused, changes nothing, and echoes nothing.
    let out = agboard(home.path())
        .args([
            "setup",
            "remote",
            "--domain",
            "a.ngrok-free.dev",
            "--token-stdin",
            "--json",
        ])
        .write_stdin(format!("{TOKEN}\n"))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1), "{}", text(&out));
    assert!(
        text(&out).contains("keychain_unavailable"),
        "{}",
        text(&out)
    );
    assert!(!text(&out).contains(TOKEN));
    assert!(!home.path().join(FILE).exists());

    // An empty or malformed token is refused without echoing it.
    for input in ["", "two words"] {
        let out = agboard(home.path())
            .args(["setup", "remote", "--token-stdin"])
            .write_stdin(input)
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(2), "{}", text(&out));
        assert!(!text(&out).contains("two words"));
    }
    for entry in walk(home.path()) {
        let bytes = std::fs::read(&entry).unwrap_or_default();
        assert!(
            !String::from_utf8_lossy(&bytes).contains(TOKEN),
            "{entry:?}"
        );
    }
}

#[test]
fn without_a_terminal_and_without_options_nothing_is_asked() {
    let home = tempfile::tempdir().unwrap();
    let out = agboard(home.path())
        .args(["setup", "remote"])
        .timeout(std::time::Duration::from_secs(20))
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("--domain"), "{}", text(&out));
}

#[test]
fn it_does_not_take_a_project_or_mix_with_the_agent_options() {
    let home = tempfile::tempdir().unwrap();
    for args in [
        vec!["--project", "demo", "setup", "remote", "--enabled", "true"],
        vec!["setup", "--yes", "remote", "--enabled", "true"],
        vec![
            "setup",
            "remote",
            "--domain",
            "a.ngrok-free.dev",
            "--clear-domain",
        ],
    ] {
        let out = agboard(home.path()).args(&args).output().unwrap();
        assert_eq!(out.status.code(), Some(2), "{args:?}: {}", text(&out));
    }
    assert!(!home.path().join(FILE).exists());
}

fn walk(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(walk(&path));
        } else {
            files.push(path);
        }
    }
    files
}

#[test]
fn show_remote_reads_what_was_saved_and_show_still_takes_a_task_id() {
    let home = tempfile::tempdir().unwrap();
    let out = agboard(home.path())
        .args([
            "setup",
            "remote",
            "--domain",
            "a.ngrok-free.dev",
            "--enabled",
            "true",
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let out = agboard(home.path())
        .args(["--json", "show", "remote"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{}", text(&out));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["domain"], "a.ngrok-free.dev");
    assert_eq!(json["enabled"], true);
    assert_eq!(json["tokenSaved"], false);
    // `show <id>` is a task and needs a project; `show` alone says what to give.
    let out = agboard(home.path()).args(["show", "12"]).output().unwrap();
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    assert!(text(&out).contains("--project"), "{}", text(&out));
    let out = agboard(home.path())
        .args(["--project", "x", "show"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
    let out = agboard(home.path())
        .args(["--project", "x", "show", "abc"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2), "{}", text(&out));
}

#[test]
fn the_provider_is_saved_and_a_name_other_than_ngrok_or_tailscale_is_refused() {
    let home = tempfile::tempdir().unwrap();
    let out = agboard(home.path())
        .args([
            "--json",
            "setup",
            "remote",
            "--provider",
            "tailscale",
            "--enabled",
            "true",
        ])
        .output()
        .unwrap();
    assert!(out.status.success(), "{}", text(&out));
    let json: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(json["provider"], "tailscale");
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(home.path().join(FILE)).unwrap()).unwrap();
    assert_eq!(file["provider"], "tailscale");
    assert_eq!(file["enabled"], true);

    let bad = agboard(home.path())
        .args(["setup", "remote", "--provider", "zerotier"])
        .output()
        .unwrap();
    assert_eq!(bad.status.code(), Some(2), "{}", text(&bad));
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(home.path().join(FILE)).unwrap()).unwrap();
    assert_eq!(file["provider"], "tailscale");
}

/// A directory holding a fake `tailscale` that prints `status` for any call.
#[cfg(unix)]
fn fake_tailscale(status: &str) -> tempfile::TempDir {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("tailscale");
    std::fs::write(&script, format!("#!/bin/sh\necho '{status}'\n")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    dir
}

/// Save the Tailscale provider with `path` as the `PATH`, and return the JSON and the text.
fn save_tailscale(home: &Path, path: &Path) -> (Value, String) {
    let run = |json: bool| {
        let mut cmd = agboard(home);
        cmd.env("PATH", path)
            .args(["setup", "remote", "--provider", "tailscale"]);
        if json {
            cmd.arg("--json");
        }
        let out = cmd.output().unwrap();
        assert_eq!(out.status.code(), Some(0), "{}", text(&out));
        out
    };
    let json = serde_json::from_slice(&run(true).stdout).unwrap();
    let human = String::from_utf8_lossy(&run(false).stdout).into_owned();
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(home.join(FILE)).unwrap()).unwrap();
    assert_eq!(file["provider"], "tailscale");
    (json, human)
}

#[cfg(unix)]
#[test]
fn saving_tailscale_says_what_to_do_in_each_state_and_never_fails() {
    let cases = [
        (
            r#"{"BackendState":"NeedsLogin"}"#,
            "not_logged_in",
            "tailscale up",
        ),
        (
            r#"{"BackendState":"Running","Self":{"DNSName":"a.ts.net."},"CertDomains":[]}"#,
            "https_disabled",
            "https://login.tailscale.com/admin/dns",
        ),
        (
            r#"{"BackendState":"Running","Self":{"DNSName":"A.ts.net."},"CertDomains":["a.ts.net"]}"#,
            "ready",
            "https://a.ts.net",
        ),
    ];
    for (status, code, advice) in cases {
        let home = tempfile::tempdir().unwrap();
        let fake = fake_tailscale(status);
        let (json, human) = save_tailscale(home.path(), fake.path());
        assert_eq!(json["saved"], true);
        assert_eq!(json["tailscale"]["status"], code, "{json}");
        assert!(human.contains(advice), "{human}");
    }
    let home = tempfile::tempdir().unwrap();
    let fake = fake_tailscale(cases[2].0);
    let (json, _) = save_tailscale(home.path(), fake.path());
    assert_eq!(json["tailscale"]["domain"], "a.ts.net");
}

#[cfg(unix)]
#[test]
fn saving_tailscale_without_it_installed_explains_both_ways_to_install() {
    // The macOS app, when present, is found even with an empty PATH.
    if Path::new("/Applications/Tailscale.app").exists() {
        return;
    }
    let home = tempfile::tempdir().unwrap();
    let empty = tempfile::tempdir().unwrap();
    let (json, human) = save_tailscale(home.path(), empty.path());
    assert_eq!(json["tailscale"]["status"], "not_installed");
    assert!(
        human.contains("brew install --cask tailscale-app"),
        "{human}"
    );
    assert!(human.contains("brew install tailscale"), "{human}");
    assert!(human.contains("cannot be used together"), "{human}");
}
