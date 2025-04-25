# Tool Calling Library for Rust

A simple and flexible library for defining and calling tools (functions) via JSON in Rust. This library enables you to expose Rust functions as "tools" that can be called via JSON objects, especially useful when working with LLMs.

## Features

- Define tools using a simple `#[tool]` attribute
- Support for any number of arguments in tool functions
- Automatic JSON schema generation for tools
- Built-in Ollama integration for LLM-based tool calling

## Usage

### Defining a Tool

```rust
use tool_calling::tool;

/// Calculate BMI (Body Mass Index)
#[tool]
pub fn calculate_bmi(weight_kg: f32, height_m: f32) -> String {
    if height_m <= 0.0 || weight_kg <= 0.0 {
        return "Error: Weight and height must be positive values".to_string();
    }
    let bmi = weight_kg / (height_m * height_m);
    format!("BMI: {:.1}", bmi)
}
```

### Calling a Tool

When working with LLMs, you'll typically get a function call payload that includes just the function name and arguments. The library's `call_tool` method will automatically look up registered tools and use their parameter schema.

```rust
use serde_json::{json, Value};
use tool_calling::ToolHandler;

fn main() {
    let tool_handler = ToolHandler::new();
    
    // When an LLM generates a function call, it typically looks like this:
    let raw_function_call = r#"{
        "type": "function",
        "function": {
            "name": "calculate_bmi",
            "arguments": {
                "weight_kg": 70,
                "height_m": 1.75
            }
        }
    }"#;
    
    // Parse the model's function call
    let model_function_call: Value = serde_json::from_str(raw_function_call).unwrap();
    
    // Call the tool directly with the model's output
    // The ToolHandler will automatically look up the tool and use its parameter schema
    match tool_handler.call_tool(&model_function_call) {
        Ok(result) => println!("Result: {}", result),
        Err(err) => println!("Error calling function: {}", err),
    }
}
```

### Getting Tool Schemas for LLMs

You need to provide LLMs with schema information about available tools:

```rust
use tool_calling::ToolHandler;

fn main() {
    let tool_handler = ToolHandler::new();
    
    // Get the JSON schema for all tools
    let tool_schemas = tool_handler.all_tools_schema();
    
    // This is what you'd send to the LLM to inform it about available tools
    println!("Available tools: {}", serde_json::to_string_pretty(&tool_schemas).unwrap());
}
```

### Ollama Integration

This library includes built-in support for Ollama, allowing you to use your tools with Ollama-based language models:

```rust
use tool_calling::{tool, ToolHandler};
use tool_calling::ollama;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tool_handler = ToolHandler::new();
    let tools_schema = tool_handler.all_tools_schema();
    
    // Call Ollama with the tools
    ollama::have_conversation_with_llm(
        "llama3", // Ollama model ID
        "Calculate the BMI for a person weighing 70kg and 1.75m tall",
        &tools_schema,
        &tool_handler,
    ).await?;
    
    Ok(())
}
```

## License

MIT or Apache-2.0