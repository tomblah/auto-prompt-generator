use extract_types::extract_types_from_file;
use std::fs;
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
    
    let result_path = extract_types_from_file(temp_file.path())?;
    let result = fs::read_to_string(result_path)?;
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
    
    let result_path = extract_types_from_file(temp_file.path())?;
    let result = fs::read_to_string(result_path)?;
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
    
    let result_path = extract_types_from_file(temp_file.path())?;
    let result = fs::read_to_string(result_path)?;
    // Expecting no types to be extracted.
    assert!(result.trim().is_empty());
    Ok(())
}

// New integration test: Ensure that when substring markers are present,
// only the content between the markers is considered.
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
    
    let result_path = extract_types_from_file(temp_file.path())?;
    let result = fs::read_to_string(result_path)?;
    // Expect only "InsideType" to be extracted.
    let expected = "InsideType";
    assert_eq!(result.trim(), expected);
    Ok(())
}

// New integration test: Ensure that trigger comments are processed correctly.
#[test]
fn integration_extract_types_trigger_comment() -> Result<()> {
    let swift_content = r#"
        import Foundation
        // TODO: - TriggeredType
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;
    
    let result_path = extract_types_from_file(temp_file.path())?;
    let result = fs::read_to_string(result_path)?;
    let expected = "TriggeredType";
    assert_eq!(result.trim(), expected);
    Ok(())
}

#[test]
fn integration_extract_types_todo_outside_markers() -> Result<()> {
    // The Swift file content:
    // - The substring markers ("// v" and "// ^") enclose a declaration that should yield "TypeThatIsInsideMarker".
    // - Outside the markers, there's a type "TypeThatIsOutSideMarker" that should be ignored.
    // - In a function (which is outside the markers) with a TODO marker, we have "TypeThatIsInsideEnclosingFunction",
    //   and this type should be recognized.
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

    // Call the extraction function.
    let result_path = extract_types_from_file(temp_file.path())?;
    let result = fs::read_to_string(result_path)?;
    
    // The expected types are those inside the markers and inside the enclosing function,
    // sorted in alphabetical order. Since "TypeThatIsInsideEnclosingFunction" comes before
    // "TypeThatIsInsideMarker" lexicographically, we expect:
    let expected = "TypeThatIsInsideEnclosingFunction\nTypeThatIsInsideMarker";
    assert_eq!(result.trim(), expected);
    Ok(())
}

#[test]
fn test_extract_types_with_markers_and_enclosing_todo_no_types_inside_markers() -> anyhow::Result<()> {
    // This Swift file content includes:
    // - A marked section (between "// v" and "// ^") that does NOT contain any capitalized type.
    // - An outside declaration "TypeThatIsOutSideMarker" which should be ignored.
    // - A function with a TODO marker that contains "TypeThatIsInsideEnclosingFunction", which should be included.
    let mut swift_file = tempfile::NamedTempFile::new()?;
    writeln!(swift_file, r#"
        // v
        // No types are declared in this marked block.
        let x = 42;
        // ^
        
        let bar = TypeThatIsOutSideMarker()
        
        func hello() {{
            let hi = TypeThatIsInsideEnclosingFunction()
            // TODO: - example
        }}
    "#)?;
    let result_path = extract_types::extract_types_from_file(swift_file.path())?;
    let result = std::fs::read_to_string(&result_path)?;
    // Expect that only "TypeThatIsInsideEnclosingFunction" is extracted.
    let expected = "TypeThatIsInsideEnclosingFunction";
    assert_eq!(result.trim(), expected);
    Ok(())
}

