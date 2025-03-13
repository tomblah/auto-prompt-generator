// src/utils/marker_utils.rs

use std::fs;
use regex::Regex;

/// Filters the file’s content by returning only the text between substring markers.
/// Instead of always using "// ...", the caller can supply a custom placeholder.
///
/// # Arguments
///
/// * `content` - The file content.
/// * `placeholder` - The string to use as a placeholder for omitted code.
pub fn filter_substring_markers(content: &str, placeholder: &str) -> String {
    let mut output = String::new();
    let mut state = "omitted";
    let mut omitted_line_count = 0;
    let mut last_was_closing = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "// v" {
            if omitted_line_count > 0 {
                output.push_str("\n\n");
                output.push_str(placeholder);
                output.push_str("\n\n");
            }
            omitted_line_count = 0;
            state = "included";
            last_was_closing = false;
            continue;
        } else if trimmed == "// ^" {
            state = "omitted";
            omitted_line_count = 0;
            last_was_closing = true;
            continue;
        }

        match state {
            "included" => {
                output.push_str(line);
                output.push('\n');
                last_was_closing = false;
            }
            "omitted" => {
                omitted_line_count += 1;
                last_was_closing = false;
            }
            _ => unreachable!(),
        }
    }

    if state == "omitted" && (omitted_line_count > 0 || last_was_closing) {
        output.push_str("\n\n");
        output.push_str(placeholder);
        output.push_str("\n\n");
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

/// Private helper: determines if a given line is a candidate declaration line.
/// It uses regex patterns for Swift functions, JS functions/assignments, or Parse.Cloud.define.
fn is_candidate_line(line: &str) -> bool {
    let swift_function = Regex::new(
        r#"^\s*(?:(?:public|private|internal|fileprivate)\s+)?func\s+\w+(?:<[^>]+>)?\s*\([^)]*\)\s*(?:->\s*\S+)?\s*\{"#
    ).unwrap();
    let js_assignment = Regex::new(
        r#"^\s*(?:(?:const|var|let)\s+)?\w+\s*=\s*function\s*\([^)]*\)\s*\{"#
    ).unwrap();
    let js_function = Regex::new(
        r#"^\s*(?:async\s+)?function\s+\w+\s*\([^)]*\)\s*\{"#
    ).unwrap();
    let parse_cloud = Regex::new(
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
pub fn extract_enclosing_block(file_path: &str) -> Option<String> {
    let content = fs::read_to_string(file_path).ok()?;
    if !file_uses_markers(&content) {
        return None;
    }
    let todo_idx = content.lines().position(|line| line.contains("// TODO: - "))?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_filter_substring_markers() {
        let input = "\
Line before
// v
content line 1
content line 2
// ^
Line after";
        // The expected output has two newlines before and after the placeholder,
        // and due to the included region ending with a newline, an extra newline appears.
        let expected = "\n\n// ...\n\ncontent line 1\ncontent line 2\n\n\n// ...\n\n";
        let result = filter_substring_markers(input, "// ...");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_file_uses_markers_true() {
        let content = "Some text\n// v\nmarker content\n// ^\nMore text";
        assert!(file_uses_markers(content));
    }

    #[test]
    fn test_file_uses_markers_false() {
        let content = "Some text\n// v\nmarker content\nMore text";
        assert!(!file_uses_markers(content));
    }

    #[test]
    fn test_todo_index() {
        let content = "Line1\nLine2 // TODO: - Fix issue\nLine3";
        let idx = todo_index(content);
        assert!(idx.is_some());
        let lines: Vec<&str> = content.lines().collect();
        let index = idx.unwrap();
        assert!(lines[index].contains("// TODO: -"));
    }

    #[test]
    fn test_is_todo_inside_markers_true() {
        let content = "\
Line1
// v
// TODO: - inside markers
// ^
Line after";
        // TODO is on line 2 (0-indexed)
        let idx = todo_index(content).unwrap();
        let result = is_todo_inside_markers(content, idx);
        assert!(result);
    }

    #[test]
    fn test_is_todo_inside_markers_false() {
        let content = "\
Line1
// TODO: - outside markers
// v
Marker start
// ^
More text";
        // TODO is on line 1 (0-indexed)
        let idx = todo_index(content).unwrap();
        let result = is_todo_inside_markers(content, idx);
        assert!(!result);
    }

    #[test]
    fn test_extract_enclosing_block_success() {
        // Create a temporary file with candidate declaration, markers, and a TODO.
        let content = "\
Some irrelevant text
func myFunction() {
    let x = 10;
}
Other text
// v
Marker content
// ^
More text
// TODO: - Do something";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path().to_str().unwrap());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("func myFunction() {"));
        assert!(block_str.contains("let x = 10;"));
        assert!(block_str.contains("}"));
    }

    #[test]
    fn test_extract_enclosing_block_no_markers() {
        // Create a temporary file without both markers.
        let content = "\
func myFunction() {
    let x = 10;
}";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path().to_str().unwrap());
        assert!(block.is_none());
    }
}
