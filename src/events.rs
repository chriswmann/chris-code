use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::state::{AppState, Message, Mode, ToolCall, ToolResponse};

/// Evey possible thing that can happen in the application.
pub enum AppEvent {
    Input(InputEvent),
    Llm(LlmEvent),
    App(AppCommand),
}

/// Events from the keyboard/terminal.
pub enum InputEvent {
    Key(KeyEvent),
    Resize(u16, u16),
}

/// Events from the LLM worker thread.
pub enum LlmEvent {
    StreamStart,
    TokenReceived(String),
    StreamComplete,
    StreamError(String),
    ToolCallRequested(ToolCall),
    ToolResponseReady(ToolResponse),
}

/// Internal application commands.
pub enum AppCommand {
    Quit,
}

/// Process an event and update the application state accordingly.
pub fn handle(state: &mut AppState, event: AppEvent) {
    match event {
        AppEvent::Input(input_event) => handle_input(state, &input_event),
        AppEvent::Llm(llm_event) => handle_llm(state, llm_event),
        AppEvent::App(app_command) => handle_app(state, &app_command),
    }
}

fn handle_input(state: &mut AppState, event: &InputEvent) {
    // For now, just handle Ctrl+C to quit
    if let InputEvent::Key(key_event) = event
        && key_event.modifiers.contains(KeyModifiers::CONTROL)
        && key_event.code == KeyCode::Char('c')
    {
        state.mode = Mode::Exiting;
    }
}

fn handle_llm(state: &mut AppState, event: LlmEvent) {
    match event {
        LlmEvent::StreamStart => {
            state.streaming_response = Some(String::new());
        }
        LlmEvent::TokenReceived(token) => {
            if let Some(ref mut response) = state.streaming_response {
                response.push_str(&token);
            }
        }
        LlmEvent::StreamComplete => {
            if let Some(response) = state.streaming_response.take() {
                state.messages.push(Message::Agent(response));
            }
        }
        LlmEvent::StreamError(error) => {
            state.streaming_response = None;
            state
                .messages
                .push(Message::Agent(format!("Error: {error}")));
        }
        LlmEvent::ToolCallRequested(tool_call) => {
            state.messages.push(Message::ToolCall(tool_call));
        }
        LlmEvent::ToolResponseReady(tool_response) => {
            state.messages.push(Message::ToolResponse(tool_response));
        }
    }
}

fn handle_app(state: &mut AppState, command: &AppCommand) {
    match command {
        AppCommand::Quit => {
            state.mode = Mode::Exiting;
        }
    }
}
