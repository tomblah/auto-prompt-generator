use anyhow::{Context, Result};
use std::collections::BTreeSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use tree_sitter::Parser;

// Ensure you have the following dependency in your Cargo.toml:
// tree-sitter-swift = "0.7.0"
extern crate tree_sitter_swift;

/// Recursively traverses the syntax tree starting at `node`, and if the node is one of the
/// target type declarations ("class_declaration", "struct_declaration", "enum_declaration",
/// "protocol_declaration", or "typealias_declaration"), it attempts to extract the identifier child.
fn extract_nodes(node: tree_sitter::Node, source: &[u8], types: &mut BTreeSet<String>) {
    match node.kind() {
        "class_declaration"
        | "struct_declaration"
        | "enum_declaration"
        | "protocol_declaration"
        | "typealias_declaration" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                if child.kind() == "identifier" {
                    if let Ok(text) = child.utf8_text(source) {
                        types.insert(text.to_string());
                    }
                }
            }
        }
        _ => {}
    }
    // Recurse into children.
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        extract_nodes(child, source, types);
    }
}

/// Uses Tree-sitter to parse the Swift source and extract type names.
fn extract_types_tree_sitter(source: &str) -> BTreeSet<String> {
    let mut types = BTreeSet::new();
    let mut parser = Parser::new();
    parser
        .set_language(unsafe {
            *(&tree_sitter_swift::LANGUAGE as *const _ as *const tree_sitter::Language)
        })
        .expect("Error loading Swift grammar");
    if let Some(tree) = parser.parse(source, None) {
        let root_node = tree.root_node();
        extract_nodes(root_node, source.as_bytes(), &mut types);
    }
    types
}

/// Reads a Swift file, extracts type names using Tree-sitter, writes the sorted unique type names
/// to a temporary file (persisted), and returns the path to that file as a String.
pub fn extract_types_from_file<P: AsRef<Path>>(swift_file: P) -> Result<String> {
    // Read the entire Swift file.
    let source = fs::read_to_string(&swift_file)
        .with_context(|| format!("Failed to open file {}", swift_file.as_ref().display()))?;
    // Extract types with Tree-sitter.
    let types = extract_types_tree_sitter(&source);
    // Write the sorted type names to a temporary file.
    let mut temp_file = NamedTempFile::new()?;
    for type_name in &types {
        writeln!(temp_file, "{}", type_name)?;
    }
    // Persist the temporary file so it isn’t deleted on drop.
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
