//! `GET`/`PUT /api/projects/{id}/mode` and the `mode` event over HTTP: the saved interaction
//! mode, the write guard, and the notice for a change another process makes.

mod common;

use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

use agent_project::mode::{self, ModeKind};
use common::{Running, demo_project};
use serde_json::json;

fn project_id(home: &Path) -> String {
    let mut ids: Vec<String> = std::fs::read_dir(home.join("shared").join("projects"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(ids.len(), 1, "{ids:?}");
    ids.remove(0)
}

fn put(server: &Running, id: &str, body: &serde_json::Value) -> common::Reply {
    server.write(
        "PUT",
        &format!("/api/projects/{id}/mode"),
        Some(&body.to_string()),
    )
}

#[test]
fn a_project_starts_in_sparring_and_a_save_is_read_back_and_kept() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let id = project_id(home.path());
    let server = Running::start(home.path());
    let path = format!("/api/projects/{id}/mode");
    let first = server.get(&path);
    assert_eq!(first.status, 200, "{}", first.body);
    assert_eq!(first.header("cache-control"), Some("no-store"));
    assert_eq!(first.json(), json!({"mode": "sparring"}));

    let saved = put(&server, &id, &json!({"mode": "sleep"}));
    assert_eq!(saved.status, 200, "{}", saved.body);
    assert_eq!(saved.json()["mode"], "sleep");
    assert!(saved.json()["since"].is_string());
    assert_eq!(server.get(&path).json(), saved.json());
    // The same file the CLI reads.
    assert_eq!(mode::current(home.path(), &id), ModeKind::Sleep);

    let back = put(&server, &id, &json!({"mode": "sparring"}));
    assert_eq!(back.json(), json!({"mode": "sparring"}));
    assert_eq!(mode::current(home.path(), &id), ModeKind::Sparring);
    server.stop();
}

#[test]
fn an_invalid_body_or_an_unknown_project_changes_nothing() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let id = project_id(home.path());
    let server = Running::start(home.path());
    for body in [
        json!({}),
        json!({"mode": "brainstorm"}),
        json!({"mode": "sleep", "extra": 1}),
        json!({"mode": 1}),
    ] {
        let reply = put(&server, &id, &body);
        assert_eq!(reply.status, 422, "{body}: {}", reply.body);
        assert_eq!(reply.code(), "invalid_mode", "{body}");
    }
    assert_eq!(mode::current(home.path(), &id), ModeKind::Sparring);
    let unknown = put(
        &server,
        "0123456789abcdef0123456789abcdef",
        &json!({"mode": "sleep"}),
    );
    assert_eq!(unknown.status, 404, "{}", unknown.body);
    assert_eq!(unknown.code(), "project_not_found");
    server.stop();
}

#[test]
fn a_write_without_the_page_token_is_refused_and_changes_nothing() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let id = project_id(home.path());
    let server = Running::start(home.path());
    let refused = server.request(
        "PUT",
        &format!("/api/projects/{id}/mode"),
        Some(&json!({"mode": "sleep"}).to_string()),
        &[
            (
                "Origin".into(),
                format!("http://127.0.0.1:{}", server.port()),
            ),
            ("Sec-Fetch-Site".into(), "same-origin".into()),
        ],
        Some("application/json"),
    );
    assert_eq!(refused.status, 403, "{}", refused.body);
    assert_eq!(mode::current(home.path(), &id), ModeKind::Sparring);
    server.stop();
}

#[test]
fn a_change_made_by_another_process_is_sent_as_a_mode_event_and_never_changed() {
    let home = tempfile::tempdir().unwrap();
    demo_project(home.path());
    let id = project_id(home.path());
    let server = Running::start(home.path());
    let mut stream = TcpStream::connect(("127.0.0.1", server.port())).unwrap();
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    write!(
        stream,
        "GET /api/events HTTP/1.1\r\nHost: 127.0.0.1:{}\r\nAccept: text/event-stream\r\nConnection: close\r\n\r\n",
        server.port()
    )
    .unwrap();
    let mut lines = BufReader::new(stream);
    let mut seen = String::new();
    while !seen.contains("event: ready") {
        let mut line = String::new();
        lines.read_line(&mut line).unwrap();
        seen.push_str(&line);
    }

    // What `agpj mode sleep` does, from outside the server.
    mode::set(home.path(), &id, ModeKind::Sleep, chrono::Utc::now()).unwrap();

    let mut found = String::new();
    loop {
        let mut line = String::new();
        let read = lines.read_line(&mut line).unwrap();
        assert!(read > 0, "the stream ended before the notice: {found}");
        found.push_str(&line);
        if found.contains("event: mode") && found.contains(&format!("\"{id}\"")) {
            break;
        }
        assert!(!found.contains("event: changed"), "{found}");
    }
    assert!(found.contains("\"mode\":\"sleep\""), "{found}");
    assert!(!found.contains("event: changed"), "{found}");
    server.stop();
}
