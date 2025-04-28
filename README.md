# tool_calling

<!-- CI / Workflow Badges -->
[<img alt="crates.io" src="https://img.shields.io/crates/v/tool_calling.svg?style=for-the-badge&color=fc8d62&logo=rust" height="19">](https://crates.io/crates/tool_calling)
[<img alt="docs.rs" src="https://img.shields.io/badge/docs.rs-tool_calling-66c2a5?style=for-the-badge&labelColor=555555&logo=docs.rs" height="19">](https://docs.rs/tool_calling)
![Build](https://github.com/ljt019/tool_calling/actions/workflows/build_and_release.yaml/badge.svg?branch=main)
![Tests](https://github.com/ljt019/tool_calling/actions/workflows/tests.yaml/badge.svg?branch=main)
![Doc Tests](https://github.com/ljt019/tool_calling/actions/workflows/doc_tests.yaml/badge.svg?branch=main)

A procedural-macro framework for defining, registering, and invoking Rust functions as tools with automatic JSON Schema validation and error handling.

## Features

- **Attribute-based**: Annotate Rust functions with `#[tool]` to register them automatically.
- **Automatic Schema Generation**: Generates a JSON Schema from function signature and doc comments.
- **Synchronous & Asynchronous**: Support both sync and async functions out of the box.
- **Optional Parameters**: Use `Option<T>` for optional arguments; `#[default = ...]` for defaults.
- **Type Safety**: Denies reference types (`&T`) to ensure tools use owned types like `String` and `Vec<T>`.
- **Error Handling**: Provides clear errors for missing tools, argument validation failures, and execution errors (including panics).

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tool_calling = "0.1"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

Or use cargo:

```shell
cargo add tool_calling
cargo add tokio --features macros,rt-multi-thread
```

> Note: The `tokio` dependency is only required if you are calling tools asynchronously (most CLI or server contexts).

## Quickstart

```rust
use tool_calling::{tool, ToolHandler};
use serde_json::json;

#[tool]
/// Get user info from database
fn get_user_info(user_id: u32) -> String {
    match user_id {
        1 => "User 1 info...".to_string(),
        _ => "User not found".to_string(),
    }
}

#[tokio::main]
async fn main() {
    // Initialize handler
    let handler = ToolHandler::default();
    
    // Call the tool with a JSON payload
    let payload = json!({
        "type": "function",
        "function": {
            "name": "get_user_info",
            "arguments": { "user_id": 1 }
        }
    });
    
    let result = handler.call_tool(&payload).await.unwrap();
    println!("Result: {}", result);
}
```

## Defining Tools

Use the `#[tool]` attribute to mark any free function as a tool. The macro will:

1. Collect the doc comment (`///`) as the tool's **description**.
2. Inspect parameters to generate a **JSON Schema** (`u32`/`i*`/`usize` &rarr; `integer`, `f32`/`f64` &rarr; `number`, `bool` &rarr; `boolean`, `String` &rarr; `string`).
3. Treat `Option<T>` parameters as optional fields in the schema (allowing `null`).
4. Enforce owned types (no `&T`).

```rust
use tool_calling::tool;

#[tool]
/// Get user info from database
fn get_user_info(user_id: u32) -> String {
    match user_id {
        1 => "User 1 info...".to_string(),
        _ => "User not found".to_string(),
    }
}
```

Optionally provide a default literal for `Option<T>` parameters:

```rust
use tool_calling::{tool, default};

#[tool]
/// Greet a user with optional punctuation
fn greet(
    name: String,
    #[default = "?" ] punctuation: Option<String>
) -> String {
    let punct = punctuation.unwrap_or_else(|| "!".to_string());
    format!("Hello, {}{}", name, punct)
}
```

## Calling Tools

### 1. Initialize a handler

```rust
use tool_calling::ToolHandler;

#[tokio::main]
async fn main() {
    let handler = ToolHandler::default();
    // ...
}
```

### 2. Discover tools & schemas

```rust
// List a single tool
if let Some(tool) = handler.get_tool("get_user_info") {
    println!("Description: {}", tool.description);
    println!("Schema: {}", serde_json::to_string_pretty(&tool.parameter_schema).unwrap());
}

// List all tools for LLM function-calling
println!("{}", serde_json::to_string_pretty(&handler.all_tools_schema()).unwrap());
```

### 3. Call by JSON payload

```rust
use serde_json::json;

let payload = json!({
    "type": "function",
    "function": {
        "name": "get_user_info",
        "arguments": { "user_id": 1 }
    }
});

match handler.call_tool(&payload).await {
    Ok(res) => println!("Result: {}", res),
    Err(e) => eprintln!("Error: {}", e),
}
```

### 4. Call directly with string args

```rust
let result = handler
    .call_with_args("get_user_info", &["1".to_string()])
    .await
    .unwrap();
println!("Direct call result: {}", result);
```

## Examples

Explore the examples directory for more usage scenarios:

- [`examples/simple_example.rs`](examples/simple_example.rs) — Basic sync tool.
- [`examples/async_example.rs`](examples/async_example.rs) — Async tool with `tokio`.
- [`examples/optional_arguments_example.rs`](examples/optional_arguments_example.rs) — Tool with optional parameters and defaults.

## Testing

Run the full test suite:

```bash
cargo test
```

## API Reference

### Macros

- `#[tool]` — Marks a function as a tool, generating registration code and JSON Schema.
- `#[default = <literal>]` — Attach to `Option<T>` parameters for default values.

### `ToolHandler`

- `ToolHandler::default()` — Initializes and registers all annotated tools.
- `get_tool(name: &str) -> Option<&Tool>` — Retrieve metadata for a single tool.
- `all_tools_schema() -> serde_json::Value` — A JSON array of all tools (for LLM introspection).
- `call_tool(input: &serde_json::Value) -> Result<String, ToolError>` — Parse a function-call payload and execute.
- `call_with_args(name: &str, args: &[String]) -> Result<String, ToolError>` — Directly invoke a tool by name.

### Error Handling

`ToolError` variants:

- `NotFound(String)` — Tool name not registered.
- `BadArgs(String)` — Arguments missing or failed JSON Schema validation.
- `Execution(String)` — Underlying function panicked or returned an execution error.

## Contributing

Contributions, issues, and feature requests are welcome! Please open a GitHub issue or submit a pull request.

## License

This project is licensed under either of:

- Apache License, Version 2.0
- MIT license

at your option.
