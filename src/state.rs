use serde_json::Value;
use tokio::sync::mpsc::UnboundedSender;

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
#[derive(Clone)]
pub struct ToolCall {
    pub call_id: String,
    pub name: String,
    pub input: Value,
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
    pub main_tx: UnboundedSender<String>,
    pub user_input_buffer: String,
    pub streaming_response: Option<String>,
    pub mode: Mode,
}

impl AppState {
    pub fn new(main_tx: UnboundedSender<String>, user_input_buffer: &str) -> Self {
        Self {
            messages: Vec::new(),
            main_tx,
            user_input_buffer: user_input_buffer.into(),
            streaming_response: None,
            mode: Mode::Running,
        }
    }
}
