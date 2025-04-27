use serde_json::Value;
use tool_calling::{tool, ToolHandler};

/*

First you need to define the avalible tools.

You just use the #[tool] attribute to define the tool.

It will automatically generate a tool schema using:

- The function name as the tool name
- The function docstring as the tool description
- The function parameters as the tool parameters (use Option<T> for optional parameters)

- In this an async context, like this example,
you must call the tool using the async versions of the tool caller methods

*/

#[tool]
/// Read a file
async fn read_file(path: String) -> String {
    // In a real implementation, you would read the file, here we will simulate an async operation
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
    format!("File content: {}", path)
}

#[tokio::main]
async fn main() {
    // Use ToolHandler::default()
    let tool_handler = ToolHandler::default();

    // You can get a tools schema by name
    println!("\n=== Single Tool Information ===");
    if let Some(tool) = tool_handler.get_tool("read_file") {
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
    Name: read_file
    Description: Read a file
    Parameters: {
      "properties": {
        "path": {
          "type": "string"
        }
      },
      "required": [
        "path"
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
          "description": "Read a file",
          "name": "read_file",
          "parameters": {
            "properties": {
              "path": {
                "type": "string"
              }
            },
            "required": [
              "path"
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
            "name": "read_file",
            "arguments": {
                "path": "test.txt"
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

    // Rename call_tool_async to call_tool (await is already present)
    match tool_handler.call_tool(&example_call).await {
        Ok(result) => println!("Result: {}", result),
        // Print ToolError
        Err(err) => println!("Error calling tool: {}", err),
    }

    /*
    Println output:
    === Function Call Example ===
    Input: {
      "type": "function",
      "function": {
        "name": "read_file",
        "arguments": {
          "path": "test.txt"
        }
      }
    }
    Result: File content: test.txt
    */

    // Rename call_with_args_async to call_with_args (await is already present)
    match tool_handler
        .call_with_args("read_file", &["another_file.txt".to_string()])
        .await
    {
        Ok(result) => println!("\nDirect call result: {}", result),
        // Print ToolError
        Err(err) => println!("\nDirect call error: {}", err),
    }
}
