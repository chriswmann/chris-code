use crate::events::{AppEvent, LlmEvent};
use crate::state::{ToolCall, ToolResponse};
use crate::tools::execute;
use anyhow::{Context, Result, bail};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::chat::{
    ChatCompletionMessageToolCalls, ChatCompletionRequestAssistantMessageArgs,
    ChatCompletionRequestMessage, ChatCompletionRequestToolMessage,
    ChatCompletionRequestToolMessageContent, ChatCompletionRequestUserMessage, ChatCompletionTool,
    ChatCompletionTools, CreateChatCompletionRequest, CreateChatCompletionRequestArgs,
    FunctionObjectArgs,
};
use serde_json::{Value, json};
use std::env;
use std::sync::mpsc::Sender;
use tokio::sync::mpsc::UnboundedReceiver;

pub async fn run(
    model: String,
    tx: Sender<AppEvent>,
    mut rx: UnboundedReceiver<String>,
) -> Result<()> {
    let config = get_openai_config()?;

    let client = Client::with_config(config);

    let tools = load_tools()?;
    let mut request = CreateChatCompletionRequestArgs::default()
        .max_completion_tokens(128_u32)
        .model(&model)
        .tools(tools.clone())
        .build()?;
    loop {
        let user_prompt = rx.recv().await.unwrap();
        request.messages.push(ChatCompletionRequestMessage::User(
            ChatCompletionRequestUserMessage::from(user_prompt),
        ));
        let response = client.chat().create(request.clone()).await?;
        let response_message = response.choices.first().context("No choices")?;

        if let Some(ref tool_calls) = response_message.message.tool_calls {
            let num_tool_calls = &tool_calls.len();
            let mut executed_tool_calls: Vec<ChatCompletionMessageToolCalls> =
                Vec::with_capacity(*num_tool_calls);
            let mut tool_responses = Vec::with_capacity(*num_tool_calls);
            for tool_call_enum in executed_tool_calls.clone() {
                // Extract the function tool call from the enum.
                if let ChatCompletionMessageToolCalls::Function(tool_call) = tool_call_enum.clone()
                {
                    let tc = ToolCall {
                        call_id: tool_call.id.clone(),
                        name: tool_call.function.name.clone(),
                        input: json!(tool_call.function.arguments),
                    };
                    tx.send(AppEvent::Llm(LlmEvent::ToolCallRequested(tc.clone())))?;
                    let tool_response = execute(&tc).await;
                    executed_tool_calls.push(tool_call_enum.clone());
                    tool_responses.push(tool_response);
                }
            }
            append_tool_responses_to_chat(&mut request, &executed_tool_calls, &tool_responses)?;
        } else if let Some(message) = &response_message.message.content {
            tx.send(AppEvent::Llm(LlmEvent::TokenReceived(message.clone())))?;
            tx.send(AppEvent::Llm(LlmEvent::StreamComplete))?;
            break;
        } else {
            bail!("Response had neither tool calls nor content");
        }
    }
    Ok(())
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

fn load_tools() -> Result<Vec<ChatCompletionTools>> {
    let read_tool_parameters =
        serde_json::from_str(include_str!("tool_definitions/read_tool_params.json")).unwrap();
    let read_tool = tool_definition_factory(
        "Read",
        "Read and return the contents of a file. Takes a `file_path` argument.",
        read_tool_parameters,
    )?;
    let write_tool_parameters =
        serde_json::from_str(include_str!("tool_definitions/write_tool_params.json")).unwrap();
    let write_tool = tool_definition_factory(
        "Write",
        "Write contents to a file. Takes `file_path` and `content` arguments.",
        write_tool_parameters,
    )?;

    let bash_tool_parameters =
        serde_json::from_str(include_str!("tool_definitions/bash_tool_params.json")).unwrap();
    let bash_tool = tool_definition_factory(
        "Bash",
        "Execute a shell command. Takes a `command` argument.",
        bash_tool_parameters,
    )?;
    Ok(vec![
        ChatCompletionTools::Function(read_tool),
        ChatCompletionTools::Function(write_tool),
        ChatCompletionTools::Function(bash_tool),
    ])
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

fn append_tool_responses_to_chat(
    request: &mut CreateChatCompletionRequest,
    tool_calls: &[ChatCompletionMessageToolCalls],
    tool_responses: &[ToolResponse],
) -> Result<()> {
    // Convert ChatCompletionMessageToolCall to ChatCompletionMessageToolCalls enum
    // Build ChatCompletionMessageToolCalls enum from ToolResponse.
    // We need a ChatCompletionMessageToolCall struct, with a FunctionCall struct as the function field
    // in order to do this.
    let assistant_message: ChatCompletionRequestMessage =
        ChatCompletionRequestAssistantMessageArgs::default()
            .tool_calls(tool_calls)
            .build()?
            .into();

    let tool_messages: Vec<ChatCompletionRequestMessage> = tool_responses
        .iter()
        .map(|r| {
            ChatCompletionRequestMessage::Tool(ChatCompletionRequestToolMessage {
                content: ChatCompletionRequestToolMessageContent::Text(r.content.to_string()),
                tool_call_id: r.tool_call_id.clone(),
            })
        })
        .collect();

    request.messages.push(assistant_message);
    request.messages.extend(tool_messages);
    Ok(())
}
