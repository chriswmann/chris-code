use std::io::Write;
use std::path::Path;
use std::process;

use anyhow::{Context, Result};
use async_openai::types::chat::{ChatCompletionMessageToolCall, ChatCompletionTool, FunctionObjectArgs};
use serde_json::{Value, json};


fn call_read_tool<'tool_call>(
    tool_call: &'tool_call ChatCompletionMessageToolCall,
    args: &Value,
    function_responses: &mut Vec<(&'tool_call ChatCompletionMessageToolCall, Value)>,
) -> Result<()> {
    let file_path = args["file_path"]
        .as_str()
        .context("Should have a `file_path` argument.")?;
    let file_contents = read_file_to_string(file_path)?;
    eprintln!("file contents: {file_contents}");
    let file_contents = json!(&file_contents);
    function_responses.push((tool_call, file_contents));
    Ok(())
}

fn call_write_tool<'tool_call>(
    tool_call: &'tool_call ChatCompletionMessageToolCall,
    args: &Value,
    function_responses: &mut Vec<(&'tool_call ChatCompletionMessageToolCall, Value)>,
) -> Result<()> {
    let file_path = args["file_path"]
        .as_str()
        .context("Should have a `file_path` argument.")?;
    let content = args["content"]
        .as_str()
        .context("Should have a `content` argument.")?;
    write_to_file(file_path, content)?;
    let new_file_value = json!(content);
    function_responses.push((tool_call, new_file_value));
    Ok(())
}

fn call_bash_tool<'tool_call>(
    tool_call: &'tool_call ChatCompletionMessageToolCall,
    args: &Value,
    function_responses: &mut Vec<(&'tool_call ChatCompletionMessageToolCall, Value)>,
) -> Result<()> {
    let command = args["command"]
        .as_str()
        .context("Should have a `command` argument.")?;
    let output = execute_bash_command(command)?;
    function_responses.push((tool_call, json!(output)));
    Ok(())
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

fn tool_definition_factory(
    name: &str,
    description: &str,
    parameters: Value,
) -> Result<ChatCompletionTool> {
    let chat_completion_tool = ChatCompletionTool {
        function: FunctionObjectArgs::default()
            .name(name)
            .description(description)
            .parameters(parameters)
            .build()?,
    };
    Ok(chat_completion_tool)
}
