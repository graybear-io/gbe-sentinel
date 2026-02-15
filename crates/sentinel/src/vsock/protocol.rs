use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Messages sent from operative (guest) to sentinel (host) over vsock.
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
