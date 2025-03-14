use std::path::Path;
use std::process::{Command, Stdio};

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
                std::process::exit(1);
            });
        if !status.success() {
            eprintln!("Error: Branch '{}' does not exist.", diff_branch);
            std::process::exit(1);
        }
    }
}
