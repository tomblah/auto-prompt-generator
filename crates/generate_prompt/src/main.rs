// crates/generate_prompt/src/main.rs

use anyhow::{Context, Result};
use clap::{Arg, Command};
use std::env;
use std::path::PathBuf;
use std::process::{Command as ProcessCommand, Stdio};

// Library dependencies.
use extract_instruction_content::extract_instruction_content;
use get_git_root::get_git_root;
use find_prompt_instruction::find_prompt_instruction_in_dir;
use assemble_prompt;
use post_processing;

// Import our new modules.
mod clipboard;
mod search_root;
mod file_selector;
mod prompt_validation; // Added the prompt_validation module

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
        .get_matches();

    let singular = *matches.get_one::<bool>("singular").unwrap();
    let force_global = *matches.get_one::<bool>("force_global").unwrap();
    let include_references = *matches.get_one::<bool>("include_references").unwrap();

    if env::var("DIFF_WITH_BRANCH").is_err() {
        if let Some(diff_branch) = matches.get_one::<String>("diff_with") {
            env::set_var("DIFF_WITH_BRANCH", diff_branch);
        }
    }
    let _verbose = *matches.get_one::<bool>("verbose").unwrap();
    let excludes: Vec<String> = matches
        .get_many::<String>("exclude")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect();

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

    env::set_current_dir(&git_root).context("Failed to change directory to Git root")?;

    // 2. Locate the TODO instruction file.
    let file_path = if let Ok(instruction_override) = env::var("GET_INSTRUCTION_FILE") {
        instruction_override
    } else {
        let instruction_path_buf = find_prompt_instruction_in_dir(&git_root, false)
            .context("Failed to locate the TODO instruction")?;
        instruction_path_buf.to_string_lossy().into_owned()
    };
    println!("Found exactly one instruction in {}", file_path);
    println!("--------------------------------------------------");

    // 3. Set environment variable TODO_FILE_BASENAME.
    let todo_file_basename = PathBuf::from(&file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    env::set_var("TODO_FILE_BASENAME", &todo_file_basename);

    if file_path.ends_with(".js") && !singular {
        eprintln!("WARNING: JavaScript support is beta – enforcing singular mode.");
    }
    if include_references && !file_path.ends_with(".swift") {
        eprintln!("Error: --include-references is only supported for Swift files.");
        std::process::exit(1);
    }

    // 4. Determine package scope.
    let base_dir = if force_global {
        println!("Force global enabled: using Git root for context");
        PathBuf::from(&git_root)
    } else {
        PathBuf::from(&git_root)
    };

    let search_root = if force_global {
        base_dir.clone()
    } else {
        search_root::determine_search_root(&base_dir, &file_path)
    };
    println!("Search root: {}", search_root.display());

    // 5. Extract instruction content.
    let instruction_content = extract_instruction_content(&file_path)
        .context("Failed to extract instruction content")?;
    println!("Instruction content: {}", instruction_content.trim());
    println!("--------------------------------------------------");

    // 6 & 7. Determine the list of files to include.
    let found_files = file_selector::determine_files_to_include(
        &file_path,
        singular,
        &search_root,
        &excludes,
        include_references,
    )?;

    // 8. Assemble the final prompt.
    let final_prompt = assemble_prompt::assemble_prompt(
        &found_files,
        instruction_content.trim(),
    )
    .context("Failed to assemble prompt")?;

    let diff_enabled = env::var("DIFF_WITH_BRANCH").is_ok();

    // 9a. Post-process the prompt.
    let final_prompt = post_processing::scrub_extra_todo_markers(&final_prompt, diff_enabled, instruction_content.trim())
        .unwrap_or_else(|err| {
            eprintln!("Error during post-processing: {}", err);
            std::process::exit(1);
        });

    // 10. Validate the marker count using the new prompt_validation module.
    prompt_validation::validate_marker_count(&final_prompt, diff_enabled)
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            std::process::exit(1);
        });

    println!("--------------------------------------------------");
    println!("Success:\n");
    println!("{}", instruction_content.trim());
    if include_references {
        println!("\nWarning: The --include-references option is experimental.");
    }
    println!("--------------------------------------------------\n");
    println!("Prompt has been copied to clipboard.");

    clipboard::copy_to_clipboard(&final_prompt);

    Ok(())
}
