# Sentinel: Per-Host VM Lifecycle Manager

## Role

One sentinel per physical/virtual host. It owns the Firecracker VMs on that box.
It is a bus participant — not a coordinator. Coordination lives in the bus topology.

## Responsibilities

1. **Claim tasks** — subscribe to task queue, claim work based on local capacity
2. **Provision VM** — boot a Firecracker microVM with the required rootfs/tools
3. **Inject task** — pass task payload into VM via vsock
4. **Monitor execution** — track VM health, enforce timeouts, relay progress events
5. **Collect result** — receive output via vsock, publish to bus
6. **Teardown** — destroy VM, reclaim resources
7. **Advertise capacity** — publish slot availability so upstream can route intelligently

---

## Bus Integration (gbe-transport)

Sentinel is a native bus participant using the same traits as watcher.

### Constructor Pattern (mirrors watcher)

```rust
pub struct Sentinel {
    config: SentinelConfig,
    transport: Arc<dyn Transport>,
    store: Arc<dyn StateStore>,
    slots: SlotTracker,
}

impl Sentinel {
    pub async fn new(
        config: SentinelConfig,
        transport: Arc<dyn Transport>,
        store: Arc<dyn StateStore>,
    ) -> Result<Self, SentinelError> { ... }

    pub async fn run(&self, token: CancellationToken) -> Result<(), SentinelError> { ... }
}
```

### Subjects Used

Following the existing hierarchy from `gbe.tasks.*`:

```
# Sentinel subscribes to:
gbe.tasks.{task_type}.queue            # claim pending work (consumer group: {task_type}-workers)

# Sentinel publishes to:
gbe.tasks.{task_type}.progress         # relay progress events from VM
gbe.tasks.{task_type}.terminal         # completed/failed/cancelled

# Sentinel-specific (under events):
gbe.events.sentinel.{host_id}.health   # periodic heartbeat (beacon)
gbe.events.sentinel.{host_id}.capacity # slot availability changes
```

### State Store Keys

Following the existing convention `gbe:state:tasks:{task_type}:{task_id}`:

```
# Fields the sentinel reads:
state          — "pending" (to CAS claim)
task_type      — routing key
params_ref     — task payload reference

# Fields the sentinel writes:
state          — "claimed" → "running" → "completed"/"failed"
worker         — "{host_id}:{vm_cid}"
updated_at     — unix millis (keeps watcher happy)
timeout_at     — unix millis (watcher uses this for stuck detection)
started_at     — when VM entered RUNNING
completed_at   — when result received
error          — on failure
result_ref     — output payload reference
```

### Task Claiming Flow

```
1. Subscribe to gbe.tasks.{task_type}.queue (consumer group: {task_type}-workers)
2. Receive message → extract state key from payload
3. CAS: compare_and_swap(key, "state", "pending", "claimed")
   - Success → set worker, updated_at, timeout_at → begin provisioning
   - Failure → msg.nak() (another sentinel claimed it)
4. msg.ack() only after CAS succeeds and state is written
```

This matches the watcher's retry pattern: if sentinel crashes between claim and
ack, the message reclaims via XAUTOCLAIM and another sentinel can try.

### Progress Relay

VM sends progress over vsock → sentinel publishes to bus:

```rust
// vsock message from operative (guest agent):
{ "type": "progress", "id": "task_123", "step": "compile", "status": "complete" }

// sentinel publishes:
transport.publish(
    "gbe.tasks.{task_type}.progress",
    Bytes::from(progress_json),
    Some(PublishOpts { trace_id: envelope_trace_id.clone(), ..Default::default() }),
).await?;

// sentinel updates state store:
store.set_fields(&state_key, HashMap::from([
    ("updated_at".to_string(), Bytes::from(now_millis().to_string())),
    ("current_step".to_string(), Bytes::from("compile")),
])).await?;
```

### Terminal Events

On task completion or failure, sentinel publishes to `.terminal` and updates state:

```rust
// Success path
store.set_fields(&state_key, HashMap::from([
    ("state", "completed"),
    ("updated_at", now_str),
    ("completed_at", now_str),
    ("result_ref", result_ref),
])).await?;
transport.publish("gbe.tasks.{type}.terminal", result_payload, opts).await?;

// Failure path (timeout, crash, error)
store.set_fields(&state_key, HashMap::from([
    ("state", "failed"),
    ("updated_at", now_str),
    ("error", reason),
])).await?;
transport.publish("gbe.tasks.{type}.terminal", failure_payload, opts).await?;
```

### Watcher (Sweeper) Compatibility

