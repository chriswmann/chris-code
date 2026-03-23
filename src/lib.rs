use anyhow::Result;
use async_openai::config::OpenAIConfig;
use clap::Parser;
use std::env;
use tracing::warn;

#[derive(Parser)]
#[command(author, version, about)]
pub struct Args {
    #[arg(short = 'p', long)]
    pub prompt: String,

    #[arg(short = 'm', long)]
    pub model: Option<String>,
}

pub struct App {
    pub exit: bool,
}
