// crates/assemble_prompt/tests/file_processor_js.rs

use std::fs;
use std::path::PathBuf;

use assemble_prompt::{process_file_with_processor, DefaultFileProcessor};
use substring_marker_snippet_extractor::filter_substring_markers;

fn create_temp_file_with_content(content: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    let file_name = format!("temp_test_{}.js", rand::random::<u32>());
    path.push(file_name);
    fs::write(&path, content).expect("Failed to write temporary file");
    path
}

#[test]
fn test_no_markers() {
    let raw_content = "function main() {\n    console.log(\"Hello, world!\");\n}\n";
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
function myFunction() {
    console.log("Hello");
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
function myFunction() {
    console.log("Hello");
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
        result.contains("function myFunction()"),
        "Result should contain the extracted function context"
    );

    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_file_not_found() {
    let path = PathBuf::from("non_existent_file.js");
    let result =
        process_file_with_processor(&DefaultFileProcessor, &path, Some("non_existent_file.js"));
    assert!(
        result.is_err(),
        "process_file should error for a non-existent file"
    );
}

#[test]
fn test_multiple_marker_blocks() {
    let content = r#"
function foo() {
    console.log("Foo");
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

#[test]
fn test_parse_cloud_after_save_quoted() {
    let content = r#"
    // v
    Header content that is within markers
    // ^
    Parse.Cloud.afterSave("Message", async (request) => {
        console.log("AfterSave Setup");
        // TODO: - Handle after save logic
        console.log("AfterSave Teardown");
    });
    Trailing footer text
    "#;
    let path = create_temp_file_with_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for Parse.Cloud.afterSave with quoted argument");
    assert!(result.contains("Parse.Cloud.afterSave(\"Message\", async (request) => {"));
    assert!(result.contains("// TODO: - Handle after save logic"));
    assert!(result.contains("console.log(\"AfterSave Teardown\");"));
    assert!(!result.contains("Trailing footer text"));
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_parse_cloud_before_save_parse_user() {
    let content = r#"
    // v
    Header section that is not part of the function block
    // ^
    Parse.Cloud.beforeSave(Parse.User, async (request) => {
        console.log("BeforeSave Init");
        // TODO: - Process user before save
        console.log("BeforeSave Complete");
    });
    Extra footer text that should be omitted
    "#;
    let path = create_temp_file_with_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for Parse.Cloud.beforeSave with Parse.User");
    assert!(result.contains("Parse.Cloud.beforeSave(Parse.User, async (request) => {"));
    assert!(result.contains("// TODO: - Process user before save"));
    assert!(result.contains("console.log(\"BeforeSave Complete\");"));
    assert!(!result.contains("Extra footer text"));
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_parse_cloud_after_save_parse_user() {
    let content = r#"
    // v
    Some header that is filtered out
    // ^
    Parse.Cloud.afterSave(Parse.User, async (request) => {
        console.log("AfterSave Start");
        // TODO: - Process user after save
        console.log("AfterSave End");
    });
    Irrelevant footer text
    "#;
    let path = create_temp_file_with_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for Parse.Cloud.afterSave with Parse.User");
    assert!(result.contains("Parse.Cloud.afterSave(Parse.User, async (request) => {"));
    assert!(result.contains("// TODO: - Process user after save"));
    assert!(result.contains("console.log(\"AfterSave End\");"));
    assert!(!result.contains("Irrelevant footer text"));
    fs::remove_file(&path).expect("Failed to remove temporary file");
}
