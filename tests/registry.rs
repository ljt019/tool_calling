use tool_calling::{tool, tools};

// Define a tool to ensure it's registered
#[tool]
/// Adds two integers and returns the result as a string.
pub fn add(a: i32, b: i32) -> String {
    (a + b).to_string()
}

// Tests

#[tokio::test]
async fn registry_enumerator() {
    let reg = tools(); // Get the static tool list
                       // Check if a specific tool exists and has expected properties
    assert!(reg.iter().any(|t| {
        t.name == "add" && t.description == "Adds two integers and returns the result as a string."
        // We could also check parameter schema here if needed
    }));
}
