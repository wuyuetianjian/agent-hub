use agent_orbit::{config, discovery, protocol::RpcClient, providers};
use serde_json::json;
#[tokio::test]
async fn json_rpc_correlates_responses_and_reports_disconnect() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("fake agent.mjs");
    std::fs::write(&file,r#"import readline from 'node:readline';readline.createInterface({input:process.stdin}).on('line',line=>{const m=JSON.parse(line);if(m.method==='exit'){process.exit(0);}else console.log(JSON.stringify({id:m.id,result:{echo:m.params}}));});"#).unwrap();
    let rpc = RpcClient::spawn("node", &[file.to_string_lossy().into_owned()], None, &[], 3)
        .await
        .unwrap();
    let answer = rpc
        .call("echo", json!({"unicode":"你好","zero":0}))
        .await
        .unwrap();
    assert_eq!(answer["echo"]["unicode"], "你好");
    assert_eq!(answer["echo"]["zero"], 0);
    assert!(rpc.call("exit", json!({})).await.is_err());
    rpc.close();
}
#[tokio::test]
async fn collector_returns_real_zero_and_does_not_forward_extra_fields() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("quota file.json");
    std::fs::write(&file,r#"{"account":"test@example.invalid","windows":[{"label":"5h","remainingPercent":0,"windowMinutes":300}],"access_token":"DO_NOT_FORWARD"}"#).unwrap();
    let cfg = config::load().unwrap();
    let store = agent_orbit::store::Store::open(&dir.path().join("state.db"), &cfg).unwrap();
    let mut profile = store.profiles().unwrap().remove(0);
    profile.quota_file = file.to_string_lossy().into_owned();
    let snapshot = providers::refresh(&profile, &cfg.providers[0], &cfg, &cfg.settings).await;
    assert_eq!(snapshot.status, "ready");
    assert_eq!(snapshot.windows[0].remaining_percent, Some(0.0));
    assert!(!serde_json::to_string(&snapshot)
        .unwrap()
        .contains("DO_NOT_FORWARD"));
}
#[test]
fn changed_origin_cannot_receive_existing_credentials() {
    let mut cfg = config::load().unwrap();
    cfg.endpoints
        .insert("claudeUsage".into(), "https://example.invalid/usage".into());
    assert!(providers::endpoint(&cfg, "claudeUsage").is_err());
}
#[test]
fn nonexistent_cli_path_does_not_report_installed_executable() {
    let cfg = config::load().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let store = agent_orbit::store::Store::open(&dir.path().join("state.db"), &cfg).unwrap();
    let mut p = store.profiles().unwrap().remove(0);
    p.executable = dir
        .path()
        .join("does not exist")
        .to_string_lossy()
        .into_owned();
    assert!(discovery::discover(&p, &cfg.providers[0])
        .executable
        .is_none());
}
#[tokio::test]
async fn dropping_rpc_scope_terminates_the_owned_process() {
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("alive");
    let script = dir.path().join("idle.mjs");
    std::fs::write(&script,r#"import fs from 'node:fs';setInterval(()=>fs.writeFileSync(process.argv[2],String(Date.now())),20);"#).unwrap();
    let rpc = RpcClient::spawn(
        "node",
        &[
            script.to_string_lossy().into_owned(),
            marker.to_string_lossy().into_owned(),
        ],
        None,
        &[],
        3,
    )
    .await
    .unwrap();
    let guard = agent_orbit::protocol::RpcCloseGuard::new(rpc.clone());
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while !marker.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    drop(guard);
    tokio::time::sleep(std::time::Duration::from_millis(120)).await;
    let first = std::fs::read_to_string(&marker).unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    assert_eq!(std::fs::read_to_string(&marker).unwrap(), first);
}
#[tokio::test]
async fn blocked_rpc_input_obeys_the_request_deadline() {
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("blocked.mjs");
    std::fs::write(&script, "setInterval(()=>{},1000)").unwrap();
    let rpc = RpcClient::spawn(
        "node",
        &[script.to_string_lossy().into_owned()],
        None,
        &[],
        1,
    )
    .await
    .unwrap();
    let _guard = agent_orbit::protocol::RpcCloseGuard::new(rpc.clone());
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        rpc.call("blocked", json!({"text":"x".repeat(2*1024*1024)})),
    )
    .await;
    assert!(
        result.is_ok(),
        "stdin write must not outlive configured deadline"
    );
    assert!(result.unwrap().is_err());
}
#[tokio::test]
async fn pty_handles_unicode_and_keeps_its_original_account_binding() {
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::sync::Arc;
    let dir = tempfile::tempdir().unwrap();
    let script = dir.path().join("terminal agent.mjs");
    std::fs::write(&script,r#"import readline from 'node:readline';console.log('READY');readline.createInterface({input:process.stdin}).on('line',line=>{console.log('received:'+line);process.exit(0);});"#).unwrap();
    let cfg = config::load().unwrap();
    let store = agent_orbit::store::Store::open(&dir.path().join("state.db"), &cfg).unwrap();
    let mut p = store
        .profiles()
        .unwrap()
        .into_iter()
        .find(|p| p.provider_id == "claude")
        .unwrap();
    p.executable = which::which("node").unwrap().to_string_lossy().into_owned();
    p.args = vec![script.to_string_lossy().into_owned()];
    p.label = "Account A".into();
    let (events, _) = tokio::sync::broadcast::channel(128);
    let sessions = Arc::new(agent_orbit::sessions::Sessions::new(
        events,
        Arc::new(agent_orbit::broker::Broker::default()),
    ));
    let provider = cfg
        .providers
        .iter()
        .find(|v| v.id == p.provider_id)
        .unwrap();
    let info = sessions
        .start(
            &p,
            provider,
            dir.path().to_str().unwrap(),
            "terminal",
            &cfg.settings,
            "",
        )
        .await
        .unwrap();
    assert!(sessions.profile_matches(&info.id, &p));
    let mut changed = p.clone();
    changed.config_dir = "different-account".into();
    assert!(!sessions.profile_matches(&info.id, &changed));
    assert_eq!(
        sessions.original_profile(&info.id).unwrap().label,
        "Account A"
    );
    let mut last_output = String::new();
    let received = tokio::time::timeout(std::time::Duration::from_secs(4), async {
        let mut cursor_replied = false;
        loop {
            let replay = sessions.replay(&info.id).unwrap();
            let output = String::from_utf8_lossy(
                &STANDARD.decode(replay["bytes"].as_str().unwrap()).unwrap(),
            )
            .into_owned();
            last_output = output.clone();
            // ConPTY inherits the terminal cursor and waits for a DSR reply.
            // Replay accumulates bytes, so split requests are handled as well.
            if !cursor_replied && output.contains("\x1b[6n") {
                sessions.write(&info.id, "\x1b[1;1R").unwrap();
                cursor_replied = true;
            }
            if output.contains("READY") {
                sessions.write(&info.id, "你好\r").unwrap();
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
        loop {
            let replay = sessions.replay(&info.id).unwrap();
            let output = String::from_utf8_lossy(
                &STANDARD.decode(replay["bytes"].as_str().unwrap()).unwrap(),
            )
            .into_owned();
            last_output = output.clone();
            if output.contains("received:你好") {
                return output;
            }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await;
    sessions.stop_all();
    assert!(
        received.is_ok(),
        "PTY must deliver Unicode input and output; captured terminal output: {last_output:?}"
    );
}
