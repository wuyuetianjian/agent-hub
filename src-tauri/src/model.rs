use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub publisher: String,
    pub icon: String,
    pub color: String,
    pub executables: Vec<String>,
    pub adapter: String,
    pub usage_url: String,
    pub credential_paths: Vec<String>,
    pub detection_paths: Vec<String>,
    #[serde(default)]
    pub executable_paths: Vec<String>,
    #[serde(default)]
    pub desktop_paths: Vec<String>,
    #[serde(default)]
    pub session_index_paths: Vec<String>,
    #[serde(default)]
    pub session_log_paths: Vec<String>,
    #[serde(default)]
    pub config_env: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DockPreset {
    pub id: String,
    pub size: f64,
    pub hover_size: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub locale: String,
    pub theme: String,
    pub refresh_seconds: u64,
    pub stale_seconds: u64,
    pub request_timeout_seconds: u64,
    pub interaction_timeout_seconds: u64,
    pub warning_percent: u8,
    pub window_mode: String,
    pub summary_policy: String,
    pub island_width: f64,
    pub island_height: f64,
    pub always_on_top: bool,
    pub compact: bool,
    pub dock_presets: Vec<DockPreset>,
    pub dock_locked: bool,
    pub dock_side: String,
    pub dock_size: f64,
    pub dock_hover_size: f64,
    pub dock_gap: f64,
    pub dock_padding: f64,
    pub dock_panel_width: f64,
    pub dock_edge_inset: f64,
    pub dock_hover_delay_ms: u64,
    pub dock_chrome_height: f64,
    pub dock_popup_height: f64,
    pub start_at_login: bool,
    pub visible_providers: Vec<String>,
    pub window_width: f64,
    pub window_height: f64,
    pub expanded_width: f64,
    pub expanded_height: f64,
    pub max_output_bytes: usize,
    pub external_session_limit: usize,
    pub external_session_refresh_seconds: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub app_name: String,
    pub version: String,
    pub settings: Settings,
    pub providers: Vec<Provider>,
    pub endpoints: std::collections::HashMap<String, String>,
    pub headers: std::collections::HashMap<String, String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub id: String,
    pub provider_id: String,
    pub label: String,
    pub executable: String,
    pub args: Vec<String>,
    pub config_dir: String,
    pub quota_command: String,
    pub quota_args: Vec<String>,
    pub quota_file: String,
    pub enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuotaWindow {
    pub label: String,
    pub remaining_percent: Option<f64>,
    pub used: Option<f64>,
    pub limit: Option<f64>,
    pub unit: String,
    pub resets_at: Option<i64>,
    #[serde(default)]
    pub window_minutes: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuotaSnapshot {
    pub profile_id: String,
    pub account: Option<String>,
    pub plan: Option<String>,
    pub status: String,
    pub source: String,
    pub message: String,
    pub windows: Vec<QuotaWindow>,
    pub fetched_at: i64,
}
impl QuotaSnapshot {
    pub fn state(profile: &str, status: &str, message: &str) -> Self {
        Self {
            profile_id: profile.into(),
            account: None,
            plan: None,
            status: status.into(),
            source: String::new(),
            message: message.into(),
            windows: vec![],
            fetched_at: chrono::Utc::now().timestamp(),
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub installed: bool,
    pub desktop_found: bool,
    pub executable: Option<String>,
    pub config_found: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentView {
    pub provider: Provider,
    pub profile: Profile,
    pub discovery: Discovery,
    pub snapshot: QuotaSnapshot,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionInfo {
    pub read_only: bool,
    pub title: String,
    pub source: String,
    pub updated_at: i64,
    pub id: String,
    pub profile_id: String,
    pub provider_id: String,
    pub cwd: String,
    pub mode: String,
    pub status: String,
    pub started_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choice {
    pub id: String,
    pub label: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Question {
    pub id: String,
    pub title: String,
    pub options: Vec<Choice>,
    pub multiple: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Interaction {
    pub profile_label: String,
    pub cwd: String,
    pub id: String,
    pub session_id: String,
    pub provider_id: String,
    pub title: String,
    pub detail: String,
    pub kind: String,
    pub choices: Vec<Choice>,
    pub questions: Vec<Question>,
    pub created_at: i64,
    pub expires_at: i64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reply {
    pub choice: Option<String>,
    #[serde(default)]
    pub answers: std::collections::HashMap<String, Vec<String>>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Dashboard {
    pub agents: Vec<AgentView>,
    pub settings: Settings,
    pub sessions: Vec<SessionInfo>,
    pub interactions: Vec<Interaction>,
    pub platform: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppEvent {
    pub kind: String,
    pub session_id: Option<String>,
    pub data: Value,
}
