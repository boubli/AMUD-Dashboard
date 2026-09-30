//! GUI-aware idle/active runtime — gates pollers and caches for RAM efficiency.

use crate::agent::push_agent_config;
use crate::models::AppState;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Deep idle: no GUI, pollers stopped, caches cleared.
pub const MODE_DEEP_IDLE: u8 = 0;
/// Grace period after disconnect before deep idle.
pub const MODE_GRACE: u8 = 1;
/// User in dashboard/settings — full polling for visible cards.
pub const MODE_ACTIVE: u8 = 2;

pub const MAX_VISIBLE_APPS: usize = 50;

pub(crate) fn activity_mode_name(mode: u8) -> &'static str {
    match mode {
        MODE_ACTIVE => "Active",
        MODE_GRACE => "Grace period",
        _ => "Deep idle",
    }
}

pub(crate) fn is_active(state: &AppState) -> bool {
    state.activity_mode.load(Ordering::Relaxed) == MODE_ACTIVE
}

pub(crate) fn is_deep_idle(state: &AppState) -> bool {
    state.activity_mode.load(Ordering::Relaxed) == MODE_DEEP_IDLE
}

pub(crate) fn visible_app_ids(state: &AppState) -> Vec<i64> {
    state
        .visible_app_ids
        .read()
        .unwrap_or_else(|p| p.into_inner())
        .clone()
}

pub(crate) fn set_visible_app_ids(state: &AppState, ids: Vec<i64>) {
    let mut capped: Vec<i64> = ids.into_iter().take(MAX_VISIBLE_APPS).collect();
    capped.sort_unstable();
    capped.dedup();
    *state.visible_app_ids.write().unwrap() = capped;
}

fn grace_secs(state: &AppState) -> u64 {
    let cache = state.settings_cache.read().unwrap();
    crate::settings::setting_u64_bounded(&cache, "idle_grace_secs", 45, 15, 300)
}

pub(crate) fn signal_ws_connected(state: &Arc<AppState>) {
    state.active_ws_count.fetch_add(1, Ordering::Relaxed);
    transition_toward_active(state);
}

pub(crate) fn signal_ws_disconnected(state: &Arc<AppState>) {
    let prev = state.active_ws_count.fetch_sub(1, Ordering::Relaxed);
    if prev <= 1 {
        state.active_ws_count.store(0, Ordering::Relaxed);
    }
    maybe_enter_grace(state);
}

pub(crate) fn signal_gui_session_start(state: &Arc<AppState>) {
    state.active_gui_sessions.fetch_add(1, Ordering::Relaxed);
    transition_toward_active(state);
}

pub(crate) fn signal_gui_session_end(state: &Arc<AppState>) {
    let prev = state.active_gui_sessions.fetch_sub(1, Ordering::Relaxed);
    if prev <= 1 {
        state.active_gui_sessions.store(0, Ordering::Relaxed);
    }
    maybe_enter_grace(state);
}

pub(crate) fn signal_viewport(state: &Arc<AppState>, ids: Vec<i64>) {
    set_visible_app_ids(state, ids);
    if is_active(state) {
        return;
    }
    if has_gui_presence(state) {
        transition_toward_active(state);
    }
}

fn has_gui_presence(state: &AppState) -> bool {
    state.active_ws_count.load(Ordering::Relaxed) > 0
        || state.active_gui_sessions.load(Ordering::Relaxed) > 0
}

fn transition_toward_active(state: &Arc<AppState>) {
    let prev = state.activity_mode.swap(MODE_ACTIVE, Ordering::Relaxed);
    if prev != MODE_ACTIVE {
        enter_active(state);
    }
    *state.last_activity_at.lock().unwrap() = Instant::now();
}

fn maybe_enter_grace(state: &Arc<AppState>) {
    if has_gui_presence(state) {
        state.activity_mode.store(MODE_ACTIVE, Ordering::Relaxed);
        *state.last_activity_at.lock().unwrap() = Instant::now();
        return;
    }
    state.activity_mode.store(MODE_GRACE, Ordering::Relaxed);
    *state.last_activity_at.lock().unwrap() = Instant::now();
}

