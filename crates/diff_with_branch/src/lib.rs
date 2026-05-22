// crates/diff_with_branch/src/lib.rs

use anyhow::{anyhow, Context, Result};
use std::env;
use std::path::Path;
use std::process::{Command, Stdio};

/// Returns the diff for the given file (if any), comparing the current working copy
/// against the branch specified in the `DIFF_WITH_BRANCH` environment variable (or "main"
/// if not set). If the file is not tracked by Git or there is no diff, returns Ok(None).
pub fn run_diff(file_path: &Path) -> Result<Option<String>> {
    let branch = env::var("DIFF_WITH_BRANCH").unwrap_or_else(|_| "main".to_string());
    run_diff_against(file_path, &branch)
}

/// Returns the diff for the given file (if any), comparing the current working copy
/// against the provided branch. If the file is not tracked by Git or there is no diff,
/// returns Ok(None).
pub fn run_diff_against(file_path: &Path, branch: &str) -> Result<Option<String>> {
    let file_dir = file_path
        .parent()
        .ok_or_else(|| anyhow!("Failed to determine file directory"))?;

    let file_path_str = file_path.to_string_lossy();

    let ls_files_status = Command::new("git")
        .args(["ls-files", "--error-unmatch", file_path_str.as_ref()])
        .current_dir(file_dir)
        .stderr(Stdio::null())
        .status()
        .context("Error executing git ls-files")?;

    if !ls_files_status.success() {
        return Ok(None);
    }

    let diff_output = Command::new("git")
        .args(["diff", branch, "--", file_path_str.as_ref()])
        .current_dir(file_dir)
        .stderr(Stdio::null())
        .output()
        .context("Error executing git diff")?;

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

    fn init_git_repo(dir: &std::path::Path) {
        Command::new("git")
            .arg("init")
            .current_dir(dir)
            .output()
            .expect("Failed to initialize git repo");

        Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(dir)
            .output()
            .expect("Failed to configure git user.email");
        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir)
            .output()
            .expect("Failed to configure git user.name");
    }

    #[test]
    fn test_file_not_tracked() {
        let dir = tempdir().expect("Failed to create temp dir");
        let temp_path = dir.path();
        init_git_repo(temp_path);

        let file_path = temp_path.join("untracked.txt");
        File::create(&file_path).expect("Failed to create file");

        let result = run_diff(&file_path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_tracked_file_no_diff() {
        let dir = tempdir().expect("Failed to create temp dir");
        let temp_path = dir.path();
        init_git_repo(temp_path);

        let file_path = temp_path.join("tracked.txt");
        {
            let mut file = File::create(&file_path).expect("Failed to create file");
            writeln!(file, "Initial content").expect("Failed to write to file");
        }
        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add file");
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit");

        let result = run_diff(&file_path);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }

    #[test]
    fn test_tracked_file_with_diff() {
        let dir = tempdir().expect("Failed to create temp dir");
        let temp_path = dir.path();
        init_git_repo(temp_path);

        let file_path = temp_path.join("tracked.txt");
        {
            let mut file = File::create(&file_path).expect("Failed to create file");
            writeln!(file, "Initial content").expect("Failed to write to file");
        }
        Command::new("git")
            .args(["add", "tracked.txt"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to add file");
        Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp_path)
            .output()
            .expect("Failed to commit");

        {
            let mut file = File::create(&file_path).expect("Failed to open file for modification");
            writeln!(file, "Modified content").expect("Failed to write modification");
        }

        let result = run_diff_against(&file_path, "HEAD");
        assert!(result.is_ok());
        let diff = result.unwrap();
        assert!(diff.is_some(), "Expected diff but got None");
        let diff_str = diff.unwrap();
        assert!(diff_str.contains("Modified content"));
    }
}
