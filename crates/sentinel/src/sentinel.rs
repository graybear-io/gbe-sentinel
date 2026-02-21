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
    #[must_use]
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
    /// # Errors
    ///
    /// Returns `SentinelError::Config` if config validation fails.
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

    /// # Errors
    ///
    /// Returns `SentinelError` on transport or state store failures.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_tracker_has_full_capacity() {
        let t = SlotTracker::new(4);
        assert_eq!(t.available(), 4);
    }

    #[test]
    fn claim_reduces_available() {
        let t = SlotTracker::new(2);
        assert!(t.try_claim());
        assert_eq!(t.available(), 1);
    }

    #[test]
    fn claim_at_capacity_fails() {
        let t = SlotTracker::new(1);
        assert!(t.try_claim());
        assert!(!t.try_claim());
        assert_eq!(t.available(), 0);
    }

    #[test]
    fn release_restores_capacity() {
        let t = SlotTracker::new(1);
        assert!(t.try_claim());
        t.release();
        assert_eq!(t.available(), 1);
        assert!(t.try_claim());
    }

    #[test]
    fn zero_slots_never_claims() {
        let t = SlotTracker::new(0);
        assert_eq!(t.available(), 0);
        assert!(!t.try_claim());
    }

    #[test]
    fn concurrent_claims_respect_limit() {
        let tracker = Arc::new(SlotTracker::new(3));
        let mut handles = vec![];

        for _ in 0..10 {
            let t = Arc::clone(&tracker);
            handles.push(std::thread::spawn(move || t.try_claim()));
        }

        let successes: usize = handles
            .into_iter()
            .map(|h| h.join().unwrap())
            .filter(|&claimed| claimed)
            .count();
        assert_eq!(successes, 3);
        assert_eq!(tracker.available(), 0);
    }
}
