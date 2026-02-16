use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::SentinelError;

/// Maximum size of a single vsock message in bytes (1 MB).
const MAX_VSOCK_MESSAGE_SIZE: usize = 1_048_576;

/// Messages sent from operative (guest) to sentinel (host) over vsock.
///
/// Fields using `Value` (`data`, `output`, `params`) are intentionally
/// untyped — their schemas vary by task type and tool. Validation
/// happens downstream in task-specific handlers, not at the protocol layer.
/// Size limits are enforced at deserialization time via `parse_operative_message`.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OperativeMessage {
    Progress {
        id: String,
        step: String,
        status: String,
        #[serde(default)]
        data: Option<Value>,
    },
    Result {
        id: String,
        output: Value,
        exit_code: i32,
    },
    Error {
        id: String,
        error: String,
        exit_code: i32,
    },
    ToolCall {
        id: String,
        call_id: String,
        tool: String,
        params: Value,
    },
}

/// Messages sent from sentinel (host) to operative (guest) over vsock.
///
/// `payload` and `result` use `Value` because task payloads and tool
/// results vary by type. Size limits are enforced at serialization time.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SentinelMessage {
    Task {
        id: String,
        payload: Value,
        tools: Vec<String>,
    },
    ToolResult {
        id: String,
        call_id: String,
        result: Value,
    },
}

/// Deserialize an operative message with size limit enforcement.
pub fn parse_operative_message(raw: &[u8]) -> Result<OperativeMessage, SentinelError> {
    if raw.len() > MAX_VSOCK_MESSAGE_SIZE {
        return Err(SentinelError::Vsock(format!(
            "message too large: {} bytes (max {})",
            raw.len(),
            MAX_VSOCK_MESSAGE_SIZE
        )));
    }
    serde_json::from_slice(raw).map_err(|e| SentinelError::Vsock(format!("invalid message: {e}")))
}
