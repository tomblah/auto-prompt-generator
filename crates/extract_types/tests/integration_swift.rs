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
