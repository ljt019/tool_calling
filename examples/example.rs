use linkme::distributed_slice;
use serde_json::Value;
use tool_calling::{tool, ToolHandler};
/*
--- MODEL FUNCTION CALL EXAMPLE ---
    "message": {
        "role": "assistant",
        "content": "",
        "tool_calls": [
            {
                "function": {
                    "name": "readFile",
                    "arguments": {
                        "filePath": "C:\\Users\\lucie\\Desktop\\Projects\\personal\\isotope\\src\\files_for_ai\\main.py"
                    }
                }
            }
        ]
    },

--- MODEL FUNCTION CALL INPUT EXAMPLE ---
{"type":"function","function":{"name":"readFile","description":"Reads the contents of a file from the authorized directory. For security reasons, only files within the authorized directory can be accessed. Use listdirectory tool first to get the path to files. Returns the file contents as a UTF-8 encoded string.","parameters":{"type":"object","required":["filePath"],"properties":{"filePath":{"type":"string","description":"The absolute path to the file you want to read. Must be located within the authorized directory (use the listDirectory tool first)."}}}}}

*/

#[tool(description = "Get user info from database")]
pub fn get_user_info(user_id: u32) -> String {
    println!("Getting user info for user ID: {}", user_id);

    // In a real implementation, this would query a database
    match user_id {
        1 => "User 1 info: Name: John Doe, Email: john.doe@example.com".to_string(),
        2 => "User 2 info: Name: Jane Smith, Email: jane.smith@example.com".to_string(),
        _ => "User not found".to_string(),
    }
}

fn main() {
    let example_function_call_input_raw = r#"{"type":"function","function":{"name":"get_user_info","description":"Get user info from database","parameters":{"type":"object","required":["user_id"],"properties":{"user_id":{"type":"integer","description":"The ID of the user to get info for"}}},"arguments":{"user_id":1}}}"#;

    let example_function_call_input_parsed: Value =
        serde_json::from_str(example_function_call_input_raw).unwrap();

    let tool_handler = ToolHandler::new();

    let result = tool_handler.call_tool(&example_function_call_input_parsed);

    println!("{:?}", result);
}
