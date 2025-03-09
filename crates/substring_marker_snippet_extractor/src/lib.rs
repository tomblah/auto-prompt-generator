// crates/substring_marker_snippet_extractor/src/lib.rs

use std::fs;

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
    // Updated Swift function regex:
    // - Allows optional generic parameters: (?:<[^>]+>)?
    // - Allows an optional return type: (?:->\s*\S+)?
    let swift_function = regex::Regex::new(
        r#"^\s*(?:(?:public|private|internal|fileprivate)\s+)?func\s+\w+(?:<[^>]+>)?\s*\([^)]*\)\s*(?:->\s*\S+)?\s*\{"#
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

// Embrace the new trait-based API by re-exporting the processor module.
// Consumers should now use the API provided in the processor module, for example:
//
// use substring_marker_snippet_extractor::processor::{DefaultFileProcessor, process_file_with_processor};
//
pub mod processor;
