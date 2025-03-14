// crates/diff_with_branch/src/lib.rs

use std::env;
use std::path::Path;
use std::process::{Command, Stdio};

/// Returns the diff for the given file (if any), comparing the current working copy
/// against the branch specified in the `DIFF_WITH_BRANCH` environment variable (or "main"
/// if not set). If the file is not tracked by Git or there is no diff, returns Ok(None).
///
/// # Errors
///
/// Returns an Err with an error message if any Git command fails.
pub fn run_diff(file_path: &str) -> Result<Option<String>, String> {
    let file_path_obj = Path::new(file_path);
    let file_dir = file_path_obj
        .parent()
        .ok_or_else(|| "Failed to determine file directory".to_string())?;
    
    // Read the branch name from the environment variable, default to "main".
    let branch = env::var("DIFF_WITH_BRANCH").unwrap_or_else(|_| "main".to_string());

    // Check if the file is tracked by Git.
    let ls_files_status = Command::new("git")
        .args(&["ls-files", "--error-unmatch", file_path])
        .current_dir(file_dir)
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("Error executing git ls-files: {}", err))?;

    if !ls_files_status.success() {
        // File is not tracked.
        return Ok(None);
    }

    // Get the diff between the current branch and the specified branch.
    let diff_output = Command::new("git")
        .args(&["diff", &branch, "--", file_path])
        .current_dir(file_dir)
        .stderr(Stdio::null())
        .output()
        .map_err(|err| format!("Error executing git diff: {}", err))?;

    let diff_str = String::from_utf8_lossy(&diff_output.stdout);
    let diff_trimmed = diff_str.trim();

    if diff_trimmed.is_empty() {
        Ok(None)
    } else {
        Ok(Some(diff_trimmed.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;
    use tempfile::tempdir;

    /// Helper function to initialize a git repository in the given directory.
    fn init_git_repo(dir: &std::path::Path) {
        // Initialize the repository.
        Command::new("git")
            .arg("init")
            .current_dir(dir)
            .output()
            .expect("Failed to initialize git repo");

        // Configure user.name and user.email so that commits don't fail.
        Command::new("git")
            .args(&["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .output()
            .expect("Failed to configure git user.email");
        Command::new("git")
            .args(&["config", "user.name", "Test User"])
            .current_dir(dir)
            .output()
            .expect("Failed to configure git user.name");
    }

    #[test]
    fn test_file_not_tracked() {
        // Create a temporary directory and initialize a git repository.
        let dir = tempdir().expect("Failed to create temp dir");
        let temp_path = dir.path();
        init_git_repo(temp_path);

        // Create an untracked file.
        let file_path = temp_path.join("untracked.txt");
        File::create(&file_path).expect("Failed to create file");

        // Since the file is untracked, run_diff should return Ok(None).
        let result = run_diff(file_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_tracked_file_no_diff() {
        let dir = tempdir().expect("Failed to create temp dir");
        let temp_path = dir.path();
        init_git_repo(temp_path);

        // Create a file and commit it.
        let file_path = temp_path.join("tracked.txt");
        {
            let mut file = File::create(&file_path).expect("Failed to create file");
            writeln!(file, "Initial content").expect("Failed to write to file");
        }
        Command::new("git")
            .args(&["add", "tracked.txt"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add file");
        Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit");

        // No modifications made; run_diff should return Ok(None).
        let result = run_diff(file_path.to_str().unwrap());
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_tracked_file_with_diff() {
        let dir = tempdir().expect("Failed to create temp dir");
        let temp_path = dir.path();
        init_git_repo(temp_path);

        // Create a file and commit it.
        let file_path = temp_path.join("tracked.txt");
        {
            let mut file = File::create(&file_path).expect("Failed to create file");
            writeln!(file, "Initial content").expect("Failed to write to file");
        }
        Command::new("git")
            .args(&["add", "tracked.txt"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add file");
        Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit");

        // Modify the file.
        {
            let mut file = File::create(&file_path).expect("Failed to open file for modification");
            writeln!(file, "Modified content").expect("Failed to write modification");
        }

        let result = run_diff(file_path.to_str().unwrap());
        assert!(result.is_ok());
        let diff = result.unwrap();
        assert!(diff.is_some(), "Expected diff but got None");
        let diff_str = diff.unwrap();
        // Check that the diff output contains the modified content.
        assert!(diff_str.contains("Modified content"));
    }
}
