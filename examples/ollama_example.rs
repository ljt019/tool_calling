use linkme::distributed_slice;
use serde_json::{json, Value};
use tool_calling::ollama;
use tool_calling::{tool, ToolHandler};

/// Get user info from database
#[tool]
pub fn get_user_info(user_id: u32) -> String {
    println!("Getting user info for user ID: {}", user_id);

    // In a real implementation, this would query a database
    match user_id {
        1 => "User 1 info: Name: John Doe, Email: john.doe@example.com".to_string(),
        2 => "User 2 info: Name: Jane Smith, Email: jane.smith@example.com".to_string(),
        _ => "User not found".to_string(),
    }
}

/// Calculate BMI (Body Mass Index)
#[tool]
pub fn calculate_bmi(weight_kg: f32, height_m: f32) -> String {
    if height_m <= 0.0 || weight_kg <= 0.0 {
        return "Error: Weight and height must be positive values".to_string();
    }
    let bmi = weight_kg / (height_m * height_m);
    format!(
        "BMI: {:.1}. {}",
        bmi,
        match bmi {
            bmi if bmi < 18.5 => "Classification: Underweight",
            bmi if bmi < 25.0 => "Classification: Normal weight",
            bmi if bmi < 30.0 => "Classification: Overweight",
            _ => "Classification: Obesity",
        }
    )
}

#[tool]
/// Buy a stock, for example:
/// buy_stock("AAPL", 100, 100.0)
pub fn buy_stock(ticker: String, quantity: u32, max_price: f32, min_price: f32) -> String {
    println!(
        "Buying {} shares of {} stock between {} and {}",
        quantity, ticker, max_price, min_price
    );
    format!(
        "Stock purchased successfully: {} shares of {} at $100",
        quantity, ticker
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Tool Calling Demo with Ollama ===");

    // This is what a typical model would return as a function call - just the name and arguments
    let example_function_call_input_raw = r#"{
        "type": "function",
        "function": {
            "name": "get_user_info",
            "arguments": {
                "user_id": 1
            }
        }
    }"#;

    let example_function_call_input_parsed: Value =
        serde_json::from_str(example_function_call_input_raw).unwrap();

    let tool_handler = ToolHandler::new();

    // Test a direct call to the tool - the improved call_tool will automatically use the tool's parameter schema
    println!("\n=== Direct Tool Call ===");
    println!(
        "Input: {}",
        serde_json::to_string_pretty(&example_function_call_input_parsed).unwrap()
    );
    let direct_result = tool_handler.call_tool(&example_function_call_input_parsed);
    match &direct_result {
        Ok(result) => println!("Result: {}", result),
        Err(err) => println!("Error: {}", err),
    }

    // Get the JSON schema for all tools
    println!("\n=== Available Tools Schema ===");
    let tools_schema = tool_handler.all_tools_schema();
    println!("{}", serde_json::to_string_pretty(&tools_schema).unwrap());

    // Call Ollama with the tools
    println!("\n=== Ollama Conversation ===");
    println!("Model: cogito:14b");
    println!("Prompt: Can you please buy 20 shares of AAPL between 80 and 100 each?");
    println!("Connecting to Ollama API...");
    match ollama::have_conversation_with_llm(
        "cogito:14b", // Replace with your actual model
        "Can you please buy 20 shares of AAPL between 80 and 100 each?",
        &tools_schema,
        &tool_handler,
    )
    .await
    {
        Ok(_) => {
            println!("\n=> Conversation completed successfully ✓");
        }
        Err(e) => {
            println!("\n=> Error connecting to Ollama: {}", e);
            println!("\n=== How to Run with Ollama ===");
            println!("1. Install Ollama: https://ollama.com/");
            println!("2. Pull a function-calling model: `ollama pull llama3`");
            println!("3. Update the model name in this example");
            println!("4. Start the Ollama server locally");

            // Mock a tool call for demonstration purposes
            println!("\n=== Mock Tool Calls Demo ===");

            // Create a mock tool call for BMI calculation
            println!("\n--- BMI Calculation Example ---");
            let bmi_args = json!({
                "weight_kg": 70,
                "height_m": 1.75
            });
            println!(
                "Arguments: {}",
                serde_json::to_string_pretty(&bmi_args).unwrap()
            );

            let mock_tool_calls = ollama::create_mock_tool_call("calculate_bmi", bmi_args);

            // Process the mock tool call
            let _ = ollama::process_mock_tool_call(&mock_tool_calls, &tool_handler);

            // Demo for buy_stock function with 4 arguments (demonstrating multi-arg support)
            println!("\n--- Stock Purchase Example ---");
            let buy_stock_args = json!({
                "ticker": "AAPL",
                "quantity": 20,
                "max_price": 100.0,
                "min_price": 80.0
            });
            println!(
                "Arguments: {}",
                serde_json::to_string_pretty(&buy_stock_args).unwrap()
            );

            let mock_buy_stock = ollama::create_mock_tool_call("buy_stock", buy_stock_args);
            let _ = ollama::process_mock_tool_call(&mock_buy_stock, &tool_handler);
        }
    }

    Ok(())
}
