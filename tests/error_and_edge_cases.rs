use serde_json::json;
use tool_calling::{tool, ToolError, ToolHandler};

// Define necessary tools for these tests

#[tool]
pub fn boom() -> String {
    panic!("oh no")
}

// Need add for schema injection test
#[tool]
pub fn add(a: i32, b: i32) -> String {
    (a + b).to_string()
}

// Tests

#[tokio::test]
async fn panic_in_tool() {
    let handler = ToolHandler::default();
    let err = handler.call_with_args("boom", &[]).await.unwrap_err();
    assert!(matches!(err, ToolError::Execution(msg) if msg == "panic in tool"));
}

#[tokio::test]
async fn ignore_schema_injection() {
    let handler = ToolHandler::default();
    let input = json!({
        "type": "function",
        "function": {
            "name": "add",
            "arguments": {"a": "1", "b": "2"},
            // Attempt to inject a different schema
            "parameters": {"properties": {"a": {"type":"string"}}, "required": []}
        }
    });
    // Use call_tool which performs validation
    let res = handler.call_tool(&input).await;
    // Expect BadArgs due to schema validation failure (input is string, schema expects integer)
    assert!(
        matches!(res, Err(ToolError::BadArgs(msg)) if msg.contains("Argument validation failed for tool 'add':"))
    );
}
