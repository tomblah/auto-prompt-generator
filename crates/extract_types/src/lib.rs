// crates/extract_types/src/lib.rs

use anyhow::{Result, Context};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
// Import necessary items from the shared utility crate
use substring_marker_snippet_extractor::utils::marker_utils::{
    TODO_MARKER, // Use the constant
    file_uses_markers, // Keep this helper
    filter_substring_markers, // Keep this helper
    extract_enclosing_block_around_todo, // Use the new consolidated helper
    extract_inner_block_from_content, // Keep this helper (used for targeted mode)
};
use once_cell::sync::Lazy;

// Updated regex to match Swift function declarations that may include "async".
// These regexes are now used internally by the consolidated helper, but keeping
// them here for TypeExtractor's potential needs or if the helper's regexes
// were not fully comprehensive and needed to be passed in.
// For this refactor, we will rely on the regexes inside extract_enclosing_block_around_todo.
/*
static SWIFT_FUNCTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^\s*(?:(?:public|private|internal|fileprivate)\s+)?func\s+\w+(?:<[^>]+>)?\s*\([^)]*\)\s*(?:->\s*\S+)?(?:\s+async)?\s*\{"
    ).unwrap()
});
// Existing regexes for JS and Objective-C.
static JS_ASSIGNMENT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:(?:const|var|let)\s+)?\w+\s*=\s*function\s*\([^)]*\)\s*\{").unwrap()
});
static JS_FUNCTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?:async\s+)?function\s+\w+\s*\([^)]*\)\s*\{").unwrap()
});
static PARSE_CLOUD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"^\s*Parse\.Cloud\.(?:define|beforeSave|afterSave)\s*\(\s*(?:"[^"]+"|[A-Za-z][A-Za-z0-9_.]*)\s*,\s*(?:async\s+)?\([^)]*\)\s*=>\s*\{"
    )
    .unwrap()
});
static OBJC_METHOD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*[-+]\s*\([^)]*\)\s*[a-zA-Z_][a-zA-Z0-9_]*(?::\s*\([^)]*\)\s*[a-zA-Z_][a-zA-Z0-9_]*)*\s*\{").unwrap()
});
// New regex to match Swift class declarations.
static SWIFT_CLASS_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*class\s+\w+.*\{").unwrap()
});
// New regex to match Swift enum declarations.
static SWIFT_ENUM_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*enum\s+\w+.*\{").unwrap()
});
*/

/// Updated helper function to determine if a line is a candidate declaration.
/// This function is no longer needed here as the logic is consolidated
/// in `extract_enclosing_block_around_todo`.
/*
fn is_candidate_line(line: &str) -> bool {
    SWIFT_FUNCTION_RE.is_match(line)
        || JS_ASSIGNMENT_RE.is_match(line)
        || JS_FUNCTION_RE.is_match(line)
        || PARSE_CLOUD_RE.is_match(line)
        || OBJC_METHOD_RE.is_match(line)
        || SWIFT_CLASS_RE.is_match(line)
        || SWIFT_ENUM_RE.is_match(line)
}
*/

/// A helper struct to encapsulate type extraction logic.
struct TypeExtractor {
    re_simple: Regex,
    re_bracket: Regex,
}

impl TypeExtractor {
    /// Creates a new `TypeExtractor` with precompiled regexes.
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            re_simple: Regex::new(r"^[A-Z][A-Za-z0-9]+$")?,
            re_bracket: Regex::new(r"^\[([A-Z][A-Za-z0-9]+)\]$")?,
        })
    }

    /// Cleans a line by replacing non-alphanumeric characters with whitespace,
    /// trimming it, and then splitting it into tokens.
    /// Returns `None` if the cleaned line is empty or starts with "import " or a comment
    /// (unless it starts with the trigger comment "// TODO: -").
    fn extract_tokens(&self, line: &str) -> Option<Vec<String>> {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("import ")
            || trimmed.starts_with("#import")
            || trimmed.starts_with("#include")
            // Use the imported constant for the TODO check
            || (trimmed.starts_with("//") && !trimmed.starts_with(TODO_MARKER))
        {
            return None;
        }
        // Use the imported constant for the TODO check
        let content = if trimmed.starts_with(TODO_MARKER) {
            trimmed.trim_start_matches(TODO_MARKER).trim_start()
        } else {
            trimmed
        };
        let cleaned: String = content
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let cleaned = cleaned.trim();
        Some(cleaned.split_whitespace().map(String::from).collect())
    }

    /// Processes an iterator over lines and returns a sorted set of type names.
    fn extract_types<I>(&self, lines: I) -> BTreeSet<String>
    where
        I: Iterator<Item = String>,
    {
        let mut types = BTreeSet::new();
        for line in lines {
            if let Some(tokens) = self.extract_tokens(&line) {
                for token in tokens {
                    if self.re_simple.is_match(&token) {
                        types.insert(token);
                    } else if let Some(caps) = self.re_bracket.captures(&token) {
                        if let Some(inner) = caps.get(1) {
                            types.insert(inner.as_str().to_string());
                        }
                    }
                }
            }
        }
        types
    }
}

