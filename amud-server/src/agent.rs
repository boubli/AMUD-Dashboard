use crate::auth::{agent_challenge_nonce, parse_agent_auth_proof, verify_agent_auth};
use crate::db::{load_active_webhooks_for_event, with_db};
use crate::models::{
    ActionResult, ActionResultMsg, AgentCommandHandle, AgentTelemetry, AppState, NodeMeta,
    PveTestResult,
};
use crate::webhooks::{check_container_alerts, send_webhook_notification};
use serde::Deserialize;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
#[cfg(unix)]
use std::path::Path as FilePath;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Instant;
use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt};
use tokio::net::TcpListener as TokioTcpListener;
#[cfg(unix)]
use tokio::net::UnixListener as TokioUnixListener;

const NODE_STALE_SECS: u64 = 300;
const MAX_AGENT_TCP_CONNECTIONS: usize = 64;

fn evict_stale_nodes(state: &AppState, now: u64) {
    let stale: Vec<String> = state
        .node_last_seen
        .read()
        .unwrap()
        .iter()
        .filter(|(_, &seen)| now.saturating_sub(seen) > NODE_STALE_SECS)
        .map(|(tag, _)| tag.clone())
        .collect();
    if stale.is_empty() {
        return;
    }
    let mut nodes = state.telemetry_by_node.write().unwrap();
    let mut seen = state.node_last_seen.write().unwrap();
    let mut meta = state.nodes_meta.write().unwrap();
    let sessions = state.agent_sessions.lock().unwrap();
    for tag in stale {
        if sessions.contains_key(&tag) {
            continue;
        }
        nodes.remove(&tag);
        seen.remove(&tag);
        meta.remove(&tag);
    }
}

fn sync_agent_connected_flag_with_webhook(state: &Arc<AppState>) {
    let any = !state.agent_sessions.lock().unwrap().is_empty();
    let was = {
        let mut lock = state.agent_connected.write().unwrap();
        let old = *lock;
        *lock = any;
        old
    };
    if was != any {
        fire_agent_connection_webhook(state.clone(), any);
    }
}

pub(crate) fn agent_session_for(state: &AppState, node_tag: &str) -> Option<AgentCommandHandle> {
    let tag = if node_tag.trim().is_empty() {
        "Local"
    } else {
        node_tag.trim()
    };
    state.agent_sessions.lock().unwrap().get(tag).cloned()
}

/// Prefer `Local`, else the first connected session (for discover / PVE test).
pub(crate) fn any_agent_session(state: &AppState) -> Option<AgentCommandHandle> {
    let sessions = state.agent_sessions.lock().unwrap();
    if let Some(h) = sessions.get("Local") {
        return Some(h.clone());
    }
    sessions.values().next().cloned()
}

pub(crate) fn agent_session_for_or_any(
    state: &AppState,
    node_tag: Option<&str>,
) -> Option<AgentCommandHandle> {
    if let Some(tag) = node_tag.map(str::trim).filter(|t| !t.is_empty()) {
        return agent_session_for(state, tag);
    }
    any_agent_session(state)
}

pub(crate) fn register_agent_session_arc(
    state: &Arc<AppState>,
    node_tag: &str,
    handle: AgentCommandHandle,
    capabilities: Vec<String>,
    platform: String,
) -> Result<(), String> {
    let tag = node_tag.trim();
    if tag.is_empty() {
        return Err("node_tag is required".into());
    }
    {
        let mut sessions = state.agent_sessions.lock().unwrap();
        if let Some(existing) = sessions.get(tag) {
            if existing.id != handle.id {
                return Err(format!("node_tag '{tag}' is already connected"));
            }
        }
        sessions.retain(|_, h| h.id != handle.id);
        sessions.insert(tag.to_string(), handle);
    }
    let now = crate::auth::now_epoch_secs();
    state
        .node_last_seen
        .write()
        .unwrap()
        .insert(tag.to_string(), now);
    state.nodes_meta.write().unwrap().insert(
        tag.to_string(),
        NodeMeta {
            connected: true,
            last_seen: now,
            capabilities,
            platform,
        },
    );
    sync_agent_connected_flag_with_webhook(state);
    Ok(())
}

