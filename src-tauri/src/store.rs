use crate::model::{Config, Profile, QuotaSnapshot, Settings};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

pub struct Store {
    connection: Mutex<Connection>,
}
impl Store {
    pub fn open(path: &Path, config: &Config) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)?;
        connection.execute_batch("CREATE TABLE IF NOT EXISTS window_state (id TEXT PRIMARY KEY, value TEXT NOT NULL); PRAGMA journal_mode=WAL; PRAGMA user_version=1; CREATE TABLE IF NOT EXISTS settings (id INTEGER PRIMARY KEY CHECK (id=1), value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS profiles (id TEXT PRIMARY KEY, value TEXT NOT NULL); CREATE TABLE IF NOT EXISTS snapshots (id TEXT PRIMARY KEY, value TEXT NOT NULL);")?;
        for p in &config.providers {
            let profile = Profile {
                id: p.id.clone(),
                provider_id: p.id.clone(),
                label: p.name.clone(),
                executable: String::new(),
                args: vec![],
                config_dir: String::new(),
                quota_command: String::new(),
                quota_args: vec![],
                quota_file: String::new(),
                enabled: true,
            };
            connection.execute(
                "INSERT OR IGNORE INTO profiles(id,value) VALUES(?1,?2)",
                params![profile.id, serde_json::to_string(&profile)?],
            )?;
        }
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }
    pub fn load_settings(&self, defaults: &Settings) -> Result<Settings> {
        let saved: Option<String> = self
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT value FROM settings WHERE id=1", [], |r| r.get(0))
            .optional()?;
        let mut value = serde_json::to_value(defaults)?;
        if let Some(s) = saved {
            crate::config::merge(&mut value, serde_json::from_str(&s)?);
        }
        Ok(serde_json::from_value(value)?)
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<()> {
        crate::config::validate_settings(settings)?;
        self.connection.lock().unwrap().execute("INSERT INTO settings(id,value) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value",[serde_json::to_string(settings)?])?;
        Ok(())
    }
    pub fn profiles(&self) -> Result<Vec<Profile>> {
        let c = self.connection.lock().unwrap();
        let mut stmt = c.prepare("SELECT value FROM profiles ORDER BY rowid")?;
        let rows = stmt.query_map([], |r| r.get::<_, String>(0))?;
        rows.map(|r| Ok(serde_json::from_str(&r?)?)).collect()
    }
    pub fn save_profile(&self, p: &Profile) -> Result<()> {
        let c = self.connection.lock().unwrap();
        c.execute("INSERT INTO profiles(id,value) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET value=excluded.value",params![p.id,serde_json::to_string(p)?])?;
        c.execute("DELETE FROM snapshots WHERE id=?1", [&p.id])?;
        Ok(())
    }
    pub fn snapshot(&self, id: &str) -> Result<Option<QuotaSnapshot>> {
        let value: Option<String> = self
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT value FROM snapshots WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        value.map(|v| Ok(serde_json::from_str(&v)?)).transpose()
    }
    pub fn save_snapshot(&self, snapshot: &QuotaSnapshot) -> Result<()> {
        self.connection.lock().unwrap().execute("INSERT INTO snapshots(id,value) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET value=excluded.value",params![snapshot.profile_id,serde_json::to_string(snapshot)?])?;
        Ok(())
    }
    pub fn window_bounds(&self, id: &str) -> Result<Option<crate::window_state::Bounds>> {
        let value: Option<String> = self
            .connection
            .lock()
            .unwrap()
            .query_row("SELECT value FROM window_state WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .optional()?;
        value.map(|v| Ok(serde_json::from_str(&v)?)).transpose()
    }
    pub fn save_window_bounds(&self, id: &str, bounds: crate::window_state::Bounds) -> Result<()> {
        self.connection.lock().unwrap().execute("INSERT INTO window_state(id,value) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET value=excluded.value",params![id,serde_json::to_string(&bounds)?])?;
        Ok(())
    }
    pub fn save_bound_snapshot(&self, snapshot: &QuotaSnapshot, profile: &Profile) -> Result<()> {
        let connection = self.connection.lock().unwrap();
        let saved: Option<String> = connection
            .query_row(
                "SELECT value FROM profiles WHERE id=?1",
                [&profile.id],
                |r| r.get(0),
            )
            .optional()?;
        anyhow::ensure!(
            saved.as_deref() == Some(serde_json::to_string(profile)?.as_str()),
            "Account profile changed during quota collection; restart the session or refresh again"
        );
        connection.execute("INSERT INTO snapshots(id,value) VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET value=excluded.value",params![snapshot.profile_id,serde_json::to_string(snapshot)?])?;
        Ok(())
    }
}