The sentinel keeps the watcher happy by:
- Setting `updated_at` on every state transition (watcher scans for stale `updated_at`)
- Setting `timeout_at` when entering RUNNING (watcher can detect stuck jobs)
- Using terminal states `completed`/`failed` (watcher skips these)
- Using CAS for claims (prevents double-processing)

---

## Communication Model

```
[Nexus (Redis/NATS)]
    |
    |--- gbe.tasks.{type}.queue              (sentinel subscribes, consumer group)
    |--- gbe.tasks.{type}.progress           (sentinel publishes)
    |--- gbe.tasks.{type}.terminal           (sentinel publishes)
    |--- gbe.events.sentinel.{host}.health   (sentinel beacon)
    |--- gbe.events.sentinel.{host}.capacity (sentinel slot updates)
```

## VM Lifecycle

```
IDLE ─── claim task ──→ PROVISIONING ──→ RUNNING ──→ COLLECTING ──→ TEARDOWN ──→ IDLE
                              │              │              │
                              ▼              ▼              ▼
                           FAILED         TIMEOUT        FAILED
                              │              │              │
                              └──── all ─────┴──→ TEARDOWN ─┘
```

Every terminal state results in VM destruction. No VM survives its task.

## Sentinel ↔ Operative Channel: vsock

Firecracker exposes virtio-vsock — a socket interface between host and guest
with no network stack. This gives us:

- No IP addressing or firewall config per VM
- No network-based attack surface from guest to host
- Simple stream protocol (connect to CID + port)

### Protocol (over vsock)

```
Sentinel → Operative (port 5000):
  { "type": "task", "id": "...", "payload": { ... }, "tools": [...] }

Operative → Sentinel:
  { "type": "progress", "id": "...", "step": "...", "status": "..." }
  { "type": "result", "id": "...", "output": { ... }, "exit_code": 0 }
  { "type": "error", "id": "...", "error": "...", "exit_code": 1 }
  { "type": "tool_call", "id": "...", "tool": "...", "params": { ... } }

Sentinel → Operative (tool_call response, phase 3):
  { "type": "tool_result", "id": "...", "call_id": "...", "result": { ... } }
```

JSON-lines over the vsock stream. One message per line.

## Capacity Model

Each host has a fixed number of **slots** (based on CPU/RAM allocation per VM).
Sentinel tracks:

- `total_slots` — configured max concurrent VMs
- `used_slots` — currently running VMs
- `available_slots` — total - used

Published to `gbe.events.sentinel.{host_id}.capacity` on every state change and
on a periodic timer.

## Task Claiming

Pull-based, not push-based. Sentinel only claims when it has available slots.

Strategy for Redis POC:
- Subscribe to `gbe.tasks.{task_type}.queue` via consumer group `{task_type}-workers`
- On message: CAS claim in state store, ack on success, nak on conflict
- Backpressure via `max_inflight` in `SubscribeOpts` (matches available slots)

Strategy for NATS phase:
- Queue group subscription on `gbe.tasks.{task_type}.queue`
- NATS handles distribution across sentinels in the same group
- Exactly-once delivery via JetStream ack

## VM Configuration

Each task type maps to a **VM profile**:

```toml
[profiles.default]
vcpus = 2
mem_mb = 512
rootfs = "/var/lib/sentinel/images/base.ext4"
kernel = "/var/lib/sentinel/kernels/vmlinux"
timeout_sec = 300
network = "nat"           # phase 1: tap+NAT, phase 2: "proxy", phase 3: "none"

[profiles.heavy]
vcpus = 4
mem_mb = 2048
rootfs = "/var/lib/sentinel/images/heavy.ext4"
kernel = "/var/lib/sentinel/kernels/vmlinux"
timeout_sec = 600
network = "nat"

[profiles.default.network_policy]  # phase 2+
mode = "proxy"
allow = ["api.openai.com:443", "api.anthropic.com:443"]

[profiles.default.tool_policy]     # phase 3
allowed_tools = ["llm.complete", "web.fetch"]
rate_limit = { calls_per_minute = 60 }
```

Tasks carry a `profile` label. Sentinel selects the matching config.

## Timeout Enforcement

- Sentinel starts a timer when VM enters RUNNING
- On expiry: send SIGKILL to Firecracker process, publish `task.failed` with timeout reason
- Operative can request extensions via vsock (sentinel decides whether to grant)
- `timeout_at` field in state store keeps watcher aligned

## Rootfs Management

The rootfs is the filesystem the VM boots from — a single `.ext4` file containing
the OS, operative (guest agent), and pre-installed tools.

### Image Distribution Strategy

**Phase 1: Pre-stage (start here)**
- Build images in CI, push to all hosts via rsync/ansible
- Zero boot-time delay — images are always local
- Simple, predictable, works when host set is known

