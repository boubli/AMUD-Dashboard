use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

#[derive(Clone)]
pub struct AgentCommandHandle {
    pub id: u64,
    pub tx: tokio::sync::mpsc::UnboundedSender<String>,
}

/// Per-node connection metadata for the multi-node dashboard.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct NodeMeta {
    pub connected: bool,
    pub last_seen: u64,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(default)]
    pub platform: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct App {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) icon: String,
    pub(crate) description: String,
    pub(crate) category: String,
    pub(crate) node_tag: String,
    #[serde(default)]
    pub(crate) mac_address: String,
    #[serde(default)]
    pub(crate) integration_type: String,
    #[serde(default)]
    pub(crate) api_key: String,
    #[serde(default)]
    pub(crate) sort_order: i64,
    #[serde(default)]
    pub(crate) card_span: String,
    #[serde(default = "default_show_container_metrics")]
    pub(crate) show_container_metrics: bool,
    #[serde(default = "default_guest_visible")]
    pub(crate) guest_visible: bool,
    #[serde(default = "default_embed_mode")]
    pub(crate) embed_mode: String,
    /// JSON array of integration metric keys to show; empty string = all; "[]" = hide API metrics.
    #[serde(default)]
    pub(crate) integration_visible_metrics: String,
}

fn default_guest_visible() -> bool {
    true
}

fn default_embed_mode() -> String {
    "link".to_string()
}

fn default_show_container_metrics() -> bool {
    true
}

#[derive(Clone, Serialize, Deserialize)]
pub struct WolDevice {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) mac_address: String,
    pub(crate) ip_address: String,
    pub(crate) icon: String,
}

#[derive(Clone, Serialize)]
pub struct Session {
    pub username: String,
    pub role: String,
    pub expires_at_epoch: u64,
    pub csrf_token: String,
}

pub(crate) use amud_protocol::{AgentTelemetry, LxcContainer, NetworkTelemetry};

#[derive(Serialize, Clone, Default)]
pub struct MediaStream {
    pub(crate) status: String,
    pub(crate) active: bool,
    pub(crate) title: String,
    pub(crate) current_time: String,
    pub(crate) total_time: String,
    pub(crate) progress_percent: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) poster: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) session_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) is_paused: Option<bool>,
}

#[derive(Serialize, Clone, Default)]
pub struct AppStatus {
    pub(crate) status: String,
    pub(crate) latency_ms: Option<u128>,
}

#[derive(Serialize, Clone)]
pub struct FullTelemetry {
    pub(crate) system: AgentTelemetry,
    pub(crate) network: NetworkTelemetry,
    pub(crate) streams: HashMap<String, MediaStream>,
    pub(crate) app_statuses: HashMap<String, AppStatus>,
    pub(crate) agent_connected: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) smart_home: Option<crate::smart_home::SmartHomeTelemetry>,
    #[serde(skip_serializing_if = "HashMap::is_empty")]
    pub(crate) nodes: HashMap<String, AgentTelemetry>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub(crate) nodes_meta: HashMap<String, NodeMeta>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PveTestResult {
    pub(crate) success: bool,
    pub(crate) error: Option<String>,
}

#[derive(Clone)]
pub struct ActionResult {
    pub(crate) success: bool,
    pub(crate) error: Option<String>,
    pub(crate) at: Instant,
}

#[derive(Deserialize)]
pub struct ActionResultMsg {
    pub(crate) action_result: ActionResultPayload,
}

#[derive(Deserialize)]
pub struct ActionResultPayload {
    pub(crate) request_id: String,
    pub(crate) success: bool,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DashboardWidget {
    pub(crate) id: i64,
    pub(crate) widget_type: String,
    pub(crate) title: String,
    pub(crate) content: String,
    pub(crate) sort_order: i64,
    pub(crate) guest_visible: bool,
    pub(crate) grid_span: String,
}

#[derive(Clone)]
pub struct ShareSession {
    pub allowed_paths: String,
    pub expires_at_epoch: u64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Webhook {
    pub(crate) id: i64,
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) event_types: String,
    pub(crate) is_active: i32,
}

#[allow(dead_code)]
pub struct AppState {
    pub db: Arc<Mutex<Connection>>,
    pub sessions: Arc<RwLock<HashMap<String, Session>>>,
    pub latest_telemetry: Arc<RwLock<AgentTelemetry>>,
    pub telemetry_by_node: Arc<RwLock<HashMap<String, AgentTelemetry>>>,
    pub agent_connected: Arc<RwLock<bool>>,
    pub media_streams: Arc<RwLock<HashMap<String, MediaStream>>>,
    pub app_statuses: Arc<RwLock<HashMap<String, AppStatus>>>,
    /// Active agent command channels keyed by `node_tag`.
    pub agent_sessions: Arc<Mutex<HashMap<String, AgentCommandHandle>>>,
    /// Live connection metadata per node (capabilities, platform, last_seen).
    pub nodes_meta: Arc<RwLock<HashMap<String, NodeMeta>>>,
    pub next_agent_conn_id: Arc<AtomicU64>,
    pub pve_test_response: Arc<RwLock<Option<PveTestResult>>>,
    pub docker_discover_response: Arc<RwLock<Option<serde_json::Value>>>,
    pub telemetry_discover_response: Arc<RwLock<Option<serde_json::Value>>>,
    pub share_sessions: Arc<RwLock<HashMap<String, ShareSession>>>,
    pub action_results: Arc<RwLock<HashMap<String, ActionResult>>>,
    pub settings_cache: Arc<RwLock<HashMap<String, String>>>,
    pub alert_cooldowns: Arc<Mutex<HashMap<String, std::time::Instant>>>,
    pub login_attempts: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    pub api_rate_limits: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
    pub agent_secret: Arc<String>,
    pub smart_home_telemetry: Arc<RwLock<crate::smart_home::SmartHomeTelemetry>>,
    pub logo_manifest: Arc<HashMap<String, String>>,
    pub telemetry_broadcast:
        tokio::sync::watch::Sender<Arc<crate::telemetry_broadcast::WsTelemetryBundle>>,
    pub integration_cache: Arc<crate::integration_cache::IntegrationCache>,
    pub http_clients: Arc<crate::http_client::SharedHttpClients>,
    /// Active WebSocket sessions with guest/limited telemetry (for broadcast slimming).
    pub ws_limited_clients: Arc<AtomicUsize>,
    /// GUI activity mode: 0=deep idle, 1=grace, 2=active.
    pub activity_mode: Arc<AtomicU8>,
    pub active_ws_count: Arc<AtomicUsize>,
    pub active_gui_sessions: Arc<AtomicUsize>,
    pub visible_app_ids: Arc<RwLock<Vec<i64>>>,
    pub last_activity_at: Arc<Mutex<Instant>>,
    /// Last telemetry epoch per node tag (multi-node stale eviction).
    pub node_last_seen: Arc<RwLock<HashMap<String, u64>>>,
}
