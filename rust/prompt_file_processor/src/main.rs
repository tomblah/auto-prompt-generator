#[macro_use]
extern crate lazy_static;

use regex::Regex;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

/// Filters the file’s content by returning only the text between substring markers.
/// The markers are defined as:
///   - Opening marker: a line that, when trimmed, equals "// v"
///   - Closing marker: a line that, when trimmed, equals "// ^"
/// Lines outside these markers are omitted (replaced by a placeholder).
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

/// Returns true if the given line appears to be a candidate declaration.
/// This function checks for various patterns in Swift and JavaScript.
fn is_candidate_line(line: &str) -> bool {
    lazy_static! {
        static ref SWIFT_FUNCTION: Regex = Regex::new(
            r#"^\s*(?:(?:public|private|internal|fileprivate)\s+)?func\s+\w+\s*\([^)]*\)\s*\{"#
        ).unwrap();
        static ref SWIFT_PROPERTY: Regex = Regex::new(
            r#"^\s*(?:(?:public|private|internal|fileprivate)\s+)?var\s+\w+(?:\s*:\s*[^={]+)?\s*\{"#
        ).unwrap();
        static ref PARSE_CLOUD: Regex = Regex::new(
            r#"^\s*Parse\.Cloud\.define\s*\(\s*".+?"\s*,\s*(?:async\s+)?\([^)]*\)\s*=>\s*\{"#
        ).unwrap();
        static ref JS_ASSIGNMENT: Regex = Regex::new(
            r#"^\s*(?:(?:const|var|let)\s+)?\w+\s*=\s*function\s*\([^)]*\)\s*\{"#
        ).unwrap();
        static ref JS_FUNCTION: Regex = Regex::new(
            r#"^\s*(?:async\s+)?function\s+\w+\s*\([^)]*\)\s*\{"#
        ).unwrap();
    }
    SWIFT_FUNCTION.is_match(line)
        || SWIFT_PROPERTY.is_match(line)
        || PARSE_CLOUD.is_match(line)
        || JS_ASSIGNMENT.is_match(line)
        || JS_FUNCTION.is_match(line)
}

/// Returns true if the file content contains both the opening marker ("// v")
/// and the closing marker ("// ^").
fn file_uses_markers(content: &str) -> bool {
    let has_open = content.lines().any(|line| line.trim() == "// v");
    let has_close = content.lines().any(|line| line.trim() == "// ^");
    has_open && has_close
}

/// Returns the index (zero-based) of the first line that contains "// TODO: - ", or None if not found.
pub fn todo_index(content: &str) -> Option<usize> {
    content.lines().position(|line| line.contains("// TODO: - "))
}

/// Determines whether the TODO is already inside a marker block.
/// It counts the markers from the start of the file up to the TODO line.
fn is_todo_inside_markers(content: &str, todo_idx: usize) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let mut marker_depth = 0;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "// v" {
            marker_depth += 1;
        } else if trimmed == "// ^" {
            if marker_depth > 0 {
                marker_depth -= 1;
            }
        }
        if i == todo_idx {
            break;
        }
    }
    marker_depth > 0
}

/// Extracts the enclosing block (function, computed property, etc.)
/// that contains the TODO marker. This is only done if:
///   1. The file uses markers (both "// v" and "// ^") and
///   2. The TODO is not already inside a marker block.
fn extract_enclosing_block(file_path: &str) -> Option<String> {
    let content = fs::read_to_string(file_path).ok()?;
    
    // Only proceed if the file actually uses both markers.
    if !file_uses_markers(&content) {
        return None;
    }
    
    // Find the index of the TODO marker.
    let todo_idx = content.lines().position(|line| line.contains("// TODO: - "))?;
    
    // If the TODO is already inside a marker block, do not extract additional context.
    if is_todo_inside_markers(&content, todo_idx) {
        return None;
    }
    
    // Look for the last candidate declaration before the TODO.
    let lines: Vec<&str> = content.lines().collect();
    let mut candidate_index = None;
    for (i, line) in lines.iter().enumerate().take(todo_idx) {
        if is_candidate_line(line) {
            candidate_index = Some(i);
        }
    }
    let start_index = candidate_index?;
    
    // Extract the block using a simple brace counting heuristic.
    let mut brace_count = 0;
    let mut found_open = false;
    let mut extracted_lines = Vec::new();
    for line in &lines[start_index..] {
        if !found_open {
            if line.contains('{') {
                found_open = true;
                brace_count += line.matches('{').count();
                brace_count = brace_count.saturating_sub(line.matches('}').count());
            }
            extracted_lines.push(*line);
        } else {
            extracted_lines.push(*line);
            brace_count += line.matches('{').count();
            brace_count = brace_count.saturating_sub(line.matches('}').count());
            if brace_count == 0 {
                break;
            }
        }
    }
    Some(extracted_lines.join("\n"))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path> [<todo_file_basename>]", args[0]);
        process::exit(1);
    }
    let file_path = &args[1];
    let todo_file_basename = if args.len() >= 3 { Some(&args[2]) } else { None };

    // Read the raw file content.
    let file_content = fs::read_to_string(file_path).unwrap_or_else(|err| {
        eprintln!("Error reading {}: {}", file_path, err);
        process::exit(1);
    });

    // Process the file content: if markers are present, use the filtered content.
    let processed_content = if file_content.lines().any(|line| line.trim() == "// v") {
        filter_substring_markers(&file_content)
    } else {
        file_content.clone()
    };

    let file_basename = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path);

    // Start with the processed content.
    let mut combined_content = processed_content.clone();

    // Only append the enclosing function context if the file uses markers.
    if file_uses_markers(&file_content) {
        // Optionally, check that the provided todo_file_basename matches the file's basename.
        if let Some(todo_basename) = todo_file_basename {
            if file_basename == *todo_basename {
                if let Some(context) = extract_enclosing_block(file_path) {
                    combined_content.push_str("\n\n// Enclosing function context:\n");
                    combined_content.push_str(&context);
                }
            }
        }
    }

    print!("{}", combined_content);
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test for filter_substring_markers.
    const INPUT_WITH_MARKERS: &str = r#"