pub(crate) fn unregister_agent_session_arc(state: &Arc<AppState>, conn_id: u64) {
    let mut removed_tag: Option<String> = None;
    {
        let mut sessions = state.agent_sessions.lock().unwrap();
        let tag = sessions
            .iter()
            .find(|(_, h)| h.id == conn_id)
            .map(|(t, _)| t.clone());
        if let Some(tag) = tag {
            sessions.remove(&tag);
            removed_tag = Some(tag);
        }
    }
    if let Some(tag) = removed_tag {
        if let Some(meta) = state.nodes_meta.write().unwrap().get_mut(&tag) {
            meta.connected = false;
        }
        println!("AMUD-Agent session released for node_tag={tag}");
    }
    sync_agent_connected_flag_with_webhook(state);
}

pub(crate) fn handle_new_telemetry(state: &Arc<AppState>, mut metrics: AgentTelemetry) {
    if metrics.node_tag.trim().is_empty() {
        metrics.node_tag = "Local".to_string();
    }
    let node = metrics.node_tag.clone();
    let now = crate::auth::now_epoch_secs();
    state
        .node_last_seen
        .write()
        .unwrap()
        .insert(node.clone(), now);
    {
        let mut meta = state.nodes_meta.write().unwrap();
        let entry = meta.entry(node.clone()).or_default();
        entry.last_seen = now;
        entry.connected = state.agent_sessions.lock().unwrap().contains_key(&node);
        if !metrics.capabilities.is_empty() {
            entry.capabilities = metrics.capabilities.clone();
        }
        if !metrics.platform.is_empty() {
            entry.platform = metrics.platform.clone();
        }
    }
    evict_stale_nodes(state, now);
    let old_metrics = {
        let lock = state.latest_telemetry.read().unwrap();
        lock.clone()
    };
    check_container_alerts(&old_metrics, &metrics, state);

    if crate::activity::is_deep_idle(state) {
        metrics.lxc_containers.clear();
        metrics.visible_mounts.clear();
        metrics.visible_ifaces.clear();
        metrics.disk_volumes.clear();
    }

    state
        .telemetry_by_node
        .write()
        .unwrap()
        .insert(node.clone(), metrics.clone());
    if node == "Local" || state.telemetry_by_node.read().unwrap().len() <= 1 {
        *state.latest_telemetry.write().unwrap() = metrics;
    }
}

fn fire_agent_connection_webhook(state: Arc<AppState>, connected: bool) {
    let event_type = if connected {
        "agent_connected"
    } else {
        "agent_disconnected"
    };
    let status_text = if connected { "online" } else { "offline" };
    let event = event_type.to_string();
    let status_str = status_text.to_string();
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
        let event_filter = event.clone();
        let webhooks = with_db(state.db.clone(), move |db| {
            load_active_webhooks_for_event(db, &event_filter)
        })
        .await;
        let http_client =
            crate::http_client::select_http_client(&state.http_clients, accept_invalid).clone();
        for wh in webhooks {
            let url = wh.url;
            let name = wh.name;
            let event = event.clone();
            let status_str = status_str.clone();
            let client = http_client.clone();
            tokio::spawn(async move {
                send_webhook_notification(
                    &client,
                    url,
                    name,
                    &event,
                    "AMUD-Agent Daemon",
                    0,
                    &status_str,
                    "System",
                    allow_private,
                )
                .await;
            });
        }
    });
}

pub(crate) fn start_agent_listener(state: Arc<AppState>) {
    tokio::spawn(async move {
        #[cfg(unix)]
        {
            let socket_path = std::env::var("AMUD_SOCKET_PATH")
                .unwrap_or_else(|_| "/opt/amud/run/amud.sock".to_string());
            let uds_state = state.clone();
            tokio::spawn(async move {
                run_uds_listener(&socket_path, uds_state).await;
            });
        }

        if let Some(addr) = agent_tcp_listen_addr() {
            let tcp_state = state.clone();
            tokio::spawn(async move {
                run_tcp_listener(&addr, tcp_state).await;
            });
        }
    });
}

