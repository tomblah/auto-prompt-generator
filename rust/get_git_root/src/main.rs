// File: rust/get_git_root/src/main.rs

use std::process::{Command, exit};

/// Returns the Git repository root as a trimmed String,
/// or an error message if the current directory is not inside a Git repository.
pub fn get_git_root() -> Result<String, String> {
    let output = Command::new("git")
        .args(&["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("Failed to execute git: {}", e))?;
    if output.status.success() {
        let git_root = String::from_utf8_lossy(&output.stdout)
            .trim()
            .to_string();
        Ok(git_root)
    } else {
        Err("Error: Not a git repository.".to_string())
    }
}

fn main() {
    match get_git_root() {
        Ok(git_root) => println!("{}", git_root),
        Err(err) => {
            eprintln!("{}", err);
            exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::path::Path;
    use std::process::Command;
    use tempfile::tempdir;

    /// Creates a temporary directory, initializes a Git repository there,
    /// and then verifies that `get_git_root` returns the expected path.
    #[test]
    fn test_get_git_root_in_temp_repo() {
        // Create a temporary directory.
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Initialize a Git repository in the temp directory.
        let init_output = Command::new("git")
            .arg("init")
            .current_dir(&temp_path)
            .output()
            .expect("Failed to run git init");
        assert!(init_output.status.success());

        // Change the current directory to the temporary directory.
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(&temp_path).expect("Failed to set current dir");

        // Call the function under test.
        let result = get_git_root();

        // Restore the original directory.
        env::set_current_dir(original_dir).expect("Failed to restore current dir");

        // Assert that we got the expected Git root.
        assert!(result.is_ok());
        let git_root = result.unwrap();

        // Compare canonicalized paths to account for symlinks.
        let expected = fs::canonicalize(&temp_path).expect("Failed to canonicalize temp path");
        let actual = fs::canonicalize(Path::new(&git_root)).expect("Failed to canonicalize git root");
        assert_eq!(expected, actual);
    }

    /// Verifies that when not in a Git repository, the function returns an error.
    #[test]
    fn test_get_git_root_failure() {
        // Create a temporary directory that is not a Git repo.
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let temp_path = temp_dir.path();

        // Change the current directory to the temporary directory.
        let original_dir = env::current_dir().expect("Failed to get current dir");
        env::set_current_dir(&temp_path).expect("Failed to set current dir");

        // Call the function; expect an error.
        let result = get_git_root();

        // Restore the original directory.
        env::set_current_dir(original_dir).expect("Failed to restore current dir");

        assert!(result.is_err());
    }
}
