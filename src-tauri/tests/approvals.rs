use agent_orbit::{
    broker::Broker,
    config,
    model::{Interaction, Reply},
    sessions::Sessions,
    store::Store,
};
use serde_json::{json, Value};
use std::{path::Path, sync::Arc};
use tokio::time::{timeout, Duration};
struct Harness {
    dir: tempfile::TempDir,
    sessions: Arc<Sessions>,
    broker: Arc<Broker>,
}
impl Harness {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let (events, _) = tokio::sync::broadcast::channel(128);
        let broker = Arc::new(Broker::default());
        Self {
            dir,
            sessions: Arc::new(Sessions::new(events, broker.clone())),
            broker,
        }
    }
    async fn start(&self, id: &str, scenario: &str, seconds: u64) -> (String, std::path::PathBuf) {
        let cfg = config::load().unwrap();
        let store = Store::open(&self.dir.path().join(format!("{id}.db")), &cfg).unwrap();
        let mut profile = store
            .profiles()
            .unwrap()
            .into_iter()
            .find(|p| p.provider_id == "codex")
            .unwrap();
        profile.id = id.into();
        profile.label = format!("TEST ONLY {id}");
        profile.executable = which::which("node").unwrap().to_string_lossy().into_owned();
        let receipt = self.dir.path().join(format!("{id}.jsonl"));
        profile.args = vec![
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/approval-agent.mjs")
                .to_string_lossy()
                .into_owned(),
            scenario.into(),
            receipt.to_string_lossy().into_owned(),
        ];
        let mut settings = cfg.settings.clone();
        settings.interaction_timeout_seconds = seconds;
        let info = self
            .sessions
            .start(
                &profile,
                cfg.providers.iter().find(|p| p.id == "codex").unwrap(),
                self.dir.path().to_str().unwrap(),
                "structured",
                &settings,
                "",
            )
            .await
            .unwrap();
        self.sessions.prompt(&info.id, "TEST ONLY").await.unwrap();
        (info.id, receipt)
    }
    async fn pending(&self, id: &str) -> Interaction {
        timeout(Duration::from_secs(5), async {
            loop {
                if let Some(request) = self.broker.list().into_iter().find(|r| r.session_id == id) {
                    return request;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("approval must arrive through the real session broker")
    }
    async fn result(&self, path: &Path) -> Value {
        timeout(Duration::from_secs(5), async {
            loop {
                if let Ok(text) = std::fs::read_to_string(path) {
                    if let Some(result) = text
                        .lines()
                        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
                        .find(|v| v.get("response").is_some())
                    {
                        return result["response"].clone();
                    }
                }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        })
        .await
        .expect("reply must reach the originating app-server process")
    }
}
impl Drop for Harness {
    fn drop(&mut self) {
        self.sessions.stop_all();
    }
}
#[tokio::test]
async fn allow_and_deny_round_trip_to_the_correct_subprocess() {
    let h = Harness::new();
    let (a, ap) = h.start("account-a", "permission", 10).await;
    let (b, bp) = h.start("account-b", "permission", 10).await;
    let ar = h.pending(&a).await;
    let br = h.pending(&b).await;
    assert_eq!(ar.profile_label, "TEST ONLY account-a");
    assert_eq!(ar.choices[2].label, "Allow command rule");
    assert_eq!(ar.choices[3].label, "Cancel");
    assert_eq!(br.profile_label, "TEST ONLY account-b");
    h.broker
        .respond(
            &ar.id,
            Reply {
                choice: Some("0".into()),
                answers: Default::default(),
            },
        )
        .unwrap();
    h.broker
        .respond(
            &br.id,
            Reply {
                choice: Some("1".into()),
                answers: Default::default(),
            },
        )
        .unwrap();
    assert_eq!(h.result(&ap).await, json!({"decision":"accept"}));
    assert_eq!(h.result(&bp).await, json!({"decision":"decline"}));
    assert!(h
        .broker
        .respond(
            &ar.id,
            Reply {
                choice: Some("1".into()),
                answers: Default::default()
            }
        )
        .is_err());
}
#[tokio::test]
async fn free_text_question_reaches_its_original_question_id() {
    let h = Harness::new();
    let (id, path) = h.start("questions", "question", 10).await;
    let request = h.pending(&id).await;
    assert_eq!(request.questions[0].id, "answer");
    let answers = std::collections::HashMap::from([("answer".into(), vec!["中文测试回答".into()])]);
    h.broker
        .respond(
            &request.id,
            Reply {
                choice: None,
                answers,
            },
        )
        .unwrap();
    assert_eq!(
        h.result(&path).await,
        json!({"answers":{"answer":{"answers":["中文测试回答"]}}})
    );
}
#[tokio::test]
async fn configured_policy_is_explicitly_sent_when_a_managed_thread_starts() {
    let h = Harness::new();
    let (_, path) = h.start("policy", "permission", 10).await;
    let text = std::fs::read_to_string(path).unwrap();
    let record: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
    assert_eq!(record["startup"]["approvalPolicy"], "untrusted");
}
#[tokio::test]
async fn unanswered_approval_times_out_to_decline() {
    let h = Harness::new();
    let (_, path) = h.start("timeout", "permission", 1).await;
    assert_eq!(h.result(&path).await, json!({"decision":"decline"}));
    assert!(h.broker.list().is_empty());
}
#[tokio::test]
async fn upstream_resolution_removes_a_pending_card_without_a_stale_reply() {
    let h = Harness::new();
    let (id, path) = h.start("cancelled", "cancel", 10).await;
    let request = h.pending(&id).await;
    timeout(Duration::from_secs(3), async {
        loop {
            if h.broker.list().is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    assert!(h
        .broker
        .respond(
            &request.id,
            Reply {
                choice: Some("0".into()),
                answers: Default::default()
            }
        )
        .is_err());
    let text = std::fs::read_to_string(path).unwrap();
    assert!(!text.contains("response"));
}
