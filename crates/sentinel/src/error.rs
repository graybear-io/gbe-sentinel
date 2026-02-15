use std::io;

#[derive(Debug, thiserror::Error)]
pub enum SentinelError {
    #[error("transport error: {0}")]
    Transport(#[from] gbe_transport::TransportError),

    #[error("state store error: {0}")]
    StateStore(#[from] gbe_state_store::StateStoreError),

    #[error("vm error: {0}")]
    Vm(String),

    #[error("vsock error: {0}")]
    Vsock(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("claim failed for task {task_id}: {reason}")]
    ClaimFailed { task_id: String, reason: String },

    #[error("timeout: task {0} exceeded deadline")]
    Timeout(String),

    #[error("io error: {0}")]
    Io(#[from] io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
