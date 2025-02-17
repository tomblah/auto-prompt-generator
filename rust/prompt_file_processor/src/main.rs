#[macro_use]
extern crate lazy_static;

use regex::Regex;
use std::fs;
use std::path::Path;

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

/// Returns true if the given line matches any candidate declaration.
/// We use separate lazy_static regexes for:
///   1. Swift function declarations (e.g. `func loadMessages(...) {`)
///   2. Swift computed property declarations (e.g. `var lastUpdated: Date? {`)
///   3. Parse.Cloud.define JavaScript functions
///   4. JavaScript assignment-style functions (e.g. `myFunc = function(...) {`)
///   5. Standard JavaScript function declarations (e.g. `async function myFunc(...) {`)
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

/// Extracts the enclosing block (Swift function, computed property, or JavaScript function)
/// that contains the TODO marker. The algorithm is:
///
/// 1. Read the file and split into lines.
/// 2. Find the first line containing "// TODO: -"; record its index as `todo_index`.
/// 3. Iterate over the lines from the beginning up to `todo_index` and, for each line,
///    if it is a candidate declaration (per `is_candidate_line`), run a simple brace‑counting
///    routine starting at that line.
/// 4. If the resulting block spans a range that includes the `todo_index`, consider it.
/// 5. If more than one candidate qualifies, choose the candidate that starts nearest (i.e. later)
///    to the TODO marker.
pub fn extract_enclosing_block(file_path: &str) -> Option<String> {
    let content = fs::read_to_string(file_path).ok()?;
    let lines: Vec<&str> = content.lines().collect();
    let todo_index = lines.iter().position(|line| line.contains("// TODO: -"))?;
    let mut best_candidate: Option<(usize, usize, Vec<&str>)> = None;

    for (i, line) in lines[..=todo_index].iter().enumerate() {
        if is_candidate_line(line) {
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
            if i <= todo_index && todo_index <= end_index {
                match best_candidate {
                    Some((prev_start, _, _)) if i > prev_start => {
                        best_candidate = Some((i, end_index, block_lines))
                    }
                    None => best_candidate = Some((i, end_index, block_lines)),
                    _ => (),
                }
            }
        }
    }
    best_candidate.map(|(_, _, block_lines)| block_lines.join("\n"))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Expected usage: prompt_file_processor <file_path> [<todo_file_basename>]
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <file_path> [<todo_file_basename>]", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];
    let todo_file_basename = if args.len() >= 3 { Some(&args[2]) } else { None };

    let file_content = fs::read_to_string(file_path)?;

    let processed_content = if file_content.lines().any(|line| line.trim() == "// v") {
        filter_substring_markers(&file_content)
    } else {
        file_content
    };

    let file_basename = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path);

    let mut combined_content = processed_content;

    if let Some(todo_basename) = todo_file_basename {
        if file_basename == todo_basename {
            if let Some(context) = extract_enclosing_block(file_path) {
                combined_content.push_str("\n\n// Enclosing function context:\n");
                combined_content.push_str(&context);
            }
        }
    }

    print!("{}", combined_content);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Helper: Write content to a temporary file and return its path.
    fn write_to_temp_file(content: &str) -> String {
        let mut tmpfile = NamedTempFile::new().expect("Failed to create temp file");
        write!(tmpfile, "{}", content).expect("Failed to write to temp file");
        tmpfile.into_temp_path().to_str().unwrap().to_string()
    }

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

/*
    #[test]
    fn test_extract_enclosing_block_swift_function() {
        let content = r#"
class MyClass {
    func loadMessagesFromServer(for conversation: String) {
        // Some setup code
        // TODO: - helllo
        print("Hello")
    }
}
"#;
        let path = write_to_temp_file(content);
        let extracted = extract_enclosing_block(&path).expect("Should extract a block");
        assert!(extracted.contains("func loadMessagesFromServer(for conversation: String) {"));
        assert!(extracted.contains("// TODO: - helllo"));
    }
*/

/*
    #[test]
    fn test_extract_enclosing_block_swift_computed_property() {
        let content = r#"
class MyClass {
    private var lastUpdated: Date? {
        // TODO: - helllo
        let now = Date()
        return now
    }
}
"#;
        let path = write_to_temp_file(content);
        let extracted = extract_enclosing_block(&path).expect("Should extract a block");
        assert!(extracted.contains("private var lastUpdated: Date? {"));
        assert!(extracted.contains("// TODO: - helllo"));
    }
*/

/*
    #[test]
    fn test_extract_enclosing_block_js_parse_cloud_define() {
        let content = r#"
Parse.Cloud.define("getDashboardData", async (request) => {
    // TODO: - helllo
    var data = 42;
});
"#;
        let path = write_to_temp_file(content);
        let extracted = extract_enclosing_block(&path).expect("Should extract a block");
        assert!(extracted.contains(r#"Parse.Cloud.define("getDashboardData", async (request) => {"#));
        assert!(extracted.contains("// TODO: - helllo"));
    }
*/

/*
    #[test]
    fn test_extract_enclosing_block_js_assignment() {
        let content = r#"
getAllMessagesForPersonSingleSide = function(person, isFrom) {
    // TODO: - helllo
    var messageLimit = 1000;
    return messageLimit;
};
"#;
        let path = write_to_temp_file(content);
        let extracted = extract_enclosing_block(&path).expect("Should extract a block");
        assert!(extracted.contains("getAllMessagesForPersonSingleSide = function(person, isFrom) {"));
        assert!(extracted.contains("// TODO: - helllo"));
    }
*/

/*
    #[test]
    fn test_extract_enclosing_block_js_async_function() {
        let content = r#"
async function getGTKPeople(redisRootKey, consoleTag) {
    // TODO: - helllo
    const now = new Date();
    return now;
}
"#;
        let path = write_to_temp_file(content);
        let extracted = extract_enclosing_block(&path).expect("Should extract a block");
        assert!(extracted.contains("async function getGTKPeople(redisRootKey, consoleTag) {"));
        assert!(extracted.contains("// TODO: - helllo"));
    }
*/

/*
    #[test]
    fn test_extract_enclosing_block_multiple_candidates() {
        let content = r#"
function firstCandidate() {
    // Irrelevant code
}

async function secondCandidate() {
    // TODO: - helllo
    console.log("This is the second candidate");
}
"#;
        let path = write_to_temp_file(content);
        let extracted = extract_enclosing_block(&path).expect("Should extract a block");
        // Expect the candidate closest to the TODO marker (i.e. secondCandidate) to be chosen.
        assert!(extracted.contains("async function secondCandidate() {"));
        assert!(extracted.contains("// TODO: - helllo"));
    }
*/

}

