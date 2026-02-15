use std::path::PathBuf;

/// Firecracker boot configuration builder.
///
/// Produces the JSON payloads for Firecracker's API:
/// /machine-config, /boot-source, /drives/rootfs, /vsock
pub struct FirecrackerConfig {
    pub vcpus: u32,
    pub mem_mb: u32,
    pub kernel_path: PathBuf,
    pub rootfs_path: PathBuf,
    pub vsock_cid: u32,
    pub socket_path: PathBuf,
}

impl FirecrackerConfig {
    pub fn machine_config_json(&self) -> serde_json::Value {
        serde_json::json!({
            "vcpu_count": self.vcpus,
            "mem_size_mib": self.mem_mb,
        })
    }

    pub fn boot_source_json(&self) -> serde_json::Value {
        serde_json::json!({
            "kernel_image_path": self.kernel_path.to_string_lossy(),
            "boot_args": "console=ttyS0 reboot=k panic=1 pci=off",
        })
    }

    pub fn rootfs_drive_json(&self) -> serde_json::Value {
        serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": self.rootfs_path.to_string_lossy(),
            "is_root_device": true,
            "is_read_only": false,
        })
    }

    pub fn vsock_json(&self) -> serde_json::Value {
        serde_json::json!({
            "guest_cid": self.vsock_cid,
            "uds_path": self.socket_path.to_string_lossy(),
        })
    }
}
