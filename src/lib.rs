use linkme::distributed_slice;
use serde_json::Value;
/// Re-export the proc-macro attribute
pub use tool_calling_derive::tool;

/// Metadata about a "tool" function
pub struct Tool {
    pub name: String,
    pub description: String,
    pub parameters: Vec<String>,
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
        let params_schema = function
            .get("parameters")
            .and_then(|p| p.as_object())
            .ok_or("Missing or invalid 'function.parameters' field")?;
        let required = params_schema
            .get("required")
            .and_then(|r| r.as_array())
            .ok_or("Missing or invalid 'function.parameters.required' field")?;

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
