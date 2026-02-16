use serde::Deserialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::error::SentinelError;

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

impl SentinelConfig {
    /// Validate all paths and identifiers after deserialization.
    pub fn validate(&self) -> Result<(), SentinelError> {
        Self::validate_host_id(&self.host_id)?;
        Self::require_dir(&self.image_dir, "image_dir")?;
        Self::require_file(&self.kernel_path, "kernel_path")?;
        Self::require_dir(&self.overlay_dir, "overlay_dir")?;
        Self::require_file(&self.firecracker_bin, "firecracker_bin")?;
        Self::reject_traversal(&self.image_dir, "image_dir")?;
        Self::reject_traversal(&self.kernel_path, "kernel_path")?;
        Self::reject_traversal(&self.overlay_dir, "overlay_dir")?;
        Self::reject_traversal(&self.firecracker_bin, "firecracker_bin")?;
        Ok(())
    }

    fn validate_host_id(id: &str) -> Result<(), SentinelError> {
        if id.is_empty() || id.len() > 128 {
            return Err(SentinelError::Config(format!(
                "host_id: must be 1-128 characters, got {}",
                id.len()
            )));
        }
        if !id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
        {
            return Err(SentinelError::Config(
                "host_id: must contain only alphanumeric, hyphen, underscore, or dot".to_string(),
            ));
        }
        Ok(())
    }

    fn require_dir(path: &Path, field: &str) -> Result<(), SentinelError> {
        if !path.exists() {
            return Err(SentinelError::Config(format!(
                "{field}: path does not exist: {}",
                path.display()
            )));
        }
        if !path.is_dir() {
            return Err(SentinelError::Config(format!(
                "{field}: expected directory, got file: {}",
                path.display()
            )));
        }
        Ok(())
    }

    fn require_file(path: &Path, field: &str) -> Result<(), SentinelError> {
        if !path.exists() {
            return Err(SentinelError::Config(format!(
                "{field}: path does not exist: {}",
                path.display()
            )));
        }
        if !path.is_file() {
            return Err(SentinelError::Config(format!(
                "{field}: expected file, got directory: {}",
                path.display()
            )));
        }
        Ok(())
    }

    fn reject_traversal(path: &Path, field: &str) -> Result<(), SentinelError> {
        let canonical = path
            .canonicalize()
            .map_err(|e| SentinelError::Config(format!("{field}: cannot resolve path: {e}")))?;
        if canonical
            .components()
            .any(|c| c == std::path::Component::ParentDir)
        {
            return Err(SentinelError::Config(format!(
                "{field}: path contains traversal: {}",
                path.display()
            )));
        }
        Ok(())
    }
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
