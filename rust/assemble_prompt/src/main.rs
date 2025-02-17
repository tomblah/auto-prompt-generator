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

/// Helper: determine external command path.
/// It first checks for an environment variable override (`cmd_env`),
/// then looks in the same directory as the current executable for a file named `default`,
/// and finally falls back to `default` as-is.
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
    // (The instruction_content is now ignored.)
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

    for file_path in files {
        // Check if the file exists. If not, log a warning and skip.
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
        // If that fails, fall back to reading the file content.
        // Additionally, if the file contains substring markers, try filtering them.
        let processed_content = match run_command(&prompt_cmd, &[&file_path, &todo_file_basename]) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error processing {}: {}. Falling back to file contents.", file_path, err);
                let raw_content = fs::read_to_string(&file_path).unwrap_or_default();
                if raw_content.contains("// v") {
                    // Try filtering using filter_substring_markers.
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
            // If the diff output (after whitespace removal) equals the basename, ignore it.
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

    // Check the prompt size.
    let prompt_length = final_prompt.chars().count();
    if prompt_length > MAX_PROMPT_LENGTH {
        eprintln!("Warning: The prompt is {} characters long. This may exceed what the AI can handle effectively.", prompt_length);
        // (Optional: call suggest_exclusions here.)
    }

    // Unescape literal "\n" sequences.
    let final_clipboard_content = unescape_newlines(&final_prompt);

    // If the environment variable DISABLE_PBCOPY is not set, copy to the clipboard.
    if env::var("DISABLE_PBCOPY").is_err() {
        let mut pbcopy = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| { eprintln!("Error running pbcopy: {}", err); exit(1); });
        {
            let pb_stdin = pbcopy.stdin.as_mut().expect("Failed to open pbcopy stdin");
            pb_stdin.write_all(final_clipboard_content.as_bytes())
                .expect("Failed to write to pbcopy");
        }
        pbcopy.wait().expect("Failed to wait on pbcopy");
    } else {
        // Optionally, log that we're skipping pbcopy.
        eprintln!("DISABLE_PBCOPY is set; skipping clipboard copy.");
    }

    // Also print the final prompt to stdout.
    println!("{}", final_clipboard_content);
}