/// Reads a Swift file, extracts potential type names,
/// and returns the sorted unique type names as a newline-separated String.
/// The extraction logic depends on the `TARGETED` environment variable and
/// the presence of substring markers (`// v`, `// ^`).
///
/// # Parameters
/// * `swift_file` - The file to process.
pub fn extract_types_from_file<P: AsRef<Path>>(swift_file: P) -> Result<String> {
    let full_content = fs::read_to_string(&swift_file)
        .with_context(|| format!("Failed to open file {}", swift_file.as_ref().display()))?;

    let targeted = std::env::var("TARGETED").is_ok();

    let content_to_process = if targeted {
        // Targeted mode: Extract types from the inner block immediately enclosing the TODO,
        // or the full content if no inner block is found.
        extract_inner_block_from_content(&full_content).unwrap_or_else(|| full_content.clone())
    } else {
        // Non-targeted mode:
        // Start with either filtered content (if markers used) or full content.
        let mut base_content = if file_uses_markers(&full_content) {
            filter_substring_markers(&full_content, "")
        } else {
            full_content.clone()
        };

        // Attempt to find and append the enclosing code block around the TODO marker.
        // This helper internally checks if the TODO is inside markers.
        if let Some(enclosing) = extract_enclosing_block_around_todo(&full_content) {
            // If an enclosing block is found, append it to the content being processed.
            base_content.push_str("\n"); // Add a separator
            base_content.push_str(&enclosing);
        }
        // The final content to process is the base content (filtered or full)
        // plus the enclosing block if found.
        base_content
    };

    let reader = BufReader::new(content_to_process.as_bytes());
    let extractor = TypeExtractor::new()?;
    let types = extractor.extract_types(reader.lines().filter_map(Result::ok));

    let result = types.into_iter().collect::<Vec<String>>().join("\n");
    Ok(result)
}

/// Extracts the enclosing block (such as a function) from the provided content,
/// starting from the candidate line for a declaration up to the matching closing brace.
/// This block is expected to contain the TODO marker.
/// This is the original helper used when not in targeted mode, now replaced
/// by the consolidated `extract_enclosing_block_around_todo`.
/*
fn extract_enclosing_block_from_content(content: &str) -> Option<String> {
    let todo_idx = content.lines().position(|line| line.contains(TODO_MARKER))?; // Use constant
    if is_todo_inside_markers(content, todo_idx) {
        return None;
    }
    let lines: Vec<&str> = content.lines().collect();
    let mut candidate_index = None;
    for i in 0..todo_idx {
        let line = lines[i];
        if is_candidate_line(line) {
            candidate_index = Some(i);
        } else if line.trim_start().starts_with('-') || line.trim_start().starts_with('+') {
            if i + 1 < todo_idx && lines[i + 1].contains('{') {
                candidate_index = Some(i);
            }
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
*/

