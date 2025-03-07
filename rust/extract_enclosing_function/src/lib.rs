use std::mem;
use regex::Regex;
use tree_sitter::Parser;
use tree_sitter_swift;

/// Removes common Swift access modifiers from the beginning of lines,
/// allowing for optional leading whitespace.
fn remove_access_modifiers(content: &str) -> String {
    // Updated regex: allow for leading spaces before access modifiers.
    let re = Regex::new(r"(?m)^\s*(open|public|internal|fileprivate|private)\s+").unwrap();
    re.replace_all(content, "").to_string()
}

/// Extracts the enclosing function block from the given content.
///
/// This implementation attempts to use Tree‑sitter’s Swift parser to accurately locate
/// the function declaration that contains the first occurrence of the TODO marker.
/// If the file uses marker blocks (i.e. both "// v" and "// ^") and the TODO is inside the markers,
/// then no block is extracted (returns None). Otherwise, if Tree‑sitter fails or no matching
/// function is found, the function falls back to a heuristic approach.
pub fn extract_enclosing_block(content: &str) -> Option<String> {
    // Preprocess the content to remove access modifiers.
    let preprocessed_content = remove_access_modifiers(content);

    // Get the line index of the TODO marker.
    let todo_idx = todo_index(&preprocessed_content)?;
    // If the file uses markers and the TODO is inside a marker block, return None.
    if file_uses_markers(&preprocessed_content) && is_todo_inside_markers(&preprocessed_content, todo_idx) {
        return None;
    }

    // Also get the byte offset for Tree‑sitter processing.
    let todo_offset = preprocessed_content.find("// TODO: - ")?;
    
    let mut parser = Parser::new();
    // Convert the LANGUAGE constant (a function pointer) into a Language value.
    let language: tree_sitter::Language = unsafe { mem::transmute(tree_sitter_swift::LANGUAGE) };
    if parser.set_language(&language).is_err() {
        return extract_enclosing_block_heuristic(&preprocessed_content);
    }
    
    // Parse the content into a syntax tree.
    let tree = parser.parse(&preprocessed_content, None)?;
    let root_node = tree.root_node();
    
    // Collect all named descendant nodes.
    let named_nodes = get_named_descendants(root_node);
    for node in named_nodes {
        if node.start_byte() <= todo_offset && node.end_byte() >= todo_offset {
            // Look for a Swift function declaration.
            if node.kind() == "function_declaration" {
                return Some(preprocessed_content[node.start_byte()..node.end_byte()].to_string());
            }
        }
    }
    
    // Fallback: use the heuristic approach.
    extract_enclosing_block_heuristic(&preprocessed_content)
}

/// Recursively collects all named descendant nodes of the given node.
fn get_named_descendants<'a>(node: tree_sitter::Node<'a>) -> Vec<tree_sitter::Node<'a>> {
    let mut result = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            result.push(child);
            result.extend(get_named_descendants(child));
        }
    }
    result
}

/// Fallback heuristic implementation using line scanning and brace counting.
fn extract_enclosing_block_heuristic(content: &str) -> Option<String> {
    // Compute the TODO index from the preprocessed content.
    let _ = todo_index(content)?;
    let start_index = find_enclosing_function_start(content)?;
    Some(extract_block(content, start_index))
}

/// Returns the index (zero-based) of the first line that contains "// TODO: - ".
pub fn todo_index(content: &str) -> Option<usize> {
    content.lines().position(|line| line.contains("// TODO: - "))
}

/// Returns true if the file contains both markers ("// v" and "// ^").
pub fn file_uses_markers(content: &str) -> bool {
    let has_open = content.lines().any(|line| line.trim() == "// v");
    let has_close = content.lines().any(|line| line.trim() == "// ^");
    has_open && has_close
}

/// Returns true if the TODO is inside a marker block by counting marker boundaries
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

