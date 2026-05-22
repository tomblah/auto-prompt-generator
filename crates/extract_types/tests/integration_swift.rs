// crates/extract_types/tests/integration_swift.rs

use std::collections::BTreeSet;

use anyhow::Result;
use extract_types::{
    extract_types_from_file, extract_types_from_file_with_options, ExtractTypesOptions,
};
use std::env;
use std::io::Write;
use tempfile::NamedTempFile;

fn types(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

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
    assert_eq!(result, types(&["MyClass", "MyEnum", "MyStruct"]));
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
    assert_eq!(result, types(&["CustomType"]));
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
    assert!(result.is_empty());
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
    assert_eq!(result, types(&["InsideType"]));
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
    assert_eq!(result, types(&["TriggeredType"]));
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
    assert_eq!(
        result,
        types(&[
            "TypeThatIsInsideEnclosingFunction",
            "TypeThatIsInsideMarker"
        ])
    );
    Ok(())
}

/// --- Integration tests covering the new TARGETED mode functionality ---

#[test]
fn integration_extract_types_targeted_mode() -> Result<()> {
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

    let result = extract_types_from_file_with_options(
        temp_file.path(),
        &ExtractTypesOptions { targeted: true },
    )?;
    assert_eq!(result, types(&["InnerType", "Perform"]));

    Ok(())
}

#[test]
fn integration_extract_types_without_targeted_env_processes_full_content() -> Result<()> {
    env::remove_var("TARGETED");

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
    assert_eq!(result, types(&["InnerType", "OuterType", "Perform"]));

    Ok(())
}

#[test]
fn integration_extract_types_default_ignores_targeted_env() -> Result<()> {
    env::set_var("TARGETED", "1");

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
    assert_eq!(result, types(&["InnerType", "OuterType", "Perform"]));

    env::remove_var("TARGETED");
    Ok(())
}

#[test]
fn integration_extract_types_explicit_targeted_option_ignores_env() -> Result<()> {
    env::remove_var("TARGETED");

    let swift_content = r#"
        class OuterType {}
        func testFunction() {
            class InnerType {}
            // TODO: - Perform action
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;

    let result = extract_types_from_file_with_options(
        temp_file.path(),
        &ExtractTypesOptions { targeted: true },
    )?;
    assert_eq!(result, types(&["InnerType", "Perform"]));

    Ok(())
}

#[test]
fn integration_extract_types_explicit_non_targeted_option_ignores_targeted_env() -> Result<()> {
    env::set_var("TARGETED", "1");

    let swift_content = r#"
        class OuterType {}
        func testFunction() {
            class InnerType {}
            // TODO: - Perform action
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;

    let result = extract_types_from_file_with_options(
        temp_file.path(),
        &ExtractTypesOptions { targeted: false },
    )?;
    assert_eq!(result, types(&["InnerType", "OuterType", "Perform"]));

    env::remove_var("TARGETED");
    Ok(())
}

#[test]
fn integration_extract_types_explicit_targeted_option_works_with_targeted_env() -> Result<()> {
    env::set_var("TARGETED", "1");

    let swift_content = r#"
        class OuterType {}
        func testFunction() {
            class InnerType {}
            // TODO: - Perform action
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;

    let result = extract_types_from_file_with_options(
        temp_file.path(),
        &ExtractTypesOptions { targeted: true },
    )?;
    assert_eq!(result, types(&["InnerType", "Perform"]));

    env::remove_var("TARGETED");
    Ok(())
}

#[test]
fn integration_extract_types_targeted_mode_no_enclosing_block() -> Result<()> {
    // In this content no candidate enclosing block is found.
    let swift_content = r#"
        class OuterType {}
        // TODO: - Some todo
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;

    let result = extract_types_from_file_with_options(
        temp_file.path(),
        &ExtractTypesOptions { targeted: true },
    )?;
    assert_eq!(result, types(&["OuterType", "Some"]));

    Ok(())
}

#[test]
fn integration_extract_types_targeted_inner_block_excludes_outer() -> Result<()> {
    // In this Swift content, there's an outer declaration and then inside a function block,
    // an inner block containing both an inner type and a trigger comment.
    // We expect that only the tokens from the inner block are extracted.
    let swift_content = r#"
        class OuterType {}
        func testFunction() {
            // This declaration is inside the function but outside the inner block.
            class OuterType {}
            {
                class InnerType {}
                // TODO: - Do something important
            }
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", swift_content)?;

    let result = extract_types_from_file_with_options(
        temp_file.path(),
        &ExtractTypesOptions { targeted: true },
    )?;
    assert_eq!(result, types(&["Do", "InnerType"]));

    Ok(())
}