Some initial content that should not appear.
// v
line 1 inside marker
line 2 inside marker
// ^
Some trailing content that should not appear.
"#;

    #[test]
    fn test_filter_substring_markers() {
        let expected = "\n// ...\nline 1 inside marker\nline 2 inside marker\n\n// ...\n";
        let result = filter_substring_markers(INPUT_WITH_MARKERS);
        assert_eq!(result, expected);
    }

    #[test]
    fn test_file_uses_markers_true() {
        let input = r#"
code line 1
// v
inside marker
// ^
code line 2
"#;
        assert!(file_uses_markers(input));
    }

    #[test]
    fn test_file_uses_markers_false() {
        let input = r#"
code line 1
// v
inside marker
code line 2
"#;
        assert!(!file_uses_markers(input));
    }

    // Tests for is_todo_inside_markers.
    const CONTENT_TODO_OUTSIDE: &str = r#"
func example() {
    // code
}

// v
// some extra context
// ^
 // TODO: - do something
"#;
    #[test]
    fn test_is_todo_inside_markers_false() {
        let todo_idx = todo_index(CONTENT_TODO_OUTSIDE).unwrap();
        assert!(!is_todo_inside_markers(CONTENT_TODO_OUTSIDE, todo_idx));
    }

    const CONTENT_TODO_INSIDE: &str = r#"
func example() {
    // v
    // TODO: - do something
    // ^
}
"#;
    #[test]
    fn test_is_todo_inside_markers_true() {
        let todo_idx = todo_index(CONTENT_TODO_INSIDE).unwrap();
        assert!(is_todo_inside_markers(CONTENT_TODO_INSIDE, todo_idx));
    }

    // For extract_enclosing_block, we can simulate the file content via temporary files,
    // but here we test the logic indirectly by constructing content strings.
    // Since extract_enclosing_block takes a file path, we create a temporary file.
    use std::io::Write;
    use tempfile::NamedTempFile;

    // This content has markers and the TODO is outside the marker block.
    const MARKER_CONTENT_OUTSIDE: &str = r#"
func myFunction() {
    print("Hello")
}

// v
// Some extra context not part of the function block.
// ^
 // TODO: - perform important task
"#;

    #[test]
    fn test_extract_enclosing_block_outside() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "{}", MARKER_CONTENT_OUTSIDE).expect("Failed to write to temp file");
        let path = temp_file.path().to_str().unwrap();
        let block = extract_enclosing_block(path);
        assert!(block.is_some(), "Expected an enclosing block, got None");
        let block_str = block.unwrap();
        assert!(block_str.contains("func myFunction()"), "Block should contain the function declaration");
    }

    // This content has markers and the TODO is inside the marker block.
    const MARKER_CONTENT_INSIDE: &str = r#"
func myFunction() {
    print("Hello")
    // v
    // TODO: - perform important task
    // ^
}
"#;

    #[test]
    fn test_extract_enclosing_block_inside() {
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "{}", MARKER_CONTENT_INSIDE).expect("Failed to write to temp file");
        let path = temp_file.path().to_str().unwrap();
        let block = extract_enclosing_block(path);
        assert!(block.is_none(), "Expected no enclosing block because the TODO is inside markers");
    }
}
