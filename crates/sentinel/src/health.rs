use crate::error::SentinelError;

/// Publishes periodic heartbeat beacons and capacity updates.
///
/// Beacon: `gbe.events.sentinel.{host_id}.health`
/// Capacity: `gbe.events.sentinel.{host_id}.capacity`
pub struct HealthPublisher;

impl HealthPublisher {
    pub async fn publish_beacon(&self) -> Result<(), SentinelError> {
        // TODO: publish heartbeat to gbe.events.sentinel.{host_id}.health
        Ok(())
    }

    pub async fn publish_capacity(
        &self,
        _total: u32,
        _used: u32,
    ) -> Result<(), SentinelError> {
        // TODO: publish slot availability to gbe.events.sentinel.{host_id}.capacity
        Ok(())
    }
}
