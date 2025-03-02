// rust/extract_enclosing_type/src/lib.rs

use regex::Regex;
use std::fs;
use std::path::Path;

/// Extracts the enclosing type (class, struct, or enum) from a Swift file.
/// Scans until a line containing "// TODO: - " is encountered.
/// Returns the last encountered type name or, if none is found,
/// falls back to the file’s basename (without the .swift extension).
pub fn extract_enclosing_type(file_path: &str) -> Result<String, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|err| format!("Error reading file {}: {}", file_path, err))?;

    let re = Regex::new(r"(class|struct|enum)\s+(\w+)")
        .map_err(|err| format!("Regex error: {}", err))?;

    let mut last_type: Option<String> = None;

    for line in content.lines() {
        if line.contains("// TODO: -") {
            break;
        }
        if let Some(caps) = re.captures(line) {
            if let Some(type_name) = caps.get(2) {
                last_type = Some(type_name.as_str().to_string());
            }
        }
    }

    if let Some(found_type) = last_type {
        Ok(found_type)
    } else {
        let path = Path::new(file_path);
        let fallback = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        Ok(fallback)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile;

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
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
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
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "FallbackTest");
    }

    #[test]
    fn test_ignore_types_after_todo() {
        let content = "\
class EarlyClass {
    // Some code here
}
// TODO: - Do something
struct LateStruct {
    // Some code here
}";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("AfterTodo.swift");
        fs::write(&file_path, content).unwrap();

        // Should return "EarlyClass" because the type after the TODO is ignored.
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "EarlyClass");
    }

    #[test]
    fn test_empty_file_fallback() {
        let content = "";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("Empty.swift");
        fs::write(&file_path, content).unwrap();

        // With no content, it should fallback to "Empty".
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "Empty");
    }

    // --- Additional Tests ---

    #[test]
    fn test_no_todo_marker_returns_last_type() {
        // Test a file with several type declarations but without any TODO marker.
        let content = "\
class FirstClass {
    // Some code here
}
struct SecondStruct {
    // Some code here
}
enum ThirdEnum {
    // Some code here
}";
        // Expect the last type ("ThirdEnum") to be returned.
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("NoTodo.swift");
        fs::write(&file_path, content).unwrap();

        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "ThirdEnum");
    }

    #[test]
    fn test_type_on_same_line_as_todo_marker() {
        // Test a file where the type declaration appears on the same line as the TODO marker.
        // In this case the line is skipped before updating the last_type, and the function should fall back to the file's basename.
        let content = "class MyClass { // TODO: - Do something important }";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("SameLine.swift");
        fs::write(&file_path, content).unwrap();

        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        // Expect fallback to the file stem "SameLine" rather than "MyClass".
        assert_eq!(extracted, "SameLine");
    }

    #[test]
    fn test_nonexistent_file_error() {
        // Attempt to call extract_enclosing_type on a file that doesn't exist.
        let file_path = "/path/to/nonexistent/file.swift";
        let result = extract_enclosing_type(file_path);
        assert!(result.is_err());
        // Optionally, check that the error message contains expected text.
        let err_msg = result.err().unwrap();
        assert!(err_msg.contains("Error reading file"));
    }
}
