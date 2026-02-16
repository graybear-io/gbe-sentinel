use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use gbe_nexus::Transport;
use gbe_state_store::StateStore;
use tokio_util::sync::CancellationToken;

use crate::config::SentinelConfig;
use crate::error::SentinelError;

/// Tracks VM slot usage with atomic operations. Safe to share across
/// concurrent task handlers without external locking.
pub struct SlotTracker {
    total: u32,
    used: AtomicU32,
}

impl SlotTracker {
    pub fn new(total: u32) -> Self {
        Self {
            total,
            used: AtomicU32::new(0),
        }
    }

    pub fn available(&self) -> u32 {
        self.total.saturating_sub(self.used.load(Ordering::Acquire))
    }

    /// Try to claim a slot. Returns true if successful.
    pub fn try_claim(&self) -> bool {
        self.used
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |current| {
                if current < self.total {
                    Some(current + 1)
                } else {
                    None
                }
            })
            .is_ok()
    }

    /// Release a previously claimed slot.
    pub fn release(&self) {
        self.used.fetch_sub(1, Ordering::Release);
    }
}

#[allow(dead_code)]
pub struct Sentinel {
    pub(crate) config: SentinelConfig,
    pub(crate) transport: Arc<dyn Transport>,
    pub(crate) store: Arc<dyn StateStore>,
    pub(crate) slots: SlotTracker,
}

impl Sentinel {
    pub async fn new(
        config: SentinelConfig,
        transport: Arc<dyn Transport>,
        store: Arc<dyn StateStore>,
    ) -> Result<Self, SentinelError> {
        config.validate()?;
        let slots = SlotTracker::new(config.slots);
        Ok(Self {
            config,
            transport,
            store,
            slots,
        })
    }

    pub async fn run(&self, _token: CancellationToken) -> Result<(), SentinelError> {
        // TODO: implement run loop
        // 1. Subscribe to each configured task type queue
        // 2. Start beacon (heartbeat + capacity publisher)
        // 3. Start vsock listener for all VMs
        // 4. Wait for cancellation
        // 5. Graceful shutdown: stop accepting, drain running VMs, unsubscribe
        Ok(())
    }
}
