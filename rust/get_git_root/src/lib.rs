use std::process::Command;

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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    #[test]
    fn test_get_git_root_success() {
        // Create a temporary directory and initialize a git repository in it.
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let repo_path = temp_dir.path();

        // Initialize the git repository.
        let init_output = Command::new("git")
            .arg("init")
            .current_dir(repo_path)
            .output()
            .expect("Failed to run git init");
        assert!(init_output.status.success());

        // Change current directory to the temporary git repo.
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(repo_path).unwrap();

        // Test that get_git_root returns the repository path.
        let git_root = get_git_root().expect("Failed to get git root");
        let repo_path_canon = fs::canonicalize(repo_path).unwrap();
        let git_root_canon = fs::canonicalize(&git_root).unwrap();
        assert_eq!(git_root_canon, repo_path_canon);

        // Restore the original current directory.
        env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_get_git_root_failure() {
        // Create a temporary directory that is not a git repository.
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let non_repo_path = temp_dir.path();

        // Change current directory to the non-repo directory.
        let original_dir = env::current_dir().unwrap();
        env::set_current_dir(non_repo_path).unwrap();

        // Test that get_git_root returns an error.
        let result = get_git_root();
        assert!(result.is_err());

        // Restore the original current directory.
        env::set_current_dir(original_dir).unwrap();
    }
}
