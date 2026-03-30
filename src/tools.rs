use std::path::Path;
use tokio::process::Command;

use anyhow::{Context, Result};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

use crate::state::{ToolCall, ToolResponse};

pub async fn execute(call: &ToolCall) -> ToolResponse {
    let content = match call.name.as_str() {
        "Read" => match read_file(call).await {
            Ok(response) => response,
            Err(err) => json!(format!("Tool call failed: {err}")),
        },
        "Write" => match write_file(call).await {
            Ok(response) => response,
            Err(err) => json!(format!("Tool call failed: {err}")),
        },
        "Bash" => match bash(call).await {
            Ok(response) => response,
            Err(err) => json!(format!("Tool call failed: {err}")),
        },
        _ => json!(format!("Unknown tool: {}", call.name)),
    };
    ToolResponse {
        tool_call_id: call.call_id.clone(),
        tool_name: call.name.clone(),
        content,
    }
}

async fn read_file(call: &ToolCall) -> Result<Value> {
    let file_path = call.input["file_path"]
        .as_str()
        .context("Should have a `file_path` argument.")?;
    let file_contents = read_file_to_string(file_path).await?;
    Ok(json!(file_contents))
}

async fn write_file(call: &ToolCall) -> Result<Value> {
    let file_path = call.input["file_path"]
        .as_str()
        .context("Should have a `file_path` argument.")?;
    let content = call.input["content"]
        .as_str()
        .context("Should have a `content` argument.")?;
    write_to_file(file_path, content).await?;
    Ok(json!(format!("File written to {file_path} successfully")))
}

async fn bash(call: &ToolCall) -> Result<Value> {
    let command = call.input["command"]
        .as_str()
        .context("Should have a `command` argument.")?;
    let output = execute_bash_command(command).await?;
    Ok(json!(output))
}

async fn read_file_to_string(path: impl AsRef<Path>) -> Result<String> {
    let file_contents = tokio::fs::read_to_string(&path).await?;
    eprintln!("Read file: {}", path.as_ref().display());
    Ok(file_contents)
}

async fn write_to_file(path: impl AsRef<Path>, contents: &str) -> Result<()> {
    let mut file = tokio::fs::File::create(path).await?;
    file.write_all(contents.as_bytes()).await?;
    Ok(())
}

async fn execute_bash_command(command: &str) -> Result<String> {
    let output = Command::new("bash").arg("-c").arg(command).output().await?;
    let stdout = String::from_utf8_lossy(output.stdout.trim_ascii());
    let stderr = String::from_utf8_lossy(output.stderr.trim_ascii());
    Ok(format!("stdout:{stdout}\nstderr:{stderr}"))
}
