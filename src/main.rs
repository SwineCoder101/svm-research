pub mod zero_copy_deserialization;
pub mod unsafe_rust_memory_management;
pub mod turbine_block_propagation;
pub mod account_state_management;

use zero_copy_deserialization::run_zero_copy_deserialization;
use account_state_management::run_account_state_management;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 && args[1] == "1_1" {
        run_zero_copy_deserialization();
    } else if args.len() > 1 && args[1] == "3_2" {
        run_account_state_management();
    } else if args.len() > 1 && args[1] == "turbine" {
        // Run the turbine block propagation example
        turbine_block_propagation::main();
    }
    
    else {
        println!("Usage: cargo run [1_1|3_2|turbine]");
        println!("1_1: zero-copy deserialization example");
        println!("3_2: account state management example");
        println!("turbine: turbine block propagation example");
    }
}
