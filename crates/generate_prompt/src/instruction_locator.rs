// crates/generate_prompt/src/instruction_locator.rs

use anyhow::{Context, Result};
use std::env;
use find_prompt_instruction::find_prompt_instruction_in_dir;

/// Locates the TODO instruction file.
///
/// This function checks for the environment variable `GET_INSTRUCTION_FILE`. If it's set, its value
/// is returned. Otherwise, it searches for the prompt instruction file starting from the provided
/// directory (typically the Git root).
///
/// # Arguments
///
/// * `search_dir` - The base directory to search in.
///
/// # Returns
///
/// A `Result` with the path to the instruction file as a `String` on success.
pub fn locate_instruction_file(search_dir: &str) -> Result<String> {
    if let Ok(instruction_override) = env::var("GET_INSTRUCTION_FILE") {
        Ok(instruction_override)
    } else {
        let instruction_path_buf = find_prompt_instruction_in_dir(search_dir, false)
            .context("Failed to locate the TODO instruction")?;
        Ok(instruction_path_buf.to_string_lossy().into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::tempdir;

    // Note: This test creates a temporary directory with a fake instruction file.
    #[test]
    fn test_locate_instruction_file_with_override() {
        env::set_var("GET_INSTRUCTION_FILE", "/tmp/override_instruction.txt");
        let result = locate_instruction_file("/dummy/path").unwrap();
        assert_eq!(result, "/tmp/override_instruction.txt");
        env::remove_var("GET_INSTRUCTION_FILE");
    }

    #[test]
    fn test_locate_instruction_file_search() {
        // Create a temporary directory structure with an instruction file.
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("instruction.swift");
        let mut file = File::create(&file_path).unwrap();
        // Write a fake TODO marker line.
        writeln!(file, "// TODO: - Do something important").unwrap();

        // For testing, we'll override the find function by creating a minimal file structure.
        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false).unwrap();
        assert!(Path::new(&result).exists());
    }
}
