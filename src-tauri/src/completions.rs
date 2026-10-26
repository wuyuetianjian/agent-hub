//! In-memory notifications survive webview reloads, but not application restarts.
use crate::model::{SessionInfo, TaskCompletion};
use serde_json::Value;
use std::collections::{HashSet, VecDeque};

const MAX_PENDING: usize = 100;

#[derive(Default)]
pub struct CompletionQueue {
    pending: VecDeque<TaskCompletion>,
    // Keep IDs after dismissal/eviction so delayed duplicate protocol events cannot
    // resurrect notifications. Metadata grows by one pair per successful turn for
    // this process lifetime; no transcript or protocol payload is retained.
    seen: HashSet<(String, String)>,
}
impl CompletionQueue {
    pub fn record(&mut self, info: &SessionInfo, label: &str, params: &Value) -> bool {
        if info.source != "managed"
            || info.provider_id != "codex"
            || info.mode != "structured"
            || info.status != "running"
            || info.read_only
            || params["turn"]["status"] != "completed"
            || !params["turn"]["error"].is_null()
        {
            return false;
        }
        let Some(turn_id) = params["turn"]["id"]
            .as_str()
            .filter(|id| !id.trim().is_empty())
        else {
            return false;
        };
        if !self.seen.insert((info.id.clone(), turn_id.to_owned())) {
            return false;
        }
        if self.pending.len() == MAX_PENDING {
            self.pending.pop_front();
        }
        self.pending.push_back(TaskCompletion {
            id: uuid::Uuid::new_v4().to_string(),
            session_id: info.id.clone(),
            turn_id: turn_id.into(),
            profile_id: info.profile_id.clone(),
            profile_label: label.into(),
            cwd: info.cwd.clone(),
            completed_at: chrono::Utc::now().timestamp(),
        });
        true
    }
    pub fn list(&self) -> Vec<TaskCompletion> {
        self.pending.iter().cloned().collect()
    }
    pub fn dismiss(&mut self, id: &str) -> bool {
        let before = self.pending.len();
        self.pending.retain(|item| item.id != id);
        before != self.pending.len()
    }
}
