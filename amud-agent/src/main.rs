#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

mod http_clients;

use amud_protocol::{
    agent_auth_proof, AgentHelloMessage, AgentHelloPayload, AuthProofMessage, ChallengeMessage,
    ConfigRequest, DiskMountTelemetry, LxcContainer, NetworkTelemetry,
};
use serde::Serialize;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::thread::sleep;
use std::time::{Duration, Instant};
use sysinfo::System;

const HTTP_TIMEOUT: Duration = Duration::from_secs(10);

async fn with_http_timeout<T, F>(future: F) -> Result<T, String>
where
    F: std::future::Future<Output = Result<T, String>>,
{
    tokio::time::timeout(HTTP_TIMEOUT, future)
        .await
        .map_err(|_| "Request timed out".to_string())?
}

#[derive(Clone, Default)]
struct GpuSnapshot {
    name: String,
    usage: i32,
    mem_usage: i32,
    mem_used_mb: f64,
    mem_total_mb: f64,
}

fn read_gpu_snapshot() -> GpuSnapshot {
    use std::process::Command;

    let output = match Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,utilization.gpu,utilization.memory,memory.used,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output()
    {
        Ok(output) if output.status.success() => output,
        _ => return GpuSnapshot::default(),
    };

    let line = String::from_utf8_lossy(&output.stdout);
    let line = line.lines().next().unwrap_or("").trim();
    if line.is_empty() {
        return GpuSnapshot::default();
    }

    // GPU name may contain commas; numeric fields are always last.
    let mut parts = line.rsplitn(5, ',');
    let mem_total = parts
        .next()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.0);
    let mem_used = parts
        .next()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(0.0);
    let mem_usage = parts
        .next()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.round() as i32)
        .unwrap_or(-1);
    let usage = parts
        .next()
        .and_then(|v| v.trim().parse::<f64>().ok())
        .map(|v| v.round() as i32)
        .unwrap_or(-1);
    let name = parts.next().unwrap_or("").trim().to_string();

    if name.is_empty() || usage < 0 {
        return GpuSnapshot::default();
    }

    GpuSnapshot {
        name,
        usage,
        mem_usage,
        mem_used_mb: mem_used,
        mem_total_mb: mem_total,
    }
}

#[derive(Clone, Default)]
struct NetworkSnapshot {
    internal_rx: u64,
    internal_tx: u64,
    external_rx: u64,
    external_tx: u64,
}

#[derive(Clone, Default)]
struct TelemetryConfig {
    external_ifaces: Vec<String>,
    internal_ifaces: Vec<String>,
    disk_mounts: Vec<String>,
    node_tag: String,
}

#[derive(Clone)]
struct AgentPollConfig {
    enable_proxmox: bool,
    activity_mode: String,
    linked_container_names: Vec<String>,
    telemetry_interval_secs: u64,
    lxc_poll_interval_secs: u64,
    docker_poll_interval_secs: u64,
}

fn is_idle_mode(mode: &str) -> bool {
    mode == "idle"
}

fn filter_containers_by_linked(
    containers: Vec<LxcContainer>,
    linked: &[String],
) -> Vec<LxcContainer> {
    if linked.is_empty() {
        return containers;
    }
    let linked_lower: std::collections::HashSet<String> = linked
        .iter()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect();
    containers
        .into_iter()
        .filter(|c| linked_lower.contains(&c.name.to_ascii_lowercase()))
        .collect()
}

fn merge_container_caches(
    lxc: &Arc<Vec<LxcContainer>>,
    docker: &Arc<Vec<LxcContainer>>,
) -> Arc<Vec<LxcContainer>> {
    let mut merged = Vec::with_capacity(lxc.len().saturating_add(docker.len()));
    merged.extend(lxc.iter().cloned());
    merged.extend(docker.iter().cloned());
    Arc::new(merged)
}

impl Default for AgentPollConfig {
    fn default() -> Self {
        Self {
            enable_proxmox: true,
            activity_mode: "active".to_string(),
            linked_container_names: Vec::new(),
            telemetry_interval_secs: 5,
            lxc_poll_interval_secs: 10,
            docker_poll_interval_secs: 10,
        }
    }
}

fn telemetry_config() -> &'static RwLock<TelemetryConfig> {
    static CONFIG: OnceLock<RwLock<TelemetryConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(TelemetryConfig::default()))
}

fn agent_poll_config() -> &'static RwLock<AgentPollConfig> {
    static CONFIG: OnceLock<RwLock<AgentPollConfig>> = OnceLock::new();
    CONFIG.get_or_init(|| RwLock::new(AgentPollConfig::default()))
}

fn parse_config_u64(value: Option<&serde_json::Value>, default: u64, min: u64, max: u64) -> u64 {
    let v = value
        .and_then(|v| {
            v.as_u64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .unwrap_or(default);
    v.clamp(min, max)
}

struct GpuCache {
    probed: bool,
    available: bool,
    last_read: Instant,
    snapshot: GpuSnapshot,
}

impl Default for GpuCache {
    fn default() -> Self {
        Self {
            probed: false,
            available: false,
            last_read: Instant::now()
                .checked_sub(Duration::from_secs(60))
                .unwrap_or_else(Instant::now),
            snapshot: GpuSnapshot::default(),
        }
    }
}

fn gpu_snapshot_cached(cache: &mut GpuCache) -> GpuSnapshot {
    if cache.probed && !cache.available {
        return GpuSnapshot::default();
    }
    if cache.probed && cache.available && cache.last_read.elapsed() < Duration::from_secs(5) {
        return cache.snapshot.clone();
    }
    let snap = read_gpu_snapshot();
    if !cache.probed {
        cache.probed = true;
        cache.available = !snap.name.is_empty();
    }
    if snap.name.is_empty() {
        cache.available = false;
        return GpuSnapshot::default();
    }
    cache.snapshot = snap.clone();
    cache.last_read = Instant::now();
    snap
}

static CONFIG_READY: AtomicBool = AtomicBool::new(false);

#[derive(Default)]
struct NetworkBaseline {
    snapshot: NetworkSnapshot,
    sample_at: Option<Instant>,
}

fn network_baseline() -> &'static Mutex<NetworkBaseline> {
    static BASELINE: OnceLock<Mutex<NetworkBaseline>> = OnceLock::new();
    BASELINE.get_or_init(|| Mutex::new(NetworkBaseline::default()))
}

fn agent_node_tag() -> String {
    if let Ok(tag) = std::env::var("AMUD_NODE_TAG") {
        let t = tag.trim();
        if !t.is_empty() {
            return t.to_string();
        }
    }
    let cfg = telemetry_config().read().unwrap();
    if cfg.node_tag.trim().is_empty() {
        "Local".to_string()
    } else {
        cfg.node_tag.clone()
    }
}

fn agent_platform_label() -> String {
    std::env::var("AMUD_PLATFORM")
        .ok()
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| matches!(s.as_str(), "proxmox" | "unraid" | "casaos" | "docker" | "other"))
        .unwrap_or_default()
}

fn agent_capabilities() -> Vec<String> {
    let mut caps = vec!["host".to_string()];
    #[cfg(unix)]
    if docker_enabled() {
        caps.push("docker".to_string());
    }
    if !get_pve_api_token().is_empty() || pve_token_from_env().is_some() {
        caps.push("proxmox".to_string());
    } else if std::env::var("AMUD_PLATFORM")
        .map(|s| s.eq_ignore_ascii_case("proxmox"))
        .unwrap_or(false)
    {
        caps.push("proxmox".to_string());
    }
    caps
}

fn reset_network_baseline() {
    let mut baseline = network_baseline().lock().unwrap();
    baseline.snapshot = NetworkSnapshot::default();
    baseline.sample_at = None;
}

fn parse_iface_list(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|s| s.trim().to_ascii_lowercase())
        .filter(|s| !s.is_empty())
        .collect()
}

fn parse_mount_list(raw: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.split(',')
        .map(str::trim)
        .filter(|s| s.starts_with('/') && !s.contains(".."))
        .map(|s| {
            let mut v = s.to_string();
            while v.ends_with('/') && v.len() > 1 {
                v.pop();
            }
            v
        })
        .filter(|s| seen.insert(s.clone()))
        .collect()
}

