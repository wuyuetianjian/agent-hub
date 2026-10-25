use agent_orbit::{
    codex_dispatcher as dispatch, codex_hook::proof, config, model::Reply, runtime::Runtime,
};
use axum::{
    extract::{Path, State},
    http::{HeaderMap, HeaderValue},
    Json,
};
use serde_json::{json, Value};
use std::sync::Arc;

fn runtime(dir: &std::path::Path) -> Arc<Runtime> {
    let runtime = Runtime::open(config::load().unwrap(), dir.join("data")).unwrap();
    let home = dir.join("codex-home");
    std::fs::create_dir_all(&home).unwrap();
    let (mut profile, _) = runtime.profile("codex").unwrap();
    profile.config_dir = home.to_string_lossy().into_owned();
    runtime.save_profile(profile).unwrap();
    *runtime.bridge_url.write().unwrap() = "http://127.0.0.1:12345".into();
    runtime
}

fn headers(secret: &str, input: &Value) -> HeaderMap {
    let nonce = uuid::Uuid::new_v4().to_string();
    let mut headers = HeaderMap::new();
    headers.insert("x-orbit-nonce", HeaderValue::from_str(&nonce).unwrap());
    headers.insert(
        "x-orbit-proof",
        HeaderValue::from_str(&proof(secret, "request", &nonce, input)).unwrap(),
    );
    headers
}

async fn pending(runtime: &Runtime, count: usize) {
    tokio::time::timeout(std::time::Duration::from_secs(3), async {
        while runtime.broker.list().len() != count {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("expected approval cards");
}

#[tokio::test]
async fn user_dispatcher_routes_two_new_windows_in_different_projects_independently() {
    let dir = tempfile::tempdir().unwrap();
    let runtime = runtime(dir.path());
    let setup = dispatch::prepare(&runtime, "codex", std::path::Path::new("/test/orbit")).unwrap();
    assert_eq!(
        setup.target,
        std::fs::canonicalize(dir.path().join("codex-home"))
            .unwrap()
            .join("hooks.json")
    );
    let d: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&setup.descriptor).unwrap()).unwrap();
    let mut tasks = vec![];
    for name in ["first", "second"] {
        let cwd = dir.path().join(name);
        std::fs::create_dir(&cwd).unwrap();
        let input = json!({"session_id":uuid::Uuid::new_v4().to_string(),"cwd":cwd,"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":name,"description":"Approval test; no command executes"}});
        let h = headers(&d.token, &input);
        let rt = runtime.clone();
        tasks.push(tokio::spawn(async move {
            dispatch::receive(State(rt), Path("codex".into()), h, Json(input))
                .await
                .unwrap()
                .0
        }));
    }
    pending(&runtime, 2).await;
    let requests = runtime.broker.list();
    assert_ne!(requests[0].session_id, requests[1].session_id);
    assert_ne!(requests[0].cwd, requests[1].cwd);
    let first = requests
        .iter()
        .find(|r| r.detail.contains("first"))
        .unwrap();
    runtime
        .broker
        .respond(
            &first.id,
            Reply {
                choice: Some("allow".into()),
                answers: Default::default(),
            },
        )
        .unwrap();
    pending(&runtime, 1).await;
    let second = runtime.broker.list().remove(0);
    assert!(second.detail.contains("second"));
    runtime
        .broker
        .respond(
            &second.id,
            Reply {
                choice: Some("deny".into()),
                answers: Default::default(),
            },
        )
        .unwrap();
    assert_eq!(
        tasks.remove(0).await.unwrap()["result"]["hookSpecificOutput"]["decision"]["behavior"],
        "allow"
    );
    assert_eq!(
        tasks.remove(0).await.unwrap()["result"]["hookSpecificOutput"]["decision"]["behavior"],
        "deny"
    );
}

#[test]
fn dispatcher_restores_a_stable_command_with_a_fresh_listener_and_credential() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let first = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let old: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&first.descriptor).unwrap()).unwrap();
    drop(rt);
    let rt = Runtime::open(config::load().unwrap(), dir.path().join("data")).unwrap();
    *rt.bridge_url.write().unwrap() = "http://127.0.0.1:54321".into();
    dispatch::restore(&rt).unwrap();
    let second = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let fresh: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&second.descriptor).unwrap()).unwrap();
    assert_eq!(first.configuration, second.configuration);
    assert_eq!(first.descriptor, second.descriptor);
    assert_eq!(fresh.url, "http://127.0.0.1:54321");
    assert_ne!(old.token, fresh.token);
}

#[test]
fn overlapping_account_homes_fail_instead_of_misattributing_approvals() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let (mut duplicate, _) = rt.profile("codex").unwrap();
    duplicate.id = "second-account".into();
    rt.save_profile(duplicate).unwrap();
    assert!(dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).is_err());
}

