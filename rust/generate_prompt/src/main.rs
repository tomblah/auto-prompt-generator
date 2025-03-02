use anyhow::{bail, Context, Result};
use clap::{Arg, Command};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use unescape_newlines::unescape_newlines;

// Library dependencies.
use extract_instruction_content::extract_instruction_content;
use get_search_roots::get_search_roots;
use get_git_root::get_git_root;
use find_prompt_instruction::find_prompt_instruction_in_dir;
use diff_with_branch::run_diff;
use filter_excluded_files::filter_excluded_files_lines;
use extract_types::extract_types_from_file;
use filter_files_singular;
use extract_enclosing_type::extract_enclosing_type;
use find_referencing_files;
use extract_enclosing_function::extract_enclosing_block;

// Import the assemble_prompt library.
use assemble_prompt;
// NEW: Import the new find_definition_files library function.
use find_definition_files::find_definition_files;

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
    let _diff_branch_arg = matches.get_one::<String>("diff_with").map(String::as_str);
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

    // NEW: Extract enclosing function block.
    let enclosing_context = match fs::read_to_string(&file_path) {
        Ok(content) => match extract_enclosing_block(&content) {
            Some(block) => block,
            None => String::from("No enclosing function block found."),
        },
        Err(err) => {
            eprintln!("Error reading TODO file for enclosing block extraction: {}", err);
            String::new()
        }
    };
    println!("Enclosing function block:\n{}", enclosing_context);
    println!("--------------------------------------------------");

    // 6. Determine files to include.
    let found_files_path: PathBuf;
    if singular {
        println!("Singular mode enabled: only including the TODO file");
        found_files_path = filter_files_singular::create_todo_temp_file(&file_path)
            .map_err(|e| anyhow::anyhow!(e))
            .context("Failed to create singular temp file")?;
    } else {
        let types_file_path = extract_types_from_file(&file_path)
            .context("Failed to extract types")?;
        let types_content = fs::read_to_string(&types_file_path)
            .context("Failed to read extracted types")?;
        println!("Types found:");
        println!("{}", types_content.trim());
        println!("--------------------------------------------------");

        // Call the library function directly.
        let def_files_set = find_definition_files(
            Path::new(&types_file_path),
            &search_root,
        )
        .map_err(|err| anyhow::anyhow!("Failed to find definition files: {}", err))?;
        let def_files_content = def_files_set
            .iter()
            .map(|p| p.to_string_lossy())
            .collect::<Vec<_>>()
            .join("\n");

        found_files_path = {
            let mut temp = tempfile::NamedTempFile::new()
                .context("Failed to create temporary file for found files")?;
            // Ensure trailing newline using format!
            write!(temp, "{}\n", def_files_content)
                .context("Failed to write to temporary found files file")?;
            temp.into_temp_path()
                .keep()
                .context("Failed to persist temporary found files list")?
        };
        {
            use std::fs::OpenOptions;
            let mut f = OpenOptions::new()
                .append(true)
                .open(&found_files_path)
                .context(format!("Failed to open found files list at {}", found_files_path.display()))?;
            // Append the TODO file on its own line.
            writeln!(f, "{}", file_path).context("Failed to append TODO file")?;
        }
        // Apply initial exclusion filtering.
        if !excludes.is_empty() {
            println!("Excluding files matching: {:?}", excludes);
            let initial_found_files_content = fs::read_to_string(&found_files_path)
                .context("Failed to read found files list for initial filtering")?;
            let lines: Vec<String> = initial_found_files_content
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            let filtered_lines = filter_excluded_files_lines(lines, &excludes);
            fs::write(&found_files_path, format!("{}\n", filtered_lines.join("\n")))
                .context("Failed to write initial excluded list")?;
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
            {
                use std::fs::OpenOptions;
                let mut f = OpenOptions::new()
                    .append(true)
                    .open(&found_files_path)
                    .context("Failed to open found files list for appending referencing files")?;
                // Append referencing files with a trailing newline.
                writeln!(f, "{}", referencing_files.join("\n"))
                    .context("Failed to append referencing files")?;
            }
        } else {
            println!("No enclosing type found; skipping reference search.");
        }
        // Reapply exclusion filtering after referencing files have been appended.
        if !excludes.is_empty() {
            println!("Excluding files matching: {:?}", excludes);
            let final_found_files_content = fs::read_to_string(&found_files_path)
                .context("Failed to re-read found files list for final filtering")?;
            let lines: Vec<String> = final_found_files_content
                .lines()
                .map(|line| line.trim().to_string())
                .filter(|line| !line.is_empty())
                .collect();
            let final_filtered_lines = filter_excluded_files_lines(lines, &excludes);
            fs::write(&found_files_path, format!("{}\n", final_filtered_lines.join("\n")))
                .context("Failed to write final excluded list after appending referencing files")?;
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

    // 9. Assemble the final prompt by calling the library function.
    let final_prompt = assemble_prompt::assemble_prompt(
        found_files_path.to_str().unwrap(),
        instruction_content.trim(),
    )
    .context("Failed to assemble prompt")?;

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
