use crate::model::{Interaction, Reply};
use anyhow::{anyhow, ensure, Result};
use std::{collections::HashMap, sync::Mutex};
use tokio::sync::oneshot;
#[derive(Default)]
pub struct Broker {
    pending: Mutex<HashMap<String, (Interaction, oneshot::Sender<Reply>)>>,
    wire: Mutex<HashMap<(String, String), Option<String>>>,
}
impl Broker {
    pub fn register_wire(&self, session: &str, wire: &str) {
        self.wire
            .lock()
            .unwrap()
            .entry((session.into(), wire.into()))
            .or_insert(Some(String::new()));
    }
    pub fn resolve_wire(&self, session: &str, wire: &str) {
        let mut aliases = self.wire.lock().unwrap();
        if let Some(Some(id)) = aliases.insert((session.into(), wire.into()), None) {
            self.remove(&id);
        }
    }
    pub fn insert_wire(
        &self,
        request: Interaction,
        wire: &str,
    ) -> Option<oneshot::Receiver<Reply>> {
        let mut aliases = self.wire.lock().unwrap();
        let current = aliases.get_mut(&(request.session_id.clone(), wire.into()))?;
        if current.as_deref() != Some("") {
            return None;
        }
        *current = Some(request.id.clone());
        Some(self.insert(request))
    }
    pub fn wire_active(&self, session: &str, wire: &str) -> bool {
        self.wire
            .lock()
            .unwrap()
            .get(&(session.into(), wire.into()))
            .is_some_and(Option::is_some)
    }
    pub fn insert(&self, request: Interaction) -> oneshot::Receiver<Reply> {
        let (tx, rx) = oneshot::channel();
        self.pending
            .lock()
            .unwrap()
            .insert(request.id.clone(), (request, tx));
        rx
    }
    pub fn respond(&self, id: &str, reply: Reply) -> Result<()> {
        let mut pending = self.pending.lock().unwrap();
        let (request, sender) = pending
            .get(id)
            .ok_or_else(|| anyhow!("This request has already been resolved or cancelled"))?;
        ensure!(
            !sender.is_closed() && request.expires_at > chrono::Utc::now().timestamp(),
            "This request has expired or disconnected"
        );
        if request.questions.is_empty() {
            ensure!(
                reply
                    .choice
                    .as_ref()
                    .is_some_and(|c| request.choices.iter().any(|o| &o.id == c)),
                "Unsupported approval decision"
            );
        } else {
            ensure!(
                reply.answers.len() == request.questions.len(),
                "Answer every question"
            );
            for q in &request.questions {
                let answers = reply
                    .answers
                    .get(&q.id)
                    .ok_or_else(|| anyhow!("Missing answer"))?;
                ensure!(
                    !answers.is_empty() && (q.multiple || answers.len() == 1),
                    "Invalid number of answers"
                );
                ensure!(
                    answers
                        .iter()
                        .all(|a| !a.trim().is_empty() && a.len() <= 8192),
                    "Empty or excessively long answer"
                );
            }
        }
        let (_, sender) = pending.remove(id).unwrap();
        sender
            .send(reply)
            .map_err(|_| anyhow!("Originating request disconnected"))?;
        Ok(())
    }
    pub fn cancel_session(&self, session: &str) {
        for ((owner, _), id) in self.wire.lock().unwrap().iter_mut() {
            if owner == session {
                *id = None;
            }
        }
        self.pending
            .lock()
            .unwrap()
            .retain(|_, (r, _)| r.session_id != session);
    }
    pub fn list(&self) -> Vec<Interaction> {
        let mut p = self.pending.lock().unwrap();
        p.retain(|_, (r, tx)| r.expires_at > chrono::Utc::now().timestamp() && !tx.is_closed());
        let mut rows: Vec<_> = p.values().map(|(r, _)| r.clone()).collect();
        rows.sort_by_key(|r| r.created_at);
        rows
    }
    pub fn remove(&self, id: &str) {
        self.pending.lock().unwrap().remove(id);
    }
}