fn agent_tcp_listen_addr() -> Option<String> {
    if let Ok(addr) = std::env::var("AMUD_AGENT_TCP_LISTEN") {
        let t = addr.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    #[cfg(windows)]
    {
        Some(std::env::var("AMUD_TCP_ADDR").unwrap_or_else(|_| "127.0.0.1:8050".to_string()))
    }
    #[cfg(not(windows))]
    {
        None
    }
}

fn agent_tls_enabled() -> bool {
    matches!(
        std::env::var("AMUD_AGENT_TLS")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1") | Some("true") | Some("yes")
    )
}

#[cfg(unix)]
fn resolve_uds_path(path: &str) -> Option<String> {
    let parent_ok = FilePath::new(path)
        .parent()
        .map(|p| p.exists())
        .unwrap_or(false);
    if parent_ok {
        Some(path.to_string())
    } else {
        eprintln!(
            "AMUD socket directory missing for {} — create the bind mount path (SEC-029).",
            path
        );
        None
    }
}

async fn handle_agent_stream<R, W>(reader: R, mut writer: W, state: Arc<AppState>, label: &str)
where
    R: AsyncRead + Unpin + Send + 'static,
    W: AsyncWrite + Unpin + Send + 'static,
{
    let conn_id = state.next_agent_conn_id.fetch_add(1, Ordering::SeqCst);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let nonce = agent_challenge_nonce();
    let challenge_line = format!("{{\"challenge\":\"{nonce}\"}}\n");

    tokio::spawn(async move {
        if writer.write_all(challenge_line.as_bytes()).await.is_err() {
            return;
        }
        if writer.flush().await.is_err() {
            return;
        }
        while let Some(cmd) = rx.recv().await {
            if writer.write_all(cmd.as_bytes()).await.is_err() {
                break;
            }
            if writer.flush().await.is_err() {
                break;
            }
        }
    });

    let state_clone = state.clone();
    let label = label.to_string();
    let agent_secret = state.agent_secret.clone();
    let cmd_tx = tx.clone();
    tokio::spawn(async move {
        let mut reader = tokio::io::BufReader::new(reader);
        let mut line = String::new();
        let mut authenticated = false;
        let mut registered_tag: Option<String> = None;
        let handle = AgentCommandHandle {
            id: conn_id,
            tx: cmd_tx.clone(),
        };

        while let Ok(n) = reader.read_line(&mut line).await {
            if n == 0 {
                break;
            }

            if !authenticated {
                if let Some(proof) = parse_agent_auth_proof(&line) {
                    if verify_agent_auth(&agent_secret, &nonce, &proof) {
                        authenticated = true;
                        line.clear();
                        continue;
                    }
                }
                println!("AMUD-Agent rejected: invalid IPC authentication ({label}).");
                break;
            }

            if let Ok(hello) = serde_json::from_str::<amud_protocol::AgentHelloMessage>(&line) {
                let tag = hello.hello.node_tag.trim().to_string();
                match register_agent_session_arc(
                    &state_clone,
                    &tag,
                    handle.clone(),
                    hello.hello.capabilities.clone(),
                    hello.hello.platform.clone(),
                ) {
                    Ok(()) => {
                        registered_tag = Some(tag.clone());
                        println!("AMUD-Agent registered node_tag={tag} ({label}).");
                    }
                    Err(e) => {
                        let err = serde_json::json!({ "error": e });
                        if let Ok(mut serialized) = serde_json::to_vec(&err) {
                            serialized.push(b'\n');
                            let _ = handle
                                .tx
                                .send(String::from_utf8_lossy(&serialized).into_owned());
                        }
                        println!("AMUD-Agent rejected ({label}): {e}");
                        break;
                    }
                }
                line.clear();
                continue;
            }

            if registered_tag.is_none() {
                if let Ok(metrics) = serde_json::from_str::<AgentTelemetry>(&line) {
                    let tag = if metrics.node_tag.trim().is_empty() {
                        "Local".to_string()
                    } else {
                        metrics.node_tag.trim().to_string()
                    };
                    match register_agent_session_arc(
                        &state_clone,
                        &tag,
                        handle.clone(),
                        metrics.capabilities.clone(),
                        metrics.platform.clone(),
                    ) {
                        Ok(()) => {
                            registered_tag = Some(tag);
                        }
                        Err(e) => {
                            println!("AMUD-Agent rejected ({label}): {e}");
                            break;
                        }
                    }
                }
            }

            process_agent_line(&state_clone, &handle.tx, &line, registered_tag.as_deref());
            line.clear();
        }
        println!("AMUD-Agent telemetry client disconnected ({label}).");
        if authenticated {
            unregister_agent_session_arc(&state_clone, conn_id);
        }
    });
}

#[cfg(unix)]
fn uds_socket_mode() -> u32 {
    std::env::var("AMUD_SOCKET_MODE")
        .ok()
        .and_then(|raw| {
            let trimmed = raw.trim();
            if trimmed.is_empty() {
                return None;
            }
            u32::from_str_radix(trimmed, 8).ok()
        })
        .unwrap_or(0o660)
}

#[cfg(unix)]
fn apply_uds_socket_permissions(path: &FilePath) {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(uds_socket_mode())).ok();
}

