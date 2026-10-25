use agent_orbit::{
    broker::Broker,
    model::{Choice, Interaction, Reply},
};
fn request(id: &str, session: &str) -> Interaction {
    Interaction {
        profile_label: "Test account".into(),
        cwd: "/test/project".into(),
        id: id.into(),
        session_id: session.into(),
        provider_id: "codex".into(),
        title: "Command approval".into(),
        detail: "touch example".into(),
        kind: "permission".into(),
        choices: vec![
            Choice {
                id: "allow".into(),
                label: "Allow".into(),
            },
            Choice {
                id: "deny".into(),
                label: "Deny".into(),
            },
        ],
        questions: vec![],
        created_at: chrono::Utc::now().timestamp(),
        expires_at: chrono::Utc::now().timestamp() + 30,
    }
}
fn reply(choice: &str) -> Reply {
    Reply {
        choice: Some(choice.into()),
        answers: Default::default(),
    }
}
#[tokio::test]
async fn reply_is_delivered_only_once() {
    let b = Broker::default();
    let rx = b.insert(request("a", "s1"));
    b.respond("a", reply("deny")).unwrap();
    assert_eq!(rx.await.unwrap().choice.as_deref(), Some("deny"));
    assert!(b.respond("a", reply("allow")).is_err());
}
#[test]
fn unknown_choice_does_not_consume_request() {
    let b = Broker::default();
    let _rx = b.insert(request("a", "s1"));
    assert!(b.respond("a", reply("always")).is_err());
    assert_eq!(b.list().len(), 1);
}
#[test]
fn cancelling_one_session_preserves_other_requests() {
    let b = Broker::default();
    let _a = b.insert(request("a", "s1"));
    let _b = b.insert(request("b", "s2"));
    b.cancel_session("s1");
    assert!(b.respond("a", reply("allow")).is_err());
    assert!(b.respond("b", reply("deny")).is_ok());
}
#[test]
fn expired_request_cannot_be_approved() {
    let b = Broker::default();
    let mut r = request("a", "s1");
    r.expires_at = 1;
    let _rx = b.insert(r);
    assert!(b.respond("a", reply("allow")).is_err());
}

#[test]
fn server_resolution_before_prompt_registration_cannot_leave_a_live_card() {
    let b = Broker::default();
    b.register_wire("s1", "42");
    b.resolve_wire("s1", "42");
    assert!(b.insert_wire(request("late", "s1"), "42").is_none());
    assert!(b.list().is_empty());
}
#[test]
fn resolved_wire_cancels_exact_request_without_cross_session_collision() {
    let b = Broker::default();
    b.register_wire("s1", "42");
    b.register_wire("s2", "42");
    let _a = b.insert_wire(request("a", "s1"), "42");
    let _other = b.insert_wire(request("b", "s2"), "42");
    b.resolve_wire("s1", "42");
    assert!(b.respond("a", reply("allow")).is_err());
    assert!(b.respond("b", reply("deny")).is_ok());
}
