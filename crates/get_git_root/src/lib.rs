// crates/get_git_root/src/lib.rs

use anyhow::{anyhow, Context, Result};
use std::process::Command;

/// Returns the Git repository root as a trimmed String,
/// or an error message if the current directory is not inside a Git repository.
pub fn get_git_root() -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("Failed to execute git")?;
    if output.status.success() {
        let git_root = String::from_utf8_lossy(&output.stdout).trim().to_string();
        Ok(git_root)
    } else {
        Err(anyhow!("Error: Not a git repository."))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn test_get_git_root_success() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        let init_output = Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output()
            .expect("Failed to run git init");
        assert!(init_output.status.success());

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(repo_path).unwrap();

        let git_root = get_git_root().expect("Failed to get git root");
        let repo_path_canon = fs::canonicalize(repo_path).unwrap();
        let git_root_canon = fs::canonicalize(&git_root).unwrap();
        assert_eq!(git_root_canon, repo_path_canon);

        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_get_git_root_failure() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let non_repo_path = temp_dir.path();

        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(non_repo_path).unwrap();

        let result = get_git_root();
        assert!(result.is_err());

        env::set_current_dir(original_dir).unwrap();
    }
}
