use crate::error::SentinelError;

/// Handles incoming task queue messages.
///
/// On receipt: extract state key, attempt CAS claim, provision VM on success.
pub struct TaskHandler;

impl TaskHandler {
    /// # Errors
    ///
    /// Returns `SentinelError` on claim failure or VM provisioning error.
    pub async fn handle_message(&self, _payload: &[u8]) -> Result<(), SentinelError> {
        // TODO: implement message handling
        // 1. Deserialize envelope, extract state key
        // 2. CAS claim via claim module
        // 3. On success: provision VM, inject task
        // 4. On failure: nak message
        Ok(())
    }
}
