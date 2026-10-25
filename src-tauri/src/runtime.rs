use crate::{broker::Broker, model::*, sessions::Sessions, store::Store};
use anyhow::{anyhow, ensure, Result};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex, RwLock},
};
use tokio::sync::broadcast;

pub struct Runtime {
    pub config: Config,
    pub store: Store,
    pub settings: RwLock<Settings>,
    pub sessions: Arc<Sessions>,
    pub broker: Arc<Broker>,
    pub events: broadcast::Sender<AppEvent>,
    pub bridge_url: RwLock<String>,
    refreshing: Mutex<HashSet<String>>,
    external_cache: Mutex<Option<(std::time::Instant, Vec<SessionInfo>)>>,
    pub usage_lock: Mutex<()>,
    pub data_dir: std::path::PathBuf,
}
impl Runtime {
    pub fn new() -> Result<Arc<Self>> {
        let config = crate::config::load()?;
        let data_dir = crate::config::data_dir()?;
        let store = Store::open(&data_dir.join("orbit.db"), &config)?;
        let settings = store.load_settings(&config.settings)?;
        crate::config::validate_settings(&settings)?;
        let (events, _) = broadcast::channel(1024);
        let broker = Arc::new(Broker::default());
        let sessions = Arc::new(Sessions::new(events.clone(), broker.clone()));
        Ok(Arc::new(Self {
            config,
            store,
            settings: RwLock::new(settings),
            sessions,
            broker,
            events,
            bridge_url: RwLock::new(String::new()),
            refreshing: Mutex::new(HashSet::new()),
            external_cache: Mutex::new(None),
            usage_lock: Mutex::new(()),
            data_dir,
        }))
    }
    pub fn emit(&self, kind: &str) {
        let _ = self.events.send(AppEvent {
            kind: kind.into(),
            session_id: None,
            data: serde_json::Value::Null,
        });
    }
    pub fn profile(&self, id: &str) -> Result<(Profile, Provider)> {
        let p = self
            .store
            .profiles()?
            .into_iter()
            .find(|p| p.id == id)
            .ok_or_else(|| anyhow!("Unknown account profile"))?;
        let provider = self
            .config
            .providers
            .iter()
            .find(|v| v.id == p.provider_id)
            .cloned()
            .ok_or_else(|| anyhow!("Unknown agent"))?;
        Ok((p, provider))
    }
    pub fn dashboard(&self) -> Result<Dashboard> {
        let settings = self.settings.read().unwrap().clone();
        let mut agents = vec![];
        for p in self.store.profiles()? {
            let Some(provider) = self.config.providers.iter().find(|v| v.id == p.provider_id)
            else {
                continue;
            };
            let discovery = crate::discovery::discover(&p, provider);
            let mut snapshot = self.store.snapshot(&p.id)?.unwrap_or_else(|| {
                QuotaSnapshot::state(
                    &p.id,
                    if discovery.installed {
                        "unavailable"
                    } else {
                        "notInstalled"
                    },
                    "Refresh to read account quota",
                )
            });
            if snapshot.status == "ready"
                && (chrono::Utc::now().timestamp() - snapshot.fetched_at
                    > settings.stale_seconds as i64
                    || snapshot.windows.iter().any(|w| {
                        w.resets_at
                            .is_some_and(|t| t <= chrono::Utc::now().timestamp())
                    }))
            {
                snapshot.status = "stale".into();
            }
            agents.push(AgentView {
                provider: provider.clone(),
                profile: p,
                discovery,
                snapshot,
            });
        }
        let mut sessions = self.sessions.list();
        let mut cache = self.external_cache.lock().unwrap();
        if cache.as_ref().is_none_or(|(at, _)| {
            at.elapsed().as_secs() >= settings.external_session_refresh_seconds
        }) {
            let rows = agents
                .iter()
                .flat_map(|agent| {
                    crate::external_sessions::scan(
                        &agent.profile,
                        &agent.provider,
                        settings.external_session_limit,
                    )
                })
                .collect();
            *cache = Some((std::time::Instant::now(), rows));
        }
        if let Some((_, rows)) = cache.as_ref() {
            sessions.extend(rows.clone());
        }
        drop(cache);
        sessions.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
        Ok(Dashboard {
            agents,
            settings,
            sessions,
            interactions: self.broker.list(),
            platform: std::env::consts::OS.into(),
        })
    }
    pub async fn refresh(&self, id: &str) -> Result<QuotaSnapshot> {
        {
            ensure!(
                self.refreshing.lock().unwrap().insert(id.to_owned()),
                "Refresh is already in progress"
            );
        }
        let result = async {
            let (p, provider) = self.profile(id)?;
            let s = self.settings.read().unwrap().clone();
            let snapshot = crate::providers::refresh(&p, &provider, &self.config, &s).await;
            let (current, _) = self.profile(id)?;
            ensure!(
                serde_json::to_value(&current)? == serde_json::to_value(&p)?,
                "Profile changed during refresh; retry"
            );
            let snapshot = crate::quota::retain_bridge_snapshot(self.store.snapshot(id)?, snapshot);
            self.store.save_bound_snapshot(&snapshot, &p)?;
            self.emit("dashboard");
            Ok(snapshot)
        }
        .await;
        self.refreshing.lock().unwrap().remove(id);
        result
    }
    pub fn save_profile(&self, mut p: Profile) -> Result<()> {
        ensure!(
            self.config.providers.iter().any(|v| v.id == p.provider_id),
            "Unknown agent"
        );
        if p.id.is_empty() {
            p.id = uuid::Uuid::new_v4().to_string();
        } else if let Ok((old, _)) = self.profile(&p.id) {
            ensure!(
                old.provider_id == p.provider_id,
                "Cannot change a profile's agent"
            );
        }
        ensure!(
            !p.label.trim().is_empty() && p.label.len() < 128,
            "Enter an account label"
        );
        self.store.save_profile(&p)?;
        *self.external_cache.lock().unwrap() = None;
        self.emit("dashboard");
        Ok(())
    }
    pub fn save_settings(&self, s: Settings) -> Result<()> {
        self.store.save_settings(&s)?;
        *self.external_cache.lock().unwrap() = None;
        *self.settings.write().unwrap() = s;
        self.emit("dashboard");
        Ok(())
    }
    pub async fn poll(self: Arc<Self>) {
        loop {
            let s = self.settings.read().unwrap().clone();
            if let Ok(profiles) = self.store.profiles() {
                for p in profiles {
                    if p.enabled && s.visible_providers.contains(&p.provider_id) {
                        let _ = self.refresh(&p.id).await;
                    }
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(s.refresh_seconds)).await;
        }
    }
}
