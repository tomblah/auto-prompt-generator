use std::fs;
use std::path::PathBuf;
use prompt_file_processor::{process_file, filter_substring_markers};

/// Helper function to create a temporary file with the given content.
/// Returns the full path to the temporary file.
fn create_temp_file_with_content(content: &str) -> PathBuf {
    // Create a temporary file in the OS temporary directory.
    // We include a random component in the filename to avoid collisions.
    let mut path = std::env::temp_dir();
    let file_name = format!("temp_test_{}.swift", rand::random::<u32>());
    path.push(file_name);
    fs::write(&path, content).expect("Failed to write temporary file");
    path
}

#[test]
fn test_no_markers() {
    // When there are no markers, process_file should return the raw file content.
    let raw_content = "func main() {\n    print(\"Hello, world!\")\n}\n";
    let path = create_temp_file_with_content(raw_content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file(&path, Some(file_name))
        .expect("process_file should succeed for file with no markers");
    assert_eq!(result, raw_content);
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_markers_todo_inside() {
    // When the TODO is inside a marker block, extraction of enclosing context is skipped.
    let content = r#"
func myFunction() {
    print("Hello")
    // v
    // TODO: - perform task
    // ^
}
"#;
    let path = create_temp_file_with_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file(&path, Some(file_name))
        .expect("process_file should succeed for file with markers and TODO inside marker block");
    // The expected output should be just the filtered marker content.
    let expected = filter_substring_markers(content);
    assert_eq!(result, expected);
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_markers_todo_outside() {
    // When the TODO is outside the marker block, process_file should append the extracted enclosing block.
    let content = r#"
func myFunction() {
    print("Hello")
}

// v
// Extra context that is not part of the function block.
// ^

 // TODO: - perform important task
"#;
    let path = create_temp_file_with_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file(&path, Some(file_name))
        .expect("process_file should succeed for file with markers and TODO outside marker block");

    // Compute the filtered content portion.
    let filtered = filter_substring_markers(content);
    
    // Verify that the result:
    // 1. Starts with the filtered marker content.
    // 2. Contains the header indicating that an enclosing context was appended.
    // 3. Contains some content from the extracted enclosing block (e.g. the function declaration).
    assert!(result.starts_with(&filtered), "Result should start with the filtered content");
    assert!(
        result.contains("// Enclosing function context:"),
        "Result should contain the enclosing context header"
    );
    assert!(
        result.contains("func myFunction()"),
        "Result should contain the extracted function context"
    );
    
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_file_not_found() {
    // process_file should return an error when the file does not exist.
    let path = PathBuf::from("non_existent_file.swift");
    let result = process_file(&path, Some("non_existent_file.swift"));
    assert!(result.is_err(), "process_file should error for a non-existent file");
}

#[test]
fn test_multiple_marker_blocks() {
    // Even with multiple marker blocks, if no TODO is present, no enclosing block should be appended.
    let content = r#"
func foo() {
    print("Foo")
}

// v
line a
line b
// ^

// v
line c
// ^
"#;
    let path = create_temp_file_with_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file(&path, Some(file_name))
        .expect("process_file should succeed for file with multiple marker blocks");
    
    // In this scenario, since there is no TODO marker at all,
    // the output should be solely the filtered content.
    let expected = filter_substring_markers(content);
    assert_eq!(result, expected);
    
    fs::remove_file(&path).expect("Failed to remove temporary file");
}
