use std::path::Path;
use std::process::{Command, Stdio};
use std::io::{self, Write};

/// Verifies that if the `DIFF_WITH_BRANCH` environment variable is set,
/// the specified Git branch exists in the repository rooted at `git_root`.
/// If the branch does not exist or there is an error executing Git, the process exits.
pub fn verify_diff_branch(git_root: &Path) {
    if let Ok(diff_branch) = std::env::var("DIFF_WITH_BRANCH") {
        let status = Command::new("git")
            .args(&["rev-parse", "--verify", &diff_branch])
            .current_dir(git_root)
            .stderr(Stdio::null())
            .status()
            .unwrap_or_else(|err| {
                eprintln!("Error executing git rev-parse: {}", err);
                io::stderr().flush().unwrap();
                std::process::exit(1);
            });
        if !status.success() {
            eprintln!("Error: Branch '{}' does not exist.", diff_branch);
            io::stderr().flush().unwrap();
            std::process::exit(1);
        }
    }
}

/// Helper function for testing purposes.
/// When the environment variable GIT_UTILS_RUN_CHILD is set to "1",
/// this function calls `verify_diff_branch` using the path from GIT_UTILS_TEST_PATH,
/// then exits with code 0 if verification passes.
#[allow(dead_code)]
pub fn run_child_mode() {
    if std::env::var("GIT_UTILS_RUN_CHILD").unwrap_or_default() == "1" {
        let path = std::env::var("GIT_UTILS_TEST_PATH").expect("GIT_UTILS_TEST_PATH not set");
        verify_diff_branch(Path::new(&path));
        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::process::Command;
    use tempfile::tempdir;
    use std::fs;

    /// Test that when DIFF_WITH_BRANCH is not set, verify_diff_branch returns normally.
    #[test]
    fn test_verify_diff_branch_no_diff() {
        env::remove_var("DIFF_WITH_BRANCH");
        let dir = tempdir().unwrap();
        verify_diff_branch(dir.path());
    }

    /// Test that when DIFF_WITH_BRANCH is set to an existing branch, verification passes.
    #[test]
    fn test_verify_diff_branch_existing() {
        let dir = tempdir().unwrap();
        // Initialize a git repository.
        let init_status = Command::new("git")
            .arg("init")
            .current_dir(dir.path())
            .status()
            .expect("Failed to initialize git repository");
        assert!(init_status.success());

        // Create an initial commit so that HEAD exists.
        let dummy_file = dir.path().join("dummy.txt");
        fs::write(&dummy_file, "dummy").unwrap();
        let add_status = Command::new("git")
            .args(&["add", "dummy.txt"])
            .current_dir(dir.path())
            .status()
            .expect("Failed to add dummy.txt");
        assert!(add_status.success());
        let commit_status = Command::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(dir.path())
            .status()
            .expect("Failed to commit");
        assert!(commit_status.success());

        // Create a branch called "test_branch".
        let branch_status = Command::new("git")
            .args(&["checkout", "-b", "test_branch"])
            .current_dir(dir.path())
            .status()
            .expect("Failed to create test_branch");
        assert!(branch_status.success());

        env::set_var("DIFF_WITH_BRANCH", "test_branch");
        verify_diff_branch(dir.path());
        env::remove_var("DIFF_WITH_BRANCH");
    }

    /// A helper test function that, when run in child mode, calls run_child_mode().
    #[test]
    fn run_git_utils_child_mode_test() {
        run_child_mode();
    }

    /// For the negative test, we spawn a child process with DIFF_WITH_BRANCH set to a nonexistent branch.
    #[test]
    fn test_verify_diff_branch_nonexistent_child() {
        let dir = tempdir().unwrap();
        // Initialize a git repository.
        let init_status = Command::new("git")
            .arg("init")
            .current_dir(dir.path())
            .status()
            .expect("Failed to initialize git repository");
        assert!(init_status.success());

        env::set_var("DIFF_WITH_BRANCH", "nonexistent_branch");

        let output = Command::new(env::current_exe().unwrap())
            .args(&[] as &[&str])  // Override any default arguments.
            .env("GIT_UTILS_RUN_CHILD", "1")
            .env("GIT_UTILS_TEST_PATH", dir.path().to_str().unwrap())
            .output()
            .expect("Failed to execute child process");

        // The child process should exit with a nonzero status.
        assert!(!output.status.success(), "Child process unexpectedly succeeded");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(
            stderr.contains("Error: Branch 'nonexistent_branch' does not exist."),
            "Expected error message not found in stderr: {}",
            stderr
        );
        env::remove_var("DIFF_WITH_BRANCH");
    }
}
