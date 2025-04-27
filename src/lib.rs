use futures::future::BoxFuture;
use jsonschema::JSONSchema;
use linkme::distributed_slice;
use once_cell::sync::Lazy;
use serde::Serialize;
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::Arc;
/// Attribute to specify a default value for `Option<T>` parameters in tools.
///
/// Use `#[default = <literal>]` on `Option<T>` parameters to set a default if the argument is omitted.
///
/// # Examples
///
/// ```rust
/// use tool_calling::{tool, default};
///
/// #[tool]
/// fn greet(
///     name: String,
///     #[default = "?"] punctuation: Option<String>,
/// ) -> String {
///     let p = punctuation.unwrap_or_else(|| "!".into());
///     format!("Hello, {}{}", name, p)
/// }
/// ```
pub use tool_calling_derive::default;
/// Attribute macro to mark a function as a tool.
///
/// Use `#[tool]` on free functions to register them as tools with automatic JSON Schema validation.
///
/// # Examples
///
/// ```rust
/// use tool_calling::tool;
///
/// #[tool]
/// fn add(a: i32, b: i32) -> i32 {
///     a + b
/// }
/// ```
pub use tool_calling_derive::tool;

/// Errors returned by tool operations.
///
/// # Examples
///
/// ```rust
/// use tool_calling::{ToolHandler, ToolError};
///
/// #[tokio::main]
/// async fn main() {
///     let handler = ToolHandler::default();
///     let err = handler.call_with_args("unknown", &[]).await.unwrap_err();
///     assert_eq!(err, ToolError::NotFound("unknown".to_string()));
/// }
/// ```
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ToolError {
    #[error("tool not found: {0}")]
    NotFound(String),
    #[error("invalid arguments: {0}")]
    BadArgs(String),
    #[error("execution failed: {0}")]
    Execution(String),
}

/// Represents the wrapped function of a tool, always async.
///
/// The `Async` variant holds a boxed async function that takes string arguments and returns a `Result<String, ToolError>`.
pub enum ToolFn {
    Async(Box<dyn Fn(&[String]) -> BoxFuture<'static, Result<String, ToolError>> + Send + Sync>),
}

// Implement Debug manually as Box<dyn Fn...> doesn't auto-derive Debug
impl std::fmt::Debug for ToolFn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Async").field(&"...").finish() // Don't print the function itself
    }
}

/// Wraps a synchronous tool function into the async tool function signature.
///
/// This helper catches panics and converts them into a `ToolError::Execution` if the tool panics.
///
/// # Examples
///
/// ```rust
/// use tool_calling::{wrap_sync, ToolError};
/// use std::sync::Arc;
/// use futures::executor::block_on;
///
/// let sync_fn = Arc::new(|args: &[String]| -> Result<String, ToolError> {
///     Ok(args.join(","))
/// });
/// let async_fn = wrap_sync(sync_fn);
/// let result = block_on(async_fn(&vec!["a".into(), "b".into()])).unwrap();
/// assert_eq!(result, "a,b");
/// ```
pub fn wrap_sync(
    f: Arc<dyn Fn(&[String]) -> Result<String, ToolError> + Send + Sync>,
) -> Box<dyn Fn(&[String]) -> BoxFuture<'static, Result<String, ToolError>> + Send + Sync> {
    // Wrap synchronous function to catch panics and return Execution error
    Box::new(move |args| {
        let f_clone = Arc::clone(&f);
        let owned_args = args.to_vec();
        Box::pin(async move {
            match catch_unwind(AssertUnwindSafe(|| f_clone(&owned_args))) {
                Ok(res) => res,
                Err(_) => Err(ToolError::Execution("panic in tool".into())),
            }
        })
    })
}

/// Represents metadata for a registered tool function.
///
/// Contains its name, description, parameter schema, and the execution function.
#[derive(Serialize, Debug)]
pub struct Tool {
    /// The unique name of the tool.
    pub name: String,
    /// A brief description of the tool's purpose.
    pub description: String,
    /// JSON Schema describing tool parameters.
    pub parameter_schema: Value,
    /// The internal function pointer for executing the tool. Not serialized.
    #[serde(skip)]
    pub function: ToolFn,
}

// collect all the tool factory functions emitted by the proc-macro
#[distributed_slice]
pub static TOOL_FACTORIES: [fn() -> Tool] = [..];

// Use once_cell::sync::Lazy for the global tool registry
static ALL_TOOLS: Lazy<Vec<Tool>> =
    Lazy::new(|| TOOL_FACTORIES.iter().map(|factory| factory()).collect());

/// Returns a slice of all registered tools.
///
/// Each `Tool` includes its name, description, and parameter schema.
///
/// # Examples
///
/// ```rust
/// use tool_calling::tools;
///
/// let all = tools();
/// assert!(all.iter().any(|t| t.name == "example_tool"));
/// ```
pub fn tools() -> &'static [Tool] {
    &ALL_TOOLS
}

/// Handler for discovering and invoking registered tools.
///
/// Use `ToolHandler` to list tools, call them by name with arguments,
/// or execute calls via JSON payloads.
///
/// # Examples
///
/// ```rust
/// use tool_calling::ToolHandler;
///
/// #[tokio::main]
/// async fn main() {
///     let handler = ToolHandler::default();
///     if let Ok(res) = handler.call_with_args("add", &["1".into(), "2".into()]).await {
///         assert_eq!(res, "3");
///     }
/// }
/// ```
pub struct ToolHandler {
    // Store a reference to the static tool list instead of cloning
    // This requires a lifetime, or we can just reference the static Lazy directly in methods.
    // Let's keep it simple and reference ALL_TOOLS directly in methods that need it.
    // No need to store anything here if we always use the static.
    _private: (), // Struct needs at least one field
}