fn apply_telemetry_config(config: &serde_json::Value) {
    {
        let mut tc = telemetry_config().write().unwrap();
        tc.external_ifaces = config
            .get("telemetry_external_ifaces")
            .and_then(|v| v.as_str())
            .map(parse_iface_list)
            .unwrap_or_default();
        tc.internal_ifaces = config
            .get("telemetry_internal_ifaces")
            .and_then(|v| v.as_str())
            .map(parse_iface_list)
            .unwrap_or_default();
        tc.disk_mounts = config
            .get("telemetry_disk_mounts")
            .and_then(|v| v.as_str())
            .map(parse_mount_list)
            .unwrap_or_default();
        tc.node_tag = config
            .get("agent_node_tag")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
            .unwrap_or("Local")
            .to_string();
    }
    {
        let mut pc = agent_poll_config().write().unwrap();
        pc.enable_proxmox = config
            .get("enable_proxmox")
            .and_then(|v| v.as_str())
            .map(|s| s == "1")
            .unwrap_or(true);
        pc.activity_mode = config
            .get("activity_mode")
            .and_then(|v| v.as_str())
            .unwrap_or("active")
            .to_string();
        pc.linked_container_names = config
            .get("linked_container_names")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(str::trim).filter(|s| !s.is_empty()))
                    .map(str::to_string)
                    .collect()
            })
            .unwrap_or_default();
        pc.telemetry_interval_secs =
            parse_config_u64(config.get("agent_telemetry_interval_secs"), 5, 3, 60);
        pc.lxc_poll_interval_secs =
            parse_config_u64(config.get("agent_lxc_poll_interval_secs"), 10, 5, 120);
        pc.docker_poll_interval_secs =
            parse_config_u64(config.get("agent_docker_poll_interval_secs"), 10, 5, 120);
    }
    CONFIG_READY.store(true, Ordering::Release);
    reset_network_baseline();
}

fn iface_classify_auto(iface: &str) -> &'static str {
    if iface.starts_with("vmbr") || iface.starts_with("br") || iface.starts_with("docker") {
        "internal"
    } else {
        "external"
    }
}

fn iface_classify(iface: &str, cfg: &TelemetryConfig) -> Option<&'static str> {
    let iface_lower = iface.to_ascii_lowercase();
    if !cfg.external_ifaces.is_empty() && cfg.external_ifaces.contains(&iface_lower) {
        return Some("external");
    }
    if !cfg.internal_ifaces.is_empty() && cfg.internal_ifaces.contains(&iface_lower) {
        return Some("internal");
    }
    if !cfg.external_ifaces.is_empty() || !cfg.internal_ifaces.is_empty() {
        return None;
    }
    Some(iface_classify_auto(iface))
}

fn mount_matches(path: &str, cfg: &TelemetryConfig) -> bool {
    if cfg.disk_mounts.is_empty() {
        return true;
    }
    let path_norm = {
        let mut p = path.to_string();
        while p.ends_with('/') && p.len() > 1 {
            p.pop();
        }
        p.to_ascii_lowercase()
    };
    cfg.disk_mounts.iter().any(|m| {
        let mount = m.trim_end_matches('/').to_ascii_lowercase();
        path_norm == mount || path_norm.starts_with(&format!("{mount}/"))
    })
}

fn is_skipped_disk_fs(fs: &str) -> bool {
    matches!(
        fs,
        "tmpfs"
            | "overlay"
            | "squashfs"
            | "devtmpfs"
            | "sysfs"
            | "proc"
            | "devpts"
            | "cgroup"
            | "cgroup2"
            | "none"
            | "ramfs"
    )
}

fn aggregate_disk_bytes(disks: &sysinfo::Disks, cfg: &TelemetryConfig) -> (u64, u64, u32) {
    let mut total_disk: u64 = 0;
    let mut avail_disk: u64 = 0;
    let mut matched_mounts = 0u32;
    let mut seen_devices = std::collections::HashSet::new();
    for disk in disks {
        let fs = disk.file_system().to_string_lossy().to_lowercase();
        let mount = disk.mount_point().to_string_lossy().to_string();
        if !mount_matches(&mount, cfg) {
            continue;
        }
        if is_skipped_disk_fs(&fs) {
            continue;
        }
        if mount.starts_with("/snap") || mount.starts_with("/dev/loop") {
            continue;
        }
        let device_name = disk.name().to_string_lossy().to_string();
        if !seen_devices.insert(device_name) {
            continue;
        }
        matched_mounts += 1;
        total_disk += disk.total_space();
        avail_disk += disk.available_space();
    }
    (total_disk, avail_disk, matched_mounts)
}

fn configured_mounts_satisfied_paths(visible_mounts: &[&str], cfg: &TelemetryConfig) -> bool {
    if cfg.disk_mounts.is_empty() {
        return true;
    }
    cfg.disk_mounts.iter().all(|mount| {
        let single = TelemetryConfig {
            disk_mounts: vec![mount.clone()],
            ..Default::default()
        };
        visible_mounts
            .iter()
            .any(|path| mount_matches(path, &single))
    })
}

fn configured_mounts_satisfied(disks: &sysinfo::Disks, cfg: &TelemetryConfig) -> bool {
    let visible: Vec<String> = disks
        .iter()
        .map(|d| d.mount_point().to_string_lossy().to_string())
        .collect();
    let visible_refs: Vec<&str> = visible.iter().map(String::as_str).collect();
    configured_mounts_satisfied_paths(&visible_refs, cfg)
}

fn proc_net_dev_path() -> String {
    std::env::var("AMUD_PROC_NET_DEV").unwrap_or_else(|_| "/proc/net/dev".to_string())
}

fn read_proc_net_dev_content() -> Option<String> {
    std::fs::read_to_string(proc_net_dev_path()).ok()
}

fn list_visible_mounts(disks: &sysinfo::Disks) -> Vec<String> {
    disks
        .iter()
        .map(|d| d.mount_point().to_string_lossy().to_string())
        .collect()
}

fn mount_label(mount: &str) -> String {
    mount
        .trim_end_matches('/')
        .rsplit('/')
        .next()
        .unwrap_or("disk")
        .to_string()
}

fn aggregate_disk_per_mount(
    disks: &sysinfo::Disks,
    cfg: &TelemetryConfig,
) -> Vec<DiskMountTelemetry> {
    let mut volumes = Vec::new();
    for mount_cfg in &cfg.disk_mounts {
        let single = TelemetryConfig {
            disk_mounts: vec![mount_cfg.clone()],
            ..cfg.clone()
        };
        let (total, avail, _) = aggregate_disk_bytes(disks, &single);
        if total == 0 {
            continue;
        }
        let used = total.saturating_sub(avail);
        let usage = ((used as f64 / total as f64) * 100.0).round() as i32;
        volumes.push(DiskMountTelemetry {
            mount: mount_cfg.clone(),
            label: mount_label(mount_cfg),
            usage,
            used_gb: (used as f64 / 1_073_741_824.0 * 100.0).round() / 100.0,
            total_gb: (total as f64 / 1_073_741_824.0 * 100.0).round() / 100.0,
        });
    }
    volumes
}

fn detect_telemetry_scope(cfg: &TelemetryConfig, _ifaces: &[String], mounts: &[&str]) -> String {
    if !std::path::Path::new("/.dockerenv").exists() {
        return "host".to_string();
    }
    if let Some(content) = read_proc_net_dev_content() {
        let present = list_proc_net_ifaces(&content);
        if !cfg.internal_ifaces.is_empty()
            && !cfg
                .internal_ifaces
                .iter()
                .any(|iface| present.contains(iface))
        {
            return "container".to_string();
        }
    }
    if !cfg.disk_mounts.is_empty() && !configured_mounts_satisfied_paths(mounts, cfg) {
        return "container".to_string();
    }
    "host".to_string()
}

fn telemetry_discover_payload() -> serde_json::Value {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mounts = list_visible_mounts(&disks);
    let mount_refs: Vec<&str> = mounts.iter().map(String::as_str).collect();
    let ifaces: Vec<String> = read_proc_net_dev_content()
        .map(|c| list_proc_net_ifaces(&c).into_iter().collect())
        .unwrap_or_default();
    let cfg = telemetry_config().read().unwrap().clone();
    serde_json::json!({
        "ifaces": ifaces,
        "mounts": mounts,
        "scope": detect_telemetry_scope(&cfg, &ifaces, &mount_refs),
    })
}

fn snapshot_external_delta(previous: &NetworkSnapshot, current: &NetworkSnapshot) -> u64 {
    current
        .external_rx
        .saturating_sub(previous.external_rx)
        .saturating_add(current.external_tx.saturating_sub(previous.external_tx))
}

struct NetworkHeuristicState {
    zero_external_ticks: u32,
    bond_warn_logged: bool,
}

fn network_heuristic_state() -> &'static Mutex<NetworkHeuristicState> {
    static STATE: OnceLock<Mutex<NetworkHeuristicState>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(NetworkHeuristicState {
            zero_external_ticks: 0,
            bond_warn_logged: false,
        })
    })
}

fn build_network_snapshot(content: &str, cfg: &TelemetryConfig) -> (NetworkSnapshot, bool) {
    let custom = !cfg.external_ifaces.is_empty() || !cfg.internal_ifaces.is_empty();
    if custom && !configured_network_ifaces_satisfied(content, cfg) {
        return (
            collect_network_snapshot(content, &TelemetryConfig::default()).0,
            true,
        );
    }
    let (snapshot, matched) = collect_network_snapshot(content, cfg);
    if custom && matched == 0 {
        return (
            collect_network_snapshot(content, &TelemetryConfig::default()).0,
            true,
        );
    }
    (snapshot, false)
}