/// Finds the starting line index of the enclosing function block using simple heuristics.
/// This function first applies `remove_access_modifiers` so that access modifiers are stripped.
pub fn find_enclosing_function_start(content: &str) -> Option<usize> {
    let processed = remove_access_modifiers(content);
    let _ = todo_index(&processed)?;
    let lines: Vec<&str> = processed.lines().collect();
    let todo_idx = todo_index(&processed)?;
    let start_search = if todo_idx >= 20 { todo_idx - 20 } else { 0 };
    lines[start_search..=todo_idx].iter().rposition(|line| {
        let trimmed = line.trim_start();
        trimmed.starts_with("func ")
            || trimmed.contains("= function(")
            || trimmed.contains("Parse.Cloud.define(")
            || trimmed.contains("async function")
            || ((trimmed.starts_with("var ")
                || trimmed.starts_with("private var ")
                || trimmed.starts_with("public var ")
                || trimmed.starts_with("internal var ")
                || trimmed.starts_with("fileprivate var "))
                && line.contains("{")
                && !line.contains("="))
    }).map(|idx| start_search + idx)
}

/// Extracts a block of code from the content starting at the given line index using a brace-counting heuristic.
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

#[cfg(test)]
mod tests {
    use super::*;

    // Existing test constants.
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

    // New constant: computed property with leading spaces.
    const SWIFT_COMPUTED_PROPERTY_WITH_SPACES: &str = r#"
    private var appDelegate: AppDelegate? {
        // TODO: - computed property todo example with spaces
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

    // New test constants to cover marker-specific logic.
    const MARKER_TEST_OUTSIDE: &str = r#"
func myFunction() {
    print("Hello")
}
 
// v
// Additional context that is outside the function block and will be ignored.
// ^

 // TODO: - do something important
"#;

    const MARKER_TEST_INSIDE: &str = r#"
func myFunction() {
    print("Hello")
    // v
    // TODO: - do something important
    // ^
}
"#;

    #[test]
    fn test_find_enclosing_function_start_for_cloud() {
        let content = CLOUD_GLOBAL_SETTINGS;
        let processed = remove_access_modifiers(content);
        let start_idx = find_enclosing_function_start(content).unwrap();
        let lines: Vec<&str> = processed.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.contains("Parse.Cloud.define(\"isUsernameAvailable\"")
            || start_line.contains("Parse.Cloud.define (\"isUsernameAvailable\""),
            "Expected the function header for isUsernameAvailable, got: {}", start_line);
    }

