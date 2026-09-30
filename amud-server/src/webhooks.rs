use crate::db::load_app_name_urls_for_ids;
use crate::models::{AgentTelemetry, AppState, AppStatus, LxcContainer};
use crate::security::{url_allowed_for_health_check, url_allowed_for_webhook};
use futures_util::future::join_all;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub(crate) const WEBHOOK_EVENT_TYPES: &[&str] = &[
    "container_started",
    "container_stopped",
    "agent_connected",
    "agent_disconnected",
    "host_cpu_high",
    "host_ram_high",
    "host_disk_high",
    "app_offline",
    "backup_overdue",
];

const ALERT_COOLDOWN_SECS: u64 = 60;
const STATUS_IDLE_BACKOFF_SECS: u64 = 300;
const MAX_ALERT_COOLDOWN_KEYS: usize = 512;

fn prune_alert_cooldowns(cooldowns: &mut HashMap<String, Instant>) {
    let window = Duration::from_secs(ALERT_COOLDOWN_SECS);
    cooldowns.retain(|_, last| last.elapsed() < window);
    if cooldowns.len() <= MAX_ALERT_COOLDOWN_KEYS {
        return;
    }
    let mut entries: Vec<(String, Instant)> =
        cooldowns.iter().map(|(k, v)| (k.clone(), *v)).collect();
    entries.sort_by_key(|(_, instant)| *instant);
    let drop_count = entries.len().saturating_sub(MAX_ALERT_COOLDOWN_KEYS);
    for (key, _) in entries.into_iter().take(drop_count) {
        cooldowns.remove(&key);
    }
}

pub(crate) fn normalize_webhook_event_types(raw: &str) -> Option<String> {
    let events: Vec<&str> = raw
        .split(',')
        .map(str::trim)
        .filter(|e| !e.is_empty())
        .collect();
    if events.is_empty() {
        return None;
    }
    for event in &events {
        if !WEBHOOK_EVENT_TYPES.contains(event) {
            return None;
        }
    }
    Some(events.join(","))
}

pub(crate) fn start_status_poller(state: Arc<AppState>) {
    tokio::spawn(async move {
        // Consecutive URL-health failures per app. Require 2 before writing OFFLINE
        // so a single blip during boot/reconnect does not flap the dashboard.
        let mut fail_streak: std::collections::HashMap<String, u32> =
            std::collections::HashMap::new();
        loop {
            if !crate::activity::is_active(&state) {
                tokio::time::sleep(Duration::from_secs(30)).await;
                continue;
            }

            let visible = crate::activity::visible_app_ids(&state);
            if visible.is_empty() {
                tokio::time::sleep(Duration::from_secs(15)).await;
                continue;
            }
            let accept_invalid = {
                let cache = state.settings_cache.read().unwrap();
                cache
                    .get("accept_invalid_certs")
                    .map(|s| s == "1")
                    .unwrap_or(false)
            };
            let client =
                crate::http_client::select_http_client(&state.http_clients, accept_invalid).clone();
            let db_for_blocking = state.db.clone();
            let ids = visible.clone();
            let apps = tokio::task::spawn_blocking(move || {
                let db = db_for_blocking.lock().unwrap();
                load_app_name_urls_for_ids(&db, &ids)
            })
            .await
            .unwrap_or_default();

            let checks = join_all(apps.into_iter().map(|(name, url)| {
                let client = client.clone();
                async move {
                    let started = Instant::now();
                    let status = if !url_allowed_for_health_check(&url) {
                        AppStatus {
                            status: "BLOCKED".to_string(),
                            latency_ms: None,
                        }
                    } else {
                        match client.get(&url).send().await {
                            Ok(resp)
                                if resp.status().is_success() || resp.status().is_redirection() =>
                            {
                                AppStatus {
                                    status: "ONLINE".to_string(),
                                    latency_ms: Some(started.elapsed().as_millis()),
                                }
                            }
                            Ok(_) | Err(_) => AppStatus {
                                status: "OFFLINE".to_string(),
                                latency_ms: None,
                            },
                        }
                    };
                    (name.to_lowercase(), status)
                }
            }))
            .await;
            let checks_empty = checks.is_empty();
            if !checks_empty {
                let mut statuses = state.app_statuses.write().unwrap();
                for (name, status) in checks {
                    let upper = status.status.to_uppercase();
                    if upper == "ONLINE" || upper == "BLOCKED" {
                        fail_streak.remove(&name);
                        statuses.insert(name, status);
                        continue;
                    }
                    // Probe failed — soft-start: need 2 consecutive failures for OFFLINE.
                    let streak = fail_streak.entry(name.clone()).or_insert(0);
                    *streak = streak.saturating_add(1);
                    if *streak >= 2 {
                        statuses.insert(name, status);
                    } else {
                        // First failure with no prior status: stay in CHECKING (UI waiting state).
                        // If previously ONLINE, leave it until the second consecutive failure.
                        statuses.entry(name).or_insert_with(|| AppStatus {
                            status: "CHECKING".to_string(),
                            latency_ms: None,
                        });
                    }
                }
                while statuses.len() > crate::activity::MAX_VISIBLE_APPS {
                    if let Some(key) = statuses.keys().next().cloned() {
                        statuses.remove(&key);
                    } else {
                        break;
                    }
                }
            }
            let interval = {
                let settings = state.settings_cache.read().unwrap();
                crate::settings::setting_u64_bounded(
                    &settings,
                    "status_poll_interval_secs",
                    15,
                    10,
                    300,
                )
            };
            let sleep_secs = if checks_empty {
                STATUS_IDLE_BACKOFF_SECS
            } else {
                interval
            };
            tokio::time::sleep(Duration::from_secs(sleep_secs)).await;
        }
    });
}

