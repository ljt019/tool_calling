use serde_json::Value;
use tool_calling::{tool, ToolHandler};

/*

First you need to define the avalible tools.

You just use the #[tool] attribute to define the tool.

It will automatically generate a tool schema using:

- The function name as the tool name
- The function docstring as the tool description
- The function parameters as the tool parameters (use Option<T> for optional parameters)

*/

#[tool]
/// Get user info from database
fn get_user_info(user_id: u32) -> String {
    match user_id {
        1 => "User 1 info: Name: John Doe, Email: john.doe@example.com".to_string(),
        2 => "User 2 info: Name: Jane Smith, Email: jane.smith@example.com".to_string(),
        _ => "User not found".to_string(),
    }
}

// Make main async
#[tokio::main]
async fn main() {
    // Use ToolHandler::default()
    let tool_handler = ToolHandler::default();

    // You can get a tools schema by name
    println!("\n=== Single Tool Information ===");
    if let Some(tool) = tool_handler.get_tool("get_user_info") {
        println!("Name: {}", tool.name);
        println!("Description: {}", tool.description);
        println!(
            "Parameters: {}",
            serde_json::to_string_pretty(&tool.parameter_schema).unwrap()
        );
    }

    /*
    Println output:
    === Single Tool Information ===
    Name: get_user_info
    Description: Get user info from database
    Parameters: {
      "properties": {
        "user_id": {
          "type": "integer"
        }
      },
      "required": [
        "user_id"
      ],
      "type": "object"
    }
    */

    // You can also just get all the tools registered with the handler
    println!("\n=== All Tools Schema ===");
    println!(
        "{}",
        serde_json::to_string_pretty(&tool_handler.all_tools_schema()).unwrap()
    );

    /*
    Println output:
    === All Tools Schema ===
    [
      {
        "function": {
          "description": "Get user info from database",
          "name": "get_user_info",
          "parameters": {
            "properties": {
              "user_id": {
                "type": "integer"
              }
            },
            "required": [
              "user_id"
            ],
            "type": "object"
          }
        },
        "type": "function"
      }
    ]
    */

    // Usually you would use the above schemas to let the model know what tools are avaliable
    // the model might respond with a tool call, usually something like this:
    let example_function_call_input_raw = r#"{
        "type": "function",
        "function": {
            "name": "get_user_info",
            "arguments": {
                "user_id": 1
            }
        }
    }"#;

    // Parse and process the function call
    let example_call: Value = serde_json::from_str(example_function_call_input_raw).unwrap();

    println!("\n=== Function Call Example ===");
    println!(
        "Input: {}",
        serde_json::to_string_pretty(&example_call).unwrap()
    );

    // You can then call the tool like this, handling the error:
    match tool_handler.call_tool(&example_call).await {
        Ok(result) => println!("Result: {}", result),
        Err(err) => println!("Error calling tool: {}", err),
    }

    /*
    Println output:
    === Function Call Example ===
    Input: {
      "type": "function",
      "function": {
        "name": "get_user_info",
        "arguments": {
          "user_id": 1
        }
      }
    }
    Result: User 1 info: Name: John Doe, Email: john.doe@example.com
    */

    // You can also call with arguments directly
    match tool_handler
        .call_with_args("get_user_info", &["1".to_string()])
        .await
    {
        Ok(result) => println!("\nDirect call result: {}", result),
        Err(err) => println!("\nDirect call error: {}", err),
    }

    /*
    Println output:
    === Direct Call Example ===
    Input: {
      "type": "function",
      "function": {
        "name": "get_user_info",
        "arguments": {
          "user_id": 1
        }
      }
    }
    Result: User 1 info: Name: John Doe, Email: john.doe@example.com
    */
}