**Phase 2: Pull-on-demand fallback**
- If sentinel receives a task requiring a profile whose image is missing locally,
  pull from artifact store (S3/HTTP) before provisioning
- Cache locally after first pull
- Enables dynamic host scaling without pre-coordination

Image manifest (checked by sentinel at startup and on task claim):
```
/var/lib/sentinel/
├── images/
│   ├── base.ext4           # default profile
│   ├── heavy.ext4          # heavy profile
│   └── .manifest.json      # { "base": { "sha256": "...", "version": "..." } }
├── kernels/
│   └── vmlinux
└── overlays/               # per-VM CoW working directory
```

### Per-VM Copy-on-Write

Each VM gets a CoW snapshot of the base image, not a full copy:
1. Sentinel creates a sparse copy or device-mapper snapshot
2. VM writes land in the overlay, base image stays clean
3. On teardown, delete the overlay — instant cleanup, no fsck

This means 100 VMs sharing `base.ext4` use ~base size + delta writes,
not 100x the base size.

## Network Security Evolution

### Phase 1: Outbound NAT (current)

```
VM ──tap──▶ host bridge ──▶ iptables NAT ──▶ internet
```

- Each VM gets a tap device + unique IP on a host-local bridge
- iptables rules: allow egress, deny inter-VM traffic, deny host-local services
- Sentinel creates/destroys tap devices as part of VM lifecycle
- Sufficient for operatives that call external APIs directly

### Phase 2: Sentinel Proxy

```
VM ──vsock──▶ sentinel proxy ──▶ allowlisted endpoints only
```

- VM has **no tap device** — only vsock
- Operative routes HTTP calls through a vsock-based forward proxy on the sentinel
- Sentinel enforces an allowlist per task profile
- Operative sees a CONNECT proxy on vsock port 5001
- Sentinel logs all connections for audit

### Phase 3: Zero-Trust Tool Proxy

```
VM ──vsock──▶ sentinel ──▶ tool registry ──▶ external services
```

- VM has no network device and no proxy
- Operative sends structured tool-call requests over vsock
- Sentinel validates against task-scoped policy (tool allowed? params in bounds? rate limit?)
- Sentinel executes the call on behalf of the operative and returns the result
- The VM literally cannot make arbitrary network requests — the capability doesn't exist

### Security Boundaries

```
┌─────────────────────────────────────────┐
│ Host                                    │
│  ┌─────────────────────────────┐        │
│  │ Sentinel                    │        │
│  │  ├─ VM manager              │        │
│  │  ├─ vsock listener          │        │
│  │  ├─ tool proxy (phase 3)    │        │
│  │  └─ nexus client            │        │
│  └────┬──────────┬─────────────┘        │
│       │vsock     │vsock                 │
│  ┌────┴────┐┌────┴────┐                │
│  │ VM (KVM)││ VM (KVM)│                │
│  │ no net  ││ no net  │                │
│  └─────────┘└─────────┘                │
└─────────────────────────────────────────┘
```

- Phase 1: VMs get tap devices with scoped iptables rules
- Phase 2+: VMs have vsock only, no network device in guest kernel
- Sentinel runs as non-root with CAP_NET_ADMIN + /dev/kvm access only
- Each VM gets a unique CID for vsock addressing

## Failure Modes

| Failure | Detection | Response |
|---|---|---|
| VM crashes | Firecracker process exits | Publish task.failed, teardown |
| VM hangs | Timeout expires | Kill process, publish task.failed |
| Sentinel crashes | Beacon stops | Watcher detects stuck jobs via stale `updated_at`, requeues |
| Host dies | Beacon stops | Same as above |
| Nexus unreachable | Publish fails | Sentinel pauses claiming, retries connection |
| CAS claim fails | `compare_and_swap` returns false | nak message, skip (another sentinel won) |

## Operative (Guest Agent)

Minimal process running inside the VM as PID 1 (or launched by init):

1. Listens on vsock port 5000
2. Receives task payload from sentinel
3. Executes task (runs tools, scripts, agent code)
4. Streams progress back over vsock
5. Sends final result
6. Calls `reboot -f` (signals sentinel that work is done)

The operative is baked into the rootfs image.

---

## Crate Structure

Standalone workspace at `/Users/bear/projects/gbe-sentinel/`.
Depends on gbe-transport and gbe-state-store via path.

### Module Layout