// Implement Default using the Lazy static
impl Default for ToolHandler {
    fn default() -> Self {
        // Ensure the Lazy is initialized
        Lazy::force(&ALL_TOOLS);
        Self { _private: () }
    }
}

impl ToolHandler {
    /// Retrieves a reference to a tool by its name.
    ///
    /// Returns `None` if no tool with the given name is registered.
    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        // Access the static list directly
        ALL_TOOLS.iter().find(|tool| tool.name == name)
    }

    /// Call a tool by name with pre-parsed string arguments.
    /// All tool calls are inherently async now.
    pub async fn call_with_args(&self, name: &str, args: &[String]) -> Result<String, ToolError> {
        let tool = self
            .get_tool(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;

        match &tool.function {
            ToolFn::Async(func) => func(args).await,
        }
    }

    /// Produce a JSON schema for the LLM describing all available tools
    pub fn all_tools_schema(&self) -> Value {
        let funcs: Vec<_> = ALL_TOOLS // Access the static list directly
            .iter()
            .map(|tool| {
                json!({
                    "type": "function",
                    "function": {
                        "name": tool.name,
                        "description": tool.description,
                        "parameters": tool.parameter_schema
                    }
                })
            })
            .collect();

        Value::Array(funcs)
    }

    /// Parses a JSON payload and executes the corresponding tool asynchronously.
    ///
    /// # Examples
    ///
    /// ```rust
    /// use tool_calling::{ToolHandler, ToolError};
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let handler = ToolHandler::default();
    ///     let payload = json!({
    ///         "type": "function",
    ///         "function": {
    ///             "name": "add",
    ///             "arguments": { "a": 1, "b": 2 }
    ///         }
    ///     });
    ///     let res = handler.call_tool(&payload).await.unwrap();
    ///     assert_eq!(res, "3");
    /// }
    /// ```
    pub async fn call_tool(&self, input: &Value) -> Result<String, ToolError> {
        let (name, args) = self.parse_tool_call(input)?;
        self.call_with_args(&name, &args).await
    }

    // Helper method to parse tool calls, validate against schema, and extract ordered args
    fn parse_tool_call(&self, input: &Value) -> Result<(String, Vec<String>), ToolError> {
        let obj = input
            .as_object()
            .ok_or_else(|| ToolError::BadArgs("Expected JSON object".to_string()))?;
        let input_type = obj
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or_else(|| ToolError::BadArgs("Missing or invalid 'type' field".to_string()))?;
        if input_type != "function" {
            return Err(ToolError::BadArgs(format!(
                "Invalid input type: expected 'function', got '{}'",
                input_type
            )));
        }
        let function = obj
            .get("function")
            .and_then(|f| f.as_object())
            .ok_or_else(|| ToolError::BadArgs("Missing or invalid 'function' field".to_string()))?;
        let name = function
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or_else(|| ToolError::BadArgs("Missing or invalid 'function.name'".to_string()))?;
        let args_obj = function
            .get("arguments")
            .and_then(|a| a.as_object())
            .ok_or_else(|| {
                ToolError::BadArgs("Missing or invalid 'arguments' field".to_string())
            })?;

        // --- Schema Validation ---
        let tool = self
            .get_tool(name)
            .ok_or_else(|| ToolError::NotFound(name.to_string()))?;
        let compiled_schema = JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&tool.parameter_schema)
            .map_err(|e| {
                ToolError::Execution(format!(
                    "Failed to compile schema for tool '{}': {}",
                    name, e
                ))
            })?;
        let input_args_val = Value::Object(args_obj.clone());
        if let Err(errors) = compiled_schema.validate(&input_args_val) {
            let error_messages = errors.map(|e| e.to_string()).collect::<Vec<_>>().join("; ");
            return Err(ToolError::BadArgs(format!(
                "Argument validation failed for tool '{}': {}",
                name, error_messages
            )));
        }
        // --- End Schema Validation ---

        // Prepare lists of required field names
        let tool_schema_obj = tool.parameter_schema.as_object().ok_or_else(|| {
            ToolError::Execution(format!(
                "Invalid parameter schema format for tool '{}'",
                name
            ))
        })?;
        let required_arr = tool_schema_obj
            .get("required")
            .and_then(|r| r.as_array())
            .ok_or_else(|| {
                ToolError::Execution(format!(
                    "Schema for tool '{}' missing 'required' array",
                    name
                ))
            })?;
        let required_names = required_arr
            .iter()
            .filter_map(|v| v.as_str())
            .collect::<Vec<_>>();

        // Extract arguments in order, only error if a required param is missing
        let properties = tool_schema_obj
            .get("properties")
            .and_then(|p| p.as_object())
            .ok_or_else(|| {
                ToolError::Execution(format!("Schema for tool '{}' missing 'properties'", name))
            })?;

        let mut ordered_args: Vec<String> = Vec::new();
        for (param_name, _param_schema) in properties {
            if let Some(val) = args_obj.get(param_name) {
                let arg_str = match val {
                    Value::String(s) => s.clone(),
                    _ => val.to_string(),
                };
                ordered_args.push(arg_str);
            } else if required_names.contains(&param_name.as_str()) {
                return Err(ToolError::BadArgs(format!(
                    "Missing argument for parameter '{}'",
                    param_name
                )));
            } else {
                // Optional parameter omitted: skip pushing
            }
        }

        Ok((name.to_string(), ordered_args))
    }
}
