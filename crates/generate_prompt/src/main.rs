//! crates/generate_prompt/src/main.rs

use std::{
    env,
    process::{Command, Stdio},
};
use anyhow::{Context, Result};
use log::{info, error};
use env_logger::{Builder, Target};

mod clipboard;
mod search_root;
mod file_selector;
mod prompt_validation;
mod prompt_generator;

use prompt_generator::generate_prompt;

fn main() -> Result<()> {
    // Initialize logger to stdout
    Builder::from_default_env()
        .target(Target::Stdout)
        .init();

    // --- Parse CLI flags
    let mut singular = false;
    let mut force_global = false;
    let mut include_references = false;
    let mut excludes: Vec<String> = Vec::new();

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--singular"            => singular = true,
            "--force-global"        => force_global = true,
            "--include-references"  => include_references = true,
            "--exclude"             => {
                if let Some(pat) = args.next() {
                    excludes.push(pat);
                }
            }
            "--diff-with"           => {
                if let Some(branch) = args.next() {
                    env::set_var("DIFF_WITH_BRANCH", branch);
                }
            }
            _ => { /* ignore */ }
        }
    }

    // --- Log current directory
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    info!("--------------------------------------------------");
    info!("Current directory: {}", current_dir.display());

    // --- Determine Git root
    let git_root = if let Ok(override_root) = env::var("GET_GIT_ROOT") {
        override_root
    } else {
        let out = Command::new("get_git_root")
            .stdout(Stdio::piped())
            .output()
            .context("Failed to run `get_git_root`")?;
        if !out.status.success() {
            error!("Error executing `get_git_root`");
            std::process::exit(1);
        }
        String::from_utf8(out.stdout)?
            .trim()
            .to_string()
    };
    info!("Git root: {}", git_root);
    info!("--------------------------------------------------");

    // cd into Git root so all helpers run in the right directory
    env::set_current_dir(&git_root).context("Failed to change directory to Git root")?;

    // --- Verify diff‑branch if requested
    if let Ok(diff_branch) = env::var("DIFF_WITH_BRANCH") {
        let status = Command::new("git")
            .arg("rev-parse")
            .arg("--verify")
            .arg(&diff_branch)
            .stderr(Stdio::null())
            .status()
            .unwrap_or_else(|e| {
                error!("Error executing `git rev-parse`: {}", e);
                std::process::exit(1);
            });
        if !status.success() {
            error!("Error: Branch '{}' does not exist.", diff_branch);
            std::process::exit(1);
        }
    }

    // --- Locate instruction file
    let instruction_file = if let Ok(path) = env::var("GET_INSTRUCTION_FILE") {
        path
    } else {
        let out = Command::new("find_prompt_instruction")
            .stdout(Stdio::piped())
            .output()
            .context("Failed to run `find_prompt_instruction`")?;
        if !out.status.success() {
            error!("Error executing `find_prompt_instruction`");
            std::process::exit(1);
        }
        String::from_utf8(out.stdout)?
            .trim()
            .to_string()
    };
    info!("Found exactly one instruction in {}", instruction_file);
    info!("--------------------------------------------------");

    // --- Generate the prompt (this also copies to clipboard inside)
    generate_prompt(
        &git_root,
        &instruction_file,
        singular,
        force_global,
        include_references,
        &excludes,
    )
    .unwrap_or_else(|e| {
        error!("Error during prompt generation: {}", e);
        std::process::exit(1);
    });

    Ok(())
}
