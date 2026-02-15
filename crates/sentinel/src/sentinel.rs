use std::sync::Arc;

use gbe_nexus::Transport;
use gbe_state_store::StateStore;
use tokio_util::sync::CancellationToken;

use crate::config::SentinelConfig;
use crate::error::SentinelError;

pub struct SlotTracker {
    pub total: u32,
    pub used: u32,
}

impl SlotTracker {
    pub fn new(total: u32) -> Self {
        Self { total, used: 0 }
    }

    pub fn available(&self) -> u32 {
        self.total.saturating_sub(self.used)
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