    #[test]
    fn test_find_enclosing_function_start_for_assignment() {
        let content = ASSIGNMENT_FUNCTION;
        let processed = remove_access_modifiers(content);
        let start_idx = find_enclosing_function_start(content).unwrap();
        let lines: Vec<&str> = processed.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.contains("= function("));
    }

    #[test]
    fn test_find_enclosing_function_start_for_async() {
        let content = ASYNC_FUNCTION;
        let processed = remove_access_modifiers(content);
        let start_idx = find_enclosing_function_start(content).unwrap();
        let lines: Vec<&str> = processed.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.contains("async function"));
    }

    #[test]
    fn test_find_enclosing_function_start_for_swift() {
        let content = SWIFT_FUNCTION;
        let processed = remove_access_modifiers(content);
        let start_idx = find_enclosing_function_start(content).unwrap();
        let lines: Vec<&str> = processed.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.contains("func generateRecentInteractionsString"),
                "Expected a Swift function header, got: {}", start_line);
    }

    #[test]
    fn test_find_enclosing_function_start_for_swift_computed_property() {
        let content = SWIFT_COMPUTED_PROPERTY;
        let processed = remove_access_modifiers(content);
        let start_idx = find_enclosing_function_start(content).unwrap();
        let lines: Vec<&str> = processed.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.trim_start().starts_with("var")
                && start_line.contains("{")
                && !start_line.contains("="),
                "Expected a computed property header (without access modifiers), got: {}", start_line);
    }

    #[test]
    fn test_find_enclosing_function_start_for_swift_computed_property_with_spaces() {
        let content = SWIFT_COMPUTED_PROPERTY_WITH_SPACES;
        let processed = remove_access_modifiers(content);
        let start_idx = find_enclosing_function_start(content).unwrap();
        let lines: Vec<&str> = processed.lines().collect();
        let start_line = lines[start_idx];
        assert!(start_line.trim_start().starts_with("var"),
                "Expected a computed property header (without access modifiers and leading spaces), got: {}", start_line);
    }

    #[test]
    fn test_file_uses_markers_true() {
        assert!(file_uses_markers(MARKER_TEST_OUTSIDE));
    }

    #[test]
    fn test_file_uses_markers_false() {
        assert!(!file_uses_markers(NO_MARKERS));
    }

    #[test]
    fn test_extract_enclosing_block_outside() {
        let block = extract_enclosing_block(MARKER_TEST_OUTSIDE);
        assert!(block.is_some(), "Expected an enclosing block, got None");
        let block_str = block.unwrap();
        assert!(block_str.contains("func myFunction()"), "Block should contain the function declaration");
    }

    #[test]
    fn test_extract_enclosing_block_inside() {
        let block = extract_enclosing_block(MARKER_TEST_INSIDE);
        assert!(block.is_none(), "Expected no enclosing block because the TODO is inside markers");
    }

    #[test]
    fn test_no_todo_marker() {
        let content = "function foo() { console.log('hello'); }";
        assert_eq!(todo_index(content), None);
        assert_eq!(extract_enclosing_block(content), None);
    }

    #[test]
    fn test_no_valid_function_header() {
        let content = r#"
Some random text.
Another line.
 // TODO: - stray todo with no function header
More random text.
"#;
        let idx = todo_index(content);
        assert!(idx.is_some());
        assert_eq!(find_enclosing_function_start(content), None);
        assert_eq!(extract_enclosing_block(content), None);
    }

    #[test]
    fn test_extract_block_with_missing_closing_brace() {
        let content = r#"
func incomplete() {
    let x = 10;
    let y = 20;
    // No closing brace here.
Some random text.
"#;
        let block = extract_block(content, 1);
        assert!(block.contains("func incomplete()"));
        assert!(block.contains("let x = 10;"));
        assert!(block.contains("Some random text."));
    }

    #[test]
    fn test_is_todo_inside_markers_direct() {
        let content_inside = r#"
func example() {
    // v
    // TODO: - inside todo
    // ^
}
"#;
        let idx_inside = todo_index(content_inside).unwrap();
        assert!(is_todo_inside_markers(content_inside, idx_inside));
        let content_outside = r#"
 // v
Some context here.
 // ^
func example() {
    // TODO: - outside todo
}
"#;
        let idx_outside = todo_index(content_outside).unwrap();
        assert!(!is_todo_inside_markers(content_outside, idx_outside));
    }

    #[test]
    fn test_remove_access_modifiers_with_leading_spaces() {
        let code = "    public func testFunction() { }";
        let processed = remove_access_modifiers(code);
        assert!(processed.starts_with("func testFunction()"),
            "Expected access modifiers to be stripped even with leading spaces, got: {}", processed);
    }

    #[test]
    fn test_extract_enclosing_block_complicated_swift() {
        const COMPLICATED_SWIFT: &str = r#"
import Foundation

public func unimportantFunction<T: Collection, U: Numeric>(
    input: T,
    transform: (T.Element) throws -> U
) async rethrows -> [U] where T.Element: CustomStringConvertible {
    print("This is not inside markers.")
    return try input.map { try transform($0) }
}

// v
// This content is included via substring markers.
public func importantFunction<T: Collection>(with data: T) async rethrows -> [T.Element] where T.Element: Numeric {
    print("This is inside markers.")
}
// ^

public func anotherUnimportantFunction<T: Decodable, U: Encodable>(
    input: T,
    transform: (T) throws -> U
) rethrows -> U {
    print("This is outside markers.")
    return try transform(input)
}

public func enclosingFunction<V: Equatable, W: Codable>(input: V) -> W? {
    print("This is not inside markers normally.")
    // TODO: - Correct the computation here
    print("Computation ends.")
    return nil
}
"#;
        let block = extract_enclosing_block(COMPLICATED_SWIFT);
        assert!(block.is_some(), "Expected an enclosing block to be extracted");
        let block_str = block.unwrap();
        assert!(block_str.contains("enclosingFunction"), "Expected block to contain 'enclosingFunction'");
        assert!(block_str.contains("TODO: - Correct the computation here"), "Expected block to contain the TODO marker");
        assert!(block_str.contains("Computation ends."), "Expected block to contain the final print statement");
    }
}
