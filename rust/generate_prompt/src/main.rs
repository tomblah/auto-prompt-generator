use anyhow::{Context, Result};
use clap::{Arg, Command};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use unescape_newlines::unescape_newlines;
use std::process::{Command as ProcessCommand, Stdio};

// Library dependencies.
use extract_instruction_content::extract_instruction_content;
use get_search_roots::get_search_roots;
use get_git_root::get_git_root;
use find_prompt_instruction::find_prompt_instruction_in_dir;
use filter_excluded_files::filter_excluded_files_lines;
use extract_types::extract_types_from_file;
use filter_files_singular;
use extract_enclosing_type::extract_enclosing_type;
use find_referencing_files;

// Import the assemble_prompt library.
use assemble_prompt;
// NEW: Import the new find_definition_files library function.
use find_definition_files::find_definition_files;

// NEW: Import the post_processing crate.
use post_processing;

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

    // Copy final prompt to clipboard if DISABLE_PBCOPY is not set.
    if env::var("DISABLE_PBCOPY").is_err() {
        let mut pbcopy = ProcessCommand::new("pbcopy")
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

#[cfg(test)]
mod tests {
    use predicates::prelude::*;
    use std::env;
    use std::fs;
    use tempfile::TempDir;
    use assert_cmd::prelude::*;
    use std::process::Command;

    /// On Unix systems, creates a dummy executable (a shell script) in the given temporary directory.
    /// The script simply echoes the provided output (or executes a simple shell command).
    #[cfg(unix)]
    fn create_dummy_executable(dir: &TempDir, name: &str, output: &str) -> std::path::PathBuf {
        let path = dir.path().join(name);
        fs::write(&path, format!("#!/bin/sh\n{}", output)).unwrap();
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&path).unwrap().permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&path, perms).unwrap();
        path
    }

    /// Helper: clear GET_GIT_ROOT so tests that don't need it won't use a stale value.
    fn clear_git_root() {
        env::remove_var("GET_GIT_ROOT");
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode() {
        clear_git_root();
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set up dummy commands.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create a TODO file with the expected content.
        fs::write(&todo_file, "   // TODO: - Fix critical bug").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix critical bug");
        // In singular mode, we expect the file list to contain only the TODO file.
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        // Dummy assemble_prompt (not used in singular mode output) is set up for consistency.
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Singular mode enabled: only including the TODO file"))
            .stdout(predicate::str::contains("// TODO: - Fix critical bug"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
        
        env::remove_var("GET_GIT_ROOT");
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_error_for_non_swift() {
        clear_git_root();
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        // Use a TODO file with .js extension.
        let todo_file = format!("{}/TODO.js", fake_git_root_path);
        fs::write(&todo_file, "   // TODO: - Fix issue").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix issue");
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "DummyType").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("--include-references is only supported for Swift files"));

        clear_git_root();
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode() {
        // Use a fake Git root by setting GET_GIT_ROOT.
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Write a TODO file that includes a type declaration so that extract_types_from_file extracts "TypeFixBug".
        fs::write(&todo_file, "class TypeFixBug {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Create a dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeFixBug").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

        // Create definition files in the fake Git root.
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeFixBug {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TypeFixBug {}").unwrap();

        // Dummy assemble_prompt is not used in normal mode because the final prompt is not printed (clipboard copy occurs).
        // Instead, we check that the output contains key status messages.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Instruction content: // TODO: - Fix bug"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
        
        env::remove_var("GET_GIT_ROOT");
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_for_swift() {
        clear_git_root();
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Include a type declaration so that the extractor finds "MyType".
        fs::write(&todo_file, "class MyType {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        create_dummy_executable(&temp_dir, "find_referencing_files", &format!("{}/Ref1.swift", fake_git_root_path));
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Including files that reference the enclosing type"))
            .stdout(predicate::str::contains("Enclosing type: MyType"))
            .stdout(predicate::str::contains("Searching for files referencing MyType"))
            .stdout(predicate::str::contains("Warning: The --include-references option is experimental."))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
        
        clear_git_root();
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_force_global() {
        // This test requires its own fake Git root.
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set GET_GIT_ROOT to our fake_git_root.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Include a type declaration for extraction.
        fs::write(&todo_file, "class TypeForce {}\n   // TODO: - Force global test").unwrap();
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "NonEmptyValue");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Force global test");
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeForce").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_files_output = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
        create_dummy_executable(&temp_dir, "filter_excluded_files", &def_files_output);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Also force GET_INSTRUCTION_FILE so that extraction works.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--force-global");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Force global enabled: using Git root for context"));

        env::remove_var("GET_GIT_ROOT");
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_exclude() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TypeExclude {}\n   // TODO: - Exclude test").unwrap();
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Exclude test");
        let types_file = temp_dir.path().join("types.txt");
        fs::write(&types_file, "TypeExclude").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file.to_str().unwrap());
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeExclude {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TypeExclude {}").unwrap();
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.args(&["--exclude", "ExcludePattern", "--exclude", "AnotherPattern"]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Excluding files matching:"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"));

        env::remove_var("GET_GIT_ROOT");
    }
        
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_multiple_markers() {
        use std::env;
        use std::fs;
        use tempfile::TempDir;
        use assert_cmd::prelude::*;
        use std::process::Command;

        // Create a temporary directory for dummy executables.
        let temp_dir = TempDir::new().unwrap();
        // Set up a dummy pbcopy that writes to a clipboard file.
        let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
        let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
        create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

        // Create a fake Git root.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set up dummy get_git_root.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);

        // Create an instruction file with three marker lines.
        // Ensure the CTA marker is on its own line and that there's a trailing newline.
        let instruction_path = format!("{}/Instruction.swift", fake_git_root_path);
        let multi_marker_content = "\
    // TODO: - Marker One\n\
    Some content here\n\
    // TODO: - Marker Two\n\
    More content here\n\
    // TODO: - CTA Marker\n";
        fs::write(&instruction_path, multi_marker_content).unwrap();
        env::set_var("GET_INSTRUCTION_FILE", &instruction_path);

        // Dummy find_prompt_instruction returns the instruction file.
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &instruction_path);
        // Dummy extract_instruction_content returns the primary marker (trimmed).
        create_dummy_executable(&temp_dir, "extract_instruction_content", "// TODO: - Marker One");
        // Dummy get_package_root.
        create_dummy_executable(&temp_dir, "get_package_root", "");
        // Dummy assemble_prompt returns the multi-marker content.
        create_dummy_executable(&temp_dir, "assemble_prompt", multi_marker_content);

        // Prepend our dummy executables directory to PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        // Unset DISABLE_PBCOPY so that clipboard copy occurs.
        env::remove_var("DISABLE_PBCOPY");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        // Read the final prompt from our dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Expect that the final prompt contains the primary marker and the CTA marker,
        // and that it does not contain the extra marker.
        assert!(clipboard_content.contains("// TODO: - Marker One"),
                "Clipboard missing primary marker: {}", clipboard_content);
        assert!(clipboard_content.contains("Can you do the TODO:- in the above code?"),
                "Clipboard missing CTA marker: {}", clipboard_content);
        assert!(!clipboard_content.contains("// TODO: - Marker Two"),
                "Clipboard should not contain extra marker: {}", clipboard_content);

        env::remove_var("GET_GIT_ROOT");
    }

    /// New Test: Test that when --diff-with main is passed the diff section is included.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_diff_with_main() {
        use std::fs;
        // Create a temporary directory for dummy executables.
        let temp_dir = TempDir::new().unwrap();
        // Create a dummy git executable that simulates:
        // - A successful branch verification for "main".
        // - A successful ls-files check.
        // - Returning a dummy diff output.
        let git_script = r#"#!/bin/sh
case "$@" in
    *rev-parse*--verify*main*)
        exit 0
        ;;
    *ls-files*)
        exit 0
        ;;
    *diff*)
        echo "dummy diff output"
        exit 0
        ;;
    *)
        exit 1
        ;;
