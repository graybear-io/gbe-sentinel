use std::sync::Arc;

use bytes::Bytes;
use gbe_state_store::StateStore;

use crate::error::SentinelError;

/// Attempts a CAS claim on a task in the state store.
///
/// Flow: `compare_and_swap(key, "state", "pending", "claimed")`
///   - Success → set worker, `updated_at`, `timeout_at`
///   - Failure → return `ClaimFailed` error
///
/// # Errors
///
/// Returns `SentinelError::ClaimFailed` if the CAS fails, or a store error on I/O failure.
///
/// # Panics
///
/// Panics if the system clock is before the Unix epoch.
pub async fn claim_task(
    store: &Arc<dyn StateStore>,
    state_key: &str,
    host_id: &str,
    vm_cid: u32,
    timeout_at: u64,
) -> Result<(), SentinelError> {
    let claimed = store
        .compare_and_swap(
            state_key,
            "state",
            Bytes::from("pending"),
            Bytes::from("claimed"),
        )
        .await?;

    if !claimed {
        return Err(SentinelError::ClaimFailed {
            task_id: state_key.to_string(),
            reason: "CAS failed — task already claimed".to_string(),
        });
    }

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis()
        .to_string();

    let worker = format!("{host_id}:{vm_cid}");

    store
        .set_fields(
            state_key,
            std::collections::HashMap::from([
                ("worker".to_string(), Bytes::from(worker)),
                ("updated_at".to_string(), Bytes::from(now.clone())),
                (
                    "timeout_at".to_string(),
                    Bytes::from(timeout_at.to_string()),
                ),
            ]),
        )
        .await?;

    Ok(())
}
