use std::sync::mpsc;

use agent_project::file_store::watch::WatchMode;
use agent_project::server::{self, ServeConfig};
use serde_json::{Value, json};
use tokio::sync::oneshot;

fn start_server(
    home: &std::path::Path,
) -> (String, oneshot::Sender<()>, std::thread::JoinHandle<()>) {
    let (ready_tx, ready_rx) = mpsc::channel();
    let (stop_tx, stop_rx) = oneshot::channel::<()>();
    let dir = home.to_path_buf();
    let handle = std::thread::spawn(move || {
        server::run_until_in(
            &dir,
            WatchMode::Poll,
            ServeConfig { port: 0 },
            async move {
                let _ = stop_rx.await;
            },
            move |addr| {
                let _ = ready_tx.send(format!("http://127.0.0.1:{}", addr.port()));
            },
        )
        .unwrap();
    });
    let base = ready_rx.recv().unwrap();
    (base, stop_tx, handle)
}

#[test]
fn rap_http_routes_handle_work_authorization_interrupts_summary_and_sleep() {
    let home = tempfile::tempdir().unwrap();
    let workspace = home.path().join("workspace");
    std::fs::create_dir(&workspace).unwrap();
    let (base, stop_tx, handle) = start_server(home.path());

    // Fetch the board page token for local write_guard.
    let html: String = ureq::get(&format!("{base}/"))
        .call()
        .unwrap()
        .body_mut()
        .read_to_string()
        .unwrap();
    let marker = r#"name="agpj-token" content=""#;
    let start = html.find(marker).expect("page token meta") + marker.len();
    let end = html[start..].find('"').unwrap() + start;
    let token = &html[start..end];

    let post_json = |path: &str, body: Value| -> Value {
        let text = ureq::post(&format!("{base}{path}"))
            .header("Origin", &base)
            .header("Sec-Fetch-Site", "same-origin")
            .header("X-Agpj-Token", token)
            .header("Content-Type", "application/json")
            .send(body.to_string())
            .unwrap()
            .body_mut()
            .read_to_string()
            .unwrap();
        serde_json::from_str(&text).unwrap()
    };

    // 1. Create a RAP work item via mobile/web API.
    let created = post_json(
        "/api/rap/projects/demo/work",
        json!({
            "workspace": workspace,
            "kind": "task",
            "title": "Mobile API test",
            "rank": 1
        }),
    );
    let work_id = created["id"].as_i64().unwrap();
    assert_eq!(created["lane"], "backlog");

    // 2. Authorize direct execution -> moves to todo/ready.
    let contract = post_json(
        &format!("/api/rap/work/{work_id}/authorize-direct"),
        json!({ "brief": "Execute directly", "authorized_by": "human" }),
    );
    assert_eq!(contract["kind"], "direct_execution");

    // 3. Submit an interrupt/change request.
    let req = post_json(
        &format!("/api/rap/work/{work_id}/change-requests"),
        json!({ "body": "Also log latency" }),
    );
    assert_eq!(req["state"], "pending");

    // 4. Fetch summary (catch-up after disconnect) and verify pending change + blockers.
    let summary_text = ureq::get(&format!("{base}/api/rap/projects/demo/summary"))
        .call()
        .unwrap()
        .body_mut()
        .read_to_string()
        .unwrap();
    let summary: Value = serde_json::from_str(&summary_text).unwrap();
    assert_eq!(summary["sleeping"], false);
    assert_eq!(summary["items"].as_array().unwrap().len(), 1);
    assert_eq!(
        summary["items"][0]["pending_change_requests"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // 5. Enter and wake from Sleep.
    let slept = post_json("/api/rap/sleep/enter", json!({}));
    assert_eq!(slept["sleeping"], true);
    let woken = post_json("/api/rap/sleep/wake", json!({}));
    assert!(woken["sleep_ended_at"].as_i64().is_some());

    let _ = stop_tx.send(());
    handle.join().unwrap();
}
