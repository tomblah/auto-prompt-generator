use anyhow::{Result, Context};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::io::{Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

// New imports from substring_marker_snippet_extractor for marker checking and filtering.
use substring_marker_snippet_extractor::{file_uses_markers, filter_substring_markers_with_placeholder};

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
        // Skip empty lines, import lines, or lines that are comments (unless they are trigger comments).
        if trimmed.is_empty()
            || trimmed.starts_with("import ")
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
/// writes the sorted unique type names to a temporary file (persisted),
/// and returns the path to that file as a String.
pub fn extract_types_from_file<P: AsRef<Path>>(swift_file: P) -> Result<String> {
    // Read the entire file content.
    let file_content = fs::read_to_string(&swift_file)
        .with_context(|| format!("Failed to read file {}", swift_file.as_ref().display()))?;
    
    // If the file uses substring markers, extract only the marked content.
    // Passing an empty placeholder string means no placeholder text will be inserted.
    let processed_content = if file_uses_markers(&file_content) {
        filter_substring_markers_with_placeholder(&file_content, "")
    } else {
        file_content
    };

    // Create an iterator over the lines of the processed content.
    let lines = processed_content.lines().map(String::from);

    // Create the extractor and process the file lines.
    let extractor = TypeExtractor::new()?;
    let types = extractor.extract_types(lines);

    // Write the sorted type names to a temporary file.
    let mut temp_file = NamedTempFile::new()?;
    for type_name in &types {
        writeln!(temp_file, "{}", type_name)?;
    }

    // Persist the temporary file so it won't be deleted when dropped.
    let temp_path: PathBuf = temp_file
        .into_temp_path()
        .keep()
        .context("Failed to persist temporary file")?;
    Ok(temp_path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;
    use anyhow::Result;

    #[test]
    fn test_extract_types_returns_empty_for_file_with_no_capitalized_words() -> Result<()> {
        // Create a temporary Swift file with no capitalized words.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import foundation\nlet x = 5")?;
        let result_path = extract_types_from_file(swift_file.path())?;
        let result = fs::read_to_string(&result_path)?;
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
        let result_path = extract_types_from_file(swift_file.path())?;
        let result = fs::read_to_string(&result_path)?;
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
        let result_path = extract_types_from_file(swift_file.path())?;
        let result = fs::read_to_string(&result_path)?;
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
        let result_path = extract_types_from_file(swift_file.path())?;
        let result = fs::read_to_string(&result_path)?;
        // Only one instance should appear.
        assert_eq!(result.trim(), "DuplicateType");
        Ok(())
    }

    #[test]
    fn test_extract_types_mixed_tokens_in_one_line() -> Result<()> {
        // Create a file with multiple declarations separated by punctuation.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class MyClass, struct MyStruct; enum MyEnum.")?;
        let result_path = extract_types_from_file(swift_file.path())?;
        let result = fs::read_to_string(&result_path)?;
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
        let result_path = extract_types_from_file(swift_file.path())?;
        let result = fs::read_to_string(&result_path)?;
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
        let result_path = extract_types_from_file(swift_file.path())?;
        let result = fs::read_to_string(&result_path)?;
        // The trailing punctuation should be removed.
        assert_eq!(result.trim(), "MyEnum");
        Ok(())
    }

    // New test: Ensure that types in trigger comments (// TODO: -) are extracted.
    #[test]
    fn test_extract_types_includes_trigger_comment() -> Result<()> {
        // Create a temporary Swift file with a trigger comment.
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import Foundation\n// TODO: - TriggeredType")?;
        let result_path = extract_types_from_file(swift_file.path())?;
        let result = fs::read_to_string(&result_path)?;
        // Expect that only "TriggeredType" is extracted.
        assert_eq!(result.trim(), "TriggeredType");
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