/// Extracts the inner block—that is, the content inside the braces that immediately enclose
/// the first occurrence of the TODO marker. This is done using a stack-based approach
/// to correctly identify the innermost unclosed '{'.
/// (This function is kept as it performs a different task than extracting the *enclosing* declaration block).
/*
fn extract_inner_block_from_content(content: &str) -> Option<String> {
    let todo_marker = TODO_MARKER; // Use constant
    let pos = content.find(todo_marker)?;
    let mut stack = Vec::new();
    // Process characters up to the TODO marker.
    for (i, ch) in content[..pos].char_indices() {
        if ch == '{' {
            stack.push(i);
        } else if ch == '}' {
            stack.pop();
        }
    }
    let open_brace = stack.pop()?;
    // Now find the matching closing brace.
    let mut brace_count = 1;
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
    if brace_count == 0 {
        Some(content[open_brace + 1..index - 1].to_string())
    } else {
        None
    }
}
*/

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use anyhow::Result;
    use std::env;

    #[test]
    fn test_extract_inner_block_success() {
        let content = r#"
            func foo() {
                class OuterType {}
                {
                    class InnerType {}
                    // TODO: - Do something important
                }
            }
        "#;
        let inner = extract_inner_block_from_content(content);
        assert!(inner.is_some());
        let inner_str = inner.unwrap();
        // Ensure that the extracted inner block contains the inner declaration and the TODO marker,
        // and that it does not include "OuterType".
        assert!(inner_str.contains("class InnerType"), "Extracted block: {}", inner_str);
        assert!(inner_str.contains("// TODO: -"), "Extracted block: {}", inner_str);
        assert!(!inner_str.contains("OuterType"), "Extracted block should not contain OuterType: {}", inner_str);
    }

    #[test]
    fn test_extract_inner_block_no_marker() {
        let content = "func foo() { class InnerType {} }";
        assert!(extract_inner_block_from_content(content).is_none());
    }

    #[test]
    fn test_extract_types_returns_empty_for_file_with_no_capitalized_words() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import foundation\nlet x = 5")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert!(result.trim().is_empty());
        Ok(())
    }

    #[test]
    fn test_extract_types_extracts_capitalized_words() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import Foundation\nclass MyClass {{}}\nstruct MyStruct {{}}\nenum MyEnum {{}}")?;
        let result = extract_types_from_file(swift_file.path())?;
        let expected = "MyClass\nMyEnum\nMyStruct";
        assert_eq!(result.trim(), expected);
        Ok(())
    }

    #[test]
    fn test_extract_types_extracts_type_names_from_bracket_notation() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import UIKit\nlet array: [CustomType] = []")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result.trim(), "CustomType");
        Ok(())
    }

    #[test]
    fn test_extract_types_deduplicates_type_names() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class DuplicateType {{}}")?;
        writeln!(swift_file, "struct DuplicateType {{}}")?;
        writeln!(swift_file, "enum DuplicateType {{}}")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result.trim(), "DuplicateType");
        Ok(())
    }

    #[test]
    fn test_extract_types_mixed_tokens_in_one_line() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class MyClass, struct MyStruct; enum MyEnum.")?;
        let result = extract_types_from_file(swift_file.path())?;
        let expected = "MyClass\nMyEnum\nMyStruct";
        assert_eq!(result.trim(), expected);
        Ok(())
    }

    #[test]
    fn test_extract_types_with_underscores() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class My_Class {{}}")?;
        let result = extract_types_from_file(swift_file.path())?;
        for token in result.lines() {
            assert_ne!(token, "My_Class", "Found token 'My_Class', which should have been split.");
        }
        Ok(())
    }

    #[test]
    fn test_extract_types_trailing_punctuation() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "enum MyEnum.")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result.trim(), "MyEnum");
        Ok(())
    }

    #[test]
    fn test_extract_types_includes_trigger_comment() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import Foundation\n// TODO: - TriggeredType")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result.trim(), "TriggeredType");
        Ok(())
    }

    #[test]
    fn test_extract_types_with_substring_markers() -> Result<()> {
        let swift_content = r#"
            // This type is outside the markers and should be ignored.
            class OutsideType {}
            // v
            // Only this section should be processed:
            class InsideType {}
            // ^
            // This type is also outside and should be ignored.
            class OutsideType2 {}
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file(swift_file.path())?;
        // Expect only the content between markers to yield "InsideType"
        let expected = "InsideType";
        assert_eq!(result.trim(), expected);
        Ok(())
    }

    #[test]
    fn test_extract_types_with_markers_and_enclosing_todo() -> Result<()> {
        let swift_content = r#"
            // v
            let foo = TypeThatIsInsideMarker()
            // ^
            
            let bar = TypeThatIsOutSideMarker()
            
            func hello() {
                let hi = TypeThatIsInsideEnclosingFunction()
                // TODO: - example
            }
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file(swift_file.path())?;
        // Expected output: both types extracted and sorted alphabetically.
        // "TypeThatIsInsideEnclosingFunction" comes before "TypeThatIsInsideMarker".
        let expected = "TypeThatIsInsideEnclosingFunction\nTypeThatIsInsideMarker";
        assert_eq!(result.trim(), expected);
        Ok(())
    }

    #[test]
    fn test_extract_types_targeted_mode() -> Result<()> {
        env::set_var("TARGETED", "1");
        let swift_content = r#"
            class OuterType {}
            func testFunction() {
                class InnerType {}
                // TODO: - Perform action
            }
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file(swift_file.path())?;
        // In targeted mode, from the function block:
        // - "class InnerType {}" produces "InnerType"
        // - The trigger comment yields "Perform" (ignoring "action" since it's lowercase)
        let expected = "InnerType\nPerform";
        assert_eq!(result.trim(), expected);
        env::remove_var("TARGETED");
        Ok(())
    }

    #[test]
    fn test_extract_types_targeted_mode_no_enclosing_block() -> Result<()> {
        env::set_var("TARGETED", "1");
        let swift_content = r#"
            class OuterType {}
            // TODO: - Some todo
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file(swift_file.path())?;
        // Expect "OuterType" and "Some"
        let expected = "OuterType\nSome";
        assert_eq!(result.trim(), expected);
        env::remove_var("TARGETED");
        Ok(())
    }
}