esac
"#;
        create_dummy_executable(&temp_dir, "git", git_script);

        // Create a dummy pbcopy that writes to a file (simulate clipboard copy).
        let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
        let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
        create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

        // Set up a fake Git root.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Create a dummy TODO file in the fake Git root.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TestDiff {}\n   // TODO: - Diff test").unwrap();
        // Explicitly set GET_INSTRUCTION_FILE so that the instruction content is extracted from our dummy file.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Diff test");

        // Create a dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TestDiff").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

        // Create a dummy definition file in the fake Git root.
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        fs::write(&def_file, "class TestDiff {}").unwrap();
        // Create dummy find_definition_files that echoes the definition file path.
        let find_def_script = format!("echo \"{}\"", def_file);
        create_dummy_executable(&temp_dir, "find_definition_files", &find_def_script);

        // Create dummy filter_excluded_files (which can simply echo its input).
        create_dummy_executable(&temp_dir, "filter_excluded_files", "");

        // Ensure clipboard copy is enabled by unsetting DISABLE_PBCOPY.
        env::remove_var("DISABLE_PBCOPY");

        // Prepend our temp_dir (which contains our dummy git and pbcopy) to PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt with --diff-with "main".
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.args(&["--diff-with", "main"]);
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the clipboard content includes the diff section.
        assert!(clipboard_content.contains("The diff for"),
                "Clipboard content missing diff header: {}", clipboard_content);
        assert!(clipboard_content.contains("against branch main"),
                "Clipboard content missing branch info: {}", clipboard_content);
        assert!(clipboard_content.contains("dummy diff output"),
                "Clipboard content missing dummy diff output: {}", clipboard_content);

        env::remove_var("GET_GIT_ROOT");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_diff_with_nonexistent_branch() {
        let temp_dir = TempDir::new().unwrap();
        // Create a dummy git executable that fails when rev-parse --verify is called with "nonexistent".
        let git_script = r#"#!/bin/sh
case "$@" in
    *rev-parse*--verify*nonexistent*)
        echo "fatal: ambiguous argument 'nonexistent': unknown revision or path not in the working tree." >&2
        exit 1
        ;;
    *)
        exit 0
        ;;
