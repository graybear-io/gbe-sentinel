# VM/Container Options for Agent Sandboxes (Non-Docker)

Research notes from initial investigation. Decision: **Firecracker** for production,
with tap+NAT (phase 1) evolving to vsock-only zero-trust (phase 3).

## Linux-Native Solutions

| Solution | Isolation | Efficiency | Maintenance | Notes |
|---|---|---|---|---|
| **Firecracker** | 5/5 | 5/5 | 4/5 | microVM, ~125ms boot, built for serverless sandboxes (AWS Lambda). KVM-based. |
| **Cloud Hypervisor** | 5/5 | 5/5 | 3/5 | Rust-based microVM, more hardware flexibility. Intel/ARM. |
| **QEMU/KVM (direct)** | 5/5 | 4/5 | 3/5 | Full hypervisor, heavier but maximum flexibility. |
| **gVisor (runsc)** | 4/5 | 4/5 | 4/5 | User-space kernel, OCI-compatible, not Docker-dependent. |
| **Kata Containers** | 5/5 | 4/5 | 2/5 | Lightweight VMs behind OCI interface. Uses Firecracker or QEMU backend. |
| **crun / youki + namespaces** | 3/5 | 5/5 | 4/5 | Bare OCI runtimes. Manual namespace/seccomp/cgroup setup. |
| **bubblewrap (bwrap)** | 3/5 | 5/5 | 5/5 | Unprivileged sandboxing (Flatpak). Dead simple. |
| **systemd-nspawn** | 3/5 | 5/5 | 5/5 | "chroot on steroids". Already on most Linux systems. |
| **LXC/LXD (Incus)** | 3.5/5 | 5/5 | 4/5 | System containers. Incus is the community fork. |

## macOS Solutions

| Solution | Isolation | Efficiency | Maintenance | Notes |
|---|---|---|---|---|
| **Virtualization.framework** | 5/5 | 5/5 | 4/5 | Apple native hypervisor. Use via tart, vfkit, or direct API. |
| **Lima** | 5/5 | 4/5 | 5/5 | Wraps QEMU / Virtualization.framework. File sharing, port forwarding. |
| **OrbStack** | 5/5 | 5/5 | 5/5 | Commercial. Linux VMs on macOS with near-zero overhead. |
| **UTM / QEMU** | 5/5 | 4/5 | 3/5 | Full QEMU on macOS. Heavier, runs anything. |

## Chosen Architecture

- **Linux host**: Firecracker — sub-second boot, minimal attack surface, KVM isolation.
- **macOS dev**: Lima or OrbStack to host a Linux VM, then Firecracker inside it.
- **Lightweight alternative**: bubblewrap or systemd-nspawn with seccomp filters.

## Quick Reference

- Firecracker: ~3MB binary, 125ms boot, 5MB memory overhead
- systemd-nspawn: zero install, instant boot, shared kernel
- bubblewrap: single binary, microsecond setup, unprivileged
- LXC/Incus: system containers, seconds to boot, snapshot support
