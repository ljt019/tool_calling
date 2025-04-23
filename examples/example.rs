use tool_calling::print_hello_world;

/*
PLANNING SCRATCHPAD

#[tool (description="Get user info from database")]
pub fn get_user_info() -> String {
    // get user info from database
}

struct Tool {
    name: String,
    description: String,
    parameters: Vec<String>,
}

let tool = Tool::new("get_user_info");

println!("Tool name: {}", tool.name);
println!("Tool description: {}", tool.description);
println!("Tool parameters: {:?}", tool.parameters);

*/

fn main() {
    print_hello_world();
}