#[cfg(unix)]
async fn run_uds_listener(path: &str, state: Arc<AppState>) {
    let Some(uds_path) = resolve_uds_path(path) else {
        return;
    };

    println!(
        "Starting agent listener via UNIX Domain Socket at {}",
        uds_path
    );
    std::fs::remove_file(&uds_path).ok();

    let listener = match TokioUnixListener::bind(&uds_path) {
        Ok(l) => {
            apply_uds_socket_permissions(FilePath::new(&uds_path));
            l
        }
        Err(e) => {
            eprintln!("UDS bind failed: {}. Telemetry offline listener active.", e);
            return;
        }
    };

    loop {
        if let Ok((stream, _)) = listener.accept().await {
            println!("AMUD-Agent telemetry client UDS stream accepted.");
            let (reader, writer) = stream.into_split();
            handle_agent_stream(reader, writer, state.clone(), "UDS").await;
        }
    }
}

fn load_tls_acceptor() -> Option<tokio_rustls::TlsAcceptor> {
    if !agent_tls_enabled() {
        return None;
    }
    let _ = rustls::crypto::ring::default_provider().install_default();
    let cert_path = match std::env::var("AMUD_AGENT_TLS_CERT") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => {
            eprintln!("AMUD_AGENT_TLS=1 requires AMUD_AGENT_TLS_CERT");
            return None;
        }
    };
    let key_path = match std::env::var("AMUD_AGENT_TLS_KEY") {
        Ok(p) if !p.trim().is_empty() => p,
        _ => {
            eprintln!("AMUD_AGENT_TLS=1 requires AMUD_AGENT_TLS_KEY");
            return None;
        }
    };
    let cert_bytes = match std::fs::read(&cert_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read AMUD_AGENT_TLS_CERT: {e}");
            return None;
        }
    };
    let key_bytes = match std::fs::read(&key_path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read AMUD_AGENT_TLS_KEY: {e}");
            return None;
        }
    };

    let mut cert_reader = std::io::Cursor::new(cert_bytes);
    let certs: Vec<rustls::pki_types::CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .filter_map(|r| r.ok())
        .collect();
    if certs.is_empty() {
        eprintln!("AMUD_AGENT_TLS_CERT contained no certificates.");
        return None;
    }

    let mut key_reader = std::io::Cursor::new(key_bytes);
    let key = match rustls_pemfile::private_key(&mut key_reader) {
        Ok(Some(k)) => k,
        Ok(None) => {
            eprintln!("AMUD_AGENT_TLS_KEY contained no private key.");
            return None;
        }
        Err(e) => {
            eprintln!("Failed to parse AMUD_AGENT_TLS_KEY: {e}");
            return None;
        }
    };

    let config = match rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
    {
        Ok(c) => c,
        Err(e) => {
            eprintln!("TLS server config failed: {e}");
            return None;
        }
    };
    Some(tokio_rustls::TlsAcceptor::from(Arc::new(config)))
}

