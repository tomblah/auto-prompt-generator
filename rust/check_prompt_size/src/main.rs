use std::io::{self, Read};

fn main() {
    const MAX_LENGTH: usize = 100_000;
    let mut input = String::new();
    // Read all input from STDIN.
    if let Err(e) = io::stdin().read_to_string(&mut input) {
        eprintln!("Error reading input: {}", e);
        std::process::exit(1);
    }
    let prompt_length = input.chars().count();
    if prompt_length > MAX_LENGTH {
        eprintln!(
            "Warning: The prompt is {} characters long. This may exceed what the AI can handle effectively.",
            prompt_length
        );
    }
}
