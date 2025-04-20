// crates/extract_instruction_content/src/lib.rs

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use anyhow::{Context, Result};
// Import the TODO_MARKER constant from the shared utility crate
use substring_marker_snippet_extractor::utils::marker_utils::TODO_MARKER;

/// Reads the given Swift file and returns the first line that contains the TODO marker.
/// The returned string is trimmed of any leading whitespace.
///
/// # Arguments
///
/// * `file_path` - Path to the Swift file.
///
/// # Errors
///
/// Returns an error if the file cannot be opened, read, or if no valid TODO instruction is found.
pub fn extract_instruction_content<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let file_path_ref = file_path.as_ref();
    let file = File::open(file_path_ref)
        .with_context(|| format!("Error opening file {}", file_path_ref.display()))?;
    let reader = BufReader::new(file);
    // Use the imported constant instead of a hardcoded string
    let marker = TODO_MARKER;

    for line in reader.lines() {
        let line = line.with_context(|| format!("Error reading file {}", file_path_ref.display()))?;
        if line.contains(marker) {
            return Ok(line.trim_start().to_string());
        }
    }

    anyhow::bail!("No valid TODO instruction found in {}", file_path_ref.display());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use std::path::Path;

    #[test]
    fn test_extract_valid_todo() {
        // Create a temporary file with a valid TODO marker.
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let content = "\n// Some comment\n    // TODO: - Fix the bug\n// Another comment";
        write!(temp_file, "{}", content).expect("Failed to write to temp file");

        let result = extract_instruction_content(temp_file.path());
        assert!(result.is_ok());
        let extracted = result.unwrap();
        // The function should trim leading whitespace.
        assert_eq!(extracted, "// TODO: - Fix the bug");
    }

    #[test]
    fn test_extract_multiple_todo_returns_first() {
        // Create a file with multiple TODO markers.
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let content = "\n// TODO: - First todo\nSome code\n// TODO: - Second todo";
        write!(temp_file, "{}", content).expect("Failed to write to temp file");

        let result = extract_instruction_content(temp_file.path());
        assert!(result.is_ok());
        let extracted = result.unwrap();
        assert_eq!(extracted, "// TODO: - First todo");
    }

    #[test]
    fn test_extract_no_todo() {
        // Create a file that does not contain a TODO marker.
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let content = "\n// Some comment\n// Another comment without marker";
        write!(temp_file, "{}", content).expect("Failed to write to temp file");

        let result = extract_instruction_content(temp_file.path());
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No valid TODO instruction found"));
    }

    #[test]
    fn test_non_existent_file() {
        // Pass a non-existent file path.
        let fake_path = Path::new("non_existent_file.swift");
        let result = extract_instruction_content(fake_path);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Error opening file"));
    }
    
    #[test]
    fn test_extract_instruction_read_error() {
        use std::io::Write;

        // Create a temporary file and write invalid UTF-8 bytes
        let mut temp_file = tempfile::NamedTempFile::new()
            .expect("Failed to create temp file");
        let invalid_utf8 = [0xFF, 0xFE, 0xFD];
        temp_file
            .write_all(&invalid_utf8)
            .expect("Failed to write invalid UTF-8");

        // Now attempt to extract the TODO instruction
        let result = extract_instruction_content(temp_file.path());

        // This should fail due to a read error
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Error reading file"),
            "Expected error message to contain 'Error reading file', got: {}",
            err_msg
        );
    }
}
