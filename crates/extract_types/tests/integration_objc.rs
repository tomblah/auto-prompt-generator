// crates/extract_types/tests/integration_objc.rs

use std::collections::BTreeSet;

use anyhow::Result;
use extract_types::extract_types_from_file;
use std::io::Write;
use tempfile::NamedTempFile;

fn types(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn integration_extract_types_objc_basic() -> Result<()> {
    // Objective‑C content with an interface and implementation.
    let objc_content = r#"
        #import <Foundation/Foundation.h>
        
        @interface MyClass {}
        @end
        
        @implementation MyClass {}
        @end
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", objc_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert_eq!(result, types(&["MyClass"]));
    Ok(())
}

#[test]
fn integration_extract_types_objc_bracket_notation() -> Result<()> {
    // Objective‑C content with a message send using bracket notation.
    let objc_content = r#"
        #import <Foundation/Foundation.h>
        int main() {
            [CustomType doSomething];
            return 0;
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", objc_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert_eq!(result, types(&["CustomType"]));
    Ok(())
}

#[test]
fn integration_extract_types_objc_no_types() -> Result<()> {
    // Objective‑C content without any valid type names.
    let objc_content = r#"
        #import <Foundation/Foundation.h>
        int main() {
            int x = 5;
            return x;
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", objc_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert!(result.is_empty());
    Ok(())
}

#[test]
fn integration_extract_types_objc_with_substring_markers() -> Result<()> {
    // Objective‑C content with substring markers.
    let objc_content = r#"
        // This type is outside the markers and should be ignored.
        @interface OutsideClass {}
        @end
        
        // v
        // Only this section should be processed:
        @interface InsideClass {}
        @end
        // ^
        
        // This type is also outside and should be ignored.
        @interface OutsideClass2 {}
        @end
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", objc_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert_eq!(result, types(&["InsideClass"]));
    Ok(())
}

#[test]
fn integration_extract_types_objc_trigger_comment() -> Result<()> {
    // Objective‑C content with a trigger comment.
    let objc_content = r#"
        #import <Foundation/Foundation.h>
        // TODO: - TriggeredObjCType
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", objc_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert_eq!(result, types(&["TriggeredObjCType"]));
    Ok(())
}

#[test]
fn integration_extract_types_objc_todo_outside_markers() -> Result<()> {
    // Objective‑C content with markers and a TODO marker outside the markers.
    // The markers enclose a section that yields "TypeInsideMarker",
    // while a function later contains a TODO with "TypeInsideEnclosingFunction".
    let objc_content = r#"
        @interface Dummy {}
        @end

        // v
        int foo() {
            return [TypeInsideMarker doSomething];
        }
        // ^
        
        - (void)function {
            // TODO: - TypeInsideEnclosingFunction
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", objc_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert_eq!(
        result,
        types(&["TypeInsideEnclosingFunction", "TypeInsideMarker"])
    );
    Ok(())
}
