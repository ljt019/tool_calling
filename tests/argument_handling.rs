use serde_json::json;
use tool_calling::{tool, ToolError, ToolHandler};

// Define necessary tools for these tests

#[tool]
pub fn opt(x: Option<u32>) -> String {
    x.map(|n| n.to_string()).unwrap_or("none".into())
}

#[tool]
pub fn def(x: Option<u32>) -> String {
    x.unwrap_or(42).to_string()
}

#[tool]
pub fn req(a: u32, b: String) -> String {
    format!("{} {}", a, b)
}

#[tool]
pub fn add(a: i32, b: i32) -> String {
    (a + b).to_string()
}

#[tool]
pub fn order(a: String, b: String, c: String) -> String {
    format!("{},{},{}", a, b, c)
}

// Tests

#[tokio::test]
async fn optional_param_omitted() {
    let handler = ToolHandler::default();
    let res = handler.call_with_args("opt", &[]).await;
    assert_eq!(res, Ok("none".into()));
}

#[tokio::test]
async fn default_value_used() {
    let handler = ToolHandler::default();
    let res = handler.call_with_args("def", &[]).await;
    assert_eq!(res, Ok("42".into()));
}

#[tokio::test]
async fn missing_required() {
    let handler = ToolHandler::default();
    // Test missing one required arg
    let err = handler
        .call_with_args("req", &["1".into()])
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::BadArgs(msg) if msg.contains("Expected between 2 and 2 arguments"))
    );
    // Test missing all required args
    let err2 = handler.call_with_args("req", &[]).await.unwrap_err();
    assert!(
        matches!(err2, ToolError::BadArgs(msg) if msg.contains("Expected between 2 and 2 arguments"))
    );
}

#[tokio::test]
async fn type_mismatch() {
    let handler = ToolHandler::default();
    let err = handler
        .call_with_args("add", &["foo".into(), "2".into()])
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::BadArgs(msg) if msg.contains("Failed to parse argument 'foo' for parameter 'a'"))
    );
}

#[tokio::test]
async fn extra_args() {
    let handler = ToolHandler::default();
    let err = handler
        .call_with_args("add", &["1".into(), "2".into(), "3".into()])
        .await
        .unwrap_err();
    assert!(
        matches!(err, ToolError::BadArgs(msg) if msg.contains("Expected between 2 and 2 arguments"))
    );
}

#[tokio::test]
async fn argument_ordering() {
    let handler = ToolHandler::default();
    let input = json!({
        "type": "function",
        "function": {
            "name": "order",
            "arguments": {"c": "third", "a": "first", "b": "second"}
        }
    });
    // Use call_tool here as call_with_args bypasses JSON parsing/ordering logic
    let res = handler.call_tool(&input).await.unwrap();
    assert_eq!(res, "first,second,third");
}
