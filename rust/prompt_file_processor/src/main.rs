use regex::Regex;
use std::env;
use std::fs;
use std::io::{BufRead};
use std::path::Path;
use std::process;

/// Filters the content by returning only the text between substring markers.
/// The markers are an opening marker (“// v”) and a closing marker (“// ^”).
///
/// For every marker line encountered, a placeholder ("\n// ...\n") is inserted.
/// This means that if multiple blocks are present, there will be consecutive placeholders
/// (one for the closing marker of one block and one for the opening marker of the next block).
fn filter_substring_markers(content: &str) -> String {
    let mut output = String::new();
    let mut in_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "// v" {
            output.push_str("\n// ...\n");
            in_block = true;
            continue;
        }
        if trimmed == "// ^" {
            in_block = false;
            output.push_str("\n// ...\n");
            continue;
        }
        if in_block {
            output.push_str(line);
            output.push('\n');
        }
    }
    output
}

/// Extracts the enclosing block (Swift function, computed property, or JavaScript function)
/// that contains the TODO marker.
///
/// This implementation:
/// 1. Reads the file and splits it into lines.
/// 2. Locates the TODO marker (the first line containing "// TODO: -") and records its line index.
/// 3. Searches all lines up to that marker for candidate declarations matching one of:
///    - A Swift function declaration or computed property header.
///    - A JavaScript cloud code function (Parse.Cloud.define(...)).
///    - A JavaScript assignment function declaration (with an optional const/var/let).
/// 4. For each candidate, it uses a simple brace-counting algorithm (starting from the candidate’s line)
///    to determine the block boundaries.
/// 5. If the TODO marker falls within that block, the candidate is considered valid.
/// 6. Among all valid candidates, the one with the declaration closest to the TODO marker is returned.
pub fn extract_enclosing_block(file_path: &str) -> Option<String> {
    // Read the file content and split into lines.
    let content = fs::read_to_string(file_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();

    // Find the first line that contains the TODO marker.
    let todo_index = lines.iter().position(|line| line.contains("// TODO: -"))?;

    // Define a regex pattern that matches one of:
    // 1. Swift function or computed property declaration.
    // 2. A Parse.Cloud.define JavaScript function.
    // 3. A JavaScript assignment function declaration (with optional const/var/let).
    let decl_pattern = Regex::new(
        r#"^\s*(?:(?:(?:public|private|internal|fileprivate)\s+)?(?:(?:func\s+\w+\s*\()|(?:var\s+\w+(?:\s*:\s*[^={]+)?\s*\{))|(Parse\.Cloud\.define\s*\(\s*".+?"\s*,\s*(?:async\s*)?\(.*\)\s*=>\s*\{)|(?:(?:(?:const|var|let)\s+)?\w+\s*=\s*function\s*\(.*\)\s*\{))"#
    ).ok()?;

    // This will hold the candidate declaration that encloses the TODO marker.
    // We'll store a tuple: (start_line_index, end_line_index, block_lines)
    let mut best_candidate: Option<(usize, usize, Vec<&str>)> = None;

    // Iterate over lines from the start up to (and including) the TODO marker.
    for (i, line) in lines[..=todo_index].iter().enumerate() {
        if decl_pattern.is_match(line) {
            // Starting at candidate line i, extract the block using brace counting.
            let mut brace_count = 0;
            let mut started = false;
            let mut block_lines = Vec::new();
            let mut end_index = i;
            for (j, &current_line) in lines[i..].iter().enumerate() {
                if !started && current_line.contains("{") {
                    started = true;
                }
                if started {
                    brace_count += current_line.matches("{").count();
                    brace_count = brace_count.saturating_sub(current_line.matches("}").count());
                }
                block_lines.push(current_line);
                if started && brace_count == 0 {
                    end_index = i + j;
                    break;
                }
            }
            // Check if the TODO marker (at todo_index) falls within this candidate's block.
            if i <= todo_index && todo_index <= end_index {
                // If multiple candidates qualify, choose the one that starts later (closer to the TODO).
                match best_candidate {
                    Some((prev_start, _, _)) => {
                        if i > prev_start {
                            best_candidate = Some((i, end_index, block_lines));
                        }
                    }
                    None => best_candidate = Some((i, end_index, block_lines)),
                }
            }
        }
    }

    best_candidate.map(|(_, _, block_lines)| block_lines.join("\n"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Expected usage: prompt_file_processor <file_path> [<todo_file_basename>]
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path> [<todo_file_basename>]", args[0]);
        process::exit(1);
    }
    let file_path = &args[1];
    let todo_file_basename = if args.len() >= 3 {
        Some(&args[2])
    } else {
        None
    };

    // Read the file's content.
    let file_content = fs::read_to_string(file_path)?;

    // If any line equals "// v" after trimming, filter the content.
    let processed_content = if file_content.lines().any(|line| line.trim() == "// v") {
        filter_substring_markers(&file_content)
    } else {
        file_content
    };

    // Get the file's basename.
    let file_basename = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path);

    // Start with the processed content.
    let mut combined_content = processed_content;

    // If this file matches the provided TODO file basename, append extra context.
    if let Some(todo_basename) = todo_file_basename {
        if file_basename == todo_basename {
            if let Some(context) = extract_enclosing_block(file_path) {
                combined_content.push_str("\n\n// Enclosing function context:\n");
                combined_content.push_str(&context);
            }
        }
    }

    // Output the final processed content.
    print!("{}", combined_content);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_substring_markers_basic() {
        let input = "\
Some intro text
// v
Hello World
// ^
Some outro text";
        let expected = "\n// ...\nHello World\n\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_filter_substring_markers_multiple_blocks() {
        let input = "\
Intro
// v
First Block
// ^
Middle text
// v
Second Block
// ^
End";
        let expected = "\n// ...\nFirst Block\n\n// ...\n\n// ...\nSecond Block\n\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_filter_substring_markers_no_marker() {
        let input = "No markers here.";
        let expected = String::new();
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_extract_enclosing_block() {
        let file_path = "dummy.rs";
        let expected = "Extra context extracted from dummy.rs";
        let output = extract_enclosing_block(file_path).unwrap();
        assert_eq!(output, expected);
    }
}
