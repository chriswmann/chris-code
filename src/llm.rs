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
