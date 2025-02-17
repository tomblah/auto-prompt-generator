use std::env;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, exit, Stdio};

/// Maximum allowed prompt length.
const MAX_PROMPT_LENGTH: usize = 100_000;

/// Unescape literal "\n" sequences to actual newlines.
fn unescape_newlines(input: &str) -> String {
    input.replace("\\n", "\n")
}

/// Runs an external command and returns its stdout as a String.
fn run_command(cmd: &str, args: &[&str]) -> io::Result<String> {
    let output = Command::new(cmd)
        .args(args)
        .output()?;
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Determines an external command path by first checking an environment variable override,
/// then looking for the command in the same directory as the current executable,
/// and finally falling back to the given default.
fn get_external_cmd(cmd_env: &str, default: &str) -> String {
    if let Ok(val) = env::var(cmd_env) {
        return val;
    }
    if let Ok(exe_path) = env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let candidate = dir.join(default);
            if candidate.exists() {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    default.to_string()
}

fn main() {
    // Expect exactly two arguments: <found_files_file> and <instruction_content>
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <found_files_file> <instruction_content>", args[0]);
        exit(1);
    }
    let found_files_file = &args[1];
    let _instruction_content = &args[2];

    // Determine external commands.
    let prompt_cmd = get_external_cmd("RUST_PROMPT_FILE_PROCESSOR", "prompt_file_processor");
    let filter_cmd = get_external_cmd("RUST_FILTER_SUBSTRING_MARKERS", "filter_substring_markers");

    // Read the found_files list.
    let file = File::open(found_files_file)
        .unwrap_or_else(|err| { eprintln!("Error opening {}: {}", found_files_file, err); exit(1); });
    let reader = BufReader::new(file);

    // Collect unique file paths.
    let mut files: Vec<String> = reader
        .lines()
        .filter_map(|line| line.ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    files.sort();
    files.dedup();

    let mut final_prompt = String::new();
    // Retrieve TODO file basename from environment.
    let todo_file_basename = env::var("TODO_FILE_BASENAME").unwrap_or_default();

    // Process each file.
    for file_path in files {
        // Skip non-existent files.
        if !Path::new(&file_path).exists() {
            eprintln!("Warning: file {} does not exist, skipping", file_path);
            continue;
        }
        let basename = Path::new(&file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_path)
            .to_string();

        // Attempt to process the file using prompt_file_processor.
        let processed_content = match run_command(&prompt_cmd, &[&file_path, &todo_file_basename]) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error processing {}: {}. Falling back to file contents.", file_path, err);
                let raw_content = fs::read_to_string(&file_path).unwrap_or_default();
                if raw_content.contains("// v") {
                    match run_command(&filter_cmd, &[&file_path]) {
                        Ok(filtered) => filtered,
                        Err(_) => raw_content,
                    }
                } else {
                    raw_content
                }
            }
        };

        final_prompt.push_str(&format!(
            "\nThe contents of {} is as follows:\n\n{}\n\n",
            basename, processed_content
        ));

        // If DIFF_WITH_BRANCH is set, append a diff report.
        if let Ok(diff_branch) = env::var("DIFF_WITH_BRANCH") {
            let diff_output = match run_command("diff_with_branch", &[&file_path]) {
                Ok(diff) => diff,
                Err(err) => {
                    eprintln!("Error running diff on {}: {}", file_path, err);
                    String::new()
                }
            };
            if !diff_output.trim().is_empty() && diff_output.trim() != basename {
                final_prompt.push_str(&format!(
                    "\n--------------------------------------------------\nThe diff for {} (against branch {}) is as follows:\n\n{}\n\n",
                    basename, diff_branch, diff_output
                ));
            }
        }

        final_prompt.push_str("\n--------------------------------------------------\n");
    }

    // Append the fixed instruction.
    let fixed_instruction = "Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen";
    final_prompt.push_str(&format!("\n\n{}", fixed_instruction));

    // Check the prompt size and print debug/warning messages.
    let prompt_length = final_prompt.chars().count();
    if prompt_length > MAX_PROMPT_LENGTH {
        println!(
            "Warning: The prompt is {} characters long. This may exceed what the AI can handle effectively.",
            prompt_length
        );
        // Attempt to get suggested exclusions.
        // We assume that the found files list is passed as the first argument.
        let found_files_arg = found_files_file; // found_files_file is the path argument passed to assemble_prompt.
        // Retrieve the TODO file basename from the environment, if available.
        let todo_file_basename = env::var("TODO_FILE_BASENAME").unwrap_or_default();
        // Call the suggest_exclusions binary.
        match run_command("suggest_exclusions", &[
            found_files_arg,
            &prompt_length.to_string(),
            &MAX_PROMPT_LENGTH.to_string(),
            &todo_file_basename,
        ]) {
            Ok(suggestions) if !suggestions.trim().is_empty() => {
                println!("Suggested exclusions:\n{}", suggestions.trim());
            }
            Ok(_) => {
                println!("No suggestions available.");
            }
            Err(e) => {
                eprintln!("Failed to get exclusion suggestions: {}", e);
            }
        }
    } else {
        println!(
            "Debug: The final prompt length is {} characters, which is within acceptable limits.",
            prompt_length
        );
    }

    // Unescape literal "\n" sequences.
    let final_clipboard_content = unescape_newlines(&final_prompt);

    // Copy the final prompt to the clipboard (unless DISABLE_PBCOPY is set).
    if env::var("DISABLE_PBCOPY").is_err() {
        let mut pbcopy = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| { eprintln!("Error running pbcopy: {}", err); exit(1); });
        {
            let pb_stdin = pbcopy.stdin.as_mut().expect("Failed to open pbcopy stdin");
            pb_stdin
                .write_all(final_clipboard_content.as_bytes())
                .expect("Failed to write to pbcopy");
        }
        pbcopy.wait().expect("Failed to wait on pbcopy");
    } else {
        println!("DISABLE_PBCOPY is set; skipping clipboard copy.");
    }
}