/// Slow alert evaluation while server is in deep idle (CPU/RAM/disk thresholds).
pub(crate) async fn evaluate_idle_alerts(state: &Arc<AppState>) {
    let settings = state.settings_cache.read().unwrap().clone();
    let cpu_threshold: f64 = settings
        .get("alert_cpu_threshold")
        .and_then(|s| s.parse().ok())
        .unwrap_or(90.0);
    let ram_threshold: f64 = settings
        .get("alert_ram_threshold")
        .and_then(|s| s.parse().ok())
        .unwrap_or(90.0);
    let disk_threshold: f64 = settings
        .get("alert_disk_threshold")
        .and_then(|s| s.parse().ok())
        .unwrap_or(95.0);

    let telemetry = state.latest_telemetry.read().unwrap().clone();
    let mut events: Vec<&str> = Vec::new();
    if telemetry.cpu_usage as f64 >= cpu_threshold {
        events.push("host_cpu_high");
    }
    if telemetry.ram_usage as f64 >= ram_threshold {
        events.push("host_ram_high");
    }
    if telemetry.disk_usage as f64 >= disk_threshold {
        events.push("host_disk_high");
    }

    if events.is_empty() {
        return;
    }

    let accept_invalid = settings
        .get("accept_invalid_certs")
        .map(|s| s == "1")
        .unwrap_or(false);
    let allow_private = settings
        .get("webhooks_allow_private_ips")
        .map(|s| s == "1")
        .unwrap_or(false);
    let client =
        crate::http_client::select_http_client(&state.http_clients, accept_invalid).clone();

    for event_type in events {
        let event = event_type.to_string();
        let webhooks = crate::db::with_db(state.db.clone(), move |db| {
            crate::db::load_active_webhooks_for_event(db, &event)
        })
        .await;
        for wh in webhooks {
            let url = wh.url;
            let name = wh.name;
            let event = event_type.to_string();
            let client = client.clone();
            tokio::spawn(async move {
                send_webhook_notification(
                    &client,
                    url,
                    name,
                    &event,
                    "AMUD Host",
                    0,
                    "threshold",
                    "System",
                    allow_private,
                )
                .await;
            });
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) async fn send_webhook_notification(
    client: &reqwest::Client,
    url: String,
    name: String,
    event_type: &str,
    container_name: &str,
    vmid: i64,
    status: &str,
    provider: &str,
    allow_private_ips: bool,
) -> bool {
    if !url_allowed_for_webhook(&url, allow_private_ips) {
        eprintln!("Webhook '{}' blocked: URL failed SSRF policy check", name);
        return false;
    }

    let is_discord = url.contains("discord.com/api/webhooks/");
    let is_telegram = url.contains("api.telegram.org/bot");

    let response = if is_discord {
        let title = if event_type == "test" {
            "AMUD Webhook Test".to_string()
        } else if event_type == "agent_connected" {
            format!("Agent Connected: {}", container_name)
        } else if event_type == "agent_disconnected" {
            format!("Agent Disconnected: {}", container_name)
        } else if status == "running" || status == "online" {
            format!("Container Started: {}", container_name)
        } else {
            format!("Container Stopped: {}", container_name)
        };

        let desc = if event_type == "test" {
            "Your AMUD Webhook Alerts Engine is successfully configured and ready to notify!"
                .to_string()
        } else {
            format!("Container **{}** is now **{}**.", container_name, status)
        };

        let color = if event_type == "test" || event_type == "agent_connected" {
            0x2ecc71
        } else if status == "running" || status == "online" {
            0x10b981
        } else {
            0xef4444
        };

        let mut fields = vec![];
        if event_type != "test" {
            fields.push(serde_json::json!({
                "name": "Provider",
                "value": provider,
                "inline": true
            }));
            fields.push(serde_json::json!({
                "name": "VMID / ID",
                "value": vmid.to_string(),
                "inline": true
            }));
        }

        let payload = serde_json::json!({
            "username": "AMUD Alerts",
            "embeds": [{
                "title": title,
                "description": desc,
                "color": color,
                "fields": fields,
                "timestamp": chrono::Utc::now().to_rfc3339()
            }]
        });

        client.post(&url).json(&payload).send().await
    } else if is_telegram {
        let text = if event_type == "test" {
            "<b>\u{1f514} AMUD Alert Test</b>\nYour Webhook Alerts Engine is successfully configured and ready to notify!".to_string()
        } else {
            let status_emoji = if status == "running" {
                "\u{1f7e2}"
            } else {
                "\u{1f534}"
            };
            format!(
                "{} <b>AMUD Alert: Container Status Changed</b>\n\n<b>Container:</b> <code>{}</code>\n<b>Status:</b> <code>{}</code>\n<b>Provider:</b> <code>{}</code>\n<b>VMID/ID:</b> <code>{}</code>",
                status_emoji, container_name, status.to_uppercase(), provider, vmid
            )
        };

        let parsed_url = reqwest::Url::parse(&url).ok();
        let chat_id = parsed_url.and_then(|u| {
            u.query_pairs()
                .find(|(k, _)| k == "chat_id")
                .map(|(_, v)| v.into_owned())
        });

        let payload = if let Some(cid) = chat_id {
            serde_json::json!({
                "chat_id": cid,
                "text": text,
                "parse_mode": "HTML"
            })
        } else {
            serde_json::json!({
                "text": text,
                "parse_mode": "HTML"
            })
        };

        client.post(&url).json(&payload).send().await
    } else {
        let payload = serde_json::json!({
            "event": event_type,
            "container": {
                "name": container_name,
                "vmid": vmid,
                "status": status,
                "provider": provider
            },
            "timestamp": chrono::Utc::now().to_rfc3339()
        });

        client.post(&url).json(&payload).send().await
    };

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                println!(
                    "Webhook '{}' successfully sent notification for '{}'",
                    name, container_name
                );
                true
            } else {
                eprintln!(
                    "Webhook '{}' failed with status code: {}. Body: {:?}",
                    name,
                    resp.status(),
                    resp.text().await.unwrap_or_default()
                );
                false
            }
        }
        Err(e) => {
            eprintln!("Failed to send webhook '{}': {}", name, e);
            false
        }
    }
}

