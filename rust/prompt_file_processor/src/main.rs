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

/// Dummy function representing extra context extraction (e.g. the enclosing function).
/// Replace this with your actual extraction logic.
fn extract_enclosing_function(file_path: &str) -> Option<String> {
    Some(format!("Extra context extracted from {}", file_path))
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
            if let Some(context) = extract_enclosing_function(file_path) {
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
    fn test_extract_enclosing_function() {
        let file_path = "dummy.rs";
        let expected = "Extra context extracted from dummy.rs";
        let output = extract_enclosing_function(file_path).unwrap();
        assert_eq!(output, expected);
    }
}
