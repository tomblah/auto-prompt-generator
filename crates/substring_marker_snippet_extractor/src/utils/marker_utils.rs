// crates/substring_marker_snippet_extractor/src/utils/marker_utils.rs

use std::fs;
use regex::Regex;
use once_cell::sync::Lazy;

/// The standard marker used to indicate the primary TODO instruction.
pub const TODO_MARKER: &str = "// TODO: - ";

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

    // Handle case where content ends in an omitted block
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

/// Returns the index (zero-based) of the first line that contains the TODO marker.
pub fn todo_index(content: &str) -> Option<usize> {
    content.lines().position(|line| line.contains(TODO_MARKER))
}

/// Determines whether the TODO is already inside a marker block by counting marker boundaries
/// from the start of the file up to the TODO line.
///
/// # Arguments
///
/// * `content` - The file content.
/// * `todo_idx` - The line index of the TODO marker.
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

/// Static, precompiled regexes for candidate line detection for enclosing blocks.
/// Includes Swift functions, classes, structs, enums, JS functions/assignments,
/// Parse Cloud functions, and Objective-C methods.
static CANDIDATE_LINE_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^\s*(?:(?:public|private|internal|fileprivate)\s+)?(?:func|class|struct|enum)\s+\w+(?:<[^>]+>)?.*\{"# // Swift
        ).unwrap()
});
static JS_ASSIGNMENT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^\s*(?:(?:const|var|let)\s+)?\w+\s*=\s*function\s*\([^)]*\)\s*\{"#).unwrap()
});
static JS_FUNCTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^\s*(?:async\s+)?function\s+\w+\s*\([^)]*\)\s*\{"#).unwrap()
});
static PARSE_CLOUD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^\s*Parse\.Cloud\.(?:define|beforeSave|afterSave)\s*\(\s*(?:"[^"]+"|[A-Za-z][A-Za-z0-9_.]*)\s*,\s*(?:async\s+)?\([^)]*\)\s*=>\s*\{"#
    )
    .unwrap()
});
static OBJC_METHOD_RE: Lazy<Regex> = Lazy::new(|| {
    // Matches Objective‑C method declarations.
    Regex::new(r"^\s*[-+]\s*\([^)]*\)\s*[a-zA-Z_][a-zA-Z0-9_]*(?::\s*\([^)]*\)\s*[a-zA-Z_][a-zA-Z0-9_]*)*\s*\{")
        .unwrap()
});


/// Checks if a line is a candidate for the start of an enclosing code block.
fn is_candidate_line(line: &str) -> bool {
    CANDIDATE_LINE_RE.is_match(line)
        || JS_ASSIGNMENT_RE.is_match(line)
        || JS_FUNCTION_RE.is_match(line)
        || PARSE_CLOUD_RE.is_match(line)
        || OBJC_METHOD_RE.is_match(line)
}


