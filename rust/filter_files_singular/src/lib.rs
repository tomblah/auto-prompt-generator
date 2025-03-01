use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// Creates a unique temporary file containing the given TODO file path.
///
/// # Arguments
///
/// * `todo_file` - The TODO file content (or path) to write into the temporary file.
///
/// # Returns
///
/// A `Result` with the path to the temporary file on success, or an error message on failure.
pub fn create_todo_temp_file(todo_file: &str) -> Result<PathBuf, String> {
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

    // Create the temporary file and write the TODO file content into it.
    let mut file = File::create(&temp_path)
        .map_err(|e| format!("Error creating temporary file: {}", e))?;
    writeln!(file, "{}", todo_file)
        .map_err(|e| format!("Error writing to temporary file: {}", e))?;

    Ok(temp_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Read;

    #[test]
    fn test_create_todo_temp_file_creates_temp_file_with_correct_content() {
        let todo = "Test TODO content";
        // Call the refactored create_todo_temp_file function.
        let temp_file_path = create_todo_temp_file(todo).expect("create_todo_temp_file() should succeed");

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