fn main() {
    println!("AMUD-Agent telemetry client starting up...");
    let _ = rustls::crypto::ring::default_provider().install_default();

    if std::env::var("AMUD_AGENT_SECRET")
        .unwrap_or_default()
        .trim()
        .is_empty()
    {
        eprintln!("FATAL: AMUD_AGENT_SECRET is not set.");
        std::process::exit(1);
    }

    loop {
        println!("Attempting to connect to AMUD dashboard daemon...");
        match establish_connection() {
            Ok(stream) => {
                println!("Connected successfully. Beginning telemetry stream.");
                if let Err(e) = run_telemetry_loop(stream) {
                    eprintln!("Telemetry stream error occurred: {}", e);
                }
            }
            Err(e) => {
                eprintln!(
                    "Failed to connect to dashboard daemon: {}. Retrying in 5 seconds...",
                    e
                );
            }
        }
        sleep(Duration::from_secs(5));
    }
}

struct SharedTlsStream {
    inner: Arc<Mutex<rustls::StreamOwned<rustls::ClientConnection, std::net::TcpStream>>>,
}

impl SharedTlsStream {
    fn try_clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

impl Read for SharedTlsStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        self.inner.lock().unwrap().read(buf)
    }
}

impl Write for SharedTlsStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.inner.lock().unwrap().write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.inner.lock().unwrap().flush()
    }
}

enum AgentIpcStream {
    #[cfg(unix)]
    Unix(std::os::unix::net::UnixStream),
    Tcp(std::net::TcpStream),
    Tls(SharedTlsStream),
}

impl AgentIpcStream {
    fn try_clone(&self) -> std::io::Result<Self> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => Ok(Self::Unix(s.try_clone()?)),
            Self::Tcp(s) => Ok(Self::Tcp(s.try_clone()?)),
            Self::Tls(s) => Ok(Self::Tls(s.try_clone())),
        }
    }
}

impl Read for AgentIpcStream {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.read(buf),
            Self::Tcp(s) => s.read(buf),
            Self::Tls(s) => s.read(buf),
        }
    }
}

impl Write for AgentIpcStream {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.write(buf),
            Self::Tcp(s) => s.write(buf),
            Self::Tls(s) => s.write(buf),
        }
    }
    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            Self::Unix(s) => s.flush(),
            Self::Tcp(s) => s.flush(),
            Self::Tls(s) => s.flush(),
        }
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

fn wrap_tcp_maybe_tls(tcp: std::net::TcpStream, server_name: &str) -> std::io::Result<AgentIpcStream> {
    if !agent_tls_enabled() {
        return Ok(AgentIpcStream::Tcp(tcp));
    }

    let mut root_store = rustls::RootCertStore::empty();
    let insecure = matches!(
        std::env::var("AMUD_AGENT_TLS_INSECURE")
            .ok()
            .as_deref()
            .map(str::trim)
            .map(str::to_ascii_lowercase)
            .as_deref(),
        Some("1") | Some("true") | Some("yes")
    );

    if let Ok(ca_path) = std::env::var("AMUD_AGENT_TLS_CA") {
        let ca_bytes = std::fs::read(ca_path.trim())?;
        let mut reader = std::io::Cursor::new(ca_bytes);
        for cert in rustls_pemfile::certs(&mut reader).flatten() {
            let _ = root_store.add(cert);
        }
    } else if !insecure {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "AMUD_AGENT_TLS=1 requires AMUD_AGENT_TLS_CA (or AMUD_AGENT_TLS_INSECURE=1 for lab only)",
        ));
    }

    let config = if insecure {
        eprintln!(
            "WARNING: AMUD_AGENT_TLS_INSECURE=1 — TLS certificate verification disabled (lab only)."
        );
        rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(NoCertVerifier))
            .with_no_client_auth()
    } else {
        rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth()
    };

    let name = rustls::pki_types::ServerName::try_from(server_name.to_string()).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            format!("Invalid TLS server name '{server_name}': {e}"),
        )
    })?;
    let conn = rustls::ClientConnection::new(Arc::new(config), name).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("TLS client setup failed: {e}"),
        )
    })?;
    let tls = rustls::StreamOwned::new(conn, tcp);
    Ok(AgentIpcStream::Tls(SharedTlsStream {
        inner: Arc::new(Mutex::new(tls)),
    }))
}

#[derive(Debug)]
struct NoCertVerifier;

impl rustls::client::danger::ServerCertVerifier for NoCertVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

fn establish_connection() -> Result<AgentIpcStream, std::io::Error> {
    // Remote TCP (any OS): AMUD_SERVER_ADDR=host:port
    if let Ok(addr) = std::env::var("AMUD_SERVER_ADDR") {
        let addr = addr.trim().to_string();
        if !addr.is_empty() {
            let tag = std::env::var("AMUD_NODE_TAG").unwrap_or_default();
            if tag.trim().is_empty() {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "AMUD_NODE_TAG is required when using AMUD_SERVER_ADDR (remote agent)",
                ));
            }
            println!("Connecting via TCP to {addr}");
            let tcp = std::net::TcpStream::connect(&addr)?;
            let host = addr
                .rsplit_once(':')
                .map(|(h, _)| h)
                .unwrap_or(addr.as_str())
                .trim_start_matches('[')
                .trim_end_matches(']');
            return wrap_tcp_maybe_tls(tcp, host);
        }
    }

    #[cfg(unix)]
    {
        let path = std::env::var("AMUD_SOCKET_PATH")
            .unwrap_or_else(|_| "/opt/amud/run/amud.sock".to_string());
        println!("Connecting via UDS to {path}");
        let sock = std::os::unix::net::UnixStream::connect(&path)?;
        Ok(AgentIpcStream::Unix(sock))
    }

    #[cfg(windows)]
    {
        let addr = std::env::var("AMUD_TCP_ADDR").unwrap_or_else(|_| "127.0.0.1:8050".to_string());
        println!("Connecting via TCP to {addr}");
        let tcp = std::net::TcpStream::connect(&addr)?;
        let host = addr
            .rsplit_once(':')
            .map(|(h, _)| h)
            .unwrap_or("localhost");
        wrap_tcp_maybe_tls(tcp, host)
    }

    #[cfg(not(any(unix, windows)))]
    {
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Unsupported platform for AMUD agent IPC",
        ))
    }
}

static AGENT_RT: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

fn agent_runtime() -> &'static tokio::runtime::Runtime {
    AGENT_RT.get_or_init(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("failed to build agent tokio runtime")
    })
}

fn pve_token_from_env() -> Option<String> {
    std::env::var("PVE_API_TOKEN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

#[cfg(unix)]
fn docker_enabled() -> bool {
    match std::env::var("AMUD_DOCKER")
        .ok()
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("0") | Some("false") | Some("no") => false,
        Some("1") | Some("true") | Some("yes") => true,
        // When the socket is mounted (standard Docker Compose), enable monitoring and
        // container controls without requiring users to set AMUD_DOCKER manually.
        _ => std::path::Path::new("/var/run/docker.sock").exists(),
    }
}

fn pve_node_name() -> String {
    std::env::var("PVE_NODE")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| {
            std::fs::read_to_string("/etc/hostname")
                .unwrap_or_else(|_| "localhost".to_string())
                .trim()
                .to_string()
        })
}

static PVE_API_TOKEN_CACHE: OnceLock<RwLock<String>> = OnceLock::new();

fn get_pve_api_token() -> String {
    if let Some(env) = pve_token_from_env() {
        return env;
    }
    if let Some(lock) = PVE_API_TOKEN_CACHE.get() {
        let val = lock.read().unwrap().clone();
        if !val.is_empty() {
            return val;
        }
    }
    String::new()
}

fn update_pve_api_token(token: &str) {
    if pve_token_from_env().is_some() {
        return;
    }
    let lock = PVE_API_TOKEN_CACHE.get_or_init(|| RwLock::new(String::new()));
    let mut w = lock.write().unwrap();
    *w = token.to_string();
}

#[derive(Serialize, serde::Deserialize)]
struct PveTestResult {
    success: bool,
    error: Option<String>,
}

fn perform_pve_test(token: &str) -> PveTestResult {
    if token.is_empty() {
        return PveTestResult {
            success: false,
            error: Some("Token is empty".to_string()),
        };
    }
    match fetch_lxc_containers_with_token(token) {
        Ok(_) => PveTestResult {
            success: true,
            error: None,
        },
        Err(e) => PveTestResult {
            success: false,
            error: Some(e),
        },
    }
}

// Native Proxmox LXC fetch over HTTPS (replaces the `pvesh` subprocess fork).
// Reads PVE_API_TOKEN and queries the local PVE API.
// Ok(empty) = intentional empty (Proxmox disabled, or API returned zero CTs).
// Err = transient/API failure — caller must retain last-known cache.
fn fetch_lxc_containers() -> Result<Vec<LxcContainer>, String> {
    if !agent_poll_config().read().unwrap().enable_proxmox {
        return Ok(Vec::new());
    }
    let token = get_pve_api_token();
    if token.is_empty() {
        return Err("PVE_API_TOKEN not set or empty".to_string());
    }
    fetch_lxc_containers_with_token(&token)
}

