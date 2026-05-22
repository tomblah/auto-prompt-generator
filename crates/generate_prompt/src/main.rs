// crates/generate_prompt/src/main.rs

use anyhow::{anyhow, Context, Result};
use clap::{Arg, Command};
use std::env;
use std::path::Path;
use std::process::{Command as ProcessCommand, Stdio};

use generate_prompt_core::instruction_locator;
use generate_prompt_core::prompt_generator::{self, GeneratePromptOptions};
use get_git_root::get_git_root;

mod clipboard;

fn init_logging(verbose: bool) {
    if verbose && env::var_os("RUST_LOG").is_none() {
        env::set_var("RUST_LOG", "debug");
    }
    env_logger::init();
}

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
    let diff_branch = matches.get_one::<String>("diff_with").cloned();
    let targeted = *matches.get_one::<bool>("tgtd").unwrap();
    let verbose = *matches.get_one::<bool>("verbose").unwrap();

    init_logging(verbose);

    let current_dir = env::current_dir().context("Failed to get current directory")?;
    println!("--------------------------------------------------");
    println!("Current directory: {}", current_dir.display());

    // Test seam: GET_GIT_ROOT overrides git-root discovery for integration tests.
    let git_root = if let Ok(git_root_override) = env::var("GET_GIT_ROOT") {
        git_root_override
    } else {
        get_git_root().context("Failed to determine Git root")?
    };
    println!("Git root: {}", git_root);
    println!("--------------------------------------------------");

    if let Some(diff_branch) = &diff_branch {
        let verify_status = ProcessCommand::new("git")
            .args(["rev-parse", "--verify", diff_branch])
            .current_dir(&git_root)
            .stderr(Stdio::null())
            .status()
            .with_context(|| "Error executing git rev-parse")?;
        if !verify_status.success() {
            return Err(anyhow!("Error: Branch '{}' does not exist.", diff_branch));
        }
    }

    env::set_current_dir(&git_root).context("Failed to change directory to Git root")?;

    let file_path = instruction_locator::locate_instruction_file(Path::new(&git_root))
        .context("Failed to locate the instruction file")?;
    println!("Found exactly one instruction in {}", file_path.display());
    println!("--------------------------------------------------");

    if force_global {
        println!("Force global enabled: using Git root for context");
    }
    if singular {
        println!("Singular mode enabled: only including the TODO file");
    }

    let output = prompt_generator::generate_prompt_with_options(
        &git_root,
        &file_path,
        &GeneratePromptOptions {
            singular,
            force_global,
            include_references,
            excludes,
            diff_branch,
            targeted,
        },
    )?;

    println!("Search root: {}", output.search_root.display());
    println!("Instruction content: {}", output.instruction_content);
    println!("--------------------------------------------------");
    if !output.types_found.is_empty() {
        println!("Types found:");
        for ty in &output.types_found {
            println!("{}", ty);
        }
        println!("--------------------------------------------------");
    }
    println!("Files (final list):");
    for file in &output.found_files {
        let basename = file.file_name().unwrap_or_default().to_string_lossy();
        println!("{}", basename);
    }
    println!("--------------------------------------------------");
    println!("Success:\n");
    println!("{}", output.instruction_content);
    println!("--------------------------------------------------\n");
    println!("Prompt has been copied to clipboard.");

    clipboard::copy_to_clipboard(&output.final_prompt)?;

    Ok(())
}
