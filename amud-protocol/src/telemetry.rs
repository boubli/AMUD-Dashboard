use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct DiskMountTelemetry {
    pub mount: String,
    pub label: String,
    pub usage: i32,
    pub used_gb: f64,
    pub total_gb: f64,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct AgentTelemetry {
    pub cpu_usage: i32,
    pub ram_usage: i32,
    pub ram_used_gb: f64,
    pub ram_total_gb: f64,
    pub cpu_temp: f64,
    pub disk_usage: i32,
    pub disk_used_gb: f64,
    pub disk_total_gb: f64,
    #[serde(default)]
    pub cpu_model: String,
    #[serde(default)]
    pub cpu_cores: u32,
    #[serde(default)]
    pub gpu_name: String,
    #[serde(default)]
    pub gpu_usage: i32,
    #[serde(default)]
    pub gpu_mem_usage: i32,
    #[serde(default)]
    pub gpu_mem_used_mb: f64,
    #[serde(default)]
    pub gpu_mem_total_mb: f64,
    #[serde(default)]
    pub lxc_containers: Vec<LxcContainer>,
    #[serde(default)]
    pub network: Option<NetworkTelemetry>,
    #[serde(default)]
    pub disk_mapping_fallback: bool,
    #[serde(default)]
    pub network_mapping_fallback: bool,
    #[serde(default)]
    pub telemetry_scope: String,
    #[serde(default)]
    pub visible_ifaces: Vec<String>,
    #[serde(default)]
    pub visible_mounts: Vec<String>,
    #[serde(default)]
    pub disk_volumes: Vec<DiskMountTelemetry>,
    /// Host label for multi-node agent deployments (matches app `node_tag`).
    #[serde(default)]
    pub node_tag: String,
    /// Capability profiles: `host`, `docker`, `proxmox`.
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// Soft platform label: `proxmox`, `unraid`, `casaos`, `docker`, or empty.
    #[serde(default)]
    pub platform: String,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct LxcContainer {
    pub vmid: i64,
    pub status: String,
    pub name: String,
    pub cpu: Option<f64>,
    pub maxmem: Option<i64>,
    pub mem: Option<i64>,
    pub maxdisk: Option<i64>,
    pub disk: Option<i64>,
    pub uptime: Option<i64>,
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct NetworkTelemetry {
    pub internal_tx: String,
    pub internal_rx: String,
    pub external_tx: String,
    pub external_rx: String,
}