async fn run_tcp_listener(addr: &str, state: Arc<AppState>) {
    let tls = load_tls_acceptor();
    let mode = if tls.is_some() { "TCP+TLS" } else { "TCP" };
    println!("Starting agent listener via {mode} on {addr}");
    let listener = match TokioTcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Agent TCP bind failed on {addr}: {e}.");
            return;
        }
    };

    let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    loop {
        let Ok((stream, peer)) = listener.accept().await else {
            continue;
        };
        let current = active.load(Ordering::Relaxed);
        if current >= MAX_AGENT_TCP_CONNECTIONS {
            eprintln!("Rejecting agent TCP from {peer}: connection cap reached.");
            continue;
        }
        active.fetch_add(1, Ordering::Relaxed);
        println!("AMUD-Agent telemetry client {mode} stream accepted from {peer}.");
        let state = state.clone();
        let tls = tls.clone();
        let active = active.clone();
        tokio::spawn(async move {
            let label = mode.to_string();
            if let Some(acceptor) = tls {
                match acceptor.accept(stream).await {
                    Ok(tls_stream) => {
                        let (reader, writer) = tokio::io::split(tls_stream);
                        handle_agent_stream(reader, writer, state, &label).await;
                    }
                    Err(e) => eprintln!("Agent TLS handshake failed: {e}"),
                }
            } else {
                let (reader, writer) = stream.into_split();
                handle_agent_stream(reader, writer, state, &label).await;
            }
            active.fetch_sub(1, Ordering::Relaxed);
        });
    }
}

pub(crate) fn agent_config_payload(
    settings: &std::collections::HashMap<String, String>,
    pve_token_override: Option<&str>,
    activity_mode: &str,
    linked_container_names: &[String],
    node_tag: &str,
) -> serde_json::Value {
    let token = pve_token_override
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .or_else(|| settings.get("pve_api_token").map(String::as_str))
        .unwrap_or("");
    let configured = !token.is_empty();
    let enable_proxmox = settings
        .get("enable_proxmox")
        .map(|s| s.as_str())
        .unwrap_or("1");

    let (tel_interval, lxc_interval, docker_interval) = if activity_mode == "active" {
        (
            settings
                .get("agent_telemetry_interval_secs")
                .cloned()
                .unwrap_or_else(|| "5".into()),
            settings
                .get("agent_lxc_poll_interval_secs")
                .cloned()
                .unwrap_or_else(|| "10".into()),
            settings
                .get("agent_docker_poll_interval_secs")
                .cloned()
                .unwrap_or_else(|| "10".into()),
        )
    } else {
        ("60".into(), "300".into(), "300".into())
    };

    let tag = if node_tag.trim().is_empty() {
        settings
            .get("agent_node_tag")
            .cloned()
            .unwrap_or_else(|| "Local".into())
    } else {
        node_tag.to_string()
    };

    serde_json::json!({
        "config": {
            "pve_api_token_configured": configured,
            "pve_api_token": token,
            "enable_proxmox": enable_proxmox,
            "activity_mode": activity_mode,
            "linked_container_names": linked_container_names,
            "telemetry_external_ifaces": settings.get("telemetry_external_ifaces").cloned().unwrap_or_default(),
            "telemetry_internal_ifaces": settings.get("telemetry_internal_ifaces").cloned().unwrap_or_default(),
            "telemetry_disk_mounts": settings.get("telemetry_disk_mounts").cloned().unwrap_or_default(),
            "agent_node_tag": tag,
            "agent_telemetry_interval_secs": tel_interval,
            "agent_lxc_poll_interval_secs": lxc_interval,
            "agent_docker_poll_interval_secs": docker_interval,
        }
    })
}

