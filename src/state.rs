use serde_json::Value;

/// The running mode of the application
pub enum Mode {
    Running,
    Exiting,
}

/// A single message in the conversation history
pub enum Message {
    User(String),
    Agent(String),
    ToolCall(ToolCall),
    ToolResponse(ToolResponse),
}

/// Describes a tool call the LLM wants to make
pub struct ToolCall {
    pub tool_call_id: String,
    pub tool_name: String,
    pub tool_input: Value,
}

/// The result of executing a tool
pub struct ToolResponse {
    pub tool_call_id: String,
    pub tool_name: String,
    pub content: Value,
}

/// The complete application state. Only the main thread owns this.
pub struct AppState {
    pub messages: Vec<Message>,
    pub streaming_response: Option<String>,
    pub mode: Mode,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            streaming_response: None,
            mode: Mode::Running,
        }
    }
}
