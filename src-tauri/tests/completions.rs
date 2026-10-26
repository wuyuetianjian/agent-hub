use agent_orbit::{completions::CompletionQueue, model::SessionInfo};
use serde_json::json;

fn session(id: &str) -> SessionInfo {
    SessionInfo {
        id: id.into(),
        profile_id: "profile".into(),
        provider_id: "codex".into(),
        interaction_channel: "codex-rpc".into(),
        read_only: false,
        title: "My task".into(),
        source: "managed".into(),
        updated_at: 1,
        cwd: "/project".into(),
        mode: "structured".into(),
        status: "running".into(),
        started_at: 1,
    }
}

#[test]
fn successful_turns_are_queued_once_and_dismissal_does_not_reset_dedup() {
    let mut queue = CompletionQueue::default();
    let info = session("session-a");
    let params = json!({"turn":{"id":"turn-1","status":"completed","error":null}});
    assert!(queue.record(&info, "Account", &params));
    assert!(!queue.record(&info, "Account", &params));
    let list = queue.list();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].session_id, "session-a");
    assert_eq!(list[0].turn_id, "turn-1");
    assert_eq!(list[0].profile_label, "Account");
    assert!(queue.dismiss(&list[0].id));
    assert!(!queue.dismiss(&list[0].id));
    assert!(!queue.record(&info, "Account", &params));
    assert!(queue.list().is_empty());
    assert!(queue.record(&session("session-b"), "Account", &params));
}

#[test]
fn only_explicit_success_of_managed_running_codex_turns_is_eligible() {
    let mut queue = CompletionQueue::default();
    for params in [
        json!({}),
        json!({"turn":{"id":"t"}}),
        json!({"turn":{"id":"","status":"completed"}}),
        json!({"turn":{"id":"t","status":"failed"}}),
        json!({"turn":{"id":"t","status":"interrupted"}}),
        json!({"turn":{"id":"t","status":"completed","error":{"message":"failed"}}}),
    ] {
        assert!(!queue.record(&session("s"), "Account", &params));
    }
    let params = json!({"turn":{"id":"t","status":"completed"}});
    for field in ["provider", "source", "mode", "status", "readonly"] {
        let mut info = session("s");
        match field {
            "provider" => info.provider_id = "claude".into(),
            "source" => info.source = "external".into(),
            "mode" => info.mode = "terminal".into(),
            "status" => info.status = "exited".into(),
            _ => info.read_only = true,
        }
        assert!(!queue.record(&info, "Account", &params));
    }
    assert!(queue.list().is_empty());
}

#[test]
fn queue_is_bounded_and_keeps_newest_completions_in_arrival_order() {
    let mut queue = CompletionQueue::default();
    for index in 0..130 {
        assert!(queue.record(
            &session("s"),
            "Account",
            &json!({"turn":{"id":format!("turn-{index}"),"status":"completed"}})
        ));
    }
    let list = queue.list();
    assert_eq!(list.len(), 100);
    assert_eq!(list.first().unwrap().turn_id, "turn-30");
    assert_eq!(list.last().unwrap().turn_id, "turn-129");
    assert!(!queue.record(
        &session("s"),
        "Account",
        &json!({"turn":{"id":"turn-0","status":"completed"}})
    ));
}
