use tool_calling::{tool, ToolError, ToolHandler};

// Define necessary tools for these tests

#[tool]
pub fn hello() -> String {
    "hi".into()
}

#[tool]
pub fn echo(s: String) -> String {
    s.clone()
}

#[tool]
pub fn add(a: i32, b: i32) -> String {
    (a + b).to_string()
}

#[tool]
pub async fn hey() -> String {
    "yo".into()
}

#[tool]
pub async fn concat(a: String, b: String) -> String {
    a + &b
}

#[tool]
pub fn mix(flag: bool, x: f64) -> String {
    format!("{}:{}", flag, x)
}

// Tests

#[tokio::test]
async fn sync_zero_args() {
    let handler = ToolHandler::default();
    let res = handler.call_with_args("hello", &[]).await;
    assert_eq!(res, Ok("hi".into()));
}

#[tokio::test]
async fn sync_one_arg() {
    let handler = ToolHandler::default();
    let ok = handler.call_with_args("echo", &["foo".into()]).await;
    assert_eq!(ok, Ok("foo".into()));
    let err = handler.call_with_args("echo", &[]).await.unwrap_err();
    assert!(matches!(err, ToolError::BadArgs(_))); // Check length error
    let err2 = handler
        .call_with_args("echo", &["foo".into(), "bar".into()])
        .await
        .unwrap_err();
    assert!(matches!(err2, ToolError::BadArgs(_))); // Check length error
}

#[tokio::test]
async fn sync_multi_args() {
    let handler = ToolHandler::default();
    let ok = handler
        .call_with_args("add", &["2".into(), "3".into()])
        .await;
    assert_eq!(ok, Ok("5".into()));
    // Length check is covered by missing_required/extra_args tests in argument_handling.rs
    // Type mismatch check is covered by type_mismatch test in argument_handling.rs
}

#[tokio::test]
async fn async_zero_args() {
    let handler = ToolHandler::default();
    let res = handler.call_with_args("hey", &[]).await;
    assert_eq!(res, Ok("yo".into()));
}

#[tokio::test]
async fn async_multi_args() {
    let handler = ToolHandler::default();
    let res = handler
        .call_with_args("concat", &["a".into(), "b".into()])
        .await;
    assert_eq!(res, Ok("ab".into()));
}

#[tokio::test]
async fn bool_and_float() {
    let handler = ToolHandler::default();
    let res = handler
        .call_with_args("mix", &["true".into(), "3.14".into()])
        .await;
    assert_eq!(res, Ok("true:3.14".into()));
}
