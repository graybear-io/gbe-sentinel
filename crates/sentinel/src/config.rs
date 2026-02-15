use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct SentinelConfig {
    pub host_id: String,
    pub slots: u32,
    pub image_dir: PathBuf,
    pub kernel_path: PathBuf,
    pub overlay_dir: PathBuf,
    pub firecracker_bin: PathBuf,
    pub profiles: HashMap<String, VmProfile>,
    pub task_types: Vec<String>,
    #[serde(default = "default_heartbeat")]
    pub heartbeat_interval_secs: u64,
}

fn default_heartbeat() -> u64 {
    10
}

#[derive(Debug, Clone, Deserialize)]
pub struct VmProfile {
    pub vcpus: u32,
    pub mem_mb: u32,
    pub rootfs: String,
    #[serde(default = "default_timeout")]
    pub timeout_sec: u64,
    #[serde(default)]
    pub network: NetworkMode,
    pub network_policy: Option<NetworkPolicy>,
    pub tool_policy: Option<ToolPolicy>,
}

fn default_timeout() -> u64 {
    300
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkMode {
    #[default]
    Nat,
    Proxy,
    None,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkPolicy {
    pub mode: String,
    #[serde(default)]
    pub allow: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolPolicy {
    #[serde(default)]
    pub allowed_tools: Vec<String>,
    pub rate_limit: Option<RateLimit>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimit {
    pub calls_per_minute: u32,
}