fn fetch_lxc_containers_with_token(token: &str) -> Result<Vec<LxcContainer>, String> {
    let token = token.to_string();
    agent_runtime().block_on(async move {
        with_http_timeout(async move {
            use http_body_util::{BodyExt, Empty};
            use hyper::body::Bytes;

            let client = http_clients::pve_https_client();

            let node_name = pve_node_name();

            let api_url = format!("https://localhost:8006/api2/json/nodes/{}/lxc", node_name);
            eprintln!("[LXC] Fetching containers from: {}...", api_url);

            let req = hyper::Request::builder()
                .method("GET")
                .uri(&api_url)
                .header("Authorization", token)
                .body(Empty::<Bytes>::new())
                .map_err(|e| format!("Failed to build HTTP request: {}", e))?;

            let resp = client
                .request(req)
                .await
                .map_err(|e| format!("HTTP request to PVE API failed: {}", e))?;

            let status = resp.status();
            let body = resp
                .into_body()
                .collect()
                .await
                .map_err(|e| format!("Failed to read response body: {}", e))?
                .to_bytes();

            if !status.is_success() {
                let body_str = String::from_utf8_lossy(&body);
                let snippet = body_str.chars().take(200).collect::<String>();
                return Err(format!("PVE API returned HTTP {}: {}", status, snippet));
            }

            // The PVE REST API wraps the array in a `{ "data": [...] }` envelope,
            // unlike the bare array that `pvesh --output-format json` returned.
            #[derive(serde::Deserialize)]
            struct PveResponse {
                data: Vec<LxcContainer>,
            }

            match serde_json::from_slice::<PveResponse>(&body) {
                Ok(parsed) => {
                    eprintln!(
                        "[LXC] Successfully fetched {} containers from PVE.",
                        parsed.data.len()
                    );
                    Ok(parsed.data)
                }
                Err(e) => {
                    let body_str = String::from_utf8_lossy(&body);
                    let snippet = body_str.chars().take(200).collect::<String>();
                    Err(format!(
                        "Failed to parse PVE response: {}. Body snippet: {}",
                        e, snippet
                    ))
                }
            }
        })
        .await
    })
}

fn format_rate(bytes_per_sec: u64) -> String {
    let bits = bytes_per_sec as f64 * 8.0;
    if bits >= 1_000_000_000.0 {
        format!("{:.2} Gbit/s", bits / 1_000_000_000.0)
    } else if bits >= 1_000_000.0 {
        format!("{:.1} Mbit/s", bits / 1_000_000.0)
    } else {
        format!("{:.0} kbit/s", bits / 1_000.0)
    }
}

fn list_proc_net_ifaces(content: &str) -> std::collections::HashSet<String> {
    let mut present = std::collections::HashSet::new();
    for line in content.lines().skip(2) {
        let Some((iface, _)) = line.split_once(':') else {
            continue;
        };
        let iface = iface.trim();
        if iface != "lo" && !iface.is_empty() {
            present.insert(iface.to_ascii_lowercase());
        }
    }
    present
}

fn configured_network_ifaces_satisfied(content: &str, cfg: &TelemetryConfig) -> bool {
    let present = list_proc_net_ifaces(content);
    if !cfg.external_ifaces.is_empty()
        && !cfg
            .external_ifaces
            .iter()
            .any(|iface| present.contains(iface))
    {
        return false;
    }
    if !cfg.internal_ifaces.is_empty()
        && !cfg
            .internal_ifaces
            .iter()
            .any(|iface| present.contains(iface))
    {
        return false;
    }
    true
}

fn collect_network_snapshot(content: &str, cfg: &TelemetryConfig) -> (NetworkSnapshot, u32) {
    let mut snapshot = NetworkSnapshot::default();
    let mut matched = 0u32;

    for line in content.lines().skip(2) {
        let Some((iface, stats)) = line.split_once(':') else {
            continue;
        };
        let iface = iface.trim();
        if iface == "lo" {
            continue;
        }
        let values: Vec<u64> = stats
            .split_whitespace()
            .filter_map(|v| v.parse::<u64>().ok())
            .collect();
        if values.len() < 16 {
            continue;
        }

        let rx = values[0];
        let tx = values[8];
        match iface_classify(iface, cfg) {
            Some("internal") => {
                matched += 1;
                snapshot.internal_rx = snapshot.internal_rx.saturating_add(rx);
                snapshot.internal_tx = snapshot.internal_tx.saturating_add(tx);
            }
            Some("external") => {
                matched += 1;
                snapshot.external_rx = snapshot.external_rx.saturating_add(rx);
                snapshot.external_tx = snapshot.external_tx.saturating_add(tx);
            }
            _ => {}
        }
    }

    (snapshot, matched)
}

fn network_rates(
    previous: &NetworkSnapshot,
    current: &NetworkSnapshot,
    elapsed: f64,
) -> NetworkTelemetry {
    let seconds = elapsed.max(1.0);
    let rate = |now: u64, before: u64| -> u64 {
        ((now.saturating_sub(before)) as f64 / seconds).round() as u64
    };

    NetworkTelemetry {
        internal_tx: format_rate(rate(current.internal_tx, previous.internal_tx)),
        internal_rx: format_rate(rate(current.internal_rx, previous.internal_rx)),
        external_tx: format_rate(rate(current.external_tx, previous.external_tx)),
        external_rx: format_rate(rate(current.external_rx, previous.external_rx)),
    }
}

// Native Docker fetch over the Engine API UNIX socket (replaces the `curl` fork).
// Ok(empty) = Docker disabled or truly no containers.
// Err = socket/API failure — caller must retain last-known cache.
#[cfg(unix)]
fn fetch_docker_containers() -> Result<Vec<LxcContainer>, String> {
    if !docker_enabled() {
        return Ok(Vec::new());
    }
    if !std::path::Path::new("/var/run/docker.sock").exists() {
        return Err("docker.sock not available".to_string());
    }

    let rt = agent_runtime();

    rt.block_on(async move {
        use http_body_util::{BodyExt, Empty};
        use hyper::body::Bytes;
        use hyper_util::client::legacy::Client;
        use hyperlocal::{UnixConnector, Uri as UnixUri};

        let client = http_clients::docker_unix_client();

        let uri: hyper::Uri = UnixUri::new("/var/run/docker.sock", "/containers/json").into();
        let req = match hyper::Request::builder()
            .method("GET")
            .uri(uri)
            .header("Host", "localhost")
            .body(Empty::<Bytes>::new())
        {
            Ok(r) => r,
            Err(e) => return Err(format!("Failed to build Docker request: {}", e)),
        };

        let resp = match client.request(req).await {
            Ok(r) => r,
            Err(e) => return Err(format!("Docker Engine API request failed: {}", e)),
        };

        let body = match resp.into_body().collect().await {
            Ok(b) => b.to_bytes(),
            Err(e) => return Err(format!("Failed to read Docker response: {}", e)),
        };

        #[derive(serde::Deserialize)]
        struct DockerContainer {
            #[serde(rename = "Id")]
            id: String,
            #[serde(rename = "Names")]
            names: Vec<String>,
            #[serde(rename = "State")]
            state: String,
        }

        #[derive(serde::Deserialize, Default)]
        struct CpuUsage {
            total_usage: Option<u64>,
            percpu_usage: Option<Vec<u64>>,
        }

        #[derive(serde::Deserialize, Default)]
        struct CpuStats {
            cpu_usage: Option<CpuUsage>,
            system_cpu_usage: Option<u64>,
            online_cpus: Option<u64>,
        }

        #[derive(serde::Deserialize, Default)]
        struct MemoryStats {
            usage: Option<i64>,
            limit: Option<i64>,
        }

        #[derive(serde::Deserialize, Default)]
        struct DockerStats {
            cpu_stats: Option<CpuStats>,
            precpu_stats: Option<CpuStats>,
            memory_stats: Option<MemoryStats>,
        }

        async fn docker_stats(
            client: &Client<UnixConnector, Empty<Bytes>>,
            id: &str,
        ) -> (Option<f64>, Option<i64>, Option<i64>) {
            let path = format!("/containers/{}/stats?stream=false", id);
            let uri: hyper::Uri = UnixUri::new("/var/run/docker.sock", &path).into();
            let req = match hyper::Request::builder()
                .method("GET")
                .uri(uri)
                .header("Host", "localhost")
                .body(Empty::<Bytes>::new())
            {
                Ok(req) => req,
                Err(_) => return (None, None, None),
            };
            let Ok(resp) = client.request(req).await else {
                return (None, None, None);
            };
            let Ok(body) = resp.into_body().collect().await else {
                return (None, None, None);
            };
            let Ok(stats) = serde_json::from_slice::<DockerStats>(&body.to_bytes()) else {
                return (None, None, None);
            };

            let cpu = match (&stats.cpu_stats, &stats.precpu_stats) {
                (Some(cpu), Some(pre)) => {
                    let cpu_total = cpu
                        .cpu_usage
                        .as_ref()
                        .and_then(|u| u.total_usage)
                        .unwrap_or(0);
                    let pre_total = pre
                        .cpu_usage
                        .as_ref()
                        .and_then(|u| u.total_usage)
                        .unwrap_or(0);
                    let sys_total = cpu.system_cpu_usage.unwrap_or(0);
                    let pre_sys = pre.system_cpu_usage.unwrap_or(0);
                    let cpu_delta = cpu_total.saturating_sub(pre_total) as f64;
                    let sys_delta = sys_total.saturating_sub(pre_sys) as f64;
                    let cpus = cpu
                        .online_cpus
                        .or_else(|| {
                            cpu.cpu_usage
                                .as_ref()
                                .and_then(|u| u.percpu_usage.as_ref().map(|p| p.len() as u64))
                        })
                        .unwrap_or(1) as f64;
                    if sys_delta > 0.0 && cpu_delta > 0.0 {
                        Some((cpu_delta / sys_delta) * cpus)
                    } else {
                        Some(0.0)
                    }
                }
                _ => None,
            };
            let mem = stats.memory_stats.as_ref().and_then(|m| m.usage);
            let maxmem = stats.memory_stats.as_ref().and_then(|m| m.limit);
            (cpu, mem, maxmem)
        }

        match serde_json::from_slice::<Vec<DockerContainer>>(&body) {
            Ok(dockers) => {
                use std::sync::Arc as StdArc;
                use tokio::sync::Semaphore;
                use tokio::task::JoinSet;

                const DOCKER_STATS_CONCURRENCY: usize = 32;
                let sem = StdArc::new(Semaphore::new(DOCKER_STATS_CONCURRENCY));
                let mut set = JoinSet::new();
                for (i, d) in dockers.into_iter().enumerate() {
                    let sem = sem.clone();
                    let id = d.id;
                    let state = d.state;
                    let name = d
                        .names
                        .into_iter()
                        .next()
                        .unwrap_or_default()
                        .replace('/', "");
                    set.spawn(async move {
                        let _permit = sem.acquire().await.ok()?;
                        let stats = if state == "running" {
                            docker_stats(client, &id).await
                        } else {
                            (Some(0.0), None, None)
                        };
                        Some(LxcContainer {
                            vmid: -1000 - i as i64,
                            status: state,
                            name,
                            cpu: stats.0,
                            maxmem: stats.2,
                            mem: stats.1,
                            maxdisk: None,
                            disk: None,
                            uptime: None,
                        })
                    });
                }
                let mut out = Vec::new();
                while let Some(joined) = set.join_next().await {
                    if let Ok(Some(container)) = joined {
                        out.push(container);
                    }
                }
                out.sort_by_key(|c| c.vmid);
                Ok(out)
            }
            Err(e) => Err(format!("Failed to parse Docker containers list: {}", e)),
        }
    })
}

