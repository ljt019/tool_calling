use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;

use crate::ToolHandler;

/// Request structure for the Ollama API
#[derive(Debug, Serialize, Deserialize)]
pub struct OllamaRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_results: Option<Vec<ToolResult>>,
    pub stream: bool,
}

/// Chat message structure for LLM communication
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// Tool call description from LLM
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolCall {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub function: ToolFunction,
}

/// Function call description
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ToolFunction {
    pub name: String,
    pub arguments: Value,
}

/// Tool execution result
#[derive(Debug, Serialize, Deserialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub result: String,
}

/// Response from Ollama API
#[derive(Debug, Deserialize)]
pub struct OllamaResponse {
    pub created_at: String,
    pub done: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_reason: Option<String>,
    pub model: String,
    pub message: ChatMessage,
}

/// Have a complete conversation with the LLM, handling any tool calls
pub async fn have_conversation_with_llm(
    model: &str,
    prompt: &str,
    tools: &Value,
    tool_handler: &ToolHandler,
) -> Result<Value, Box<dyn std::error::Error>> {
    let client = Client::new();

    // Ollama endpoint (default for local installation)
    let ollama_api =
        env::var("OLLAMA_API").unwrap_or_else(|_| "http://localhost:11434/api".to_string());

    // Initialize conversation
    let mut messages = vec![
        ChatMessage {
            role: "system".to_string(),
            content: "You are a helpful assistant that uses tools when appropriate.".to_string(),
            tool_calls: None,
        },
        ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
            tool_calls: None,
        },
    ];

    // Start conversation loop
    loop {
        println!("\n┌─ Sending message to Ollama API");
        println!("└─ Endpoint: {}", ollama_api);

        // Create request
        let request = OllamaRequest {
            model: model.to_string(),
            messages: messages.clone(),
            tools: Some(tools.clone()),
            tool_results: None,
            stream: false,
        };

        // Send request to Ollama
        let response = client
            .post(format!("{}/chat", ollama_api))
            .json(&request)
            .send()
            .await?;

        // Parse response
        let ollama_response: OllamaResponse = response.json().await?;
        println!("\n┌─ LLM Response");
        println!("│");
        if !ollama_response.message.content.is_empty() {
            // Split response into lines for better readability
            for line in ollama_response.message.content.lines() {
                println!("│ {}", line);
            }
        }
        println!("└─");

        // Check for tool calls
        if let Some(tool_calls) = &ollama_response.message.tool_calls {
            if !tool_calls.is_empty() {
                println!("\n┌─ LLM Requested Tool Calls: {}", tool_calls.len());

                let mut tool_results = Vec::new();

                // Execute each tool call
                for tool_call in tool_calls {
                    let tool_id = tool_call.id.clone().unwrap_or_else(|| "call-1".to_string());
                    let function_name = &tool_call.function.name;
                    let args = &tool_call.function.arguments;

                    println!("│");
                    println!("│ Tool: {}", function_name);
                    println!("│ ID: {}", tool_id);
                    println!("│ Arguments:");

                    // Format arguments with indentation
                    let args_str = serde_json::to_string_pretty(args).unwrap();
                    for line in args_str.lines() {
                        println!("│   {}", line);
                    }

                    // Format the tool call with parameters for our handler
                    let formatted_tool_call =
                        if let Some(tool) = tool_handler.get_tool(function_name) {
                            let param_schema = tool.parameter_schema.clone();
                            json!({
                                "type": "function",
                                "function": {
                                    "name": function_name,
                                    "arguments": args,
                                    "parameters": param_schema
                                }
                            })
                        } else {
                            json!({
                                "type": "function",
                                "function": {
                                    "name": function_name,
                                    "arguments": args
                                }
                            })
                        };

                    // Call the tool and get the result
                    let result = match tool_handler.call_tool(&formatted_tool_call) {
                        Ok(output) => {
                            println!("│ Result: {}", output);
                            output
                        }
                        Err(err) => {
                            println!("│ Error: {}", err);
                            format!("Error: {}", err)
                        }
                    };

                    // Add to tool results
                    tool_results.push(ToolResult {
                        tool_call_id: tool_id,
                        result,
                    });
                }
                println!("└─");

                // Add assistant's response to messages
                messages.push(ollama_response.message);

                // Add tool results message
                messages.push(ChatMessage {
                    role: "tool".to_string(),
                    content: "".to_string(),
                    tool_calls: None,
                });

                // Continue the conversation with tool results
                println!("\n┌─ Sending Tool Results to LLM");
                println!("└─ Tool results count: {}", tool_results.len());
                let request_with_results = OllamaRequest {
                    model: model.to_string(),
                    messages: messages.clone(),
                    tools: Some(tools.clone()),
                    tool_results: Some(tool_results),
                    stream: false,
                };

                let response = client
                    .post(format!("{}/chat", ollama_api))
                    .json(&request_with_results)
                    .send()
                    .await?;

                let final_response: OllamaResponse = response.json().await?;
                println!("\n┌─ Final LLM Response");
                println!("│");
                // Split response into lines for better readability
                for line in final_response.message.content.lines() {
                    println!("│ {}", line);
                }
                println!("└─");

                // Add final response to conversation history
                messages.push(final_response.message);

                // Return the final response
                return Ok(json!(messages));
            }
        }

        // If no tool calls, add the response to history and return
        messages.push(ollama_response.message);
        return Ok(json!(messages));
    }
}

/// Create a mock tool call for testing purposes
pub fn create_mock_tool_call(function_name: &str, arguments: Value) -> Value {
    json!([{
        "id": "mock-call-1",
        "function": {
            "name": function_name,
            "arguments": arguments
        }
    }])
}

/// Process a mock tool call for demonstration purposes
pub fn process_mock_tool_call(
    tool_call: &Value,
    tool_handler: &ToolHandler,
) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(tool_calls) = tool_call.as_array() {
        println!("Processing {} tool call(s):", tool_calls.len());

        for (i, call) in tool_calls.iter().enumerate() {
            println!("┌─ Tool Call #{}", i + 1);

            if let Some(function) = call.get("function") {
                if let (Some(name), Some(args)) = (
                    function.get("name").and_then(|n| n.as_str()),
                    function.get("arguments"),
                ) {
                    println!("│ Function: {}", name);
                    println!("│ Arguments:");
                    // Format arguments with indentation
                    let args_str = serde_json::to_string_pretty(args).unwrap();
                    for line in args_str.lines() {
                        println!("│   {}", line);
                    }

                    // Format the tool call with parameters for our handler
                    let formatted_call = if let Some(tool) = tool_handler.get_tool(name) {
                        let param_schema = tool.parameter_schema.clone();
                        json!({
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": args,
                                "parameters": param_schema
                            }
                        })
                    } else {
                        json!({
                            "type": "function",
                            "function": {
                                "name": name,
                                "arguments": args
                            }
                        })
                    };

                    // Use the handler to execute the tool
                    match tool_handler.call_tool(&formatted_call) {
                        Ok(result) => println!("│ Result: {}", result),
                        Err(err) => println!("│ Error: {}", err),
                    }
                }
            }
            println!("└─");
        }
    }

    Ok(())
}
