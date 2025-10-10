pub mod zero_copy_deserialization;

use zero_copy_deserialization::run_zero_copy_deserialization;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    
    if args.len() > 1 && args[1] == "1_1" {
        run_zero_copy_deserialization();
    } else if args.len() > 1 && args[1] == "1_2" {
        run_example_2();
    } else {
        println!("Usage: cargo run 1_1");
        println!("This will run the zero-copy deserialization example.");
    }
}
