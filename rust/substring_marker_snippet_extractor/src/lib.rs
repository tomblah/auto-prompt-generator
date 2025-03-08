// rust/substring_marker_snippet_extractor/src/lib.rs

use std::fs;
use std::path::Path;
use anyhow::{Result, anyhow};

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

/// Checks if the file uses both markers ("// v" and "// ^").
pub fn file_uses_markers(content: &str) -> bool {
    let has_open = content.lines().any(|line| line.trim() == "// v");
    let has_close = content.lines().any(|line| line.trim() == "// ^");
    has_open && has_close
}

/// Returns the index (zero-based) of the first line that contains "// TODO: - ".
pub fn todo_index(content: &str) -> Option<usize> {
    content.lines().position(|line| line.contains("// TODO: - "))
}

/// Determines whether the TODO is already inside a marker block by counting marker boundaries
/// from the start of the file up to the TODO line.
pub fn is_todo_inside_markers(content: &str, todo_idx: usize) -> bool {
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

//
// Helper function to determine if a given line is a candidate declaration line.
// We use regex patterns for Swift functions, JS functions/assignments, or Parse.Cloud.define.
//
fn is_candidate_line(line: &str) -> bool {
    // We inline the regexes so we don’t need to keep lazy_static imports if they’re unused elsewhere.
    let swift_function = regex::Regex::new(
        r#"^\s*(?:(?:public|private|internal|fileprivate)\s+)?func\s+\w+\s*\([^)]*\)\s*\{"#
    ).unwrap();
    let js_assignment = regex::Regex::new(
        r#"^\s*(?:(?:const|var|let)\s+)?\w+\s*=\s*function\s*\([^)]*\)\s*\{"#
    ).unwrap();
    let js_function = regex::Regex::new(
        r#"^\s*(?:async\s+)?function\s+\w+\s*\([^)]*\)\s*\{"#
    ).unwrap();
    let parse_cloud = regex::Regex::new(
        r#"^\s*Parse\.Cloud\.define\s*\(\s*".+?"\s*,\s*(?:async\s+)?\([^)]*\)\s*=>\s*\{"#
    ).unwrap();

    swift_function.is_match(line)
        || js_assignment.is_match(line)
        || js_function.is_match(line)
        || parse_cloud.is_match(line)
}

/// Extracts the enclosing block (such as a function) that should contain the TODO marker.
/// It does so by scanning upward from the TODO marker for the last candidate declaration,
/// then using a simple brace counting heuristic to extract from that line until the block closes.
fn extract_enclosing_block(file_path: &str) -> Option<String> {
    let content = fs::read_to_string(file_path).ok()?;
    // Only proceed if the file uses both markers.
    if !file_uses_markers(&content) {
        return None;
    }
    let todo_idx = content.lines().position(|line| line.contains("// TODO: - "))?;
    // If the TODO is already inside a marker block, skip extraction.
    if is_todo_inside_markers(&content, todo_idx) {
        return None;
    }
    let lines: Vec<&str> = content.lines().collect();
    let mut candidate_index = None;
    for (i, line) in lines.iter().enumerate().take(todo_idx) {
        if is_candidate_line(line) {
            candidate_index = Some(i);
        }
    }
    let start_index = candidate_index?;
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

/// Public API: processes the file at `file_path` by filtering its content based on markers
/// and, if applicable, appending an enclosing context block extracted via candidate heuristics.
/// The optional parameter `todo_file_basename` is used so that context is only appended if
/// the file's basename matches.
/// Returns the processed content as a `String`.
pub fn process_file<P: AsRef<Path>>(file_path: P, todo_file_basename: Option<&str>) -> Result<String> {
    let file_path_ref = file_path.as_ref();
    let file_path_str = file_path_ref.to_str().ok_or_else(|| anyhow!("Invalid file path"))?;
    let file_content = fs::read_to_string(file_path_ref)?;
    
    // If the file contains the marker, use the filtered content; otherwise, use the raw content.
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
    
    // Append the enclosing block context if the file uses markers and the basename matches.
    if file_uses_markers(&file_content) {
        if let Some(expected_basename) = todo_file_basename {
            if file_basename == expected_basename {
                if let Some(context) = extract_enclosing_block(file_path_str) {
                    combined_content.push_str("\n\n// Enclosing function context:\n");
                    combined_content.push_str(&context);
                }
            }
        }
    }
    
    Ok(combined_content)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

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

    // For extract_enclosing_block, we simulate file content via temporary files.
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
        // Expect that the extracted block starts at the function declaration.
        assert!(block_str.contains("func myFunction()"), "Block should contain the function declaration");
        // And should not include content before the candidate (i.e. not include leading unrelated lines).
        assert!(!block_str.contains("Some extra context"), "Block should not include unrelated context outside the function block");
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
    
    // Test that if the file does not contain any marker,
    // process_file returns the raw file content.
    #[test]
    fn test_process_file_no_markers() {
        let raw_content = "fn main() {\n    println!(\"Hello, world!\");\n}\n";
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "{}", raw_content).expect("Failed to write to temp file");
        // Pass None for todo_file_basename (or any value) since there are no markers.
        let result = process_file(temp_file.path(), Some("irrelevant.txt"))
            .expect("process_file should succeed for file without markers");
        // Without markers, process_file should return the raw content.
        assert_eq!(result, raw_content);
    }

    // Test that if markers are present but the provided expected basename does not match,
    // the function returns the filtered marker content without appending the enclosing block.
    #[test]
    fn test_process_file_markers_basename_mismatch() {
        // A file that uses markers and contains a TODO.
        let content_with_markers = r#"
func sampleFunction() {
    println("Start");
}

// v
// Marker block content line 1
// Marker block content line 2
// ^

 // TODO: - perform a task
"#;
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "{}", content_with_markers).expect("Failed to write to temp file");
        let file_path = temp_file.path();
        // Use an expected basename that does NOT match the file's actual basename.
        let wrong_basename = "mismatch.txt";
        let result = process_file(file_path, Some(wrong_basename))
            .expect("process_file should succeed");
        // Expect that the result equals the filtered content produced by filter_substring_markers,
        // without the appended enclosing block.
        let expected_filtered = filter_substring_markers(content_with_markers);
        assert_eq!(result, expected_filtered, "When the expected basename does not match, no context should be appended");
    }

    // Test that if markers are present and the provided expected basename matches,
    // the function returns the filtered marker content with the extracted enclosing block appended.
    #[test]
    fn test_process_file_markers_basename_match() {
        // This content has markers and a TODO outside the marker block.
        let content_with_markers = r#"
func myFunction() {
    println("Hello");
}

// v
// Extra context that is not part of the function block.
// ^
 // TODO: - perform important task
"#;
        let mut temp_file = NamedTempFile::new().expect("Failed to create temp file");
        write!(temp_file, "{}", content_with_markers).expect("Failed to write to temp file");
        let file_path = temp_file.path();
        // Provide the expected basename that exactly matches the file's basename.
        let expected_basename = file_path.file_name().unwrap().to_str().unwrap();
        let result = process_file(file_path, Some(expected_basename))
            .expect("process_file should succeed");

        // The expected result is the filtered content (from markers) plus the enclosing block context appended.
        let filtered = filter_substring_markers(content_with_markers);
        let expected_context = extract_enclosing_block(file_path.to_str().unwrap())
            .expect("Expected to extract an enclosing block");
        let expected = format!("{}\
\n\n// Enclosing function context:\n{}",
                                filtered, expected_context);
        assert_eq!(result, expected, "When the expected basename matches, the enclosing block should be appended");
    }

    // Test that process_file returns an error when the file does not exist.
    #[test]
    fn test_process_file_file_not_found() {
        // Create a temporary file path and then delete the file so it no longer exists.
        let temp_file = NamedTempFile::new().expect("Failed to create temp file");
        let file_path = temp_file.into_temp_path().keep().expect("Failed to persist temp file");
        // Delete the file.
        fs::remove_file(&file_path).expect("Failed to delete temporary file");

        let result = process_file(&file_path, Some("dummy.txt"));
        assert!(result.is_err(), "Expected process_file to error when file does not exist");
    }
}
