use anyhow::{Result, Context};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;
use substring_marker_snippet_extractor::utils::marker_utils::{
    file_uses_markers, filter_substring_markers, is_todo_inside_markers,
};
use once_cell::sync::Lazy;

// Static regex definitions for candidate line detection.
static SWIFT_FUNCTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^\s*(?:(?:public|private|internal|fileprivate)\s+)?func\s+\w+(?:<[^>]+>)?\s*\([^)]*\)\s*(?:->\s*\S+)?\s*\{"#).unwrap()
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
// New static regex for Objective‑C method declarations (for one-line declarations).
static OBJC_METHOD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*[-+]\s*\([^)]*\)\s*[a-zA-Z_][a-zA-Z0-9_]*(?::\s*\([^)]*\)\s*[a-zA-Z_][a-zA-Z0-9_]*)*\s*\{").unwrap()
});

// Private helper: determines if a given line is a candidate declaration line.
fn is_candidate_line(line: &str) -> bool {
    SWIFT_FUNCTION_RE.is_match(line)
        || JS_ASSIGNMENT_RE.is_match(line)
        || JS_FUNCTION_RE.is_match(line)
        || PARSE_CLOUD_RE.is_match(line)
        || OBJC_METHOD_RE.is_match(line) // added to match Objective‑C methods on one line
}

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
        // Skip empty lines, import/include directives, or lines that are comments (unless they are trigger comments).
        if trimmed.is_empty()
            || trimmed.starts_with("import ")
            || trimmed.starts_with("#import")
            || trimmed.starts_with("#include")
            || (trimmed.starts_with("//") && !trimmed.starts_with("// TODO: -"))
        {
            return None;
        }
        // If it's a trigger comment, remove the prefix before processing.
        let content = if trimmed.starts_with("// TODO: -") {
            trimmed.trim_start_matches("// TODO: -").trim_start()
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

/// Reads a Swift file, extracts potential type names using two regexes,
/// and returns the sorted unique type names as a newline-separated String.
pub fn extract_types_from_file<P: AsRef<Path>>(swift_file: P) -> Result<String> {
    // Read the full file content.
    let full_content = fs::read_to_string(&swift_file)
        .with_context(|| format!("Failed to open file {}", swift_file.as_ref().display()))?;

    // If substring markers are used, filter the content to include only the desired parts.
    // Additionally, if the filtered content does not contain a TODO marker, append the enclosing block.
    let content_to_process = if file_uses_markers(&full_content) {
        let mut filtered = filter_substring_markers(&full_content, "");
        if !filtered.contains("// TODO: -") {
            if let Some(enclosing) = extract_enclosing_block_from_content(&full_content) {
                filtered.push_str("\n");
                filtered.push_str(&enclosing);
            }
        }
        filtered
    } else {
        full_content.clone()
    };

    let reader = BufReader::new(content_to_process.as_bytes());
    let extractor = TypeExtractor::new()?;
    let types = extractor.extract_types(reader.lines().filter_map(Result::ok));

    // Join the sorted unique type names into a single newline-separated string.
    let result = types.into_iter().collect::<Vec<String>>().join("\n");
    Ok(result)
}

/// Extracts the enclosing block (such as a function) from the provided content,
/// starting from the candidate line for a declaration up to the matching closing brace.
/// This block is expected to contain the TODO marker.
fn extract_enclosing_block_from_content(content: &str) -> Option<String> {
    let todo_idx = content.lines().position(|line| line.contains("// TODO: - "))?;
    if is_todo_inside_markers(content, todo_idx) {
        return None;
    }
    let lines: Vec<&str> = content.lines().collect();
    let mut candidate_index = None;
    // Look for a candidate line up to the TODO marker.
    for i in 0..todo_idx {
        let line = lines[i];
        if is_candidate_line(line) {
            candidate_index = Some(i);
        } else if line.trim_start().starts_with('-') || line.trim_start().starts_with('+') {
            // For Objective‑C methods split across lines, check if the next line contains '{'.
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use anyhow::Result;

    #[test]
    fn test_extract_types_returns_empty_for_file_with_no_capitalized_words() -> Result<()> {
        // Create a temporary Swift file with no capitalized words.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import foundation\nlet x = 5")?;
        let result = extract_types_from_file(swift_file.path())?;
        // Expect no types to be found.
        assert!(result.trim().is_empty());
        Ok(())
    }

    #[test]
    fn test_extract_types_extracts_capitalized_words() -> Result<()> {
        // Create a temporary Swift file with type declarations.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(
            swift_file,
            "import Foundation
class MyClass {{}}
struct MyStruct {{}}
enum MyEnum {{}}"
        )?;
        let result = extract_types_from_file(swift_file.path())?;
        // BTreeSet sorts alphabetically.
        let expected = "MyClass\nMyEnum\nMyStruct";
        assert_eq!(result.trim(), expected);
        Ok(())
    }

    #[test]
    fn test_extract_types_extracts_type_names_from_bracket_notation() -> Result<()> {
        // Create a Swift file using bracket notation.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import UIKit\nlet array: [CustomType] = []")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result.trim(), "CustomType");
        Ok(())
    }

    #[test]
    fn test_extract_types_deduplicates_type_names() -> Result<()> {
        // Create a file with duplicate declarations of the same type.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class DuplicateType {{}}")?;
        writeln!(swift_file, "struct DuplicateType {{}}")?;
        writeln!(swift_file, "enum DuplicateType {{}}")?;
        let result = extract_types_from_file(swift_file.path())?;
        // Only one instance should appear.
        assert_eq!(result.trim(), "DuplicateType");
        Ok(())
    }

    #[test]
    fn test_extract_types_mixed_tokens_in_one_line() -> Result<()> {
        // Create a file with multiple declarations separated by punctuation.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class MyClass, struct MyStruct; enum MyEnum.")?;
        let result = extract_types_from_file(swift_file.path())?;
        // The tokens should be split correctly and sorted alphabetically.
        let expected = "MyClass\nMyEnum\nMyStruct";
        assert_eq!(result.trim(), expected);
        Ok(())
    }

    #[test]
    fn test_extract_types_with_underscores() -> Result<()> {
        // Create a file where a type name includes an underscore.
        // The preprocessing replaces non-alphanumeric characters with spaces,
        // so "My_Class" should not appear as a single token.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class My_Class {{}}")?;
        let result = extract_types_from_file(swift_file.path())?;
        // Ensure that the token "My_Class" does not appear.
        for token in result.lines() {
            assert_ne!(token, "My_Class", "Found token 'My_Class', which should have been split.");
        }
        Ok(())
    }

    #[test]
    fn test_extract_types_trailing_punctuation() -> Result<()> {
        // Create a file where the type declaration is followed by punctuation.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "enum MyEnum.")?;
        let result = extract_types_from_file(swift_file.path())?;
        // The trailing punctuation should be removed.
        assert_eq!(result.trim(), "MyEnum");
        Ok(())
    }

    #[test]
    fn test_extract_types_includes_trigger_comment() -> Result<()> {
        // Create a temporary Swift file with a trigger comment.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import Foundation\n// TODO: - TriggeredType")?;
        let result = extract_types_from_file(swift_file.path())?;
        // Expect that only "TriggeredType" is extracted.
        assert_eq!(result.trim(), "TriggeredType");
        Ok(())
    }

    #[test]
    fn test_extract_types_with_substring_markers() -> Result<()> {
        // Create a temporary Swift file with substring markers.
        // Only the type declaration between "// v" and "// ^" should be considered.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class OutsideType {{}}")?;
        writeln!(swift_file, "// v")?;
        writeln!(swift_file, "class InsideType {{}}")?;
        writeln!(swift_file, "// ^")?;
        writeln!(swift_file, "class OutsideType2 {{}}")?;
        let result = extract_types_from_file(swift_file.path())?;
        // Expect only "InsideType" to be extracted.
        assert_eq!(result.trim(), "InsideType");
        Ok(())
    }

    #[test]
    fn test_extract_types_with_markers_and_enclosing_todo() -> Result<()> {
        // The Swift file content includes:
        // - A marked section (between "// v" and "// ^") containing a call that yields "TypeThatIsInsideMarker".
        // - Outside the markers, a declaration "TypeThatIsOutSideMarker" which should be ignored.
        // - A function with a TODO marker that contains "TypeThatIsInsideEnclosingFunction", which should be included.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, r#"
            // v
            let foo = TypeThatIsInsideMarker()
            // ^
            
            let bar = TypeThatIsOutSideMarker()
            
            func hello() {{
                let hi = TypeThatIsInsideEnclosingFunction()
                // TODO: - example
            }}
        "#)?;
        let result = extract_types_from_file(swift_file.path())?;
        // Expected output: both types are extracted and sorted alphabetically.
        // "TypeThatIsInsideEnclosingFunction" comes before "TypeThatIsInsideMarker".
        let expected = "TypeThatIsInsideEnclosingFunction\nTypeThatIsInsideMarker";
        assert_eq!(result.trim(), expected);
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
        // Simply verify that an instance can be created.
        assert!(extractor.re_simple.is_match("MyType"));
    }

    #[test]
    fn test_extract_tokens_returns_none_for_empty_or_non_eligible_lines() {
        let extractor = TypeExtractor::new().unwrap();
        // Empty or whitespace-only lines return None.
        assert!(extractor.extract_tokens("").is_none());
        assert!(extractor.extract_tokens("   ").is_none());
        // Lines that start with "import " or regular comments should be skipped.
        assert!(extractor.extract_tokens("import Foundation").is_none());
        assert!(extractor.extract_tokens("// comment").is_none());
    }

    #[test]
    fn test_extract_tokens_splits_and_cleans_input() {
        let extractor = TypeExtractor::new().unwrap();
        // Input with punctuation: non-alphanumeric chars become spaces.
        let tokens = extractor.extract_tokens("MyClass,struct MyStruct").unwrap();
        // "MyClass,struct MyStruct" becomes "MyClass struct MyStruct", then splits into tokens.
        assert_eq!(tokens, vec!["MyClass", "struct", "MyStruct"]);
    }

    // New test: Ensure that trigger comments are processed correctly.
    #[test]
    fn test_extract_tokens_for_trigger_comment() {
        let extractor = TypeExtractor::new().unwrap();
        // For a trigger comment, the prefix should be stripped and only the remainder processed.
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

    // Test that the new OBJC_METHOD_RE regex correctly recognizes a one‐line Objective‑C method declaration.
    #[test]
    fn test_is_candidate_line_objc_method_single_line() {
        let objc_line = "- (void)MyObjCMethod {";
        // The line should be recognized as a candidate declaration.
        assert!(is_candidate_line(objc_line));
    }

    // Test that an Objective‑C method declaration split across lines is recognized.
    #[test]
    fn test_extract_enclosing_block_with_objc_method_split_lines() {
        // Simulate a file content where an Objective‑C method declaration is split:
        // The declaration is on one line without the opening brace and the next line contains the brace.
        let content = "\
- (void)MyObjCMethod\n\
{\n\
    // method implementation\n\
}\n\
void anotherFunction() {\n\
    // TODO: - Fix issue\n\
}";
        // Since the TODO marker appears later, extract the enclosing block for the TODO.
        // The candidate should be selected based on the split Objective‑C declaration.
        let block = extract_enclosing_block_from_content(content);
        assert!(block.is_some());
        let block_str = block.unwrap();
        // The extracted block should include the Objective‑C declaration parts.
        assert!(block_str.contains("- (void)MyObjCMethod"));
        assert!(block_str.contains("{"));
    }

    // Test that the candidate line is chosen from an Objective‑C method when it appears as a one‐line declaration.
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
        // Ensure the block contains the Objective‑C method declaration.
        assert!(block_str.contains("- (void)MyObjCMethod {"));
    }
}