#[test]
fn installing_global_hook_removes_only_known_orbit_project_handlers() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let cwd = dir.path().join("project");
    std::fs::create_dir_all(cwd.join(".codex")).unwrap();
    let thread = uuid::Uuid::new_v4().to_string();
    let info = agent_orbit::model::SessionInfo {
        id: format!("external:codex:{thread}"),
        profile_id: "codex".into(),
        provider_id: "codex".into(),
        cwd: cwd.to_string_lossy().into_owned(),
        mode: "external".into(),
        status: "external".into(),
        read_only: true,
        interaction_channel: "history".into(),
        title: "Original".into(),
        source: "test".into(),
        started_at: 0,
        updated_at: 0,
    };
    let old =
        agent_orbit::codex_hook::prepare(&rt, info, std::path::Path::new("/test/orbit")).unwrap();
    let project_hooks = cwd.join(".codex/hooks.json");
    let mut config = old.configuration;
    config["hooks"]["PermissionRequest"]
        .as_array_mut()
        .unwrap()
        .push(json!({"hooks":[{"type":"command","command":"unrelated-permission-hook"}]}));
    config["hooks"]["Stop"] =
        json!([{"hooks":[{"type":"command","command":"unrelated-stop-hook"}]}]);
    std::fs::write(&project_hooks, serde_json::to_vec(&config).unwrap()).unwrap();
    let setup = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    agent_orbit::codex_hook::install(
        &setup.descriptor.with_extension("hooks.json"),
        &setup.target,
    )
    .unwrap();
    let migrated: Value = serde_json::from_slice(&std::fs::read(&project_hooks).unwrap()).unwrap();
    assert_eq!(
        migrated["hooks"]["PermissionRequest"],
        json!([{"hooks":[{"type":"command","command":"unrelated-permission-hook"}]}])
    );
    assert_eq!(migrated["hooks"]["Stop"], config["hooks"]["Stop"]);
    let user: Value = serde_json::from_slice(&std::fs::read(&setup.target).unwrap()).unwrap();
    assert_eq!(
        user["hooks"]["PermissionRequest"].as_array().unwrap().len(),
        1
    );
    agent_orbit::codex_hook::install(
        &setup.descriptor.with_extension("hooks.json"),
        &setup.target,
    )
    .unwrap();
    let again: Value = serde_json::from_slice(&std::fs::read(&setup.target).unwrap()).unwrap();
    assert_eq!(user, again);
}