#[cfg(not(unix))]
fn fetch_docker_containers() -> Result<Vec<LxcContainer>, String> {
    Ok(Vec::new())
}

#[derive(Serialize)]
struct AgentTelemetryTick<'a> {
    cpu_usage: i32,
    ram_usage: i32,
    ram_used_gb: f64,
    ram_total_gb: f64,
    cpu_temp: f64,
    disk_usage: i32,
    disk_used_gb: f64,
    disk_total_gb: f64,
    cpu_model: String,
    cpu_cores: u32,
    gpu_name: String,
    gpu_usage: i32,
    gpu_mem_usage: i32,
    gpu_mem_used_mb: f64,
    gpu_mem_total_mb: f64,
    #[serde(skip_serializing_if = "slice_is_empty")]
    lxc_containers: &'a [LxcContainer],
    network: Option<NetworkTelemetry>,
    disk_mapping_fallback: bool,
    network_mapping_fallback: bool,
    telemetry_scope: String,
    disk_volumes: &'a [DiskMountTelemetry],
    node_tag: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    capabilities: Vec<String>,
    #[serde(skip_serializing_if = "String::is_empty")]
    platform: String,
}

fn slice_is_empty<T>(slice: &&[T]) -> bool {
    slice.is_empty()
}

fn run_telemetry_loop(mut stream: AgentIpcStream) -> Result<(), std::io::Error> {
    let mut sys = System::new();

    let agent_secret = std::env::var("AMUD_AGENT_SECRET").unwrap_or_default();
    if agent_secret.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "AMUD_AGENT_SECRET is not set",
        ));
    }

    // Server sends a challenge first; respond with SHA-256(secret || nonce) — secret never on the wire.
    {
        use std::io::{BufRead, BufReader};
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut challenge_line = String::new();
        reader.read_line(&mut challenge_line)?;
        let nonce = serde_json::from_str::<ChallengeMessage>(&challenge_line)
            .ok()
            .and_then(|m| m.challenge)
            .filter(|n| !n.is_empty())
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Missing IPC auth challenge from server",
                )
            })?;
        let proof = agent_auth_proof(&agent_secret, &nonce);
        let auth = AuthProofMessage { auth: proof };
        if let Ok(mut serialized) = serde_json::to_vec(&auth) {
            serialized.push(b'\n');
            stream.write_all(&serialized)?;
            stream.flush()?;
        }
    }

    // Register this agent under its node_tag before config/telemetry.
    {
        let hello = AgentHelloMessage {
            hello: AgentHelloPayload {
                node_tag: agent_node_tag(),
                capabilities: agent_capabilities(),
                platform: agent_platform_label(),
            },
        };
        if let Ok(mut serialized) = serde_json::to_vec(&hello) {
            serialized.push(b'\n');
            stream.write_all(&serialized)?;
            stream.flush()?;
        }
    }

    let req = ConfigRequest {
        request: "get_config".to_string(),
        pve_token_configured: Some(pve_token_from_env().is_some()),
    };
    if let Ok(mut serialized) = serde_json::to_vec(&req) {
        serialized.push(b'\n');
        stream.write_all(&serialized)?;
        stream.flush()?;
    }

    let reader_stream = stream.try_clone()?;
    let mut response_stream = stream.try_clone()?;
    std::thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let mut reader = BufReader::new(reader_stream);
        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line) {
            if n == 0 {
                break;
            }
            execute_command_from_server(&line, &mut response_stream);
            line.clear();
        }
        println!("AMUD-Agent command reader thread exiting.");
    });

    // Proxmox LXC data is polled on its own slower cadence to minimize overhead.
    // We cache the last result and reuse it between fetches.
    let mut cached_lxc: Arc<Vec<LxcContainer>> = Arc::new(Vec::new());
    let mut last_lxc_fetch = Instant::now()
        .checked_sub(Duration::from_secs(10))
        .unwrap_or_else(Instant::now);
    let mut cached_docker: Arc<Vec<LxcContainer>> = Arc::new(Vec::new());
    let mut last_docker_fetch = Instant::now()
        .checked_sub(Duration::from_secs(10))
        .unwrap_or_else(Instant::now);
    let mut cached_merged: Arc<Vec<LxcContainer>> = Arc::new(Vec::new());
    let mut last_lxc_err_log = Instant::now()
        .checked_sub(Duration::from_secs(60))
        .unwrap_or_else(Instant::now);
    let mut last_docker_err_log = Instant::now()
        .checked_sub(Duration::from_secs(60))
        .unwrap_or_else(Instant::now);
    let mut telemetry_buf: Vec<u8> = Vec::with_capacity(4096);
    let mut gpu_cache = GpuCache::default();
    let mut components = sysinfo::Components::new_with_refreshed_list();
    let mut disks = sysinfo::Disks::new_with_refreshed_list();
    CONFIG_READY.store(false, Ordering::Release);
    reset_network_baseline();

    let config_wait_started = Instant::now();

    loop {
        if CONFIG_READY.load(Ordering::Acquire)
            || config_wait_started.elapsed() >= Duration::from_secs(5)
        {
            break;
        }
        sleep(Duration::from_millis(100));
    }

    loop {
        let poll_cfg = agent_poll_config().read().unwrap().clone();
        let idle = is_idle_mode(&poll_cfg.activity_mode);
        sys.refresh_cpu();
        sys.refresh_memory();

        let cpus = sys.cpus();
        let cpu_usage = if !cpus.is_empty() {
            let sum: f32 = cpus.iter().map(|cpu| cpu.cpu_usage()).sum();
            (sum / cpus.len() as f32).round() as i32
        } else {
            0
        };
        let cpu_model = cpus
            .first()
            .map(|cpu| cpu.brand().trim().to_string())
            .filter(|name| !name.is_empty())
            .unwrap_or_default();
        let cpu_cores = cpus.len() as u32;
        let gpu = gpu_snapshot_cached(&mut gpu_cache);

        let total_mem = sys.total_memory();
        let used_mem = sys.used_memory();
        let ram_usage = if total_mem > 0 {
            ((used_mem as f64 / total_mem as f64) * 100.0).round() as i32
        } else {
            0
        };
        let ram_total_gb = (total_mem as f64 / 1_073_741_824.0 * 100.0).round() / 100.0;
        let ram_used_gb = (used_mem as f64 / 1_073_741_824.0 * 100.0).round() / 100.0;

        components.refresh();
        let mut cpu_temp = 0.0;
        for c in &components {
            let label = c.label().to_lowercase();
            if label.contains("cpu") || label.contains("core") || label.contains("tctl") {
                cpu_temp = c.temperature() as f64;
                break;
            }
        }
        if cpu_temp == 0.0 && !components.is_empty() {
            cpu_temp = components[0].temperature() as f64;
        }

        disks.refresh_list();
        disks.refresh();
        let visible_mounts = list_visible_mounts(&disks);
        let visible_mount_refs: Vec<&str> = visible_mounts.iter().map(String::as_str).collect();
        let disk_cfg = telemetry_config().read().unwrap().clone();
        let mut disk_mapping_fallback = false;
        let mut disk_volumes = Vec::new();
        let (mut total_disk, mut avail_disk, _) = aggregate_disk_bytes(&disks, &disk_cfg);
        if !disk_cfg.disk_mounts.is_empty() && !configured_mounts_satisfied(&disks, &disk_cfg) {
            disk_mapping_fallback = true;
            let auto_cfg = TelemetryConfig {
                disk_mounts: vec![],
                ..disk_cfg.clone()
            };
            (total_disk, avail_disk, _) = aggregate_disk_bytes(&disks, &auto_cfg);
        } else if !disk_cfg.disk_mounts.is_empty() {
            disk_volumes = aggregate_disk_per_mount(&disks, &disk_cfg);
        }
        let used_disk = total_disk - avail_disk;
        let disk_usage = if total_disk > 0 {
            ((used_disk as f64 / total_disk as f64) * 100.0).round() as i32
        } else {
            0
        };
        let disk_total_gb = (total_disk as f64 / 1_073_741_824.0 * 100.0).round() / 100.0;
        let disk_used_gb = (used_disk as f64 / 1_073_741_824.0 * 100.0).round() / 100.0;

        if !idle {
            let mut containers_refreshed = false;
            if last_lxc_fetch.elapsed() >= Duration::from_secs(poll_cfg.lxc_poll_interval_secs) {
                match fetch_lxc_containers() {
                    Ok(containers) => {
                        cached_lxc = Arc::new(filter_containers_by_linked(
                            containers,
                            &poll_cfg.linked_container_names,
                        ));
                        containers_refreshed = true;
                    }
                    Err(e) => {
                        if last_lxc_err_log.elapsed() >= Duration::from_secs(30) {
                            eprintln!(
                                "[LXC] Fetch failed (keeping last-known cache, {} containers): {}",
                                cached_lxc.len(),
                                e
                            );
                            last_lxc_err_log = Instant::now();
                        }
                    }
                }
                last_lxc_fetch = Instant::now();
            }
            if last_docker_fetch.elapsed()
                >= Duration::from_secs(poll_cfg.docker_poll_interval_secs)
            {
                match fetch_docker_containers() {
                    Ok(containers) => {
                        cached_docker = Arc::new(filter_containers_by_linked(
                            containers,
                            &poll_cfg.linked_container_names,
                        ));
                        containers_refreshed = true;
                    }
                    Err(e) => {
                        if last_docker_err_log.elapsed() >= Duration::from_secs(30) {
                            eprintln!(
                                "[Docker] Fetch failed (keeping last-known cache, {} containers): {}",
                                cached_docker.len(),
                                e
                            );
                            last_docker_err_log = Instant::now();
                        }
                    }
                }
                last_docker_fetch = Instant::now();
            }
            if containers_refreshed {
                cached_merged = merge_container_caches(&cached_lxc, &cached_docker);
            }
        } else if !cached_merged.is_empty() {
            cached_lxc = Arc::new(Vec::new());
            cached_docker = Arc::new(Vec::new());
            cached_merged = Arc::new(Vec::new());
        }

        let proc_content = read_proc_net_dev_content();
        let net_cfg = telemetry_config().read().unwrap().clone();
        let custom_net = !net_cfg.external_ifaces.is_empty() || !net_cfg.internal_ifaces.is_empty();
        let (now_custom_snapshot, mut network_mapping_fallback) = proc_content
            .as_ref()
            .map(|c| build_network_snapshot(c, &net_cfg))
            .unwrap_or((NetworkSnapshot::default(), false));
        let now_auto_snapshot = proc_content
            .as_ref()
            .map(|c| collect_network_snapshot(c, &TelemetryConfig::default()).0)
            .unwrap_or_default();

        let visible_ifaces: Vec<String> = proc_content
            .as_ref()
            .map(|c| list_proc_net_ifaces(c).into_iter().collect())
            .unwrap_or_default();
        let telemetry_scope =
            detect_telemetry_scope(&disk_cfg, &visible_ifaces, &visible_mount_refs);

        let network = {
            let mut baseline = network_baseline().lock().unwrap();
            let (network, snapshot_for_baseline) = if let Some(sample_at) = baseline.sample_at {
                let elapsed = sample_at.elapsed().as_secs_f64();
                let mut snapshot_for_baseline = now_custom_snapshot.clone();
                let mut rates = network_rates(&baseline.snapshot, &now_custom_snapshot, elapsed);
                if custom_net {
                    let ext_delta =
                        snapshot_external_delta(&baseline.snapshot, &now_custom_snapshot);
                    let mut heuristic = network_heuristic_state().lock().unwrap();
                    if ext_delta == 0 {
                        heuristic.zero_external_ticks =
                            heuristic.zero_external_ticks.saturating_add(1);
                    } else {
                        heuristic.zero_external_ticks = 0;
                    }
                    if heuristic.zero_external_ticks >= 3 {
                        let auto_ext_delta =
                            snapshot_external_delta(&baseline.snapshot, &now_auto_snapshot);
                        if auto_ext_delta > 0 {
                            network_mapping_fallback = true;
                            if !heuristic.bond_warn_logged {
                                eprintln!(
                                    "[telemetry] External interfaces show no traffic; falling back to auto-detect. If WAN uses bond0, try bond0 instead of eth0 in Settings."
                                );
                                heuristic.bond_warn_logged = true;
                            }
                            rates = network_rates(&baseline.snapshot, &now_auto_snapshot, elapsed);
                            snapshot_for_baseline = now_auto_snapshot.clone();
                        }
                    }
                }
                (rates, snapshot_for_baseline)
            } else {
                (NetworkTelemetry::default(), now_custom_snapshot.clone())
            };
            baseline.snapshot = snapshot_for_baseline;
            baseline.sample_at = Some(Instant::now());
            network
        };

        let telemetry = AgentTelemetryTick {
            cpu_usage,
            ram_usage,
            ram_used_gb,
            ram_total_gb,
            cpu_temp,
            disk_usage,
            disk_used_gb,
            disk_total_gb,
            cpu_model,
            cpu_cores,
            gpu_name: gpu.name,
            gpu_usage: gpu.usage,
            gpu_mem_usage: gpu.mem_usage,
            gpu_mem_used_mb: gpu.mem_used_mb,
            gpu_mem_total_mb: gpu.mem_total_mb,
            lxc_containers: if idle { &[] } else { cached_merged.as_slice() },
            network: Some(network),
            disk_mapping_fallback,
            network_mapping_fallback,
            telemetry_scope,
            disk_volumes: &disk_volumes,
            node_tag: agent_node_tag(),
            capabilities: agent_capabilities(),
            platform: agent_platform_label(),
        };

        telemetry_buf.clear();
        if serde_json::to_writer(&mut telemetry_buf, &telemetry).is_ok() {
            telemetry_buf.push(b'\n');
            stream.write_all(&telemetry_buf)?;
        }
        stream.flush()?;

        sleep(Duration::from_secs(poll_cfg.telemetry_interval_secs));
    }
}