/// Extracts the enclosing block (such as a function, class, struct, or enum)
/// that contains the first occurrence of the TODO marker.
///
/// It scans upward from the TODO marker for the last candidate declaration line,
/// then uses a simple brace counting heuristic to extract the block from that line
/// until the matching closing brace.
///
/// Returns `None` if the TODO marker is not found, is inside `// v` / `// ^` markers,
/// no candidate line is found before the TODO, or the block structure is unexpected.
///
/// # Arguments
///
/// * `content` - The file content as a string slice.
pub fn extract_enclosing_block_around_todo(content: &str) -> Option<String> {
    let todo_idx = todo_index(content)?; // Find the line index of the TODO marker

    if is_todo_inside_markers(content, todo_idx) {
        return None; // Don't extract if TODO is already inside markers
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut candidate_index = None;

    // Scan upwards from the line before the TODO for the last candidate line
    for i in (0..todo_idx).rev() {
        let line = lines[i];
        if is_candidate_line(line) {
            candidate_index = Some(i);
            break; // Found the last candidate line before the TODO
        } else if (line.trim_start().starts_with('-') || line.trim_start().starts_with('+')) {
             // Special handling for ObjC method declarations that might be split over two lines (selector on one, { on next)
            if i + 1 < todo_idx && lines[i + 1].contains('{') {
                candidate_index = Some(i);
                break; // Found the last candidate line before the TODO
            }
        }
    }

    let start_index = candidate_index?; // If no candidate found, return None

    // Now, perform brace counting starting from the candidate line
    let mut brace_count = 0;
    let mut found_open = false;
    let mut extracted_lines = Vec::new();

    for line in &lines[start_index..] {
        extracted_lines.push(*line); // Always include lines from candidate start

        if !found_open {
            if line.contains('{') {
                found_open = true;
                brace_count += line.matches('{').count();
                brace_count = brace_count.saturating_sub(line.matches('}').count());
            }
        } else {
            brace_count += line.matches('{').count();
            brace_count = brace_count.saturating_sub(line.matches('}').count());
            if brace_count == 0 {
                // Found matching closing brace
                break;
            }
        }
    }

    // Return the extracted lines joined by newlines, but only if a block was properly found and closed
    if found_open && brace_count == 0 {
         Some(extracted_lines.join("\n"))
    } else {
         None // Block structure was not as expected (e.g., never found an opening brace, or brace count didn't return to zero)
    }
}


/// Extracts the inner block—that is, the content inside the braces that immediately enclose
/// the first occurrence of the TODO marker. This is done using a stack-based approach
/// to correctly identify the innermost unclosed '{'.
///
/// This function is distinct from `extract_enclosing_block_around_todo` as it finds the
/// *immediate* enclosing block, not the nearest containing declaration.
///
/// # Arguments
///
/// * `content` - The file content as a string slice.
pub fn extract_inner_block_from_content(content: &str) -> Option<String> {
    let pos = content.find(TODO_MARKER)?; // Find byte position of the TODO marker
    let mut stack = Vec::new();

    // Process characters up to the TODO marker to find the innermost unclosed '{'.
    for (i, ch) in content[..pos].char_indices() {
        if ch == '{' {
            stack.push(i);
        } else if ch == '}' {
            stack.pop(); // Pop the last '{' if a '}' is found
        }
    }

    let open_brace = stack.pop()?; // The last '{' on the stack is the immediate enclosing one

    // Now find the matching closing brace starting from the character after the opening brace.
    let mut brace_count = 1; // Start with 1 for the initial open_brace
    let mut index = open_brace + 1;
    let bytes = content.as_bytes();

    while index < content.len() && brace_count > 0 {
        match bytes[index] {
            b'{' => brace_count += 1,
            b'}' => brace_count -= 1,
            _ => {}
        }
        index += 1;
    }

    // If brace_count is zero, we found the matching closing brace at `index - 1`.
    if brace_count == 0 {
        Some(content[open_brace + 1..index - 1].to_string())
    } else {
        None // Matching closing brace not found
    }
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
        // Create a temporary file with a candidate declaration, markers, and a TODO.
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

    #[test]
    fn test_extract_enclosing_block_parse_cloud_success() {
        // Create a temporary file where substring markers wrap header content
        // and a Parse.Cloud.beforeSave function (which contains the TODO marker)
        // appears after the markers. The "Footer text" outside the function should be omitted.
        let content = "\
Header text
// v
Header content inside markers
// ^
Parse.Cloud.beforeSave(\"Message\", async (request) => {
    console.log(\"Setup\");
    // TODO: - Do something important
    console.log(\"Teardown\");
});
Footer text that should be omitted";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path().to_str().unwrap());
        assert!(block.is_some());
        let block_str = block.unwrap();
        // Verify that the extracted block is the entire Parse.Cloud function
        assert!(block_str.contains("Parse.Cloud.beforeSave(\"Message\", async (request) => {"));
        assert!(block_str.contains("// TODO: - Do something important"));
        assert!(block_str.contains("console.log(\"Teardown\");"));
        // Ensure that footer text is not included
        assert!(!block_str.contains("Footer text"));
    }

    #[test]
    fn test_extract_enclosing_block_after_save_success() {
        // Test a Parse.Cloud.afterSave function with a quoted first argument.
        let content = "\
Header text
// v
Header section that is not part of the function
// ^
Parse.Cloud.afterSave(\"Message\", async (request) => {
    console.log(\"AfterSave Setup\");
    // TODO: - Handle after save logic
    console.log(\"AfterSave Teardown\");
});
Some trailing footer text";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path().to_str().unwrap());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("Parse.Cloud.afterSave(\"Message\", async (request) => {"));
        assert!(block_str.contains("// TODO: - Handle after save logic"));
        assert!(block_str.contains("console.log(\"AfterSave Teardown\");"));
        assert!(!block_str.contains("trailing footer text"));
    }

    #[test]
    fn test_extract_enclosing_block_before_save_parse_user_success() {
        // Test a Parse.Cloud.beforeSave function with Parse.User as the first argument.
        let content = "\
Some header information
// v
Ignored header details
// ^
Parse.Cloud.beforeSave(Parse.User, async (request) => {
    console.log(\"BeforeSave Init\");
    // TODO: - Process user before save
    console.log(\"BeforeSave Complete\");
});
Extra text that should be omitted";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path().to_str().unwrap());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("Parse.Cloud.beforeSave(Parse.User, async (request) => {"));
        assert!(block_str.contains("// TODO: - Process user before save"));
        assert!(block_str.contains("console.log(\"BeforeSave Complete\");"));
        assert!(!block_str.contains("Extra text"));
    }

    #[test]
    fn test_extract_enclosing_block_after_save_parse_user_success() {
        // Test a Parse.Cloud.afterSave function with Parse.User as the first argument.
        let content = "\
Introductory header
// v
Header content that is not part of the function
// ^
Parse.Cloud.afterSave(Parse.User, async (request) => {
    console.log(\"AfterSave Start\");
    // TODO: - Process user after save
    console.log(\"AfterSave End\");
});
Irrelevant footer";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path().to_str().unwrap());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("Parse.Cloud.afterSave(Parse.User, async (request) => {"));
        assert!(block_str.contains("// TODO: - Process user after save"));
        assert!(block_str.contains("console.log(\"AfterSave End\");"));
        assert!(!block_str.contains("Irrelevant footer"));
    }

    #[test]
    fn test_objc_method_candidate_line() {
        let objc_line = " - (void)myMethod:(NSString *)arg {";
        // Verify that the OBJC method regex matches and the candidate line check returns true.
        assert!(OBJC_METHOD_RE.is_match(objc_line));
        assert!(is_candidate_line(objc_line));
    }

    #[test]
    fn test_extract_enclosing_block_objc_success() {
        // Create a temporary file with an Objective-C method declaration,
        // markers, and a TODO marker after the markers.
        let content = "\
Some header info
// v
// ^
- (void)myMethod:(NSString *)arg {
    NSLog(@\"Start\");
    // TODO: - Do something in ObjC
    NSLog(@\"End\");
}";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path().to_str().unwrap());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)myMethod:(NSString *)arg {"));
        assert!(block_str.contains("// TODO: - Do something in ObjC"));
        assert!(block_str.contains("NSLog(@\"End\");"));
    }

    #[test]
    fn test_extract_enclosing_block_objc_split_declaration_success() {
        // Test an Objective-C method declaration split across two lines:
        // The first line has the method signature without the opening brace,
        // and the following line contains the opening brace.
        let content = "\
Some header info
// v
Header details that are not part of the method
// ^
- (void)myMethod:(NSString *)arg
{
    NSLog(@\"Start split\");
    // TODO: - Do something split
    NSLog(@\"End split\");
}";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path().to_str().unwrap());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)myMethod:(NSString *)arg"));
        assert!(block_str.contains("{"));
        assert!(block_str.contains("// TODO: - Do something split"));
        assert!(block_str.contains("NSLog(@\"End split\");"));
    }
}