pub(crate) fn enter_deep_idle(state: &Arc<AppState>) {
    let prev = state.activity_mode.swap(MODE_DEEP_IDLE, Ordering::Relaxed);
    if prev == MODE_DEEP_IDLE {
        return;
    }
    state.app_statuses.write().unwrap().clear();
    state.integration_cache.clear();
    state.media_streams.write().unwrap().clear();
    *state.visible_app_ids.write().unwrap() = Vec::new();
    trim_telemetry_for_idle(state);
    push_agent_config(state, None);
}

fn trim_telemetry_for_idle(state: &AppState) {
    {
        let mut latest = state.latest_telemetry.write().unwrap();
        latest.lxc_containers.clear();
        latest.visible_mounts.clear();
        latest.visible_ifaces.clear();
        latest.disk_volumes.clear();
    }
    let mut by_node = state.telemetry_by_node.write().unwrap();
    for tel in by_node.values_mut() {
        tel.lxc_containers.clear();
        tel.visible_mounts.clear();
        tel.visible_ifaces.clear();
        tel.disk_volumes.clear();
    }
}

fn enter_active(state: &Arc<AppState>) {
    push_agent_config(state, None);
}

pub(crate) fn start_activity_supervisor(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let mode = state.activity_mode.load(Ordering::Relaxed);
            if mode == MODE_GRACE && !has_gui_presence(&state) {
                let elapsed = state.last_activity_at.lock().unwrap().elapsed();
                if elapsed >= Duration::from_secs(grace_secs(&state)) {
                    enter_deep_idle(&state);
                }
            } else if mode == MODE_ACTIVE && !has_gui_presence(&state) {
                state.activity_mode.store(MODE_GRACE, Ordering::Relaxed);
                *state.last_activity_at.lock().unwrap() = Instant::now();
            }
        }
    });
}

pub(crate) fn start_alert_evaluator(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(120)).await;
            if !is_deep_idle(&state) {
                continue;
            }
            crate::webhooks::evaluate_idle_alerts(&state).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integration_cache::IntegrationCache;
    use crate::models::AppState;
    use amud_protocol::AgentTelemetry;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU64, AtomicU8, AtomicUsize};
    use std::sync::{Mutex, RwLock};

    fn test_state() -> Arc<AppState> {
        Arc::new(AppState {
            db: Arc::new(Mutex::new(rusqlite::Connection::open_in_memory().unwrap())),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            latest_telemetry: Arc::new(RwLock::new(AgentTelemetry::default())),
            telemetry_by_node: Arc::new(RwLock::new(HashMap::new())),
            agent_connected: Arc::new(RwLock::new(false)),
            media_streams: Arc::new(RwLock::new(HashMap::new())),
            app_statuses: Arc::new(RwLock::new(HashMap::new())),
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
            telemetry_broadcast: crate::telemetry_broadcast::new_telemetry_broadcast(),
            integration_cache: Arc::new(IntegrationCache::new(32, 45)),
            http_clients: Arc::new(crate::http_client::build_shared_http_clients()),
            ws_limited_clients: Arc::new(AtomicUsize::new(0)),
            activity_mode: Arc::new(AtomicU8::new(MODE_DEEP_IDLE)),
            active_ws_count: Arc::new(AtomicUsize::new(0)),
            active_gui_sessions: Arc::new(AtomicUsize::new(0)),
            visible_app_ids: Arc::new(RwLock::new(Vec::new())),
            last_activity_at: Arc::new(Mutex::new(Instant::now())),
            node_last_seen: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    #[test]
    fn ws_connect_enters_active() {
        let state = test_state();
        assert!(is_deep_idle(&state));
        signal_ws_connected(&state);
        assert!(is_active(&state));
    }

    #[test]
    fn viewport_caps_at_fifty() {
        let state = test_state();
        let ids: Vec<i64> = (1..=100).collect();
        set_visible_app_ids(&state, ids);
        assert_eq!(visible_app_ids(&state).len(), MAX_VISIBLE_APPS);
    }
}