pub(crate) fn agent_config_for_node(
    state: &AppState,
    node_tag: &str,
    pve_token_override: Option<&str>,
) -> serde_json::Value {
    let cache = state.settings_cache.read().unwrap();
    let mode = crate::activity::activity_mode_name(
        state
            .activity_mode
            .load(std::sync::atomic::Ordering::Relaxed),
    );
    let tag = if node_tag.trim().is_empty() {
        cache
            .get("agent_node_tag")
            .map(|s| s.as_str())
            .unwrap_or("Local")
    } else {
        node_tag.trim()
    };
    let linked = {
        let db = state.db.lock().unwrap();
        crate::db::load_linked_container_names(&db, tag)
    };
    agent_config_payload(&cache, pve_token_override, mode, &linked, tag)
}

#[allow(dead_code)]
pub(crate) fn agent_config_for_state(
    state: &AppState,
    pve_token_override: Option<&str>,
) -> serde_json::Value {
    let default_tag = {
        let cache = state.settings_cache.read().unwrap();
        cache
            .get("agent_node_tag")
            .cloned()
            .unwrap_or_else(|| "Local".into())
    };
    agent_config_for_node(state, &default_tag, pve_token_override)
}

#[allow(dead_code)]
pub(crate) fn pve_config_payload(token: &str) -> serde_json::Value {
    let mut settings = std::collections::HashMap::new();
    settings.insert("pve_api_token".to_string(), token.to_string());
    agent_config_payload(&settings, Some(token), "active", &[], "Local")
}

pub(crate) fn push_agent_config(state: &Arc<AppState>, pve_token_override: Option<&str>) {
    let sessions: Vec<(String, AgentCommandHandle)> = state
        .agent_sessions
        .lock()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (tag, handle) in sessions {
        let payload = agent_config_for_node(state, &tag, pve_token_override);
        if let Ok(mut serialized) = serde_json::to_vec(&payload) {
            serialized.push(b'\n');
            let _ = handle
                .tx
                .send(String::from_utf8_lossy(&serialized).into_owned());
        }
    }
}

pub(crate) fn process_agent_line(
    state: &Arc<AppState>,
    tx: &tokio::sync::mpsc::UnboundedSender<String>,
    line: &str,
    node_tag: Option<&str>,
) {
    #[derive(Deserialize)]
    struct PveTestMsg {
        test_pve_result: PveTestResult,
    }

    if let Ok(req) = serde_json::from_str::<amud_protocol::ConfigRequest>(line) {
        if req.request == "get_config" {
            let env_configured = req.pve_token_configured.unwrap_or(false);
            let token_override = if env_configured { Some("") } else { None };
            let tag = node_tag.unwrap_or("Local");
            let mut config_payload = agent_config_for_node(state, tag, token_override);
            if env_configured {
                if let Some(config) = config_payload.get_mut("config") {
                    config["pve_api_token_configured"] = serde_json::json!(true);
                }
            }
            if let Ok(mut serialized) = serde_json::to_vec(&config_payload) {
                serialized.push(b'\n');
                let _ = tx.send(String::from_utf8_lossy(&serialized).into_owned());
            }
        }
    } else if let Ok(msg) = serde_json::from_str::<ActionResultMsg>(line) {
        state.action_results.write().unwrap().insert(
            msg.action_result.request_id,
            ActionResult {
                success: msg.action_result.success,
                error: msg.action_result.error,
                at: Instant::now(),
            },
        );
    } else if let Ok(msg) = serde_json::from_str::<PveTestMsg>(line) {
        *state.pve_test_response.write().unwrap() = Some(msg.test_pve_result);
    } else if line.contains("discover_docker_result") {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            *state.docker_discover_response.write().unwrap() = Some(val);
        }
    } else if line.contains("telemetry_discover_result") {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
            *state.telemetry_discover_response.write().unwrap() = Some(val);
        }
    } else if let Ok(metrics) = serde_json::from_str::<AgentTelemetry>(line) {
        handle_new_telemetry(state, metrics);
    }
}
