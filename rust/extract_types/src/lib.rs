use anyhow::{Result, Context};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

/// Reads a Swift file, extracts potential type names using two regexes,
/// writes the sorted unique type names to a temporary file (persisted),
/// and returns the path to that file as a String.
pub fn extract_types_from_file<P: AsRef<Path>>(swift_file: P) -> Result<String> {
    // Open the Swift file.
    let file = File::open(&swift_file)
        .with_context(|| format!("Failed to open file {}", swift_file.as_ref().display()))?;
    let reader = BufReader::new(file);

    // Regex to match tokens that start with a capital letter.
    let re_simple = Regex::new(r"^[A-Z][A-Za-z0-9]+$")?;
    // Regex to match tokens in bracket notation (e.g. [TypeName]).
    let re_bracket = Regex::new(r"^\[([A-Z][A-Za-z0-9]+)\]$")?;

    // Use a BTreeSet to store unique type names (sorted alphabetically).
    let mut types = BTreeSet::new();

    for line in reader.lines() {
        let mut line = line?;
        // Preprocessing: replace non-alphanumeric characters with whitespace.
        line = line.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
            .collect();
        let line = line.trim();

        // Skip empty lines or lines starting with "import " or "//".
        if line.is_empty() || line.starts_with("import ") || line.starts_with("//") {
            continue;
        }

        // Split the line into tokens and check each one.
        for token in line.split_whitespace() {
            if re_simple.is_match(token) {
                types.insert(token.to_string());
            } else if let Some(caps) = re_bracket.captures(token) {
                if let Some(inner) = caps.get(1) {
                    types.insert(inner.as_str().to_string());
                }
            }
        }
    }

    // Write the sorted type names to a temporary file.
    let mut temp_file = NamedTempFile::new()?;
    for type_name in &types {
        writeln!(temp_file, "{}", type_name)?;
    }

    // Persist the temporary file so it won't be deleted when dropped.
    let temp_path: PathBuf = temp_file.into_temp_path()
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
        // Optionally, you could check for the presence of the split tokens "My" and "Class".
        // (Depending on your intended behavior, you might want to adjust the extraction logic.)
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
}
