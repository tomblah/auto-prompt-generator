// File: rust/extract-enclosing-type/src/main.rs

use regex::Regex;
use std::env;
use std::fs;
use std::path::Path;

/// Extracts the enclosing type (class, struct, or enum) from a Swift file.
/// Scans until a line containing "// TODO: - " is encountered.
/// Returns the last encountered type name or, if none is found,
/// falls back to the file’s basename (without the .swift extension).
fn extract_enclosing_type(file_path: &str) -> String {
    // Read the entire file content.
    let content = fs::read_to_string(file_path)
        .unwrap_or_else(|err| panic!("Error reading file {}: {}", file_path, err));

    // Regex to match "class", "struct", or "enum" followed by whitespace and then a word.
    let re = Regex::new(r"(class|struct|enum)\s+(\w+)").unwrap();

    let mut last_type: Option<String> = None;

    for line in content.lines() {
        // If we hit the TODO marker, stop processing.
        if line.contains("// TODO: -") {
            break;
        }
        // Look for a type declaration in the line.
        if let Some(caps) = re.captures(line) {
            if let Some(type_name) = caps.get(2) {
                // Update the last seen type.
                last_type = Some(type_name.as_str().to_string());
            }
        }
    }

    // If a type was found, return it. Otherwise, use the file basename (without .swift).
    if let Some(found_type) = last_type {
        found_type
    } else {
        let path = Path::new(file_path);
        // Use file_stem to strip the extension.
        path.file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string()
    }
}

fn main() {
    // Expect exactly one argument: the path to the Swift file.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <swift_file>", args[0]);
        std::process::exit(1);
    }
    let file_path = &args[1];
    let result = extract_enclosing_type(file_path);
    println!("{}", result);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_from_file_with_type_before_todo() {
        let content = "\
class MyAwesomeClass {
    // Some code here
}
// Another type definition
struct HelperStruct {
    // TODO: - Implement something
}";
        // Write the content to a temporary file.
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("Test.swift");
        fs::write(&file_path, content).unwrap();

        // Expect that the last type encountered before the TODO is "HelperStruct".
        let extracted = extract_enclosing_type(file_path.to_str().unwrap());
        assert_eq!(extracted, "HelperStruct");
    }

    #[test]
    fn test_extract_fallback_to_basename() {
        let content = "\
func doSomething() {
    // Some code here
}
// No type declaration before TODO:
 // TODO: - Fix something";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("FallbackTest.swift");
        fs::write(&file_path, content).unwrap();

        // Since no type was found, it should fall back to "FallbackTest".
        let extracted = extract_enclosing_type(file_path.to_str().unwrap());
        assert_eq!(extracted, "FallbackTest");
    }
}
