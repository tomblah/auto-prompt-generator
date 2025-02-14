use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// Performs the main logic: creates a unique temporary file, writes the given todo file path into it,
/// and returns the path of the temporary file.
fn run(todo_file: &str) -> Result<PathBuf, String> {
    // Determine the system temporary directory.
    let mut temp_path = env::temp_dir();

    // Generate a unique filename using the process ID and current timestamp.
    let pid = process::id();
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let unique_filename = format!("filter_files_singular_{}_{}.tmp", pid, now);
    temp_path.push(unique_filename);

    // Create the temporary file and write the todo_file path into it.
    let mut file = File::create(&temp_path)
        .map_err(|e| format!("Error creating temporary file: {}", e))?;
    writeln!(file, "{}", todo_file)
        .map_err(|e| format!("Error writing to temporary file: {}", e))?;

    Ok(temp_path)
}

fn main() {
    // Expect exactly one argument: the TODO file path.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <todo_file>", args[0]);
        process::exit(1);
    }
    let todo_file = &args[1];

    match run(todo_file) {
        Ok(path) => println!("{}", path.display()),
        Err(e) => {
            eprintln!("{}", e);
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;

    #[test]
    fn test_run_creates_temp_file_with_correct_content() {
        let todo = "Test TODO content";
        // Call the refactored run function.
        let temp_file_path = run(todo).expect("run() should succeed");

        // Check that the temporary file exists.
        assert!(temp_file_path.exists(), "Temp file should exist");

        // Read the file content and verify it matches the passed todo content.
        let mut file = fs::File::open(&temp_file_path).expect("Should be able to open temp file");
        let mut content = String::new();
        file.read_to_string(&mut content)
            .expect("Should be able to read temp file");

        // Remove trailing newline (written by writeln!)
        let content = content.trim_end();
        assert_eq!(content, todo, "The file content should match the todo string");

        // Cleanup: remove the temporary file.
        fs::remove_file(&temp_file_path).expect("Failed to remove temp file");
    }
}