pub(crate) fn check_container_alerts(
    old_telemetry: &AgentTelemetry,
    new_telemetry: &AgentTelemetry,
    state: &Arc<AppState>,
) {
    let old_containers = &old_telemetry.lxc_containers;
    let new_containers = &new_telemetry.lxc_containers;

    let old_map: HashMap<i64, &LxcContainer> = old_containers.iter().map(|c| (c.vmid, c)).collect();

    let new_map: HashMap<i64, &LxcContainer> = new_containers.iter().map(|c| (c.vmid, c)).collect();

    let mut alert_jobs: Vec<(String, String, i64, String, String, String)> = Vec::new();

    for new_c in new_containers {
        let provider = if new_c.vmid < 0 {
            "Docker"
        } else {
            "Proxmox LXC"
        };
        let cooldown_key = format!(
            "{}:{}",
            if new_c.vmid < 0 { "docker" } else { "lxc" },
            new_c.name
        );

        let event_type = match old_map.get(&new_c.vmid) {
            Some(old_c) if old_c.status != new_c.status => {
                if new_c.status == "running" {
                    Some("container_started")
                } else {
                    Some("container_stopped")
                }
            }
            None if new_c.status == "running" => Some("container_started"),
            _ => None,
        };

        if let Some(event_type) = event_type {
            {
                let mut cooldowns = state.alert_cooldowns.lock().unwrap();
                if let Some(&last_alert) = cooldowns.get(&cooldown_key) {
                    if last_alert.elapsed() < Duration::from_secs(ALERT_COOLDOWN_SECS) {
                        println!("Alert for {} is suppressed due to cooldown", cooldown_key);
                        continue;
                    }
                }
                cooldowns.insert(cooldown_key.clone(), Instant::now());
                prune_alert_cooldowns(&mut cooldowns);
            }

            alert_jobs.push((
                event_type.to_string(),
                new_c.name.clone(),
                new_c.vmid,
                new_c.status.clone(),
                provider.to_string(),
                cooldown_key,
            ));
        }
    }

    for old_c in old_containers {
        if !new_map.contains_key(&old_c.vmid) {
            let provider = if old_c.vmid < 0 {
                "Docker"
            } else {
                "Proxmox LXC"
            };
            let cooldown_key = format!(
                "{}:{}",
                if old_c.vmid < 0 { "docker" } else { "lxc" },
                old_c.name
            );
            {
                let mut cooldowns = state.alert_cooldowns.lock().unwrap();
                if let Some(&last_alert) = cooldowns.get(&cooldown_key) {
                    if last_alert.elapsed() < Duration::from_secs(ALERT_COOLDOWN_SECS) {
                        continue;
                    }
                }
                cooldowns.insert(cooldown_key, Instant::now());
                prune_alert_cooldowns(&mut cooldowns);
            }
            alert_jobs.push((
                "container_stopped".to_string(),
                old_c.name.clone(),
                old_c.vmid,
                "stopped".to_string(),
                provider.to_string(),
                String::new(),
            ));
        }
    }

    if alert_jobs.is_empty() {
        return;
    }

    let state = state.clone();
    tokio::spawn(async move {
        let accept_invalid = {
            let cache = state.settings_cache.read().unwrap();
            cache
                .get("accept_invalid_certs")
                .map(|s| s == "1")
                .unwrap_or(false)
        };
        let allow_private = {
            let cache = state.settings_cache.read().unwrap();
            cache
                .get("webhooks_allow_private_ips")
                .map(|s| s == "1")
                .unwrap_or(false)
        };
        let webhooks = crate::db::with_db(state.db.clone(), crate::db::load_active_webhooks).await;
        let http_client =
            crate::http_client::select_http_client(&state.http_clients, accept_invalid).clone();
        for (event_type, container_name, vmid, status_str, provider_str, _) in alert_jobs {
            for wh in &webhooks {
                let subscribed = wh.event_types.split(',').any(|e| e.trim() == event_type);
                if !subscribed {
                    continue;
                }
                let url = wh.url.clone();
                let name = wh.name.clone();
                let event = event_type.clone();
                let container_name = container_name.clone();
                let status_str = status_str.clone();
                let provider_str = provider_str.clone();
                let client = http_client.clone();
                tokio::spawn(async move {
                    send_webhook_notification(
                        &client,
                        url,
                        name,
                        &event,
                        &container_name,
                        vmid,
                        &status_str,
                        &provider_str,
                        allow_private,
                    )
                    .await;
                });
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::apps::{is_jellyfin_app, is_plex_app};
    use crate::media::default_media_streams;
    use crate::models::App;
    use rusqlite::{params, Connection};
    use std::sync::{Mutex, RwLock};

    #[tokio::test]
    async fn test_check_container_alerts_transition() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS webhooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                event_types TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
            [],
        )
        .unwrap();

        conn.execute(
            "INSERT INTO webhooks (name, url, event_types, is_active) VALUES (?, ?, ?, ?)",
            params![
                "Test WH",
                "https://discord.com/api/webhooks/test",
                "container_stopped",
                1
            ],
        )
        .unwrap();

        let state = Arc::new(AppState {
            db: Arc::new(Mutex::new(conn)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            latest_telemetry: Arc::new(RwLock::new(AgentTelemetry::default())),
            telemetry_by_node: Arc::new(RwLock::new(HashMap::new())),
            agent_connected: Arc::new(RwLock::new(false)),
            media_streams: Arc::new(RwLock::new(default_media_streams())),
            app_statuses: Arc::new(RwLock::new(HashMap::new())),
            agent_sessions: Arc::new(Mutex::new(HashMap::new())),
            nodes_meta: Arc::new(RwLock::new(HashMap::new())),
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
            next_agent_conn_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            logo_manifest: Arc::new(HashMap::new()),
            telemetry_broadcast: crate::telemetry_broadcast::new_telemetry_broadcast(),
            integration_cache: Arc::new(crate::integration_cache::IntegrationCache::new(64, 45)),
            http_clients: Arc::new(crate::http_client::build_shared_http_clients()),
            ws_limited_clients: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            activity_mode: Arc::new(std::sync::atomic::AtomicU8::new(0)),
            active_ws_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            active_gui_sessions: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            visible_app_ids: Arc::new(RwLock::new(Vec::new())),
            last_activity_at: Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
            node_last_seen: Arc::new(RwLock::new(HashMap::new())),
        });

        let old_telemetry = AgentTelemetry {
            lxc_containers: vec![LxcContainer {
                vmid: 100,
                status: "running".to_string(),
                name: "test-lxc".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let new_telemetry = AgentTelemetry {
            lxc_containers: vec![LxcContainer {
                vmid: 100,
                status: "stopped".to_string(),
                name: "test-lxc".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        // Trigger alert check
        check_container_alerts(&old_telemetry, &new_telemetry, &state);

        // Verify that it added the cooldown key (lxc:test-lxc) to alert_cooldowns map
        let cooldowns = state.alert_cooldowns.lock().unwrap();
        assert!(cooldowns.contains_key("lxc:test-lxc"));
    }

    #[tokio::test]
    async fn test_check_container_alerts_no_change() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE IF NOT EXISTS webhooks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                url TEXT NOT NULL,
                event_types TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );",
            [],
        )
        .unwrap();

        let state = Arc::new(AppState {
            db: Arc::new(Mutex::new(conn)),
            sessions: Arc::new(RwLock::new(HashMap::new())),
            latest_telemetry: Arc::new(RwLock::new(AgentTelemetry::default())),
            telemetry_by_node: Arc::new(RwLock::new(HashMap::new())),
            agent_connected: Arc::new(RwLock::new(false)),
            media_streams: Arc::new(RwLock::new(default_media_streams())),
            app_statuses: Arc::new(RwLock::new(HashMap::new())),
            agent_sessions: Arc::new(Mutex::new(HashMap::new())),
            nodes_meta: Arc::new(RwLock::new(HashMap::new())),
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
            next_agent_conn_id: Arc::new(std::sync::atomic::AtomicU64::new(1)),
            logo_manifest: Arc::new(HashMap::new()),
            telemetry_broadcast: crate::telemetry_broadcast::new_telemetry_broadcast(),
            integration_cache: Arc::new(crate::integration_cache::IntegrationCache::new(64, 45)),
            http_clients: Arc::new(crate::http_client::build_shared_http_clients()),
            ws_limited_clients: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            activity_mode: Arc::new(std::sync::atomic::AtomicU8::new(0)),
            active_ws_count: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            active_gui_sessions: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
            visible_app_ids: Arc::new(RwLock::new(Vec::new())),
            last_activity_at: Arc::new(std::sync::Mutex::new(std::time::Instant::now())),
            node_last_seen: Arc::new(RwLock::new(HashMap::new())),
        });

        let old_telemetry = AgentTelemetry {
            lxc_containers: vec![LxcContainer {
                vmid: 100,
                status: "running".to_string(),
                name: "test-lxc".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        let new_telemetry = AgentTelemetry {
            lxc_containers: vec![LxcContainer {
                vmid: 100,
                status: "running".to_string(),
                name: "test-lxc".to_string(),
                ..Default::default()
            }],
            ..Default::default()
        };

        check_container_alerts(&old_telemetry, &new_telemetry, &state);

        let cooldowns = state.alert_cooldowns.lock().unwrap();
        assert!(!cooldowns.contains_key("lxc:test-lxc"));
    }

    fn sample_app(name: &str, url: &str, icon: &str) -> App {
        App {
            id: 1,
            name: name.to_string(),
            url: url.to_string(),
            icon: icon.to_string(),
            description: String::new(),
            category: String::new(),
            node_tag: String::new(),
            mac_address: String::new(),
            integration_type: String::new(),
            api_key: String::new(),
            sort_order: 0,
            card_span: "1x1".to_string(),
            show_container_metrics: true,
            guest_visible: true,
            embed_mode: "link".to_string(),
            integration_visible_metrics: String::new(),
        }
    }

    #[test]
    fn normalize_webhook_event_types_rejects_unknown_events() {
        assert_eq!(
            normalize_webhook_event_types("container_started,container_stopped"),
            Some("container_started,container_stopped".to_string())
        );
        assert_eq!(normalize_webhook_event_types(""), None);
        assert_eq!(normalize_webhook_event_types("not_a_real_event"), None);
        assert_eq!(
            normalize_webhook_event_types("container_started, bad_event"),
            None
        );
    }

    #[test]
    fn media_stream_cards_require_registered_apps() {
        assert!(!is_jellyfin_app(&sample_app(
            "Sonarr",
            "http://sonarr.local",
            "sonarr"
        )));
        assert!(!is_jellyfin_app(&sample_app(
            "Assembly",
            "http://assembly.local",
            "custom"
        )));
        assert!(is_jellyfin_app(&sample_app(
            "Media Server",
            "http://nas:8096",
            "jellyfin"
        )));
        assert!(is_jellyfin_app(&sample_app(
            "Jellyfin",
            "http://nas:8096",
            "jellyfin"
        )));
        assert!(!is_plex_app(&sample_app(
            "Sonarr",
            "http://sonarr.local",
            "sonarr"
        )));
        assert!(is_plex_app(&sample_app(
            "Plex",
            "http://plex.local:32400",
            "plex"
        )));
    }
}