#[tokio::test]
async fn dispatcher_rejects_replay_and_cancels_pending_requests_when_disabled() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let setup = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let d: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&setup.descriptor).unwrap()).unwrap();
    let input = json!({"session_id":uuid::Uuid::new_v4().to_string(),"cwd":dir.path(),"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"no execution"}});
    let h = headers(&d.token, &input);
    let task = {
        let rt = rt.clone();
        let h = h.clone();
        let input = input.clone();
        tokio::spawn(async move {
            dispatch::receive(State(rt), Path("codex".into()), h, Json(input))
                .await
                .unwrap()
                .0
        })
    };
    pending(&rt, 1).await;
    assert!(dispatch::receive(
        State(rt.clone()),
        Path("codex".into()),
        h.clone(),
        Json(input.clone())
    )
    .await
    .is_err());
    assert_eq!(rt.broker.list().len(), 1);
    dispatch::disable(&rt, "codex").unwrap();
    assert!(rt.broker.list().is_empty());
    assert_eq!(task.await.unwrap()["result"], json!({}));
    assert!(
        dispatch::receive(State(rt.clone()), Path("codex".into()), h, Json(input))
            .await
            .is_err()
    );
}

#[tokio::test]
async fn disconnected_thread_stays_blocked_across_restart_but_new_window_can_connect() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let setup = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let d: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&setup.descriptor).unwrap()).unwrap();
    let thread = uuid::Uuid::new_v4().to_string();
    let input = json!({"session_id":thread,"cwd":dir.path(),"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"no execution"}});
    let task = {
        let rt = rt.clone();
        let h = headers(&d.token, &input);
        let input = input.clone();
        tokio::spawn(async move {
            dispatch::receive(State(rt), Path("codex".into()), h, Json(input))
                .await
                .unwrap()
                .0
        })
    };
    pending(&rt, 1).await;
    let id = rt.broker.list()[0].session_id.clone();
    dispatch::disconnect_session(&rt, &id).unwrap();
    assert_eq!(task.await.unwrap()["result"], json!({}));
    drop(rt);
    let rt = Runtime::open(config::load().unwrap(), dir.path().join("data")).unwrap();
    *rt.bridge_url.write().unwrap() = "http://127.0.0.1:12346".into();
    dispatch::restore(&rt).unwrap();
    let fresh: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&setup.descriptor).unwrap()).unwrap();
    assert_eq!(
        dispatch::receive(
            State(rt.clone()),
            Path("codex".into()),
            headers(&fresh.token, &input),
            Json(input)
        )
        .await
        .unwrap()
        .0["result"],
        json!({})
    );
    assert!(rt.broker.list().is_empty());
    let mut input = json!({"session_id":uuid::Uuid::new_v4().to_string(),"cwd":dir.path(),"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"new window"}});
    let task = {
        let rt = rt.clone();
        let h = headers(&fresh.token, &input);
        tokio::spawn(async move {
            dispatch::receive(State(rt), Path("codex".into()), h, Json(input.take()))
                .await
                .unwrap()
                .0
        })
    };
    pending(&rt, 1).await;
    let request = rt.broker.list().remove(0);
    rt.broker
        .respond(
            &request.id,
            Reply {
                choice: Some("allow".into()),
                answers: Default::default(),
            },
        )
        .unwrap();
    assert_eq!(
        task.await.unwrap()["result"]["hookSpecificOutput"]["decision"]["behavior"],
        "allow"
    );
}

#[tokio::test]
async fn wrong_signature_origin_and_invalid_event_cannot_attach_a_window() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let setup = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let d: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&setup.descriptor).unwrap()).unwrap();
    let input = json!({"session_id":uuid::Uuid::new_v4().to_string(),"cwd":dir.path(),"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"no execution"}});
    assert!(dispatch::receive(
        State(rt.clone()),
        Path("codex".into()),
        headers("wrong", &input),
        Json(input.clone())
    )
    .await
    .is_err());
    let mut origin = headers(&d.token, &input);
    origin.insert(
        "origin",
        HeaderValue::from_static("https://example.invalid"),
    );
    assert!(dispatch::receive(
        State(rt.clone()),
        Path("codex".into()),
        origin,
        Json(input.clone())
    )
    .await
    .is_err());
    let mut bad = input.clone();
    bad["session_id"] = json!("not-a-thread");
    assert!(dispatch::receive(
        State(rt.clone()),
        Path("codex".into()),
        headers(&d.token, &bad),
        Json(bad)
    )
    .await
    .is_err());
    assert!(rt.sessions.list().is_empty());
    assert!(rt.broker.list().is_empty());
}

#[tokio::test]
async fn account_edit_revokes_inflight_approvals_and_does_not_auto_restore_access() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let setup = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let d: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&setup.descriptor).unwrap()).unwrap();
    let input = json!({"session_id":uuid::Uuid::new_v4().to_string(),"cwd":dir.path(),"hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"no execution"}});
    let task = {
        let rt = rt.clone();
        let h = headers(&d.token, &input);
        tokio::spawn(async move {
            dispatch::receive(State(rt), Path("codex".into()), h, Json(input))
                .await
                .unwrap()
                .0
        })
    };
    pending(&rt, 1).await;
    let (mut profile, _) = rt.profile("codex").unwrap();
    profile.label = "Edited account".into();
    rt.save_profile(profile).unwrap();
    assert!(rt.broker.list().is_empty());
    assert!(rt.store.dispatchers().unwrap().is_empty());
    assert_eq!(task.await.unwrap()["result"], json!({}));
}

#[test]
fn explicit_reconnect_replaces_a_stale_home_binding() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let first = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let original: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&first.descriptor).unwrap()).unwrap();
    let mut old = rt.store.dispatchers().unwrap().remove(0);
    old.home = dir.path().join("previous-home");
    rt.store.save_dispatcher(&old).unwrap();
    let second = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let fresh: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&second.descriptor).unwrap()).unwrap();
    assert_eq!(
        second.target,
        std::fs::canonicalize(dir.path().join("codex-home"))
            .unwrap()
            .join("hooks.json")
    );
    assert_ne!(original.token, fresh.token);
}

#[test]
fn changing_executable_replaces_the_old_user_dispatcher_instead_of_duplicating_it() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let first = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit-old")).unwrap();
    agent_orbit::codex_hook::install(
        &first.descriptor.with_extension("hooks.json"),
        &first.target,
    )
    .unwrap();
    // Two preparations before install must still migrate the actually installed command.
    dispatch::prepare(
        &rt,
        "codex",
        std::path::Path::new("/test/orbit-intermediate"),
    )
    .unwrap();
    let second = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit-new")).unwrap();
    agent_orbit::codex_hook::install(
        &second.descriptor.with_extension("hooks.json"),
        &second.target,
    )
    .unwrap();
    let installed: Value = serde_json::from_slice(&std::fs::read(&second.target).unwrap()).unwrap();
    assert_eq!(
        installed["hooks"]["PermissionRequest"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(installed.to_string().contains("orbit-new"));
    assert!(!installed.to_string().contains("orbit-old"));
}

#[test]
fn callbacks_from_a_previous_codex_home_cannot_be_rebound_to_the_new_account() {
    let dir = tempfile::tempdir().unwrap();
    let rt = runtime(dir.path());
    let setup = dispatch::prepare(&rt, "codex", std::path::Path::new("/test/orbit")).unwrap();
    let d: dispatch::Descriptor =
        serde_json::from_slice(&std::fs::read(&setup.descriptor).unwrap()).unwrap();
    let (profile, _) = rt.profile("codex").unwrap();
    let matching = std::collections::HashMap::from([("CODEX_HOME".into(), profile.config_dir)]);
    dispatch::validate_client_home(&d, &matching).unwrap();
    let old = dir.path().join("previous-home");
    std::fs::create_dir(&old).unwrap();
    let wrong = std::collections::HashMap::from([(
        "CODEX_HOME".into(),
        old.to_string_lossy().into_owned(),
    )]);
    assert!(dispatch::validate_client_home(&d, &wrong).is_err());
    assert!(dispatch::validate_client_home(&d, &Default::default()).is_err());
}
