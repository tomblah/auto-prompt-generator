// crates/generate_prompt_core/src/instruction_locator.rs

use anyhow::{Context, Result};
use find_prompt_instruction::find_prompt_instruction_in_dir;
use std::path::{Path, PathBuf};

/// Locates the TODO instruction file by searching the provided directory.
///
/// This is a pure, parameter-driven wrapper: it searches for the prompt instruction file
/// starting from the given directory (typically the Git root). Any caller-supplied override
/// (such as an environment-based test seam) is resolved at the binary edge, not here.
///
/// # Arguments
///
/// * `search_dir` - The base directory to search in.
///
/// # Returns
///
/// A `Result` with the path to the instruction file as a `PathBuf` on success.
pub fn locate_instruction_file(search_dir: &Path) -> Result<PathBuf> {
    find_prompt_instruction_in_dir(search_dir).context("Failed to locate the TODO instruction")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_locate_instruction_file_search() {
        // Create a temporary directory structure with an instruction file.
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("instruction.swift");
        let mut file = File::create(&file_path).unwrap();
        // Write a fake TODO marker line.
        writeln!(file, "// TODO: - Do something important").unwrap();

        let result = locate_instruction_file(dir.path()).unwrap();
        assert_eq!(result, file_path);
        assert!(result.exists());
    }

    #[test]
    fn test_locate_instruction_file_errors_when_missing() {
        // An empty directory has no instruction file, so location should fail.
        let dir = tempdir().unwrap();
        let result = locate_instruction_file(dir.path());
        assert!(result.is_err());
    }
}