fn send_action_result(
    response_stream: &mut AgentIpcStream,
    request_id: &str,
    success: bool,
    error: Option<String>,
) {
    let response = serde_json::json!({
        "action_result": {
            "request_id": request_id,
            "success": success,
            "error": error
        }
    });
    if let Ok(mut serialized) = serde_json::to_vec(&response) {
        serialized.push(b'\n');
        use std::io::Write;
        if let Err(e) = response_stream.write_all(&serialized) {
            eprintln!("Failed to write action result back to server: {}", e);
        }
        let _ = response_stream.flush();
    }
}

#[cfg(unix)]
fn discover_docker_apps() -> Vec<serde_json::Value> {
    if !docker_enabled() || !std::path::Path::new("/var/run/docker.sock").exists() {
        return Vec::new();
    }
    let rt = agent_runtime();
    rt.block_on(async {
        use http_body_util::{BodyExt, Empty};
        use hyper::body::Bytes;
        use hyperlocal::Uri as UnixUri;

        let client = http_clients::docker_unix_client();
        let uri: hyper::Uri = UnixUri::new("/var/run/docker.sock", "/containers/json").into();
        let req = hyper::Request::builder()
            .method("GET")
            .uri(uri)
            .header("Host", "localhost")
            .body(Empty::<Bytes>::new())
            .ok()?;
        let resp = client.request(req).await.ok()?;
        let body = resp.into_body().collect().await.ok()?.to_bytes();
        #[derive(serde::Deserialize)]
        struct Row {
            #[serde(rename = "Id")]
            id: String,
            #[serde(rename = "Names")]
            names: Vec<String>,
            #[serde(default, rename = "Labels")]
            labels: std::collections::HashMap<String, String>,
        }
        let rows: Vec<Row> = serde_json::from_slice(&body).ok()?;
        let mut out = Vec::new();
        for row in rows {
            let homepage_href = row.labels.get("homepage.href").cloned();
            let homepage_name = row.labels.get("homepage.name").cloned();
            let enabled = row
                .labels
                .get("amud.enable")
                .map(|s| s == "true")
                .unwrap_or(false)
                || homepage_href.is_some()
                || homepage_name.is_some();
            let url = row
                .labels
                .get("amud.url")
                .cloned()
                .or(homepage_href)
                .or_else(|| row.labels.get("traefik.http.routers.amud.rule").cloned());
            if !enabled && url.is_none() {
                continue;
            }
            let name = row
                .labels
                .get("amud.name")
                .cloned()
                .or(homepage_name)
                .or_else(|| {
                    row.names
                        .first()
                        .map(|n| n.trim_start_matches('/').to_string())
                })
                .unwrap_or_else(|| "container".to_string());
            let url = url.unwrap_or_else(|| "http://localhost".to_string());
            let category = row
                .labels
                .get("amud.category")
                .cloned()
                .or_else(|| row.labels.get("homepage.group").cloned())
                .unwrap_or_else(|| "General".to_string());
            let icon = row
                .labels
                .get("amud.icon")
                .cloned()
                .or_else(|| row.labels.get("homepage.icon").cloned())
                .unwrap_or_default();
            let integration_type = row
                .labels
                .get("amud.integration")
                .cloned()
                .or_else(|| row.labels.get("homepage.widget.type").cloned())
                .unwrap_or_default();
            let api_key = row
                .labels
                .get("amud.api_key")
                .cloned()
                .or_else(|| row.labels.get("homepage.widget.key").cloned())
                .unwrap_or_default();
            out.push(serde_json::json!({
                "name": name,
                "url": url,
                "category": category,
                "icon": icon,
                "container_id": row.id,
                "integration_type": integration_type,
                "api_key": api_key,
            }));
        }
        Some(out)
    })
    .unwrap_or_default()
}

