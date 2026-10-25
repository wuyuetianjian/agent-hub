use agent_orbit::{
    codex_hook::{decision, merge_config, validate_input, Connection},
    config,
    runtime::Runtime,
};
use serde_json::json;
#[test]
fn hook_decisions_never_grant_persistent_or_implicit_access() {
    assert_eq!(
        decision(true)["hookSpecificOutput"]["decision"]["behavior"],
        "allow"
    );
    assert_eq!(
        decision(false)["hookSpecificOutput"]["decision"]["behavior"],
        "deny"
    );
}
#[test]
fn hook_config_merge_preserves_other_handlers_and_is_idempotent() {
    let old = json!({"description":"keep", "hooks":{"Stop":[{"hooks":[{"type":"command","command":"existing"}]}]}});
    let merged = merge_config(
        old.clone(),
        "orbit codex-hook /private/connection.json",
        300,
    )
    .unwrap();
    assert_eq!(merged["hooks"]["Stop"], old["hooks"]["Stop"]);
    assert_eq!(
        merge_config(
            merged.clone(),
            "orbit codex-hook /private/connection.json",
            300
        )
        .unwrap(),
        merged
    );
    assert!(merge_config(json!({"hooks":{"PermissionRequest":"broken"}}), "test", 300).is_err());
}
#[test]
fn incoming_hook_is_bound_to_the_original_thread_and_canonical_directory() {
    let dir = tempfile::tempdir().unwrap();
    let c = Connection {
        url: "http://127.0.0.1:1".into(),
        token: "test-only".into(),
        session_id: "external:codex:thread".into(),
        thread_id: "thread".into(),
        cwd: dir.path().to_string_lossy().into_owned(),
        timeout_seconds: 300,
    };
    let good = json!({"session_id":"thread","cwd":dir.path(),"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"test-only","description":"test reason"}});
    validate_input(&c, &good).unwrap();
    let mut wrong = good.clone();
    wrong["session_id"] = "another".into();
    assert!(validate_input(&c, &wrong).is_err());
    wrong = good.clone();
    wrong["hook_event_name"] = "PreToolUse".into();
    assert!(validate_input(&c, &wrong).is_err());
    wrong = good.clone();
    wrong["cwd"] = "/wrong-project".into();
    assert!(validate_input(&c, &wrong).is_err());
}
#[test]
fn attached_hook_does_not_claim_or_terminate_the_original_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = Runtime::open(config::load().unwrap(), dir.path().into()).unwrap();
    let (profile, _) = runtime.profile("codex").unwrap();
    let thread = uuid::Uuid::new_v4().to_string();
    let info = agent_orbit::model::SessionInfo {
        id: format!("external:codex:{thread}"),
        profile_id: "codex".into(),
        provider_id: "codex".into(),
        cwd: dir.path().to_string_lossy().into_owned(),
        mode: "external".into(),
        status: "external".into(),
        read_only: true,
        interaction_channel: "history".into(),
        title: "Existing thread".into(),
        source: "test-original-client".into(),
        started_at: 0,
        updated_at: 0,
    };
    let (attached, token) = runtime
        .sessions
        .attach_codex_hook(&profile, info, &runtime.settings.read().unwrap())
        .unwrap();
    assert_eq!(attached.mode, "hook");
    assert_eq!(attached.status, "hook-pending");
    assert!(!attached.read_only);
    assert!(runtime
        .sessions
        .authenticate(&attached.id, &token)
        .is_some());
    assert!(runtime
        .sessions
        .authenticate(&attached.id, "wrong-token")
        .is_none());
    assert!(runtime.sessions.write(&attached.id, "test").is_err());
    runtime.sessions.stop(&attached.id).unwrap();
    assert!(runtime
        .sessions
        .authenticate(&attached.id, &token)
        .is_none());
}

#[test]
fn approval_proofs_bind_direction_nonce_and_exact_command_without_sending_secret() {
    use agent_orbit::codex_hook::{proof, verify_proof};
    let request = json!({"command":"test-only"});
    let signature = proof("private-key", "request", "nonce-one", &request);
    assert!(!signature.contains("private-key"));
    verify_proof("private-key", "request", "nonce-one", &request, &signature).unwrap();
    assert!(verify_proof("private-key", "response", "nonce-one", &request, &signature).is_err());
    assert!(verify_proof("private-key", "request", "nonce-two", &request, &signature).is_err());
    assert!(verify_proof(
        "private-key",
        "request",
        "nonce-one",
        &json!({"command":"changed"}),
        &signature
    )
    .is_err());
    assert!(verify_proof("wrong-key", "request", "nonce-one", &request, &signature).is_err());
}

#[tokio::test]
async fn original_session_approval_round_trip_and_disconnect_are_scoped() {
    use agent_orbit::{
        codex_hook::{proof, receive, verify_proof},
        model::{Reply, SessionInfo},
    };
    use axum::{
        extract::{Path, State},
        http::{HeaderMap, HeaderValue, StatusCode},
        Json,
    };
    let dir = tempfile::tempdir().unwrap();
    let runtime = Runtime::open(config::load().unwrap(), dir.path().into()).unwrap();
    let (profile, _) = runtime.profile("codex").unwrap();
    let thread = uuid::Uuid::new_v4().to_string();
    let info = SessionInfo {
        id: format!("external:codex:{thread}"),
        profile_id: "codex".into(),
        provider_id: "codex".into(),
        cwd: dir.path().to_string_lossy().into_owned(),
        mode: "external".into(),
        status: "external".into(),
        read_only: true,
        interaction_channel: "history".into(),
        title: "Original thread".into(),
        source: "test-only".into(),
        started_at: 0,
        updated_at: 0,
    };
    let (info, secret) = runtime
        .sessions
        .attach_codex_hook(&profile, info, &runtime.settings.read().unwrap())
        .unwrap();
    let input = json!({"session_id":thread,"cwd":dir.path(),"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"TEST ONLY: no shell is executed","description":"User approval reason"}});
    let nonce = uuid::Uuid::new_v4().to_string();
    let mut headers = HeaderMap::new();
    headers.insert("x-orbit-nonce", HeaderValue::from_str(&nonce).unwrap());
    headers.insert(
        "x-orbit-proof",
        HeaderValue::from_str(&proof(&secret, "request", &nonce, &input)).unwrap(),
    );
    let mut origin = headers.clone();
    origin.insert(
        "origin",
        HeaderValue::from_static("https://example.invalid"),
    );
    assert_eq!(
        receive(
            State(runtime.clone()),
            Path(info.id.clone()),
            origin,
            Json(input.clone())
        )
        .await
        .unwrap_err(),
        StatusCode::FORBIDDEN
    );
    let mut invalid = headers.clone();
    invalid.insert("x-orbit-proof", HeaderValue::from_static("invalid"));
    assert_eq!(
        receive(
            State(runtime.clone()),
            Path(info.id.clone()),
            invalid,
            Json(input.clone())
        )
        .await
        .unwrap_err(),
        StatusCode::UNAUTHORIZED
    );
    for allow in [true, false] {
        let task = tokio::spawn(receive(
            State(runtime.clone()),
            Path(info.id.clone()),
            headers.clone(),
            Json(input.clone()),
        ));
        let pending = tokio::time::timeout(std::time::Duration::from_secs(2), async {
            loop {
                if let Some(r) = runtime.broker.list().into_iter().next() {
                    break r;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(pending.session_id, info.id);
        assert!(pending.detail.contains("User approval reason"));
        assert_eq!(runtime.sessions.list()[0].status, "hook-connected");
        runtime
            .broker
            .respond(
                &pending.id,
                Reply {
                    choice: Some(if allow { "allow" } else { "deny" }.into()),
                    answers: Default::default(),
                },
            )
            .unwrap();
        let Json(answer) = task.await.unwrap().unwrap();
        assert_eq!(
            answer["result"]["hookSpecificOutput"]["decision"]["behavior"],
            if allow { "allow" } else { "deny" }
        );
        verify_proof(
            &secret,
            "response",
            &nonce,
            &answer["result"],
            answer["proof"].as_str().unwrap(),
        )
        .unwrap();
        assert!(runtime.broker.list().is_empty());
    }
    // Even a direct store update after insertion cannot turn a stale request into allow.
    let task = tokio::spawn(receive(
        State(runtime.clone()),
        Path(info.id.clone()),
        headers.clone(),
        Json(input.clone()),
    ));
    let pending = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if let Some(r) = runtime.broker.list().into_iter().next() {
                break r;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut different = profile.clone();
    different.config_dir = dir
        .path()
        .join("changed-during-prompt")
        .to_string_lossy()
        .into_owned();
    runtime.store.save_profile(&different).unwrap();
    runtime
        .broker
        .respond(
            &pending.id,
            Reply {
                choice: Some("allow".into()),
                answers: Default::default(),
            },
        )
        .unwrap();
    let Json(rejected) = task.await.unwrap().unwrap();
    assert_eq!(rejected["result"], json!({}));
    runtime.store.save_profile(&profile).unwrap();
    let mut wrong_project = input.clone();
    wrong_project["cwd"] = "/not-the-bound-project".into();
    let mut forged = headers.clone();
    forged.insert(
        "x-orbit-proof",
        HeaderValue::from_str(&proof(&secret, "request", &nonce, &wrong_project)).unwrap(),
    );
    assert_eq!(
        receive(
            State(runtime.clone()),
            Path(info.id.clone()),
            forged,
            Json(wrong_project)
        )
        .await
        .unwrap_err(),
        StatusCode::BAD_REQUEST
    );
    runtime
        .settings
        .write()
        .unwrap()
        .interaction_timeout_seconds = 1;
    let Json(expired) = receive(
        State(runtime.clone()),
        Path(info.id.clone()),
        headers.clone(),
        Json(input.clone()),
    )
    .await
    .unwrap();
    assert_eq!(expired["result"], json!({}));
    assert!(runtime.broker.list().is_empty());
    let mut changed = profile.clone();
    changed.config_dir = dir
        .path()
        .join("other-account")
        .to_string_lossy()
        .into_owned();
    runtime.store.save_profile(&changed).unwrap();
    assert_eq!(
        receive(
            State(runtime.clone()),
            Path(info.id.clone()),
            headers.clone(),
            Json(input.clone())
        )
        .await
        .unwrap_err(),
        StatusCode::CONFLICT
    );
    runtime.store.save_profile(&profile).unwrap();
    runtime
        .settings
        .write()
        .unwrap()
        .interaction_timeout_seconds = 10;
    let task = tokio::spawn(receive(
        State(runtime.clone()),
        Path(info.id.clone()),
        headers.clone(),
        Json(input.clone()),
    ));
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        while runtime.broker.list().is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    runtime.save_profile(changed).unwrap();
    assert!(runtime.broker.list().is_empty());
    let Json(cancelled) = task.await.unwrap().unwrap();
    assert_eq!(cancelled["result"], json!({}));
    runtime.sessions.stop(&info.id).unwrap();
    assert_eq!(
        receive(State(runtime), Path(info.id), headers, Json(input))
            .await
            .unwrap_err(),
        StatusCode::UNAUTHORIZED
    );
}

#[test]
fn connection_preparation_keeps_credentials_private_and_preserves_normal_approval_policy() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = Runtime::open(config::load().unwrap(), dir.path().into()).unwrap();
    *runtime.bridge_url.write().unwrap() = "http://127.0.0.1:1".into();
    let thread = uuid::Uuid::new_v4().to_string();
    let info = agent_orbit::model::SessionInfo {
        id: format!("external:codex:{thread}"),
        profile_id: "codex".into(),
        provider_id: "codex".into(),
        cwd: dir.path().to_string_lossy().into_owned(),
        mode: "external".into(),
        status: "external".into(),
        read_only: true,
        interaction_channel: "history".into(),
        title: "Current original client".into(),
        source: "test-only".into(),
        started_at: 0,
        updated_at: 0,
    };
    let setup = agent_orbit::codex_hook::prepare(
        &runtime,
        info.clone(),
        std::path::Path::new("/test/Agent Orbit"),
    )
    .unwrap();
    let recovered =
        agent_orbit::codex_hook::prepare(&runtime, info, std::path::Path::new("/test/Agent Orbit"))
            .unwrap();
    assert_eq!(recovered.install_command, setup.install_command);
    assert_eq!(recovered.configuration, setup.configuration);
    let value = serde_json::to_string(&setup).unwrap();
    assert!(!value.contains("approvalPolicy"));
    assert!(!value.contains("dangerously-bypass"));
    assert!(!value.contains("127.0.0.1"));
    let files: Vec<_> = std::fs::read_dir(dir.path().join("codex-hooks"))
        .unwrap()
        .map(|p| p.unwrap().path())
        .collect();
    let path = files
        .iter()
        .find(|p| !p.to_string_lossy().ends_with(".hooks.json"))
        .unwrap();
    let c: Connection = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    assert_eq!(c.thread_id, thread);
    assert!(!value.contains(&c.token));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(path).unwrap().permissions().mode() & 0o077,
            0
        );
    }
    assert!(!dir.path().join(".codex/hooks.json").exists());
}

#[test]
fn installer_preserves_other_hooks_and_rejects_malformed_configuration_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    let source = dir.path().join("generated.json");
    let target = dir.path().join("project/hooks.json");
    std::fs::create_dir_all(target.parent().unwrap()).unwrap();
    let original =
        json!({"hooks":{"Stop":[{"hooks":[{"type":"command","command":"keep-existing"}]}]}});
    std::fs::write(
        &source,
        merge_config(json!({}), "test-only-hook", 300)
            .unwrap()
            .to_string(),
    )
    .unwrap();
    std::fs::write(&target, original.to_string()).unwrap();
    agent_orbit::codex_hook::install(&source, &target).unwrap();
    let installed: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&target).unwrap()).unwrap();
    assert_eq!(installed["hooks"]["Stop"], original["hooks"]["Stop"]);
    assert_eq!(
        installed["hooks"]["PermissionRequest"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    std::fs::write(&target, "malformed").unwrap();
    assert!(agent_orbit::codex_hook::install(&source, &target).is_err());
    assert_eq!(std::fs::read_to_string(target).unwrap(), "malformed");
}
#[test]
fn disconnected_helper_defers_to_the_original_client_instead_of_approving() {
    let dir = tempfile::tempdir().unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_agent-orbit"))
        .args([
            "codex-hook",
            dir.path().join("missing.json").to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result, json!({}));
}
