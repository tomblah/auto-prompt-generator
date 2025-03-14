use std::fs;
use std::path::PathBuf;
use substring_marker_snippet_extractor::{filter_substring_markers};
use substring_marker_snippet_extractor::processor::{DefaultFileProcessor, process_file_with_processor};
// Bring the trait into scope so that methods like `process_file` are available.
use substring_marker_snippet_extractor::processor::file_processor::FileProcessor;

/// Helper function to create a temporary file with the given content.
/// Returns the full path to the temporary file.
fn create_temp_file_with_content(content: &str) -> PathBuf {
    // Create a temporary file in the OS temporary directory.
    // A random component in the filename avoids collisions.
    let mut path = std::env::temp_dir();
    let file_name = format!("temp_test_{}.swift", rand::random::<u32>());
    path.push(file_name);
    fs::write(&path, content).expect("Failed to write temporary file");
    path
}

#[test]
fn test_no_markers() {
    // When there are no markers and SLIM_MODE is not set,
    // process_file should return the raw file content.
    let raw_content = "func main() {\n    print(\"Hello, world!\")\n}\n";
    let path = create_temp_file_with_content(raw_content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
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
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for file with markers and TODO inside marker block");
    // Expected output is simply the filtered marker content.
    let expected = filter_substring_markers(content, "// ...");
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
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for file with markers and TODO outside marker block");
    
    // Compute the filtered content portion.
    let filtered = filter_substring_markers(content, "// ...");
    
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
fn test_default_processor_with_markers() {
    // Create a temporary file with a candidate declaration (using Swift syntax),
    // markers, and a TODO.
    let content = concat!(
        "Some preamble text\n",
        "func myFunction() {\n",
        "let x = 10;\n",
        "}\n",
        "Other text\n",
        "// v\n",
        "ignored text\n",
        "// ^\n",
        "Trailing text\n",
        "// TODO: - Do something"
    );
    // Generate the expected filtered output using the same function.
    let expected_filtered = filter_substring_markers(content, "// ...");
    // The extract_enclosing_block function should extract the candidate declaration exactly as it appears:
    let expected_context = "func myFunction() {\nlet x = 10;\n}";
    // The implementation appends the context with two newlines before the header.
    let expected_context_appended = format!("\n\n// Enclosing function context:\n{}", expected_context);
    let expected = format!("{}{}", expected_filtered, expected_context_appended);
    
    // Create an isolated temporary file for this test.
    let mut temp_file = tempfile::NamedTempFile::new().unwrap();
    use std::io::Write;
    write!(temp_file, "{}", content).unwrap();
    let file_basename = temp_file
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    
    let processor = DefaultFileProcessor;
    let result = processor
        .process_file(temp_file.path(), Some(&file_basename))
        .unwrap();

    // Compare the output tokenized by whitespace.
    let result_tokens: Vec<_> = result.split_whitespace().collect();
    let expected_tokens: Vec<_> = expected.split_whitespace().collect();
    assert_eq!(
        result_tokens, expected_tokens,
        "\n\nTokenized output did not match expected output."
    );
}

#[test]
fn test_file_not_found() {
    // process_file should return an error when the file does not exist.
    let path = PathBuf::from("non_existent_file.swift");
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some("non_existent_file.swift"));
    assert!(result.is_err(), "process_file should error for a non-existent file");
}

#[test]
fn test_multiple_marker_blocks() {
    // Even with multiple marker blocks, if no TODO is present,
    // no enclosing block should be appended.
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
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for file with multiple marker blocks");
    
    // Expected output is solely the filtered content.
    let expected = filter_substring_markers(content, "// ...");
    assert_eq!(result, expected);
    
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_slim_mode_no_markers() {
    // This test verifies that when SLIM_MODE is enabled,
    // even a file with no markers is processed as if it uses markers.
    std::env::set_var("SLIM_MODE", "true");
    
    let raw_content = "func main() {\n    print(\"Hello, slim world!\");\n}\n";
    let path = create_temp_file_with_content(raw_content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed in slim mode");
    
    let expected = filter_substring_markers(raw_content, "// ...");
    assert_eq!(result, expected);
    
    fs::remove_file(&path).expect("Failed to remove temporary file");
    std::env::remove_var("SLIM_MODE");
}
