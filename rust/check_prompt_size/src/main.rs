use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::Path;

const MAX_LENGTH: usize = 100_000;

/// Checks the length of the prompt and returns a warning message if it exceeds MAX_LENGTH.
pub fn check_prompt_length(input: &str) -> Option<String> {
    let prompt_length = input.chars().count();
    if prompt_length > MAX_LENGTH {
        Some(format!(
            "Warning: The prompt is {} characters long. This may exceed what the AI can handle effectively.",
            prompt_length
        ))
    } else {
        None
    }
}

/// Filters the content of a file according to substring markers.
/// If the file contains a line that exactly matches "// v" (after trimming), then only
/// the text between that marker and the corresponding closing marker ("// ^") is retained,
/// with omitted regions replaced by a placeholder.
fn filter_substring_markers(content: &str) -> String {
    // First, check if an opening marker exists.
    if !content.lines().any(|line| line.trim() == "// v") {
        return content.to_string();
    }
    
    let mut output = String::new();
    let mut in_block = false;
    let mut last_was_placeholder = false;
    
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "// v" {
            if !last_was_placeholder {
                output.push_str("\n// ...\n");
                last_was_placeholder = true;
            }
            in_block = true;
            continue;
        }
        if trimmed == "// ^" {
            in_block = false;
            if !last_was_placeholder {
                output.push_str("\n// ...\n");
                last_was_placeholder = true;
            }
            continue;
        }
        if in_block {
            output.push_str(line);
            output.push('\n');
            last_was_placeholder = false;
        }
    }
    
    output
}

/// Computes the effective size (in characters) of a file's content after applying substring marker filtering.
fn compute_effective_size(file_path: &Path) -> io::Result<usize> {
    let content = fs::read_to_string(file_path)?;
    let filtered = filter_substring_markers(&content);
    Ok(filtered.chars().count())
}

fn main() {
    // Parse optional argument: --file-list <path>
    let args: Vec<String> = env::args().collect();
    let mut file_list_path: Option<String> = None;
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--file-list" && i + 1 < args.len() {
            file_list_path = Some(args[i + 1].clone());
            i += 2;
        } else {
            i += 1;
        }
    }

    // Read the assembled prompt from STDIN.
    let mut prompt = String::new();
    if let Err(e) = io::stdin().read_to_string(&mut prompt) {
        eprintln!("Error reading input: {}", e);
        std::process::exit(1);
    }

    let prompt_length = prompt.chars().count();

    // Check prompt length and print a warning if it exceeds the threshold.
    if let Some(warning) = check_prompt_length(&prompt) {
        eprintln!("{}", warning);

        // If a file list is provided, compute and print exclusion suggestions.
        if let Some(list_path) = file_list_path {
            let file_list_content = fs::read_to_string(&list_path).unwrap_or_default();
            // Retrieve the TODO file's basename from the environment (if set).
            let todo_file_basename = env::var("TODO_FILE_BASENAME").unwrap_or_default();
            for line in file_list_content.lines() {
                let file_path = line.trim();
                if file_path.is_empty() {
                    continue;
                }
                let path = Path::new(file_path);
                let basename = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if basename == todo_file_basename {
                    // Skip the TODO file.
                    continue;
                }
                match compute_effective_size(path) {
                    Ok(file_size) => {
                        let projected = prompt_length.saturating_sub(file_size);
                        let percentage = ((projected as f64 / MAX_LENGTH as f64) * 100.0).floor() as usize;
                        eprintln!(" --exclude {} (will get you to {}% of threshold)", basename, percentage);
                    }
                    Err(err) => {
                        eprintln!("Error reading {}: {}", file_path, err);
                    }
                }
            }
        }
    }
}
