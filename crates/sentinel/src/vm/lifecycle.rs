/// VM lifecycle state machine.
///
/// ```text
/// IDLE → PROVISIONING → RUNNING → COLLECTING → TEARDOWN → IDLE
///              │             │           │
///              ▼             ▼           ▼
///           FAILED        TIMEOUT     FAILED
///              │             │           │
///              └──── all ────┴→ TEARDOWN ┘
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VmState {
    Idle,
    Provisioning,
    Running,
    Collecting,
    Teardown,
    Failed(String),
    Timeout,
}

pub struct VmLifecycle {
    pub state: VmState,
    pub task_id: Option<String>,
}

impl Default for VmLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl VmLifecycle {
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: VmState::Idle,
            task_id: None,
        }
    }

    pub fn transition(&mut self, next: VmState) {
        tracing::info!(from = ?self.state, to = ?next, task = ?self.task_id, "vm state transition");
        self.state = next;
    }
}
