use assert_cmd::Command;
use predicates::prelude::*;
use std::fs::File;
use std::io::Write;
use std::process::Command as StdCommand;
use tempfile::tempdir;

/// Helper function to initialize a git repository in the given directory.
fn init_git_repo(dir: &std::path::Path) {
    // Initialize the repository.
    StdCommand::new("git")
        .arg("init")
        .current_dir(dir)
        .output()
        .expect("Failed to initialize git repo");

    // Configure user.email and user.name so that commits succeed.
    StdCommand::new("git")
        .args(&["config", "user.email", "test@example.com"])
        .current_dir(dir)
        .output()
        .expect("Failed to configure git user.email");
    StdCommand::new("git")
        .args(&["config", "user.name", "Test User"])
        .current_dir(dir)
        .output()
        .expect("Failed to configure git user.name");
}

#[test]
fn test_diff_with_branch_integration() -> Result<(), Box<dyn std::error::Error>> {
    // Create a temporary directory to serve as our Git repository.
    let dir = tempdir()?;
    let repo_path = dir.path();

    // Initialize the Git repository.
    init_git_repo(repo_path);

    // Create a file, add it to Git, and commit.
    let file_path = repo_path.join("tracked.txt");
    {
        let mut file = File::create(&file_path)?;
        writeln!(file, "Initial content")?;
    }
    StdCommand::new("git")
        .args(&["add", "tracked.txt"])
        .current_dir(repo_path)
        .output()?;
    StdCommand::new("git")
        .args(&["commit", "-m", "Initial commit"])
        .current_dir(repo_path)
        .output()?;

    // Modify the file so that a diff is generated.
    {
        let mut file = File::create(&file_path)?;
        writeln!(file, "Modified content")?;
    }

    // Run the diff_with_branch binary on the file.
    let mut cmd = Command::cargo_bin("diff_with_branch")?;
    cmd.arg(file_path.to_str().unwrap())
        .current_dir(repo_path)
        .env("DIFF_WITH_BRANCH", "main");

    // Assert that the output contains "Modified content".
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Modified content"));

    Ok(())
}
