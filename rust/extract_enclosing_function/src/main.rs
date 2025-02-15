use regex::Regex;
use std::env;
use std::fs;
use std::process;

/// Returns true if the content contains any line that, when trimmed, equals "// v"
/// or if the content contains "Parse.Cloud.define(".
pub fn uses_markers(content: &str) -> bool {
    content.lines().any(|line| line.trim() == "// v") || content.contains("Parse.Cloud.define(")
}

/// Returns the index (zero-based) of the first line that contains "// TODO: - ", or None if not found.
pub fn todo_index(content: &str) -> Option<usize> {
    content.lines().position(|line| line.contains("// TODO: - "))
}

/// Given the content and the index of the TODO line, returns true if that TODO is inside a marker block.
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

/// Finds the index of the enclosing function (or computed property) definition that should contain the TODO.
///
/// This function now limits its primary search to the last 20 lines before the TODO,
/// looking for a line that contains one of the following:
/// - an assignment‑style function header ("= function(")
/// - a cloud code header ("Parse.Cloud.define(")
/// - an async function header ("async function")
/// - a Swift function header ("func ")
/// - a Swift computed property header. We now check if the trimmed line starts with one of:
///   "var ", "private var ", "public var ", "internal var ", or "fileprivate var ",
///   and that it contains "{" and does not contain "=".
///
/// If none is found in that window, it falls back to scanning backwards using a regex.
pub fn find_enclosing_function_start(content: &str, todo_idx: usize) -> Option<usize> {
    let lines: Vec<&str> = content.lines().collect();
    let start_search = if todo_idx >= 20 { todo_idx - 20 } else { 0 };
    if let Some(idx) = lines[start_search..=todo_idx].iter().rposition(|line| {
        let trimmed = line.trim_start();
        trimmed.contains("= function(")
            || trimmed.contains("Parse.Cloud.define(")
            || trimmed.contains("async function")
            || trimmed.contains("func ")
            || (
                (trimmed.starts_with("var ")
                    || trimmed.starts_with("private var ")
                    || trimmed.starts_with("public var ")
                    || trimmed.starts_with("internal var ")
                    || trimmed.starts_with("fileprivate var "))
                && line.contains("{")
                && !line.contains("=")
            )
    }) {
        return Some(start_search + idx);
    }
    // Fallback regex: allow for Swift computed properties with optional modifiers.
    let re = Regex::new(
        r"(?i)(function\b|=>|Parse\.Cloud\.define\s*\(|func\b|(?:(?:private|public|internal|fileprivate)\s+)?var\s+\w+\s*:\s*[\w<>, ?]+\s*\{)"
    ).expect("Invalid regex");
    for i in (0..=todo_idx).rev() {
        if re.is_match(lines[i]) {
            return Some(i);
        }
    }
    None
}

/// Extracts the block of code starting from `start_index` using a simple brace counting heuristic.
/// It returns the extracted block as a String.
pub fn extract_block(content: &str, start_index: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut found_opening = false;
    let mut brace_count = 0;
    let mut extracted_lines = Vec::new();

    for line in &lines[start_index..] {
        if !found_opening {
            if line.contains('{') {
                found_opening = true;
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
    extracted_lines.join("\n")
}

/// Combines the above functions to extract the enclosing block (if any) that contains the TODO.
/// Returns Some(block) if extraction is successful; otherwise returns None.
pub fn extract_enclosing_block(content: &str) -> Option<String> {
    if !uses_markers(content) {
        return None;
    }
    let todo_idx = todo_index(content)?;
    if is_todo_inside_markers(content, todo_idx) {
        return None;
    }
    let start_index = find_enclosing_function_start(content, todo_idx)?;
    Some(extract_block(content, start_index))
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        process::exit(1);
    }
    let file_path = &args[1];

    let content = fs::read_to_string(file_path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", file_path, err);
        process::exit(1);
    });

    if let Some(block) = extract_enclosing_block(&content) {
        println!("{}", block);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLOUD_GLOBAL_SETTINGS: &str = r#"
Parse.Cloud.define("getGlobalSettings", async (request) => {
    try {
        const globalSettings = await getGlobalSettings();
        return globalSettings;
    } catch (error) {
        console.error("ERROR UPGRADE getGlobalSettings " + request.params.currentUserObjectId + ": " + error.message);
        throw new Error(error);
    }
});

Parse.Cloud.define("isUsernameAvailable", async (request) => {
    // TODO: - example
    const username = request.params.username;
    return getUserFromUsername(username, true);
});
"#;

    const ASSIGNMENT_FUNCTION: &str = r#"
var someOtherVar = 123;
getUserFromUsernameDebuggingDuplicates = function(username) {
    // Some debug code
    console.log("Looking up user", username);
    // TODO: - check duplicate usernames
    return { username: username };
};
"#;

    const ASYNC_FUNCTION: &str = r#"
async function generateRecentInteractionsString(nonPremiumParticipants, premiumCount, person, mostExchangesShortConversation) {
    // TODO: - some async function todo example
    return "some string";
}
"#;

    const SWIFT_FUNCTION: &str = r#"
func generateRecentInteractionsString(nonPremiumParticipants: [String], premiumCount: Int, person: Person, mostExchangesShortConversation: Bool) -> String {
    // TODO: - handle swift function todo example
    return "result"
}
"#;

    const SWIFT_COMPUTED_PROPERTY: &str = r#"
private var appDelegate: AppDelegate? {
    // TODO: - computed property todo example
    appManager.appDelegate
}
"#;

    const NO_MARKERS: &str = r#"
function someOtherFunction() {
    // Some code
    // TODO: - example
    return 42;
}
"#;

    #[test]
    fn test_find_enclosing_function_start_for_cloud() {
        let content = CLOUD_GLOBAL_SETTINGS;
        let todo_idx = todo_index(content).unwrap();
        let start_idx = find_enclosing_function_start(content, todo_idx).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.contains("Parse.Cloud.define(\"isUsernameAvailable\"") ||
                start_line.contains("Parse.Cloud.define (\"isUsernameAvailable\""),
                "Expected the function header for isUsernameAvailable, got: {}", start_line);
    }

    #[test]
    fn test_find_enclosing_function_start_for_assignment() {
        let content = ASSIGNMENT_FUNCTION;
        let todo_idx = todo_index(content).unwrap();
        let start_idx = find_enclosing_function_start(content, todo_idx).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.contains("= function("));
    }

    #[test]
    fn test_find_enclosing_function_start_for_async() {
        let content = ASYNC_FUNCTION;
        let todo_idx = todo_index(content).unwrap();
        let start_idx = find_enclosing_function_start(content, todo_idx).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.contains("async function"));
    }

    #[test]
    fn test_find_enclosing_function_start_for_swift() {
        let content = SWIFT_FUNCTION;
        let todo_idx = todo_index(content).unwrap();
        let start_idx = find_enclosing_function_start(content, todo_idx).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.contains("func generateRecentInteractionsString"),
                "Expected a Swift function header, got: {}", start_line);
    }

    #[test]
    fn test_find_enclosing_function_start_for_swift_computed_property() {
        let content = SWIFT_COMPUTED_PROPERTY;
        let todo_idx = todo_index(content).unwrap();
        let start_idx = find_enclosing_function_start(content, todo_idx).unwrap();
        let lines: Vec<&str> = content.lines().collect();
        let start_line = lines[start_idx];
        // Now, even with a modifier ("private var") the computed property header should be detected.
        assert!(start_line.trim_start().starts_with("private var")
                && start_line.contains("{")
                && !start_line.contains("="),
                "Expected a computed property header, got: {}", start_line);
    }
}