```
gbe-sentinel/
├── Cargo.toml              # workspace root
├── crates/
│   └── sentinel/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs              # pub exports
│           ├── sentinel.rs         # Sentinel struct, run loop, slot tracking
│           ├── config.rs           # SentinelConfig, VmProfile, NetworkPolicy, ToolPolicy
│           ├── error.rs            # SentinelError (thiserror)
│           ├── handler.rs          # MessageHandler impl for task queue messages
│           ├── claim.rs            # CAS claim logic, state store field updates
│           ├── vm/
│           │   ├── mod.rs
│           │   ├── manager.rs      # Firecracker API client (HTTP over Unix socket)
│           │   ├── lifecycle.rs    # provision → run → collect → teardown state machine
│           │   ├── config.rs       # Firecracker boot config builder (vcpus, mem, drives, vsock)
│           │   ├── overlay.rs      # CoW rootfs snapshot create/destroy
│           │   └── network.rs      # tap device + iptables (phase 1), proxy (phase 2)
│           ├── vsock/
│           │   ├── mod.rs
│           │   ├── listener.rs     # accept connections from VMs, demux by CID
│           │   ├── protocol.rs     # GuestMessage / HostMessage serde types
│           │   └── proxy.rs        # tool call proxy (phase 3), CONNECT proxy (phase 2)
│           └── health.rs           # beacon + capacity publisher
└── docs/
    └── design/                     # design notes from initial research
```

### Workspace Cargo.toml

```toml
[workspace]
resolver = "2"
members = ["crates/sentinel"]

[workspace.package]
version = "0.1.0"
edition = "2024"
license = "MIT"
repository = "https://github.com/graybear-io/gbe-sentinel"

[workspace.dependencies]
# External: gbe-transport ecosystem (path deps)
gbe-transport = { path = "../gbe-transport/crates/transport" }
gbe-state-store = { path = "../gbe-transport/crates/state-store" }

# Async
tokio = { version = "1", features = ["full"] }
tokio-util = "0.7"
async-trait = "0.1"

# Observability
tracing = "0.1"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Types
bytes = "1"
ulid = "1"

# Error handling
thiserror = "2"
```

### Crate Cargo.toml

```toml
[package]
name = "gbe-sentinel"
description = "Per-host VM lifecycle manager for GBE task execution"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
gbe-transport.workspace = true
gbe-state-store.workspace = true
async-trait.workspace = true
bytes.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
tokio.workspace = true
tokio-util.workspace = true
tracing.workspace = true
ulid.workspace = true

# Sentinel-specific
nix = { version = "0.29", features = ["socket", "process", "signal", "net"] }
hyper = { version = "1", features = ["client", "http1"] }
hyper-util = { version = "0.1", features = ["tokio"] }
http-body-util = "0.1"
toml = "0.8"
hostname = "0.4"

[dev-dependencies]
gbe-transport-redis = { path = "../../gbe-transport/crates/transport-redis" }
gbe-state-store-redis = { path = "../../gbe-transport/crates/state-store-redis" }
tempfile = "3"
```

### Key Type Signatures

```rust
// config.rs
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
    pub heartbeat_interval: Duration,
    pub bus: TransportConfig,
    pub state: StateStoreConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VmProfile {
    pub vcpus: u32,
    pub mem_mb: u32,
    pub rootfs: String,
    pub timeout_sec: u64,
    pub network: NetworkMode,
    pub network_policy: Option<NetworkPolicy>,
    pub tool_policy: Option<ToolPolicy>,
}

#[derive(Debug, Clone, Deserialize)]
pub enum NetworkMode { Nat, Proxy, None }

// vsock/protocol.rs
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperativeMessage {
    Progress { id: String, step: String, status: String, data: Option<Value> },
    Result { id: String, output: Value, exit_code: i32 },
    Error { id: String, error: String, exit_code: i32 },
    ToolCall { id: String, call_id: String, tool: String, params: Value },
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SentinelMessage {
    Task { id: String, payload: Value, tools: Vec<String> },
    ToolResult { id: String, call_id: String, result: Value },
}

// sentinel.rs — run loop
pub async fn run(&self, token: CancellationToken) -> Result<(), SentinelError> {
    // 1. Subscribe to each configured task type queue
    let subs = self.subscribe_task_queues().await?;

    // 2. Start beacon (heartbeat + capacity publisher)
    let beacon_handle = tokio::spawn(self.beacon_loop(token.clone()));

    // 3. Start vsock listener for all VMs
    let vsock_handle = tokio::spawn(self.vsock_listener(token.clone()));

    // 4. Wait for cancellation
    token.cancelled().await;

    // 5. Graceful shutdown: stop accepting, drain running VMs, unsubscribe
    for sub in subs { sub.unsubscribe().await?; }
    beacon_handle.await??;
    vsock_handle.await??;
    self.drain_running_vms().await?;
    Ok(())
}
```
