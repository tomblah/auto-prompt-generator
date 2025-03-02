use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use anyhow::{Result, Context};
use prompt_file_processor;
use unescape_newlines::unescape_newlines;

/// Public API: assembles the final prompt from the found files and instruction content.
/// Instead of printing to stdout or copying to clipboard, it returns the prompt as a String.
pub fn assemble_prompt(found_files_file: &str, _instruction_content: &str) -> Result<String> {
    // Read the found_files list.
    let file = File::open(found_files_file)
        .with_context(|| format!("Error opening {}", found_files_file))?;
    let reader = BufReader::new(file);
    let mut files: Vec<String> = reader
        .lines()
        .filter_map(|l| l.ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    files.sort();
    files.dedup();

    let mut final_prompt = String::new();
    // Retrieve TODO file basename from environment.
    let todo_file_basename = env::var("TODO_FILE_BASENAME").unwrap_or_default();

    for file_path in files {
        if !Path::new(&file_path).exists() {
            eprintln!("Warning: file {} does not exist, skipping", file_path);
            continue;
        }
        let basename = Path::new(&file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_path)
            .to_string();

        // Always use the library processing since we no longer have an external prompt processor.
        let processed_content = match prompt_file_processor::process_file(&file_path, Some(&todo_file_basename)) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error processing {}: {}. Falling back to raw file contents.", file_path, err);
                fs::read_to_string(&file_path).unwrap_or_default()
            }
        };

        final_prompt.push_str(&format!(
            "\nThe contents of {} is as follows:\n\n{}\n\n",
            basename, processed_content
        ));

        // If DIFF_WITH_BRANCH is set, append a diff report.
        if let Ok(diff_branch) = env::var("DIFF_WITH_BRANCH") {
            let diff_output = match diff_with_branch::run_diff(&file_path) {
                Ok(Some(diff)) => diff,
                _ => String::new(),
            };
            if !diff_output.trim().is_empty() {
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

    // Unescape literal \"\\n\" sequences.
    let final_prompt = unescape_newlines(&final_prompt);

    Ok(final_prompt)
}

/// Helper function to run an external command and capture its stdout as a String.
fn run_command(cmd: &str, args: &[&str]) -> Result<String, anyhow::Error> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .with_context(|| format!("Failed to execute command: {} {:?}", cmd, args))?;
    if !output.status.success() {
        anyhow::bail!("Command {} {:?} failed with status {}", cmd, args, output.status);
    }
    let stdout = String::from_utf8(output.stdout).context("Output not valid UTF-8")?;
    Ok(stdout)
}
