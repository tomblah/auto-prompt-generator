// crates/assemble_prompt/tests/file_processor_swift.rs

use std::fs;
use std::path::PathBuf;

use assemble_prompt::{process_file_with_processor, DefaultFileProcessor};
use substring_marker_snippet_extractor::filter_substring_markers;

fn create_temp_file_with_content(content: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let file_name = format!("temp_test_{}.swift", rand::random::<u32>());
    path.push(file_name);
    fs::write(&path, content).expect("Failed to write temporary file");
    path
}

#[test]
fn test_no_markers() {
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
    let expected = filter_substring_markers(content, "// ...");
    assert_eq!(result, expected);
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_markers_todo_outside() {
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

    let filtered = filter_substring_markers(content, "// ...");

    assert!(
        result.starts_with(&filtered),
        "Result should start with the filtered content"
    );
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
fn test_swift_file_currently_accepts_javascript_enclosing_candidate() {
    let content = r#"
function foreignJavaScriptFunction() {
    return true;
}

// v
selected context
// ^

// TODO: - update behavior
"#;
    let path = create_temp_file_with_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should preserve the current cross-language behavior");

    assert!(result.contains("// Enclosing function context:"));
    assert!(result.contains("function foreignJavaScriptFunction() {"));

    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_file_not_found() {
    let path = PathBuf::from("non_existent_file.swift");
    let result = process_file_with_processor(
        &DefaultFileProcessor,
        &path,
        Some("non_existent_file.swift"),
    );
    assert!(
        result.is_err(),
        "process_file should error for a non-existent file"
    );
}

#[test]
fn test_multiple_marker_blocks() {
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

    let expected = filter_substring_markers(content, "// ...");
    assert_eq!(result, expected);

    fs::remove_file(&path).expect("Failed to remove temporary file");
}
