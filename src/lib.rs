use linkme::distributed_slice;
use serde::Serialize;
use serde_json::{json, Value};
/// Re-export the proc-macro attribute
pub use tool_calling_derive::tool;

// Optional Ollama integration
pub mod ollama;

/// Metadata about a "tool" function
#[derive(Serialize, Debug)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameter_schema: Value,
    #[serde(skip)] // Don't serialize the function pointer
    pub function: fn(&[String]) -> Result<String, String>,
}

// collect all the tool factory functions emitted by the proc-macro
#[distributed_slice]
pub static TOOL_FACTORIES: [fn() -> Tool] = [..];

/// Return all tools registered at runtime
pub fn all_tools() -> Vec<Tool> {
    TOOL_FACTORIES.iter().map(|factory| factory()).collect()
}

pub struct ToolHandler {
    tools: Vec<Tool>,
}

impl ToolHandler {
    pub fn new() -> Self {
        let tools = all_tools();

        Self { tools }
    }

    pub fn get_tool(&self, name: &str) -> Option<&Tool> {
        self.tools.iter().find(|tool| tool.name == name)
    }

    pub fn call_with_args(&self, name: &str, args: &[String]) -> Result<String, String> {
        let tool = self
            .get_tool(name)
            .ok_or(format!("Tool {} not found", name))?;
        (tool.function)(args)
    }

    /// Produce a JSON schema for the LLM describing all available tools
    pub fn all_tools_schema(&self) -> Value {
        let funcs: Vec<_> = self
            .tools
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

    pub fn call_tool(&self, input: &Value) -> Result<String, String> {
        let obj = input.as_object().ok_or("Expected JSON object")?;
        let input_type = obj
            .get("type")
            .and_then(|t| t.as_str())
            .ok_or("Missing or invalid 'type' field")?;
        if input_type != "function" {
            return Err(format!(
                "Invalid input type: expected 'function', got '{}'",
                input_type
            ));
        }
        let function = obj
            .get("function")
            .and_then(|f| f.as_object())
            .ok_or("Missing or invalid 'function' field")?;
        let name = function
            .get("name")
            .and_then(|n| n.as_str())
            .ok_or("Missing or invalid 'function.name'")?;
        let args_obj = function
            .get("arguments")
            .or_else(|| obj.get("arguments"))
            .and_then(|a| a.as_object())
            .ok_or("Missing or invalid 'arguments' field")?;

        // Try to get parameters from the function call
        let params_schema = function.get("parameters").and_then(|p| p.as_object());

        // If parameters are not provided but the tool exists in our registry, use its schema
        let (params_schema, required) = if let Some(schema) = params_schema {
            // Use provided schema
            let required = schema
                .get("required")
                .and_then(|r| r.as_array())
                .ok_or("Missing or invalid 'function.parameters.required' field")?;
            (schema, required)
        } else if let Some(tool) = self.get_tool(name) {
            // Use schema from registered tool
            let tool_schema = tool
                .parameter_schema
                .as_object()
                .ok_or("Invalid parameter schema in registered tool")?;
            let required = tool_schema
                .get("required")
                .and_then(|r| r.as_array())
                .ok_or("Missing or invalid 'required' field in tool parameter schema")?;
            (tool_schema, required)
        } else {
            return Err(format!(
                "Missing parameters schema and tool '{}' not found in registry",
                name
            ));
        };

        let mut args: Vec<String> = Vec::new();
        for param in required {
            let param_name = param.as_str().ok_or("Invalid parameter name")?;
            let arg_value = args_obj
                .get(param_name)
                .ok_or(format!("Missing argument for parameter '{}'", param_name))?;
            let arg_str = match arg_value {
                Value::String(s) => s.clone(),
                _ => arg_value.to_string(),
            };
            args.push(arg_str);
        }

        self.call_with_args(name, &args)
    }
}
