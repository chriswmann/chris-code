mod events;
mod input;
mod llm;
mod state;
mod tools;
mod ui;

use anyhow::Result;
use chris_code::Args;
use clap::Parser;
use llm::run;
use state::AppState;

use std::fs;
use tracing_subscriber::{
    EnvFilter, Registry, fmt::layer, layer::SubscriberExt, util::SubscriberInitExt,
};

fn main() -> Result<()> {
    fs::create_dir_all("./logs")?;
    let log_file = fs::File::create("./logs/app.log")?;
    let file_layer = layer().with_writer(log_file);
    let console_layer = layer();

    Registry::default()
        .with(EnvFilter::from_default_env())
        .with(file_layer)
        .with(console_layer)
        .init();

    let args = Args::parse();
    let mut app_state = AppState::new();

    Ok(())
}