#[cfg(test)]
mod type_extractor_tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_type_extractor_new() {
        let extractor = TypeExtractor::new().expect("Failed to create TypeExtractor");
        assert!(extractor.re_simple.is_match("MyType"));
    }

    #[test]
    fn test_extract_tokens_returns_none_for_empty_or_non_eligible_lines() {
        let extractor = TypeExtractor::new().unwrap();
        assert!(extractor.extract_tokens("").is_none());
        assert!(extractor.extract_tokens("   ").is_none());
        assert!(extractor.extract_tokens("import Foundation").is_none());
        assert!(extractor.extract_tokens("// comment").is_none());
    }

    #[test]
    fn test_extract_tokens_splits_and_cleans_input() {
        let extractor = TypeExtractor::new().unwrap();
        let tokens = extractor.extract_tokens("MyClass,struct MyStruct").unwrap();
        assert_eq!(tokens, vec!["MyClass", "struct", "MyStruct"]);
    }

    #[test]
    fn test_extract_tokens_for_trigger_comment() {
        let extractor = TypeExtractor::new().unwrap();
        let tokens = extractor.extract_tokens("// TODO: - MyTriggeredType").unwrap();
        assert_eq!(tokens, vec!["MyTriggeredType"]);
    }

    #[test]
    fn test_extract_types_basic() {
        let extractor = TypeExtractor::new().unwrap();
        let lines = vec![
            "class MyClass {}".to_string(),
            "struct MyStruct {}".to_string(),
            "enum MyEnum {}".to_string(),
        ];
        let types = extractor.extract_types(lines.into_iter());
        let expected: BTreeSet<String> = ["MyClass", "MyEnum", "MyStruct"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(types, expected);
    }

    #[test]
    fn test_extract_types_with_bracket_notation() {
        let extractor = TypeExtractor::new().unwrap();
        let lines = vec!["let array: [CustomType] = []".to_string()];
        let types = extractor.extract_types(lines.into_iter());
        let expected: BTreeSet<String> = ["CustomType"].iter().map(|s| s.to_string()).collect();
        assert_eq!(types, expected);
    }

    #[test]
    fn test_extract_types_mixed_tokens() {
        let extractor = TypeExtractor::new().unwrap();
        let lines = vec!["class MyClass, struct MyStruct; enum MyEnum.".to_string()];
        let types = extractor.extract_types(lines.into_iter());
        let expected: BTreeSet<String> = ["MyClass", "MyEnum", "MyStruct"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(types, expected);
    }

    #[test]
    fn test_extract_types_deduplication() {
        let extractor = TypeExtractor::new().unwrap();
        let lines = vec![
            "class DuplicateType {}".to_string(),
            "struct DuplicateType {}".to_string(),
            "enum DuplicateType {}".to_string(),
        ];
        let types = extractor.extract_types(lines.into_iter());
        let expected: BTreeSet<String> = ["DuplicateType"].iter().map(|s| s.to_string()).collect();
        assert_eq!(types, expected);
    }
}

#[cfg(test)]
mod objc_tests {
    use super::*;

    #[test]
    fn test_is_candidate_line_objc_method_single_line() {
        let objc_line = "- (void)MyObjCMethod {";
        assert!(is_candidate_line(objc_line));
    }

    #[test]
    fn test_extract_enclosing_block_with_objc_method_split_lines() {
        let content = "\
- (void)MyObjCMethod\n\
{\n\
    // method implementation\n\
}\n\
void anotherFunction() {\n\
    // TODO: - Fix issue\n\
}";
        let block = extract_enclosing_block_from_content(content);
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)MyObjCMethod"));
        assert!(block_str.contains("{"));
    }

    #[test]
    fn test_extract_enclosing_block_with_objc_method_single_line() {
        let content = "\
- (void)MyObjCMethod { // implementation start\n\
    // method body\n\
}\n\
void someFunction() {\n\
    // TODO: - Address bug\n\
}";
        let block = extract_enclosing_block_from_content(content);
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)MyObjCMethod {"));
    }
}

#[cfg(test)]
mod candidate_detection_tests {
    use super::*;

    #[test]
    fn test_candidate_line_swift_function_async() {
        let async_func = "func testAsyncFunction(foo: Int) async {";
        assert!(is_candidate_line(async_func), "Swift async function should be detected as a candidate");
    }

    #[test]
    fn test_candidate_line_swift_class() {
        let swift_class = "class MyInnerClass {";
        assert!(is_candidate_line(swift_class), "Swift class declaration should be detected as a candidate");
    }

    #[test]
    fn test_candidate_line_swift_enum() {
        let swift_enum = "enum MyEnum {";
        assert!(is_candidate_line(swift_enum), "Swift enum declaration should be detected as a candidate");
    }

    #[test]
    fn test_candidate_line_non_candidate() {
        let non_candidate = "let x = 10;";
        assert!(!is_candidate_line(non_candidate), "Non-declaration line should not be detected as a candidate");
    }
}
