use std::env;
use std::process::{exit};
use diff_with_branch::run_diff;

fn main() {
    // Expect exactly one argument: the file path.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        exit(1);
    }
    let file_path = &args[1];

    match run_diff(file_path) {
        Ok(Some(diff)) => print!("{}", diff),
        Ok(None) => {}, // No output if untracked or no diff.
        Err(e) => {
            eprintln!("{}", e);
            exit(1);
        }
    }
}