#[cfg(not(unix))]
fn discover_docker_apps() -> Vec<serde_json::Value> {
    Vec::new()
}

fn execute_command_from_server(line: &str, response_stream: &mut AgentIpcStream) {
    if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
        if let Some(config) = val.get("config") {
            apply_telemetry_config(config);
            if pve_token_from_env().is_none() {
                if let Some(token) = config.get("pve_api_token").and_then(|t| t.as_str()) {
                    if !token.is_empty() {
                        println!("Received PVE API Token update from server.");
                        update_pve_api_token(token);
                    }
                }
            }
        } else if let Some(action) = val.get("action").and_then(|a| a.as_str()) {
            if action == "discover_docker" {
                let apps = discover_docker_apps();
                let response = serde_json::json!({ "discover_docker_result": { "apps": apps } });
                if let Ok(mut serialized) = serde_json::to_vec(&response) {
                    serialized.push(b'\n');
                    use std::io::Write;
                    let _ = response_stream.write_all(&serialized);
                    let _ = response_stream.flush();
                }
            } else if action == "telemetry_discover" {
                let result = telemetry_discover_payload();
                let response = serde_json::json!({ "telemetry_discover_result": result });
                if let Ok(mut serialized) = serde_json::to_vec(&response) {
                    serialized.push(b'\n');
                    use std::io::Write;
                    let _ = response_stream.write_all(&serialized);
                    let _ = response_stream.flush();
                }
            } else if action == "test_pve" {
                let token = val
                    .get("id")
                    .and_then(|i| i.as_str())
                    .filter(|t| !t.is_empty())
                    .map(|s| s.to_string())
                    .unwrap_or_else(get_pve_api_token);
                println!("Received test_pve action from server.");

                let result = perform_pve_test(&token);

                let response = serde_json::json!({
                    "test_pve_result": result
                });
                if let Ok(mut serialized) = serde_json::to_vec(&response) {
                    serialized.push(b'\n');
                    use std::io::Write;
                    if let Err(e) = response_stream.write_all(&serialized) {
                        eprintln!("Failed to write test result back to server: {}", e);
                    }
                    let _ = response_stream.flush();
                }
            } else {
                #[derive(serde::Deserialize)]
                struct ServerCommand {
                    provider: String,
                    id: String,
                    action: String,
                    request_id: Option<String>,
                }
                if let Ok(cmd) = serde_json::from_value::<ServerCommand>(val) {
                    println!(
                        "Received control action: action='{}' provider='{}' id='{}'",
                        cmd.action, cmd.provider, cmd.id
                    );
                    let (success, error) = match cmd.provider.as_str() {
                        "lxc" => {
                            if let Ok(vmid) = cmd.id.parse::<i64>() {
                                execute_lxc_action(vmid, &cmd.action)
                            } else {
                                (false, Some(format!("Invalid LXC VMID: {}", cmd.id)))
                            }
                        }
                        "docker" => execute_docker_action(&cmd.id, &cmd.action),
                        _ => (false, Some(format!("Unknown provider: {}", cmd.provider))),
                    };
                    if let Some(request_id) = cmd.request_id {
                        send_action_result(response_stream, &request_id, success, error);
                    }
                }
            }
        }
    } else {
        eprintln!("Failed to parse incoming command payload: {}", line);
    }
}

/// Turns an opaque hyper client error (e.g. "client error (SendRequest)")
/// into an actionable message by walking the error source chain.
fn action_http_error_message(err: &dyn std::error::Error, target: &str) -> String {
    let mut cause: Option<String> = None;
    let mut source = err.source();
    while let Some(s) = source {
        if let Some(io) = s.downcast_ref::<std::io::Error>() {
            cause = Some(match io.kind() {
                std::io::ErrorKind::ConnectionRefused => {
                    format!(
                        "connection refused — is {} reachable from the agent?",
                        target
                    )
                }
                std::io::ErrorKind::NotFound => format!("{} not found", target),
                std::io::ErrorKind::PermissionDenied => {
                    format!("permission denied accessing {}", target)
                }
                std::io::ErrorKind::TimedOut => format!("connection to {} timed out", target),
                _ => format!("{} ({})", io, target),
            });
        } else {
            cause = Some(s.to_string());
        }
        source = s.source();
    }
    match cause {
        Some(c) => format!("HTTP request failed: {}", c),
        None => format!("HTTP request failed: {} ({})", err, target),
    }
}

