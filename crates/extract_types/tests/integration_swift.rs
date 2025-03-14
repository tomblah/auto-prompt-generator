// crates/extract_types/tests/integration_swift.rs

use extract_types::extract_types_from_file;
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;
use anyhow::Result;

#[test]
fn integration_extract_types_basic() -> Result<()> {
    // Swift content with several type declarations.
    let swift_content = r#"
        import Foundation

        class MyClass {}
        struct MyStruct {}
        enum MyEnum {}
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;
    
    let result = extract_types_from_file(temp_file.path())?;
    // Expected sorted order: MyClass, MyEnum, MyStruct.
    let expected = "MyClass\nMyEnum\nMyStruct";
    assert_eq!(result.trim(), expected);
    Ok(())
}

#[test]
fn integration_extract_types_bracket_notation() -> Result<()> {
    // Swift content that uses bracket notation for type usage.
    let swift_content = r#"
        import UIKit
        let array: [CustomType] = []
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;
    
    let result = extract_types_from_file(temp_file.path())?;
    let expected = "CustomType";
    assert_eq!(result.trim(), expected);
    Ok(())
}

#[test]
fn integration_extract_types_no_types() -> Result<()> {
    // Swift content without any valid type names.
    let swift_content = r#"
        import Foundation
        let x = 5
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;
    
    let result = extract_types_from_file(temp_file.path())?;
    assert!(result.trim().is_empty());
    Ok(())
}

/// When substring markers are used, only the content between them should be processed.
#[test]
fn integration_extract_types_with_substring_markers() -> Result<()> {
    let swift_content = r#"
        // This type is outside the markers and should be ignored.
        class OutsideType {}
        // v
        // Only this section should be processed:
        class InsideType {}
        // ^
        // This type is also outside and should be ignored.
        class OutsideType2 {}
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;
    
    let result = extract_types_from_file(temp_file.path())?;
    // Expect only the content between markers to yield "InsideType"
    let expected = "InsideType";
    assert_eq!(result.trim(), expected);
    Ok(())
}

/// A trigger comment (starting with "// TODO: -") is tokenized broadly.
#[test]
fn integration_extract_types_trigger_comment() -> Result<()> {
    let swift_content = r#"
        import Foundation
        // TODO: - TriggeredType
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;
    
    let result = extract_types_from_file(temp_file.path())?;
    let expected = "TriggeredType";
    assert_eq!(result.trim(), expected);
    Ok(())
}

/// When a TODO marker appears outside the markers, the enclosing block is appended.
#[test]
fn integration_extract_types_todo_outside_markers() -> Result<()> {
    let swift_content = r#"
        // v
        let foo = TypeThatIsInsideMarker()
        // ^
        
        let bar = TypeThatIsOutSideMarker()
        
        func hello() {
            let hi = TypeThatIsInsideEnclosingFunction()
            // TODO: - example
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    // Expected output: both types are extracted and sorted alphabetically.
    // "TypeThatIsInsideEnclosingFunction" (from the function) and "TypeThatIsInsideMarker" (from the markers)
    let expected = "TypeThatIsInsideEnclosingFunction\nTypeThatIsInsideMarker";
    assert_eq!(result.trim(), expected);
    Ok(())
}

/// --- Integration tests covering the new TARGETED mode functionality ---

#[test]
fn integration_extract_types_targeted_mode() -> Result<()> {
    // Set TARGETED so that only the enclosing block is processed.
    env::set_var("TARGETED", "1");

    // In this Swift content, an outer declaration exists, but the function block (the candidate)
    // contains both an inner type and a trigger comment.
    let swift_content = r#"
        class OuterType {}
        func testFunction() {
            class InnerType {}
            // TODO: - Perform action
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;
    
    let result = extract_types_from_file(temp_file.path())?;
    // In targeted mode:
    // - "class InnerType {}" produces "InnerType"
    // - The trigger comment " // TODO: - Perform action" yields tokens ["Perform", "action"]
    //   but only "Perform" qualifies because "action" is lower-case.
    // Therefore, the expected output is "InnerType\nPerform".
    let expected = "InnerType\nPerform";
    assert_eq!(result.trim(), expected);

    env::remove_var("TARGETED");
    Ok(())
}

#[test]
fn integration_extract_types_targeted_mode_no_enclosing_block() -> Result<()> {
    // Set TARGETED so that only the enclosing block is processed.
    env::set_var("TARGETED", "1");

    // In this content no candidate enclosing block is found.
    let swift_content = r#"
        class OuterType {}
        // TODO: - Some todo
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;
    
    let result = extract_types_from_file(temp_file.path())?;
    // Since no candidate block is found, the full content is processed:
    // - "class OuterType {}" produces "OuterType"
    // - The trigger comment " // TODO: - Some todo" yields tokens ["Some", "todo"],
    //   but only "Some" qualifies because "todo" is lower-case.
    // Expected sorted order: "OuterType\nSome"
    let expected = "OuterType\nSome";
    assert_eq!(result.trim(), expected);

    env::remove_var("TARGETED");
    Ok(())
}
