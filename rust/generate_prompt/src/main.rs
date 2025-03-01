use anyhow::{bail, Context, Result};
use clap::{Arg, Command};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use which::which;

use extract_instruction_content::extract_instruction_content;
use get_search_roots::get_search_roots;
use get_git_root::get_git_root;
use find_prompt_instruction::find_prompt_instruction_in_dir;
// Use our library to process files.
use prompt_file_processor::process_file;
// Use the diff_with_branch library directly.
use diff_with_branch::run_diff;
// NEW: Use filter_excluded_files library directly.
use filter_excluded_files::filter_excluded_files_lines;
// NEW: Use the extract_types library function directly.
use extract_types::extract_types_from_file;
// NEW: Import the refactored filter_files_singular library.
use filter_files_singular;

fn main() -> Result<()> {
    // Parse command-line arguments using Clap.
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
    let _diff_branch_arg = matches.get_one::<String>("diff_with").map(String::as_str);
    let _verbose = *matches.get_one::<bool>("verbose").unwrap(); // unused, so underscore-prefixed
    let excludes: Vec<String> = matches
        .get_many::<String>("exclude")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect();

    // 1. Save the current directory and determine the Git root.
    let current_dir = env::current_dir().context("Failed to get current directory")?;
    println!("--------------------------------------------------");
    println!("Current directory: {}", current_dir.display());

    // Allow override for Git root in tests.
    let git_root = if let Ok(git_root_override) = env::var("GET_GIT_ROOT") {
        git_root_override
    } else {
        get_git_root().expect("Failed to determine Git root")
    };
    println!("Git root: {}", git_root);
    println!("--------------------------------------------------");

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

    // 6. Determine files to include.
    let found_files_path: PathBuf;
    if singular {
        println!("Singular mode enabled: only including the TODO file");
        found_files_path = filter_files_singular::create_todo_temp_file(&file_path)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to create singular temp file")?;
    } else {
        // Instead of invoking an external command for extract_types,
        // we now call the library function directly.
        let types_file_path = extract_types_from_file(&file_path)
            .context("Failed to extract types")?;
        // Read and print the types from the temporary file.
        let types_content = fs::read_to_string(&types_file_path)
            .context("Failed to read extracted types")?;
        println!("Types found:");
        println!("{}", types_content.trim());
        println!("--------------------------------------------------");

        // For find_definition_files, we use the types file directly.
        let def_files_content = run_command(
            &[
                "find_definition_files",
                types_file_path.as_str(),
                search_root.to_str().unwrap(),
            ],
            None,
        )
        .context("Failed to find definition files")?;
        found_files_path = {
            let mut temp = tempfile::NamedTempFile::new()
                .context("Failed to create temporary file for found files")?;
            write!(temp, "{}", def_files_content)
                .context("Failed to write to temporary found files file")?;
            temp.into_temp_path()
                .keep()
                .context("Failed to persist temporary found files list")?
        };
        {
            use std::fs::OpenOptions;
            let found_files_path_str = found_files_path.to_string_lossy();
            let mut f = OpenOptions::new()
                .append(true)
                .open(&found_files_path)
                .context(format!("Failed to open found files list at {}", found_files_path_str))?;
            writeln!(f, "{}", file_path).context("Failed to append TODO file")?;
        }
        let _found_files = fs::read_to_string(&found_files_path)
            .context("Failed to read found files list")?;
        if !excludes.is_empty() {
            println!("Excluding files matching: {:?}", excludes);
            let found_files_content = fs::read_to_string(&found_files_path)
                .context("Failed to read found files list")?;
            let lines: Vec<String> = found_files_content
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            let filtered_lines = filter_excluded_files_lines(lines, &excludes);
            fs::write(&found_files_path, filtered_lines.join("\n"))
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
                &[
                    "find_referencing_files",
                    &enclosing_type,
                    search_root.to_str().unwrap(),
                ],
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
    for file in &file_paths {
        let basename = Path::new(file)
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        println!("{}", basename);
    }

    // 9. Assemble the final prompt.
    let fixed_instruction = "Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen";
    let final_prompt = if which("assemble_prompt").is_ok() {
        run_command(
            &[
                "assemble_prompt",
                found_files_path.to_str().unwrap(),
                instruction_content.trim(),
            ],
            None,
        )
        .context("Failed to assemble prompt")?
    } else {
        // Fallback: assemble using library processing.
        let mut prompt = String::new();
        for file in &file_paths {
            let processed_content = match process_file(file, Some(&todo_file_basename)) {
                Ok(content) => content,
                Err(err) => {
                    eprintln!("Error processing {}: {}. Falling back to file contents.", file, err);
                    fs::read_to_string(file).unwrap_or_default()
                }
            };
            let basename = Path::new(file)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy();
            prompt.push_str(&format!(
                "\nThe contents of {} is as follows:\n\n{}\n",
                basename, processed_content
            ));
            prompt.push_str("\n--------------------------------------------------\n");
        }
        prompt.push_str(&format!("\n\n{}", fixed_instruction));
        prompt
    };

    // 10. Check for multiple "// TODO: -" markers.
    let marker = "// TODO: -";
    let marker_lines: Vec<&str> = final_prompt
        .lines()
        .filter(|line| line.contains(marker))
        .collect();
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

    // Copy final prompt to clipboard if DISABLE_PBCOPY is not set.
    if env::var("DISABLE_PBCOPY").is_err() {
        use std::process::{Command, Stdio};
        let mut pbcopy = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| {
                eprintln!("Error running pbcopy: {}", err);
                std::process::exit(1);
            });
        {
            let pb_stdin = pbcopy.stdin.as_mut().expect("Failed to open pbcopy stdin");
            pb_stdin
                .write_all(unescape_newlines(&final_prompt).as_bytes())
                .expect("Failed to write to pbcopy");
        }
        pbcopy.wait().expect("Failed to wait on pbcopy");
    } else {
        eprintln!("DISABLE_PBCOPY is set; skipping clipboard copy.");
    }

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

/// Unescape literal "\n" sequences to actual newlines.
fn unescape_newlines(input: &str) -> String {
    input.replace("\\n", "\n")
}
