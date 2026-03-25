mod events;
mod input;
mod llm;
mod state;
mod tools;
mod ui;

use anyhow::Result;
use chris_code::Args;
use clap::Parser;
use crossterm::{
    ExecutableCommand, execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, prelude::CrosstermBackend};
use state::AppState;

use std::{fs, panic, sync::mpsc, thread};
use tracing_subscriber::{
    EnvFilter, Registry, fmt::layer, layer::SubscriberExt, util::SubscriberInitExt,
};

use crate::{events::AppEvent, state::Mode};

fn main() -> Result<()> {
    // Setup tracing subscriber - file and console
    fs::create_dir_all("./logs")?;
    let log_file = fs::File::create("./logs/app.log")?;
    let file_layer = layer().with_writer(log_file);
    let console_layer = layer();

    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(file_layer)
        .with(console_layer)
        .init();

    // Parse args
    let args = Args::parse();

    // Setup the terminal
    enable_raw_mode()?;
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original_hook(panic_info);
    }));
    let mut stdout = std::io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create the app state
    let mut app_state = AppState::new();

    // Create the channel for worker threads to send messages to
    let (tx, rx) = mpsc::channel::<AppEvent>();

    // Spawn worker threads
    let input_tx = tx.clone();
    thread::spawn(move || input::run(&input_tx));

    let llm_tx = tx.clone();
    thread::spawn(move || llm::run(llm_tx, args));

    // Run the event loop
    loop {
        // Draw the UI
        terminal.draw(|frame| ui::render(frame, &app_state))?;

        // Wait for the next event (blocks)
        let event = rx.recv()?;

        // Update state based on the event
        events::handle(&mut app_state, event);

        // Check if we should exit
        if matches!(app_state.mode, Mode::Exiting) {
            break;
        }

        // Restore the terminal
        disable_raw_mode()?;
        execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    }

    Ok(())
}
