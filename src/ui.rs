use ratatui::Frame;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::Text;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::state::{AppState, Message};

/// Draw the entire UI based on the current application state.
pub fn render(frame: &mut Frame, state: &AppState) {
    // Split the screen into two areas: messages(top) and input bar (bottom)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(3)])
        .split(frame.area());

    render_messages(frame, chunks[0], state);
    render_input_bar(frame, chunks[1]);
}

fn render_messages(frame: &mut Frame, area: Rect, state: &AppState) {
    let mut lines = Vec::new();

    for message in &state.messages {
        match message {
            Message::User(text) => {
                lines.push(format!("You: {text}"));
            }
            Message::Agent(text) => {
                lines.push(format!("Agent: {text}"));
            }
            Message::ToolCall(tc) => {
                lines.push(format!("🔧 Tool call: {}", tc.tool_name));
            }
            Message::ToolResponse(tr) => {
                lines.push(format!("📎 Tool response: {}", tr.tool_name));
            }
        }
    }

    // If there's an in-progress streaming response, show it.
    if let Some(ref partial) = state.streaming_response {
        lines.push(format!("Agent: {partial}"));
    }

    let text = Text::from(lines.join("\n"));
    let messages_widget = Paragraph::new(text)
        .block(Block::default().borders(Borders::ALL).title("Messages"))
        .wrap(Wrap { trim: false });

    frame.render_widget(messages_widget, area);
}

fn render_input_bar(frame: &mut Frame, area: Rect) {
    let input_widget = Paragraph::new("Type your message...")
        .block(Block::default().borders(Borders::ALL).title("Input"))
        .style(Style::default().fg(Color::DarkGray));

    frame.render_widget(input_widget, area);
}
