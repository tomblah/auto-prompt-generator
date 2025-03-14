// crates/substring_marker_snippet_extractor/tests/integration_objc.rs

use std::fs;
use std::path::PathBuf;
use substring_marker_snippet_extractor::{filter_substring_markers};
use substring_marker_snippet_extractor::processor::{DefaultFileProcessor, process_file_with_processor};

/// Helper function to create a temporary Objective‑C file with the given content.
/// Returns the full path to the temporary file.
fn create_temp_file_with_objc_content(content: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    // Use a .m extension for Objective‑C source files.
    let file_name = format!("temp_test_{}.m", rand::random::<u32>());
    path.push(file_name);
    fs::write(&path, content).expect("Failed to write temporary file");
    path
}

#[test]
fn test_no_markers_objc() {
    // When there are no markers, process_file should return the raw file content.
    let raw_content = "\
#import <Foundation/Foundation.h>

@interface MyClass : NSObject
- (void)doSomething;
@end

@implementation MyClass
- (void)doSomething
{
    NSLog(@\"Hello, Objective-C!\");
}
@end
";
    let path = create_temp_file_with_objc_content(raw_content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for file with no markers");
    assert_eq!(result, raw_content);
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_markers_todo_inside_objc() {
    // When the TODO is inside a marker block, extraction of enclosing context is skipped.
    let content = r#"
#import <Foundation/Foundation.h>

@interface MyClass : NSObject
- (void)doSomething;
@end

@implementation MyClass
- (void)doSomething
{
    NSLog(@"Start");
    // v
    // TODO: - perform task in ObjC
    // ^
    NSLog(@"End");
}
@end
"#;
    let path = create_temp_file_with_objc_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for file with markers and TODO inside marker block");
    // The expected output should be just the filtered marker content.
    let expected = filter_substring_markers(content, "// ...");
    assert_eq!(result, expected);
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_markers_todo_outside_objc() {
    // When the TODO is outside the marker block, process_file should append the extracted enclosing block.
    let content = r#"
#import <Foundation/Foundation.h>

@interface MyClass : NSObject
- (void)doSomething;
@end

@implementation MyClass
- (void)doSomething
{
    NSLog(@"Start");
}

 // v
 // Extra context that is not part of the method block.
 // ^
 
 // TODO: - perform important task in ObjC

@end
"#;
    let path = create_temp_file_with_objc_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for file with markers and TODO outside marker block");

    // Compute the filtered content portion.
    let filtered = filter_substring_markers(content, "// ...");
    
    // Verify that the result:
    // 1. Starts with the filtered marker content.
    // 2. Contains the header indicating that an enclosing context was appended.
    // 3. Contains some content from the extracted enclosing block (e.g. the Objective-C method declaration).
    assert!(result.starts_with(&filtered), "Result should start with the filtered content");
    assert!(
        result.contains("// Enclosing function context:"),
        "Result should contain the enclosing context header"
    );
    assert!(
        result.contains("- (void)doSomething"),
        "Result should contain the extracted ObjC method context"
    );
    
    fs::remove_file(&path).expect("Failed to remove temporary file");
}

#[test]
fn test_file_not_found_objc() {
    // process_file should return an error when the file does not exist.
    let path = PathBuf::from("non_existent_file.m");
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some("non_existent_file.m"));
    assert!(result.is_err(), "process_file should error for a non-existent file");
}

#[test]
fn test_multiple_marker_blocks_objc() {
    // Even with multiple marker blocks, if no TODO marker is present,
    // the output should be solely the filtered content.
    let content = r#"
#import <Foundation/Foundation.h>

@interface MyClass : NSObject
- (void)foo;
@end

@implementation MyClass
- (void)foo
{
    NSLog(@"Foo");
}

// v
line a
line b
// ^

// v
line c
// ^
@end
"#;
    let path = create_temp_file_with_objc_content(content);
    let file_name = path.file_name().unwrap().to_str().unwrap();
    let result = process_file_with_processor(&DefaultFileProcessor, &path, Some(file_name))
        .expect("process_file should succeed for file with multiple marker blocks");
    
    let expected = filter_substring_markers(content, "// ...");
    assert_eq!(result, expected);
    
    fs::remove_file(&path).expect("Failed to remove temporary file");
}
