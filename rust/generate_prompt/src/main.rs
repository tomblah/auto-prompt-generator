use anyhow::{bail, Context, Result};
use clap::{Arg, Command};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use get_package_root::get_package_root;
// New import: call extract_instruction_content directly as a library function.
use extract_instruction_content::extract_instruction_content;

fn main() -> Result<()> {
    // Parse command-line arguments using Clap.
    let matches = Command::new("generate_prompt")
        .version("0.1.0")
        .about("Generates an AI prompt by delegating to existing Rust binaries")
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
    let diff_branch = matches.get_one::<String>("diff_with").map(String::as_str);
    let verbose = *matches.get_one::<bool>("verbose").unwrap();
    let excludes: Vec<String> = matches
        .get_many::<String>("exclude")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect();

    // 1. Save the current directory and determine the Git root.
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    
    println!("--------------------------------------------------");
    println!("Current directory: {}", current_dir.display());
    
    // Use external binary call for git root (unchanged for now).
    let git_root = run_command(&["get_git_root"], None)
        .context("Failed to determine Git root")?
        .trim()
        .to_string();
    println!("Git root: {}", git_root);
    println!("--------------------------------------------------");
    
    env::set_current_dir(&git_root).context("Failed to change directory to Git root")?;

    // 2. Locate the TODO instruction file.
    let file_path = run_command(&["find_prompt_instruction", &git_root], None)
        .context("Failed to locate the TODO instruction")?
        .trim()
        .to_string();
    println!("Found exactly one instruction in {}", file_path);
    println!("--------------------------------------------------");

    // 3. Set environment variable TODO_FILE_BASENAME.
    let todo_file_basename = PathBuf::from(&file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    env::set_var("TODO_FILE_BASENAME", &todo_file_basename);

    // Enforce singular mode for JavaScript files.
    if file_path.ends_with(".js") && !singular {
        eprintln!("WARNING: JavaScript support is beta – enforcing singular mode.");
    }
    // If --include-references is set but the file isn't Swift, exit with an error.
    if include_references && !file_path.ends_with(".swift") {
        eprintln!("Error: --include-references is only supported for Swift files.");
        std::process::exit(1);
    }
    // 4. Determine package scope.
    // Instead of calling an external process, use the library function.
    let package_root = get_package_root(Path::new(&file_path))
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "".to_string());
    let search_root = if force_global {
        println!("Force global enabled: using Git root for context");
        git_root.clone()
    } else if !package_root.is_empty() {
        println!("Found package root: {}", package_root);
        package_root
    } else {
        git_root.clone()
    };
    println!("Search root: {}", search_root);

    // 5. Extract the instruction content using the library function.
    let instruction_content = extract_instruction_content(&file_path)
        .context("Failed to extract instruction content")?;
    println!("Instruction content: {}", instruction_content.trim());

    // 6. Determine files to include.
    let found_files_path: PathBuf;
    if singular {
        println!("Singular mode enabled: only including the TODO file");
        // Create a temporary file containing only the TODO file path.
        found_files_path = {
            let mut temp = tempfile::NamedTempFile::new()
                .context("Failed to create temporary file for singular mode")?;
            writeln!(temp, "{}", file_path)
                .context("Failed to write TODO file in singular mode")?;
            temp.into_temp_path()
                .keep()
                .context("Failed to persist singular file list")?
        };
    } else {
        // Non-singular mode:
        let types_file = run_command(&["extract_types", &file_path], None)
            .context("Failed to extract types")?;
        // Read and print the contents of the types file.
        let types_path = types_file.trim();
        let types_content = fs::read_to_string(types_path)
            .context("Failed to read types file")?;
        println!("Types found:");
        println!("{}", types_content.trim());
        println!("--------------------------------------------------");

        let def_files_content = run_command(
            &["find_definition_files", types_path, &search_root],
            None,
        )
        .context("Failed to find definition files")?;
        // Create a temporary file and write the found files content into it.
        found_files_path = {
            let mut temp = tempfile::NamedTempFile::new()
                .context("Failed to create temporary file for found files")?;
            write!(temp, "{}", def_files_content)
                .context("Failed to write to temporary found files file")?;
            temp.into_temp_path()
                .keep()
                .context("Failed to persist temporary found files list")?
        };
        // Append the TODO file to this temporary file.
        {
            use std::fs::OpenOptions;
            let found_files_path_str = found_files_path.to_string_lossy();
            let mut f = OpenOptions::new()
                .append(true)
                .open(&found_files_path)
                .context(format!("Failed to open found files list at {}", found_files_path_str))?;
            writeln!(f, "{}", file_path).context("Failed to append TODO file")?;
        }
        // If exclusion flags are provided, filter the file list.
        let mut found_files = fs::read_to_string(&found_files_path)
            .context("Failed to read found files list")?;
        if !excludes.is_empty() {
            println!("Excluding files matching: {:?}", excludes);
            let mut args = vec!["filter_excluded_files", found_files_path.to_str().unwrap()];
            for excl in &excludes {
                args.push(excl);
            }
            found_files = run_command(&args, None)
                .context("Failed to filter excluded files")?;
            fs::write(&found_files_path, found_files.trim())
                .context("Failed to write final excluded list")?;
        }
    }

    // 7. Optionally include referencing files.
    if include_references {
        println!("Including files that reference the enclosing type");
        let enclosing_type = run_command(&["extract_enclosing_type", &file_path], None)
            .unwrap_or_default()
            .trim()
            .to_string();
        if !enclosing_type.is_empty() {
            println!("Enclosing type: {}", enclosing_type);
            println!("Searching for files referencing {}", enclosing_type);
            let referencing_files = run_command(
                &["find_referencing_files", &enclosing_type, &search_root],
                None,
            )
            .context("Failed to find referencing files")?;
            {
                use std::fs::OpenOptions;
                let mut f = OpenOptions::new()
                    .append(true)
                    .open(&found_files_path)
                    .context("Failed to open found files list for appending referencing files")?;
                writeln!(f, "{}", referencing_files.trim())
                    .context("Failed to append referencing files")?;
            }
        } else {
            println!("No enclosing type found; skipping reference search.");
        }
    }

    // 8. Print the final list of files.
    let files_content = fs::read_to_string(&found_files_path)
        .context("Failed to read found files list")?;
    let mut file_paths: Vec<String> = files_content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    file_paths.sort();
    file_paths.dedup();
    println!("--------------------------------------------------");
    println!("Files (final list):");
    for file in file_paths {
        let basename = Path::new(&file)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        println!("{}", basename);
    }

    // 9. Assemble the final prompt.
    let final_prompt = run_command(
        &["assemble_prompt", found_files_path.to_str().unwrap(), instruction_content.trim()],
        None,
    )
    .context("Failed to assemble prompt")?;
    
    let marker = "// TODO: -";
    let marker_lines: Vec<&str> = final_prompt
        .lines()
        .filter(|line| line.contains(marker))
        .collect();

    // NB: > 2 b/c there's another // TODO: - marker in the CTA
    if marker_lines.len() > 2 {
        eprintln!("Multiple {} markers found. Exiting.", marker);
        for line in marker_lines.iter().take(marker_lines.len() - 1) {
            eprintln!("{}", line.trim());
        }
        std::process::exit(1);
    }

    println!("--------------------------------------------------");
    println!("Success:\n");
    println!("{}", instruction_content.trim());
    if include_references {
        println!("\nWarning: The --include-references option is experimental.");
    }
    println!("--------------------------------------------------\n");
    println!("Prompt has been copied to clipboard.");
    
    Ok(())
}

/// Helper function to run an external command and capture its stdout as a String.
fn run_command(args: &[&str], envs: Option<&[(&str, &str)]>) -> Result<String, anyhow::Error> {
    if args.is_empty() {
        bail!("No command provided");
    }
    let cmd = args[0];
    let cmd_args = &args[1..];
    let mut command = std::process::Command::new(cmd);
    command.args(cmd_args);
    if let Some(env_vars) = envs {
        for &(key, value) in env_vars {
            command.env(key, value);
        }
    }
    let output = command
        .output()
        .with_context(|| format!("Failed to execute command: {:?}", args))?;
    if !output.status.success() {
        bail!("Command {:?} failed with status {}", args, output.status);
    }
    let stdout = String::from_utf8(output.stdout).context("Output not valid UTF-8")?;
    Ok(stdout)
}
