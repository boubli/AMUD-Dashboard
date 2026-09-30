use crate::models::{AppState, AppStatus, FullTelemetry, LxcContainer};
use amud_protocol::{AgentTelemetry, NetworkTelemetry};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tokio::sync::watch;

#[derive(Clone)]
pub struct WsTelemetryBundle {
    pub full: Arc<str>,
    pub guest_public: Arc<str>,
    pub guest_redacted: Arc<str>,
}

fn empty_full_telemetry() -> FullTelemetry {
    FullTelemetry {
        system: AgentTelemetry::default(),
        network: NetworkTelemetry::default(),
        streams: HashMap::new(),
        app_statuses: HashMap::new(),
        agent_connected: false,
        smart_home: None,
        nodes: HashMap::new(),
        nodes_meta: HashMap::new(),
    }
}

fn read_rwlock<T: Clone>(lock: &RwLock<T>) -> T {
    lock.read()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone()
}

fn read_agent_connected(lock: &RwLock<bool>) -> bool {
    *lock.read().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Guest WebSocket payloads include container name + runtime status only (no vmid, CPU, RAM).
fn sanitize_containers_for_guest(containers: &[LxcContainer]) -> Vec<LxcContainer> {
    containers
        .iter()
        .map(|c| LxcContainer {
            vmid: 0,
            status: c.status.clone(),
            name: c.name.clone(),
            ..Default::default()
        })
        .collect()
}

impl Default for WsTelemetryBundle {
    fn default() -> Self {
        let empty = empty_full_telemetry();
        let mut buf = String::new();
        Self::from_payloads(&empty, &empty, &empty, &mut buf)
    }
}

impl WsTelemetryBundle {
    fn encode_payload(payload: &FullTelemetry, buf: &mut String) -> Arc<str> {
        buf.clear();
        // SAFETY: serde_json writes valid UTF-8 JSON into the string buffer.
        let bytes = unsafe { buf.as_mut_vec() };
        bytes.clear();
        if serde_json::to_writer(bytes, payload).is_err() {
            buf.clear();
            buf.push_str("{}");
        }
        Arc::from(buf.as_str())
    }

    fn from_payloads(
        full: &FullTelemetry,
        guest_public: &FullTelemetry,
        guest_redacted: &FullTelemetry,
        buf: &mut String,
    ) -> Self {
        Self {
            full: Self::encode_payload(full, buf),
            guest_public: Self::encode_payload(guest_public, buf),
            guest_redacted: Self::encode_payload(guest_redacted, buf),
        }
    }

    pub(crate) fn from_state(state: &AppState) -> Self {
        let build_guest = state
            .ws_limited_clients
            .load(std::sync::atomic::Ordering::Relaxed)
            > 0;
        Self::from_state_inner(state, build_guest)
    }

    fn from_state_inner(state: &AppState, build_guest: bool) -> Self {
        let system = read_rwlock(&state.latest_telemetry);
        let streams = read_rwlock(&state.media_streams);
        let app_statuses = read_rwlock(&state.app_statuses);
        let agent_connected = read_agent_connected(&state.agent_connected);
        let smart_home = read_rwlock(&state.smart_home_telemetry);

        let network = system.network.clone().unwrap_or_default();
        let nodes = read_rwlock(&state.telemetry_by_node);
        let nodes_meta = read_rwlock(&state.nodes_meta);

        let guest_app_statuses: HashMap<String, AppStatus> = app_statuses
            .iter()
            .map(|(name, status)| {
                let public_status = if status.status.eq_ignore_ascii_case("ONLINE") {
                    "ONLINE"
                } else {
                    "OFFLINE"
                };
                (
                    name.clone(),
                    AppStatus {
                        status: public_status.to_string(),
                        latency_ms: None,
                    },
                )
            })
            .collect();

        let guest_containers = sanitize_containers_for_guest(&system.lxc_containers);
        let mut guest_system = system.clone();
        guest_system.lxc_containers = guest_containers.clone();

        let full = FullTelemetry {
            system,
            network: network.clone(),
            streams,
            app_statuses,
            agent_connected,
            smart_home: Some(smart_home),
            nodes,
            nodes_meta,
        };

        if !build_guest {
            let mut buf = String::with_capacity(8192);
            return Self {
                full: Self::encode_payload(&full, &mut buf),
                guest_public: Arc::from("{}"),
                guest_redacted: Arc::from("{}"),
            };
        }

        let guest_public = FullTelemetry {
            system: guest_system,
            network,
            streams: HashMap::new(),
            app_statuses: guest_app_statuses.clone(),
            agent_connected,
            smart_home: None,
            nodes: HashMap::new(),
            nodes_meta: HashMap::new(),
        };

        let guest_redacted = FullTelemetry {
            system: AgentTelemetry {
                lxc_containers: guest_containers,
                ..Default::default()
            },
            network: NetworkTelemetry::default(),
            streams: HashMap::new(),
            app_statuses: guest_app_statuses,
            agent_connected,
            smart_home: None,
            nodes: HashMap::new(),
            nodes_meta: HashMap::new(),
        };

        let mut buf = String::with_capacity(8192);
        Self::from_payloads(&full, &guest_public, &guest_redacted, &mut buf)
    }
}

/// Select the WebSocket JSON frame for a connected session role.
pub(crate) fn ws_frame_from_bundle(
    bundle: &WsTelemetryBundle,
    limited_telemetry: bool,
    public: bool,
) -> Arc<str> {
    if limited_telemetry {
        if public {
            bundle.guest_public.clone()
        } else {
            bundle.guest_redacted.clone()
        }
    } else {
        bundle.full.clone()
    }
}

pub(crate) fn new_telemetry_broadcast() -> watch::Sender<Arc<WsTelemetryBundle>> {
    let (tx, _) = watch::channel(Arc::new(WsTelemetryBundle::default()));
    tx
}

pub(crate) fn start_telemetry_broadcaster(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let interval_secs = {
                let settings = state.settings_cache.read().unwrap();
                crate::settings::setting_u64_bounded(
                    &settings,
                    "telemetry_broadcast_interval_secs",
                    5,
                    3,
                    60,
                )
            };
            tokio::time::sleep(Duration::from_secs(interval_secs)).await;
            if !crate::activity::is_active(&state) {
                continue;
            }
            if state.telemetry_broadcast.receiver_count() == 0 {
                continue;
            }
            let bundle = Arc::new(WsTelemetryBundle::from_state(&state));
            let _ = state.telemetry_broadcast.send(bundle);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AppState;
    use amud_protocol::LxcContainer;
    use std::collections::HashMap;
    use std::sync::atomic::AtomicU64;
    use std::sync::{Mutex, RwLock};

    fn test_state(app_statuses: HashMap<String, AppStatus>) -> AppState {
        let system = AgentTelemetry {
            cpu_usage: 42,
            lxc_containers: vec![LxcContainer {
                vmid: 200,
                status: "running".to_string(),
                name: "jellyfin".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };
        AppState {
            db: Arc::new(Mutex::new(
                rusqlite::Connection::open_in_memory().expect("in-memory db"),
            )),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            latest_telemetry: Arc::new(RwLock::new(system)),
            telemetry_by_node: Arc::new(RwLock::new(HashMap::new())),
            agent_connected: Arc::new(RwLock::new(true)),
            media_streams: Arc::new(RwLock::new(HashMap::new())),
            app_statuses: Arc::new(RwLock::new(app_statuses)),
            agent_sessions: Arc::new(Mutex::new(HashMap::new())),
            nodes_meta: Arc::new(RwLock::new(HashMap::new())),
            next_agent_conn_id: Arc::new(AtomicU64::new(1)),
            pve_test_response: Arc::new(RwLock::new(None)),
            docker_discover_response: Arc::new(RwLock::new(None)),
            telemetry_discover_response: Arc::new(RwLock::new(None)),
            share_sessions: Arc::new(RwLock::new(HashMap::new())),
            action_results: Arc::new(RwLock::new(HashMap::new())),
            settings_cache: Arc::new(RwLock::new(HashMap::new())),
            alert_cooldowns: Arc::new(Mutex::new(HashMap::new())),
            login_attempts: Arc::new(Mutex::new(HashMap::new())),
            api_rate_limits: Arc::new(Mutex::new(HashMap::new())),
            agent_secret: Arc::new("test-secret".to_string()),
            smart_home_telemetry: Arc::new(RwLock::new(Default::default())),
            logo_manifest: Arc::new(HashMap::new()),
            telemetry_broadcast: new_telemetry_broadcast(),
            integration_cache: Arc::new(crate::integration_cache::IntegrationCache::new(64, 45)),
            http_clients: Arc::new(crate::http_client::build_shared_http_clients()),
            ws_limited_clients: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            activity_mode: Arc::new(std::sync::atomic::AtomicU8::new(0)),
            active_ws_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            active_gui_sessions: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            visible_app_ids: Arc::new(RwLock::new(Vec::new())),
            last_activity_at: Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
            node_last_seen: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[test]
    fn telemetry_broadcast_receiver_count_tracks_subscribers() {
        let tx = new_telemetry_broadcast();
        assert_eq!(tx.receiver_count(), 0);
        let _rx = tx.subscribe();
        assert_eq!(tx.receiver_count(), 1);
    }

    #[test]
    fn guest_redacted_includes_online_offline_only() {
        let mut statuses = HashMap::new();
        statuses.insert(
            "jellyfin".to_string(),
            AppStatus {
                status: "ONLINE".to_string(),
                latency_ms: Some(12),
            },
        );
        statuses.insert(
            "radarr".to_string(),
            AppStatus {
                status: "BLOCKED".to_string(),
                latency_ms: None,
            },
        );

        let state = test_state(statuses);
        state
            .ws_limited_clients
            .store(1, std::sync::atomic::Ordering::Relaxed);
        let bundle = WsTelemetryBundle::from_state(&state);
        let guest: serde_json::Value =
            serde_json::from_str(&bundle.guest_redacted).expect("telemetry json");

        assert!(guest["system"]["lxc_containers"]
            .as_array()
            .is_some_and(|v| !v.is_empty()));
        assert_eq!(guest["system"]["lxc_containers"][0]["name"], "jellyfin");
        assert_eq!(guest["system"]["lxc_containers"][0]["status"], "running");
        assert_eq!(guest["system"]["lxc_containers"][0]["vmid"], 0);
        assert!(guest["system"]["lxc_containers"][0]["cpu"].is_null());
        assert_eq!(guest["system"]["cpu_usage"], 0);
        assert_eq!(guest["agent_connected"], true);
        assert!(guest.get("smart_home").is_none());

        assert_eq!(guest["app_statuses"]["jellyfin"]["status"], "ONLINE");
        assert!(guest["app_statuses"]["jellyfin"]["latency_ms"].is_null());

        assert_eq!(guest["app_statuses"]["radarr"]["status"], "OFFLINE");
    }

    #[test]
    fn guest_public_keeps_basic_system_metrics_and_sanitized_containers() {
        let state = test_state(HashMap::new());
        state
            .ws_limited_clients
            .store(1, std::sync::atomic::Ordering::Relaxed);
        let bundle = WsTelemetryBundle::from_state(&state);
        let guest: serde_json::Value =
            serde_json::from_str(&bundle.guest_public).expect("telemetry json");

        let containers = guest["system"]["lxc_containers"]
            .as_array()
            .expect("guest containers");
        assert_eq!(containers.len(), 1);
        assert_eq!(containers[0]["name"], "jellyfin");
        assert_eq!(containers[0]["status"], "running");
        assert_eq!(containers[0]["vmid"], 0);
        assert!(containers[0]["cpu"].is_null());
        assert_eq!(guest["system"]["cpu_usage"], 42);
        assert_eq!(guest["agent_connected"], true);
    }
}
