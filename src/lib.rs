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

/// # Errors
///
/// Raises an error if the environment variables cannot be read.
pub fn get_openai_config() -> Result<OpenAIConfig> {
    let base_url = env::var("OPEN_ROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPEN_ROUTER_API_KEY")?;

    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    Ok(config)
}

#[must_use]
pub fn get_model(args: &Args) -> String {
    args.model
        .clone()
        .unwrap_or_else(|| match env::var("MODEL") {
            Ok(model) => model.clone(),
            Err(err) => {
                warn!("Using openrouter/hunter-alpha as no model set via arg or env var ({err})");
                "openrouter/hunter-alpha".to_string()
            }
        })
}
