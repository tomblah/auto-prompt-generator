// rust/prompt_file_processor/src/lib.rs

use std::fs;
use std::path::Path;
use anyhow::{Result};
use extract_enclosing_function::extract_enclosing_block as extract_enclosing_function_block;
use unescape_newlines::unescape_newlines;

/// Filters the file’s content by returning only the text between substring markers.
/// The markers are defined as:
///   - Opening marker: a line that, when trimmed, equals "// v"
///   - Closing marker: a line that, when trimmed, equals "// ^"
/// Lines outside these markers are omitted (replaced by a placeholder).
pub fn filter_substring_markers(content: &str) -> String {
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

/// Checks whether the file uses both markers ("// v" and "// ^").
pub fn file_uses_markers(content: &str) -> bool {
    let has_open = content.lines().any(|line| line.trim() == "// v");
    let has_close = content.lines().any(|line| line.trim() == "// ^");
    has_open && has_close
}

/// Public API: processes the file at `file_path` by filtering its content based on markers
/// and, if applicable, appending an enclosing context block extracted via extract_enclosing_function.
/// The optional parameter `todo_file_basename` is used so that context is only appended if
/// the file's basename matches.
/// Returns the processed content as a `String`.
pub fn process_file<P: AsRef<Path>>(file_path: P, todo_file_basename: Option<&str>) -> Result<String> {
    let file_path_ref = file_path.as_ref();
    // Read the entire file content.
    let file_content = fs::read_to_string(file_path_ref)?;
    
    // If the file contains the "// v" marker, filter it; otherwise, use the raw content.
    let processed_content = if file_content.lines().any(|line| line.trim() == "// v") {
        filter_substring_markers(&file_content)
    } else {
        file_content.clone()
    };
    
    // Determine the file's basename.
    let file_basename = file_path_ref
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    
    let mut combined_content = processed_content;
    
    // If the file uses markers and its basename matches the expected TODO file,
    // attempt to extract and append the enclosing function context.
    if file_uses_markers(&file_content) {
        if let Some(expected_basename) = todo_file_basename {
            if file_basename == expected_basename {
                if let Some(context) = extract_enclosing_function_block(&file_content) {
                    combined_content.push_str("\n\n// Enclosing function context:\n");
                    combined_content.push_str(&context);
                }
            }
        }
    }
    
    // Unescape any literal "\n" sequences.
    let final_content = unescape_newlines(&combined_content);
    Ok(final_content)
}
