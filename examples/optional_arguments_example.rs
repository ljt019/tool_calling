use serde_json::Value;
use tool_calling::{tool, ToolHandler};

/*

This example demonstrates working with optional parameters.

You just use the #[tool] attribute to define the tool.

It will automatically generate a tool schema using:

- The function name as the tool name
- The function docstring as the tool description
- The function parameters as the tool parameters

For optional parameters:
- Use Option<T> in your function signature
- The parameter will be marked as non-required in the JSON schema
- If the parameter is not provided, None will be passed to your function

*/

#[tool]
/// Greet a user with optional punctuation
fn greet(name: String, punctuation: Option<String>) -> String {
    // Use the provided punctuation or fall back to the hardcoded default
    let punct = punctuation.unwrap_or_else(|| "!".to_string());
    format!("Hello, {}{}", name, punct)
}

#[tokio::main]
async fn main() {
    let handler = ToolHandler::default();

    // Print the generated JSON Schema for this tool
    println!("\n=== Tool Schema ===");
    if let Some(tool) = handler.get_tool("greet") {
        println!(
            "{}",
            serde_json::to_string_pretty(&tool.parameter_schema).unwrap()
        );
    }

    /*
    Println output:
    === Tool Schema ===
    {
      "type": "object",
      "properties": {
        "name": {
          "type": "string"
        },
        "punctuation": {
          "type": ["string", "null"]
        }
      },
      "required": [
        "name"
      ]
    }
    */

    // Call without providing the optional parameter
    println!("\n=== Example 1: Missing Optional Parameter ===");
    let call_no_opt: Value = serde_json::json!({
        "type": "function",
        "function": {
            "name": "greet",
            "arguments": {
                "name": "Alice"
            }
        }
    });
    println!(
        "Input: {}",
        serde_json::to_string_pretty(&call_no_opt).unwrap()
    );

    let res_no_opt = handler.call_tool(&call_no_opt).await.unwrap();
    println!("Result: {}", res_no_opt);

    /*
    Println output:
    === Example 1: Missing Optional Parameter ===
    Input: {
      "type": "function",
      "function": {
        "name": "greet",
        "arguments": {
          "name": "Alice"
        }
      }
    }
    Result: Hello, Alice!
    */

    // Call with the optional parameter provided
    println!("\n=== Example 2: Providing Optional Parameter ===");
    let call_with_opt: Value = serde_json::json!({
        "type": "function",
        "function": {
            "name": "greet",
            "arguments": {
                "name": "Bob",
                "punctuation": "?"
            }
        }
    });
    println!(
        "Input: {}",
        serde_json::to_string_pretty(&call_with_opt).unwrap()
    );

    let res_with_opt = handler.call_tool(&call_with_opt).await.unwrap();
    println!("Result: {}", res_with_opt);

    /*
    Println output:
    === Example 2: Providing Optional Parameter ===
    Input: {
      "type": "function",
      "function": {
        "name": "greet",
        "arguments": {
          "name": "Bob",
          "punctuation": "?"
        }
      }
    }
    Result: Hello, Bob?
    */

    // You can also call with arguments directly
    println!("\n=== Example 3: Direct Call ===");

    let res_direct = handler
        .call_with_args("greet", &["Charlie".to_string(), "....".to_string()])
        .await
        .unwrap();
    println!("Direct call result: {}", res_direct);

    /*
    Println output:
    === Example 3: Direct Call ===
    Direct call result: Hello, Charlie....
    */
}
