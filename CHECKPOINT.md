# GBE-Sentinel Session Checkpoint

**Date**: 2026-02-15
**Previous session**: sandbox → gbe-sentinel rename caused session death

---

## What Exists on Disk

```
gbe-sentinel/
├── CHECKPOINT.md               # this file
└── docs/
    └── design/
        ├── sentinel-architecture.md   # full architecture, bus integration, crate structure
        ├── naming-themes.md           # 3 sci-fi themes (Forerunner active)
        └── vm-sandbox-research.md     # VM/container comparison research
```

## What Needs To Happen

### 1. Scaffold the workspace

Read `docs/design/sentinel-architecture.md` — the "Crate Structure" section at
the bottom has both Cargo.toml files (workspace + crate) and full module layout.

Create:
- `/Cargo.toml` — workspace root
- `/crates/sentinel/Cargo.toml` — crate manifest
- `/crates/sentinel/src/lib.rs` — pub exports
- `/crates/sentinel/src/sentinel.rs` — Sentinel struct stub
- `/crates/sentinel/src/config.rs` — SentinelConfig, VmProfile, NetworkMode
- `/crates/sentinel/src/error.rs` — SentinelError
- `/crates/sentinel/src/handler.rs` — MessageHandler impl stub
- `/crates/sentinel/src/claim.rs` — CAS claim logic stub
- `/crates/sentinel/src/vm/mod.rs` — VM module
- `/crates/sentinel/src/vm/manager.rs` — Firecracker API client stub
- `/crates/sentinel/src/vm/lifecycle.rs` — state machine stub
- `/crates/sentinel/src/vm/config.rs` — Firecracker boot config stub
- `/crates/sentinel/src/vm/overlay.rs` — CoW rootfs stub
- `/crates/sentinel/src/vm/network.rs` — tap/iptables stub
- `/crates/sentinel/src/vsock/mod.rs` — vsock module
- `/crates/sentinel/src/vsock/listener.rs` — vsock accept/demux stub
- `/crates/sentinel/src/vsock/protocol.rs` — OperativeMessage, SentinelMessage types
- `/crates/sentinel/src/vsock/proxy.rs` — tool proxy stub
- `/crates/sentinel/src/health.rs` — beacon + capacity publisher stub
- `/.gitignore`

### 2. Init git

```bash
git init
git add .
git commit -m "feat: initial scaffold with design docs"
```

### 3. Verify transport path deps

The workspace Cargo.toml references:
```toml
gbe-transport = { path = "../gbe-transport/crates/transport" }
gbe-state-store = { path = "../gbe-transport/crates/state-store" }
```

Verify `../gbe-transport/` exists and paths resolve. Run `cargo check` after scaffold.

## Key Design Decisions (for context)

- **Sentinel** = per-host VM lifecycle manager (Firecracker microVMs)
- **Operative** = guest agent inside VM (communicates over vsock)
- **Watcher** = sweeper/archiver (already exists as gbe-sweeper)
- **Nexus** = the bus (gbe-transport)
- Bus integration via `Arc<dyn Transport>` + `Arc<dyn StateStore>` (same pattern as sweeper)
- Subjects: `gbe.tasks.{type}.queue/progress/terminal`, `gbe.events.sentinel.{host}.*`
- State keys: `gbe:state:tasks:{type}:{id}`
- CAS claiming, pull-based, ephemeral VMs, vsock-only channel
- Network evolution: NAT → proxy → zero-trust tool proxy
- Standalone workspace, NOT part of gbe-transport
- Rust, shares workspace dep versions with gbe-transport