fn execute_lxc_action(vmid: i64, action: &str) -> (bool, Option<String>) {
    let token = get_pve_api_token();
    if token.is_empty() {
        return (false, Some("PVE API token is not configured".to_string()));
    }

    let action_str = match action {
        "start" | "stop" | "reboot" | "shutdown" => action,
        "restart" => "reboot",
        _ => return (false, Some(format!("Unsupported LXC action: {}", action))),
    };

    agent_runtime().block_on(async move {
        match tokio::time::timeout(HTTP_TIMEOUT, async move {
            use http_body_util::Empty;
            use hyper::body::Bytes;

            let client = http_clients::pve_https_client();

            let node_name = pve_node_name();

            let api_url = format!(
                "https://localhost:8006/api2/json/nodes/{}/lxc/{}/status/{}",
                node_name, vmid, action_str
            );
            println!("[LXC Action] POSTing: {}", api_url);

            // Stale keep-alive connections surface as "client error (SendRequest)"
            // on the first request after idle, so retry once on failure.
            let mut last_err: Option<hyper_util::client::legacy::Error> = None;
            let mut response = None;
            for attempt in 0..2 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                let req = match hyper::Request::builder()
                    .method("POST")
                    .uri(&api_url)
                    .header("Authorization", &token)
                    .body(Empty::<Bytes>::new())
                {
                    Ok(r) => r,
                    Err(e) => {
                        return (false, Some(format!("Failed to build request: {}", e)));
                    }
                };
                match client.request(req).await {
                    Ok(resp) => {
                        response = Some(resp);
                        break;
                    }
                    Err(e) => {
                        eprintln!("[LXC Action] attempt {} failed: {}", attempt + 1, e);
                        last_err = Some(e);
                    }
                }
            }

            match response {
                Some(resp) => {
                    let status = resp.status();
                    println!("[LXC Action] PVE API response status: {}", status);
                    if status.is_success() {
                        (true, None)
                    } else {
                        (false, Some(format!("PVE API returned HTTP {}", status)))
                    }
                }
                None => {
                    let err = last_err.expect("request failed without error");
                    (
                        false,
                        Some(action_http_error_message(
                            &err,
                            "the Proxmox API at localhost:8006",
                        )),
                    )
                }
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => (false, Some("LXC action timed out".to_string())),
        }
    })
}

#[cfg(unix)]
fn execute_docker_action(container_name: &str, action: &str) -> (bool, Option<String>) {
    if !docker_enabled() {
        return (
            false,
            Some(
                "Docker integration disabled (mount /var/run/docker.sock or set AMUD_DOCKER=1)"
                    .to_string(),
            ),
        );
    }
    if !std::path::Path::new("/var/run/docker.sock").exists() {
        return (false, Some("Docker Unix socket missing".to_string()));
    }

    let action_str = match action {
        "start" | "stop" | "restart" => action,
        _ => {
            return (
                false,
                Some(format!("Unsupported Docker action: {}", action)),
            )
        }
    };

    let c_name = container_name.to_string();

    agent_runtime().block_on(async move {
        match tokio::time::timeout(HTTP_TIMEOUT, async move {
            use http_body_util::Empty;
            use hyper::body::Bytes;
            use hyperlocal::Uri as UnixUri;

            let client = http_clients::docker_unix_client();
            let api_path = format!("/containers/{}/{}", c_name, action_str);

            println!("[Docker Action] POSTing: {}", api_path);

            // Retry once: idle keep-alive sockets commonly fail the first
            // request with an opaque "client error (SendRequest)".
            let mut last_err: Option<hyper_util::client::legacy::Error> = None;
            let mut response = None;
            for attempt in 0..2 {
                if attempt > 0 {
                    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                }
                let uri: hyper::Uri = UnixUri::new("/var/run/docker.sock", &api_path).into();
                let req = match hyper::Request::builder()
                    .method("POST")
                    .uri(uri)
                    .header("Host", "localhost")
                    .body(Empty::<Bytes>::new())
                {
                    Ok(r) => r,
                    Err(e) => return (false, Some(format!("Failed to build request: {}", e))),
                };
                match client.request(req).await {
                    Ok(resp) => {
                        response = Some(resp);
                        break;
                    }
                    Err(e) => {
                        eprintln!("[Docker Action] attempt {} failed: {}", attempt + 1, e);
                        last_err = Some(e);
                    }
                }
            }

            match response {
                Some(resp) => {
                    let status = resp.status();
                    println!("[Docker Action] Docker API response status: {}", status);
                    if status.is_success() || status.as_u16() == 304 {
                        (true, None)
                    } else {
                        (false, Some(format!("Docker API returned HTTP {}", status)))
                    }
                }
                None => {
                    let err = last_err.expect("request failed without error");
                    (
                        false,
                        Some(action_http_error_message(
                            &err,
                            "the Docker socket /var/run/docker.sock",
                        )),
                    )
                }
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => (false, Some("Docker action timed out".to_string())),
        }
    })
}

#[cfg(not(unix))]
fn execute_docker_action(_container_name: &str, _action: &str) -> (bool, Option<String>) {
    (
        false,
        Some("Docker actions are unavailable on this platform".to_string()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn iface_classify_auto_unraid_br0() {
        let cfg = TelemetryConfig::default();
        assert_eq!(iface_classify("br0", &cfg), Some("internal"));
        assert_eq!(iface_classify("br0.40", &cfg), Some("internal"));
        assert_eq!(iface_classify("br-abc", &cfg), Some("internal"));
        assert_eq!(iface_classify("bond0", &cfg), Some("external"));
        assert_eq!(iface_classify("eth0", &cfg), Some("external"));
        assert_eq!(iface_classify("vmbr0", &cfg), Some("internal"));
        assert_eq!(iface_classify("docker0", &cfg), Some("internal"));
    }

    #[test]
    fn configured_mounts_satisfied_paths_partial() {
        let cfg = TelemetryConfig {
            disk_mounts: vec!["/mnt/cache".into(), "/mnt/user".into()],
            ..Default::default()
        };
        assert!(!configured_mounts_satisfied_paths(&["/mnt/cache"], &cfg));
        assert!(configured_mounts_satisfied_paths(
            &["/mnt/cache", "/mnt/user"],
            &cfg
        ));
        assert!(configured_mounts_satisfied_paths(
            &["/mnt/cache", "/mnt/user/data"],
            &cfg
        ));
    }

    #[test]
    fn configured_network_ifaces_missing_external() {
        let sample = "Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  bond0: 1000 10 0 0 0 0 0 0 2000 20 0 0 0 0 0 0
  br0: 3000 30 0 0 0 0 0 0 4000 40 0 0 0 0 0 0
";
        let cfg = TelemetryConfig {
            external_ifaces: vec!["eth0".into()],
            internal_ifaces: vec!["br0".into()],
            ..Default::default()
        };
        assert!(!configured_network_ifaces_satisfied(sample, &cfg));
        let (snap, fallback) = {
            let custom = true;
            if custom && !configured_network_ifaces_satisfied(sample, &cfg) {
                (
                    collect_network_snapshot(sample, &TelemetryConfig::default()).0,
                    true,
                )
            } else {
                (NetworkSnapshot::default(), false)
            }
        };
        assert!(fallback);
        assert!(snap.external_rx > 0);
    }

    #[test]
    fn configured_network_ifaces_present_no_fallback() {
        let sample = "Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  eth0: 1000 10 0 0 0 0 0 0 2000 20 0 0 0 0 0 0
  br0: 3000 30 0 0 0 0 0 0 4000 40 0 0 0 0 0 0
";
        let cfg = TelemetryConfig {
            external_ifaces: vec!["eth0".into()],
            internal_ifaces: vec!["br0".into()],
            ..Default::default()
        };
        assert!(configured_network_ifaces_satisfied(sample, &cfg));
    }

    #[test]
    fn network_rates_uses_fractional_seconds() {
        let previous = NetworkSnapshot {
            external_rx: 0,
            ..Default::default()
        };
        let current = NetworkSnapshot {
            external_rx: 1000,
            ..Default::default()
        };
        let rates = network_rates(&previous, &current, 2.9);
        assert_eq!(rates.external_rx, "3 kbit/s");
    }

    #[test]
    fn mount_matches_unraid_paths() {
        let cfg = TelemetryConfig {
            disk_mounts: vec!["/mnt/user".into(), "/mnt/cache".into()],
            ..Default::default()
        };
        assert!(mount_matches("/mnt/user", &cfg));
        assert!(mount_matches("/mnt/cache/", &cfg));
        assert!(mount_matches("/mnt/user/data", &cfg));
        assert!(!mount_matches("/mnt/disk1", &cfg));
    }

    #[test]
    fn iface_classify_vlan_internal() {
        let cfg = TelemetryConfig {
            internal_ifaces: vec!["br0".into(), "br0.40".into()],
            external_ifaces: vec!["eth0".into()],
            ..Default::default()
        };
        assert_eq!(iface_classify("br0.40", &cfg), Some("internal"));
        assert_eq!(iface_classify("eth0", &cfg), Some("external"));
        assert_eq!(iface_classify("bond0", &cfg), None);
    }

    #[test]
    fn collect_network_snapshot_respects_mapping() {
        let sample = "Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  eth0: 1000 10 0 0 0 0 0 0 2000 20 0 0 0 0 0 0
  br0: 3000 30 0 0 0 0 0 0 4000 40 0 0 0 0 0 0
";
        let cfg = TelemetryConfig {
            external_ifaces: vec!["eth0".into()],
            internal_ifaces: vec!["br0".into()],
            ..Default::default()
        };
        let (snap, matched) = collect_network_snapshot(sample, &cfg);
        assert_eq!(matched, 2);
        assert_eq!(snap.external_rx, 1000);
        assert_eq!(snap.external_tx, 2000);
        assert_eq!(snap.internal_rx, 3000);
        assert_eq!(snap.internal_tx, 4000);
    }

    #[test]
    fn mount_label_short_name() {
        assert_eq!(mount_label("/mnt/user"), "user");
        assert_eq!(mount_label("/mnt/cache/"), "cache");
        assert_eq!(mount_label("/"), "");
    }

    #[test]
    fn detect_telemetry_scope_missing_mounts() {
        let cfg = TelemetryConfig {
            disk_mounts: vec!["/mnt/user".into(), "/mnt/cache".into()],
            ..Default::default()
        };
        let ifaces = vec!["eth0".to_string()];
        let mounts = vec!["/"];
        let scope = detect_telemetry_scope(&cfg, &ifaces, &mounts);
        if std::path::Path::new("/.dockerenv").exists() {
            assert_eq!(scope, "container");
        } else {
            assert_eq!(scope, "host");
        }
    }

    #[test]
    fn build_network_snapshot_missing_iface_fallback() {
        let sample = "Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
  bond0: 1000 10 0 0 0 0 0 0 2000 20 0 0 0 0 0 0
  br0: 3000 30 0 0 0 0 0 0 4000 40 0 0 0 0 0 0
";
        let cfg = TelemetryConfig {
            external_ifaces: vec!["eth0".into()],
            internal_ifaces: vec!["br0".into()],
            ..Default::default()
        };
        let (snap, fallback) = build_network_snapshot(sample, &cfg);
        assert!(fallback);
        assert!(snap.external_rx > 0);
    }

    #[test]
    fn configured_mounts_satisfied_paths_single() {
        let cfg = TelemetryConfig {
            disk_mounts: vec!["/mnt/user".into()],
            ..Default::default()
        };
        assert!(configured_mounts_satisfied_paths(&["/mnt/user"], &cfg));
        assert!(!configured_mounts_satisfied_paths(&["/mnt/cache"], &cfg));
    }

    #[test]
    fn parse_config_u64_clamps_and_parses_strings() {
        assert_eq!(
            parse_config_u64(Some(&serde_json::json!(999)), 5, 3, 60),
            60
        );
        assert_eq!(
            parse_config_u64(Some(&serde_json::json!("12")), 5, 3, 60),
            12
        );
        assert_eq!(parse_config_u64(None, 5, 3, 60), 5);
    }
}
