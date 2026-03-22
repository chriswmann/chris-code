use std::io::Write;
use std::path::Path;
use std::process;

use anyhow::{Context, Result};
use serde_json::{Value, json};

use crate::state::{ToolCall, ToolResponse};

pub fn execute(call: &ToolCall) -> ToolResponse {
    let content = match call.tool_name.as_str() {
        "Read" => match read_file(call) {
            Ok(response) => response,
            Err(err) => json!(format!("Tool call failed: {err}")),
        },
        "Write" => match write_file(call) {
            Ok(response) => response,
            Err(err) => json!(format!("Tool call failed: {err}")),
        },
        "Bash" => match bash(call) {
            Ok(response) => response,
            Err(err) => json!(format!("Tool call failed: {err}")),
        },
        _ => json!(format!("Unknown tool: {}", call.tool_name)),
    };
    ToolResponse {
        tool_call_id: call.tool_call_id.clone(),
        tool_name: call.tool_name.clone(),
        content,
    }
}

fn read_file(call: &ToolCall) -> Result<Value> {
    let file_path = call.tool_input["file_path"]
        .as_str()
        .context("Should have a `file_path` argument.")?;
    let file_contents = read_file_to_string(file_path)?;
    Ok(json!(file_contents))
}

fn write_file(call: &ToolCall) -> Result<Value> {
    let file_path = call.tool_input["file_path"]
        .as_str()
        .context("Should have a `file_path` argument.")?;
    let content = call.tool_input["content"]
        .as_str()
        .context("Should have a `content` argument.")?;
    write_to_file(file_path, content)?;
    Ok(json!(format!("File written to {file_path} successfully")))
}

fn bash(call: &ToolCall) -> Result<Value> {
    let command = call.tool_input["command"]
        .as_str()
        .context("Should have a `command` argument.")?;
    let output = execute_bash_command(command)?;
    Ok(json!(output))
}

fn read_file_to_string(path: impl AsRef<Path>) -> Result<String> {
    let file_contents = std::fs::read_to_string(&path)?;
    eprintln!("Read file: {}", path.as_ref().display());
    Ok(file_contents)
}

fn write_to_file(path: impl AsRef<Path>, contents: &str) -> Result<()> {
    let mut file = std::fs::File::create(path)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}

fn execute_bash_command(command: &str) -> Result<String> {
    let output = process::Command::new("bash")
        .arg("-c")
        .arg(command)
        .output()?;
    let stdout = String::from_utf8_lossy(output.stdout.trim_ascii());
    let stderr = String::from_utf8_lossy(output.stderr.trim_ascii());
    Ok(format!("stdout:{stdout}\nstderr:{stderr}"))
}
