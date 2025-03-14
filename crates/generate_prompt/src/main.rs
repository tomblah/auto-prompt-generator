use anyhow::{Context, Result};
use clap::{Arg, Command};
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};

// Library dependencies.
use extract_instruction_content::extract_instruction_content;
use get_search_roots::get_search_roots;
use get_git_root::get_git_root;
use find_prompt_instruction::find_prompt_instruction_in_dir;
use extract_types::extract_types_from_file;
use extract_enclosing_type::extract_enclosing_type;
use find_referencing_files;

// Import the assemble_prompt library.
use assemble_prompt;
// NEW: Import the updated find_definition_files library function.
use find_definition_files::find_definition_files;
// NEW: Import the post_processing crate.
use post_processing;

// Import our new clipboard module.
mod clipboard;

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

    // Use DIFF_WITH_BRANCH from the environment if it already exists; otherwise, if provided as an argument, set it.
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

    // If diff mode is enabled, verify that the specified branch exists.
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

    let candidate_roots = get_search_roots(&base_dir)
        .unwrap_or_else(|_| vec![base_dir.clone()]);

    let search_root = if candidate_roots.len() == 1 {
        candidate_roots[0].clone()
    } else {
        let todo_path = PathBuf::from(&file_path);
        candidate_roots
            .into_iter()
            .find(|p| todo_path.starts_with(p))
            .unwrap_or(base_dir)
    };

    println!("Search root: {}", search_root.display());

    // 5. Extract instruction content.
    let instruction_content = extract_instruction_content(&file_path)
        .context("Failed to extract instruction content")?;
    println!("Instruction content: {}", instruction_content.trim());
    println!("--------------------------------------------------");

    // 6. Determine files to include, using an in-memory vector.
    let mut found_files: Vec<String> = Vec::new();

    if singular {
        println!("Singular mode enabled: only including the TODO file");
        found_files.push(file_path.clone());
    } else {
        // Extract types as a newline-separated string.
        let types_content = extract_types_from_file(&file_path)
            .context("Failed to extract types")?;
        println!("Types found:");
        println!("{}", types_content.trim());
        println!("--------------------------------------------------");

        // Find definition files using the extracted types string directly.
        let def_files_set = find_definition_files(
            types_content.as_str(),
            &search_root,
        )
        .map_err(|err| anyhow::anyhow!("Failed to find definition files: {}", err))?;
        
        // Add definition files to the in-memory list.
        for path in def_files_set {
            found_files.push(path.to_string_lossy().into_owned());
        }
        
        // Append the TODO file.
        found_files.push(file_path.clone());
        
        // Apply initial exclusion filtering.
        if !excludes.is_empty() {
            println!("Excluding files matching: {:?}", excludes);
            found_files.retain(|line| {
                let basename = Path::new(line)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                !excludes.contains(&basename.to_string())
            });
        }
    }

    // 7. Optionally include referencing files.
    if include_references {
        println!("Including files that reference the enclosing type");
        let enclosing_type = match extract_enclosing_type(&file_path) {
            Ok(ty) => ty,
            Err(err) => {
                eprintln!("Error extracting enclosing type: {}", err);
                String::new()
            }
        };
        if !enclosing_type.is_empty() {
            println!("Enclosing type: {}", enclosing_type);
            println!("Searching for files referencing {}", enclosing_type);
            let referencing_files = find_referencing_files::find_files_referencing(
                &enclosing_type,
                search_root.to_str().unwrap(),
            )
            .map_err(|e| anyhow::anyhow!("Failed to find referencing files: {}", e))?;
            found_files.extend(referencing_files);
        } else {
            println!("No enclosing type found; skipping reference search.");
        }
        // Reapply exclusion filtering after appending referencing files.
        if !excludes.is_empty() {
            println!("Excluding files matching: {:?}", excludes);
            found_files.retain(|line| {
                let basename = Path::new(line)
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy();
                !excludes.contains(&basename.to_string())
            });
        }
    }

    // Sort and deduplicate the final list.
    found_files.sort();
    found_files.dedup();
    println!("--------------------------------------------------");
    println!("Files (final list):");
    for file in &found_files {
        let basename = Path::new(file)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        println!("{}", basename);
    }

    // 8. Assemble the final prompt by calling the library function.
    let final_prompt = assemble_prompt::assemble_prompt(
        &found_files,
        instruction_content.trim(),
    )
    .context("Failed to assemble prompt")?;

    // Determine if diff mode is enabled.
    let diff_enabled = env::var("DIFF_WITH_BRANCH").is_ok();

    // 9a. Post-process the prompt to scrub extra TODO markers.
    // Here we supply the trimmed instruction content as the primary marker.
    // The post_processing function will preserve the first occurrence of this primary marker
    // and will never scrub the very last marker line.
    let final_prompt = post_processing::scrub_extra_todo_markers(&final_prompt, diff_enabled, instruction_content.trim())
        .unwrap_or_else(|err| {
            eprintln!("Error during post-processing: {}", err);
            std::process::exit(1);
        });

    // 10. Check that there are exactly two markers unless diff reporting is enabled.
    let marker = "// TODO: -";
    let marker_lines: Vec<&str> = final_prompt
        .lines()
        .filter(|line| line.contains(marker))
        .collect();
    if diff_enabled {
        if marker_lines.len() != 2 && marker_lines.len() != 3 {
            eprintln!(
                "Expected 2 or 3 {} markers (with diff enabled), but found {}. Exiting.",
                marker,
                marker_lines.len()
            );
            std::process::exit(1);
        }
    } else {
        if marker_lines.len() != 2 {
            eprintln!(
                "Expected exactly 2 {} markers, but found {}. Exiting.",
                marker,
                marker_lines.len()
            );
            std::process::exit(1);
        }
    }

    println!("--------------------------------------------------");
    println!("Success:\n");
    println!("{}", instruction_content.trim());
    if include_references {
        println!("\nWarning: The --include-references option is experimental.");
    }
    println!("--------------------------------------------------\n");
    println!("Prompt has been copied to clipboard.");

    // Use the new clipboard module to copy the final prompt.
    clipboard::copy_to_clipboard(&final_prompt);

    Ok(())
}