esac
"#;
        create_dummy_executable(&temp_dir, "git", git_script);

        // Create a dummy pbcopy that writes to a file (simulate clipboard copy).
        let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
        let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
        create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

        // Set up a fake Git root.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Create a dummy TODO file in the fake Git root.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TestDiff {}\n   // TODO: - Diff test").unwrap();
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Diff test");
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TestDiff").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        fs::write(&def_file, "class TestDiff {}").unwrap();
        let find_def_script = format!("echo \"{}\"", def_file);
        create_dummy_executable(&temp_dir, "find_definition_files", &find_def_script);
        create_dummy_executable(&temp_dir, "filter_excluded_files", "");
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::remove_var("DISABLE_PBCOPY");

        // Run generate_prompt with --diff-with "nonexistent".
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.args(&["--diff-with", "nonexistent"]);

        cmd.assert()
           .failure()
           .stderr(predicate::str::contains("Error: Branch 'nonexistent' does not exist."));

        env::remove_var("GET_GIT_ROOT");
    }
        
    /// Test that final prompt is copied to clipboard.
    #[test]
    #[cfg(unix)]
    fn test_final_prompt_copied_to_clipboard() {
        let temp_dir = TempDir::new().unwrap();
        // Path to capture clipboard output.
        let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
        // Create dummy pbcopy: it reads from stdin and writes to clipboard_file.
        let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
        create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

        // Set up a fake Git root and environment.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Create a dummy TODO file.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TypeFixBug {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        // Create dummy types file and definition files.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeFixBug").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeFixBug {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TypeFixBug {}").unwrap();

        // Dummy assemble_prompt returns a simulated final prompt that includes the fixed instruction.
        let simulated_prompt = "\
    The contents of Definition1.swift is as follows:

    class TypeFixBug {}

    --------------------------------------------------
    The contents of Definition2.swift is as follows:

    class TypeFixBug {}

    --------------------------------------------------

    Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...";
        create_dummy_executable(&temp_dir, "assemble_prompt", simulated_prompt);

        // Force GET_INSTRUCTION_FILE to point to our TODO file.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Prepend our temp_dir (which contains our dummy pbcopy and other commands) to PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        // Unset DISABLE_PBCOPY so that clipboard copy occurs.
        env::remove_var("DISABLE_PBCOPY");

        // Run generate_prompt.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        // Read the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the clipboard content contains the expected fixed instruction.
        assert!(clipboard_content.contains("Can you do the TODO:- in the above code? But ignoring all FIXMEs"),
                "Clipboard content did not contain the expected fixed instruction: {}", clipboard_content);

        env::remove_var("GET_GIT_ROOT");
    }
            
    #[test]
    #[cfg(unix)]
    fn test_final_prompt_formatting_with_multiple_files() {
        use std::env;
        use std::fs;
        use std::process::Command;
        use tempfile::TempDir;

        // Create a temporary directory to host our dummy executables.
        let temp_dir = TempDir::new().unwrap();

        // Create a dummy pbcopy that writes its stdin to a file (simulate clipboard).
        let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
        let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
        create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

        // Set up a fake Git root.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Create a TODO file with known content.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TestClass {}\n   // TODO: - Refactor code").unwrap();

        // Set up dummy executables needed by generate_prompt.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Refactor code");

        // Create a dummy types file so that extract_types_from_file produces "TestClass".
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TestClass").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

        // Create two definition files in the fake Git root.
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TestClass {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TestClass {}").unwrap();

        // Create a dummy find_definition_files that echoes both definition file paths.
        let find_def_script = format!("echo \"{}\\n{}\"", def_file1.display(), def_file2.display());
        create_dummy_executable(&temp_dir, "find_definition_files", &find_def_script);

        // Create a dummy filter_excluded_files (can simply echo input).
        create_dummy_executable(&temp_dir, "filter_excluded_files", "");

        // Simulate an assemble_prompt command that returns a predictable final prompt.
        let simulated_prompt = format!(
            "The contents of {} is as follows:\n\n{}\n\n--------------------------------------------------\nThe contents of {} is as follows:\n\n{}\n\n--------------------------------------------------\n\nCan you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...",
            def_file1.file_name().unwrap().to_string_lossy(),
            fs::read_to_string(&def_file1).unwrap(),
            def_file2.file_name().unwrap().to_string_lossy(),
            fs::read_to_string(&def_file2).unwrap()
        );
        create_dummy_executable(&temp_dir, "assemble_prompt", &simulated_prompt);

        // Force GET_INSTRUCTION_FILE to point to our TODO file.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Prepend our dummy executables directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        // Ensure clipboard copy is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        // Read the simulated clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt contains headers for both definition files.
        assert!(clipboard_content.contains("The contents of Definition1.swift is as follows:"),
                "Missing header for Definition1.swift: {}", clipboard_content);
        assert!(clipboard_content.contains("The contents of Definition2.swift is as follows:"),
                "Missing header for Definition2.swift: {}", clipboard_content);
        // Assert that the fixed instruction is appended.
        assert!(clipboard_content.contains("Can you do the TODO:- in the above code?"),
                "Missing fixed instruction: {}", clipboard_content);

        env::remove_var("GET_GIT_ROOT");
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_scrubs_extra_todo_markers() {
        use std::env;
        use std::fs;
        use tempfile::TempDir;
        use assert_cmd::prelude::*;
        use std::process::Command;

        // Create a temporary directory for dummy executables.
        let temp_dir = TempDir::new().unwrap();
        // Set up a dummy pbcopy that writes to a clipboard file.
        let clipboard_file = temp_dir.path().join("dummy_clipboard.txt");
        let pbcopy_script = format!("cat > \"{}\"", clipboard_file.display());
        create_dummy_executable(&temp_dir, "pbcopy", &pbcopy_script);

        // Create a fake Git root.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set up dummy get_git_root.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);

        // Create a TODO file that will serve as the instruction file.
        // The simulated prompt includes three marker lines:
        // a primary marker, an extra marker, and a CTA marker.
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        let simulated_prompt = "\
    The contents of Definition.swift is as follows:\n\nclass DummyType {}\n\n--------------------------------------------------\n// TODO: - Primary Marker\nSome extra content here\n// TODO: - Extra Marker\nMore extra content here\n// TODO: - CTA Marker\n";
        fs::write(&todo_file, simulated_prompt).unwrap();
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Dummy find_prompt_instruction returns the TODO file.
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        // Dummy extract_instruction_content returns the primary marker.
        create_dummy_executable(&temp_dir, "extract_instruction_content", "// TODO: - Primary Marker");
        // Dummy get_package_root.
        create_dummy_executable(&temp_dir, "get_package_root", "");
        // Dummy assemble_prompt returns the simulated prompt.
        create_dummy_executable(&temp_dir, "assemble_prompt", simulated_prompt);

        // Prepend our dummy executables directory to PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        // Unset DISABLE_PBCOPY so that clipboard copy occurs.
        env::remove_var("DISABLE_PBCOPY");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        // Read the final prompt from the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the final prompt contains the primary marker and the CTA marker,
        // and that it does not include the extra marker.
        assert!(clipboard_content.contains("// TODO: - Primary Marker"),
                "Clipboard missing primary marker: {}", clipboard_content);
        assert!(clipboard_content.contains("Can you do the TODO:- in the above code?"),
                "Clipboard missing CTA marker: {}", clipboard_content);
        assert!(!clipboard_content.contains("// TODO: - Extra Marker"),
                "Clipboard should not contain extra marker: {}", clipboard_content);

        env::remove_var("GET_GIT_ROOT");
    }
}
