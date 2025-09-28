// crates/generate_prompt/src/main.rs

mod config;

use anyhow::{Context, Result};
use clap::{Arg, Command};
use std::env;
use std::process::{Command as ProcessCommand, Stdio};

// Library dependencies.
use get_git_root::get_git_root;

mod clipboard;
mod search_root;
mod file_selector;
mod prompt_validation;
mod instruction_locator;
mod prompt_generator; // New module containing the core orchestration

fn main() -> Result<()> {
    let matches = Command::new("generate_prompt")
        .version("0.1.0")
        .about("Generates an AI prompt by delegating to existing Rust libraries and binaries")
        .arg(
            Arg::new("singular")
                .long("singular")
                .help("Only include the TODO file")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("force_global")
                .long("force-global")
                .help("Force global context inclusion")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("include_references")
                .long("include-references")
                .help("Include files that reference the enclosing type")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("diff_with")
                .long("diff-with")
                .num_args(1)
                .help("Include diff report against the specified branch"),
        )
        .arg(
            Arg::new("exclude")
                .long("exclude")
                .action(clap::ArgAction::Append)
                .help("Exclude file(s) whose basename match the given name"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        // New flag for targeted type extraction.
        .arg(
            Arg::new("tgtd")
                .long("tgtd")
                .help("Only consider types from the enclosing block for extraction")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .get_matches();

    let singular = *matches.get_one::<bool>("singular").unwrap();
    let force_global = *matches.get_one::<bool>("force_global").unwrap();
    let include_references = *matches.get_one::<bool>("include_references").unwrap();
    let excludes: Vec<String> = matches
        .get_many::<String>("exclude")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect();

    // Set the diff branch from CLI if not already set via env.
    if env::var("DIFF_WITH_BRANCH").is_err() {
        if let Some(diff_branch) = matches.get_one::<String>("diff_with") {
            env::set_var("DIFF_WITH_BRANCH", diff_branch);
        }
    }

    // Set the TARGETED environment variable if the flag is enabled.
    if *matches.get_one::<bool>("tgtd").unwrap() {
        env::set_var("TARGETED", "1");
    }

    // 1. Save the current directory and determine the Git root.
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    println!("--------------------------------------------------");
    println!("Current directory: {}", current_dir.display());

    let git_root = if let Ok(git_root_override) = env::var("GET_GIT_ROOT") {
        git_root_override
    } else {
        get_git_root().expect("Failed to determine Git root")
    };
    println!("Git root: {}", git_root);
    println!("--------------------------------------------------");

    // 2. If a diff branch is specified, verify it exists.
    if let Ok(diff_branch) = env::var("DIFF_WITH_BRANCH") {
        let verify_status = ProcessCommand::new("git")
            .args(&["rev-parse", "--verify", &diff_branch])
            .current_dir(&git_root)
            .stderr(Stdio::null())
            .status()
            .unwrap_or_else(|err| {
                eprintln!("Error executing git rev-parse: {}", err);
                std::process::exit(1);
            });
        if !verify_status.success() {
            eprintln!("Error: Branch '{}' does not exist.", diff_branch);
            std::process::exit(1);
        }
    }

    // 3. Change directory to the Git root.
    env::set_current_dir(&git_root).context("Failed to change directory to Git root")?;

    // 4. Locate the instruction file.
    let file_path = instruction_locator::locate_instruction_file(&git_root)
        .context("Failed to locate the instruction file")?;
    println!("Found exactly one instruction in {}", file_path);
    // Build AppConfig (no behavior change; just logging if verbose)
    let verbose = *matches.get_one::<bool>("verbose").unwrap();
    let todo_file_basename = std::path::Path::new(&file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    let app_config = crate::config::AppConfig {
        git_root: git_root.clone(),
        instruction_file: file_path.clone(),
        singular,
        force_global,
        include_references,
        excludes: excludes.clone(),
        diff_branch: std::env::var("DIFF_WITH_BRANCH").ok(),
        targeted: std::env::var("TARGETED").is_ok(),
        disable_pbcopy: std::env::var("DISABLE_PBCOPY").is_ok(),
        todo_file_basename,
        verbose,
    };
    if app_config.verbose {
        eprintln!("[VERBOSE] AppConfig = {:?}", app_config);
    }
    println!("--------------------------------------------------");

    // 5. Delegate to the prompt generator module.
    prompt_generator::generate_prompt(
        &git_root,
        &file_path,
        singular,
        force_global,
        include_references,
        &excludes,
    )?;

    Ok(())
}
