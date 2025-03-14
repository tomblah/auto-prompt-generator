use extract_types::extract_types_from_file;
use std::io::Write;
use tempfile::NamedTempFile;
use anyhow::Result;

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
    
    // Directly get the extracted types as a String.
    let result = extract_types_from_file(temp_file.path())?;
    // Expect that only "MyClass" is extracted.
    let expected = "MyClass";
    assert_eq!(result.trim(), expected);
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
    // Expect "CustomType" to be extracted.
    let expected = "CustomType";
    assert_eq!(result.trim(), expected);
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
    // Expect no types to be extracted.
    assert!(result.trim().is_empty());
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
    // Expect only "InsideClass" to be extracted.
    let expected = "InsideClass";
    assert_eq!(result.trim(), expected);
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
    let expected = "TriggeredObjCType";
    assert_eq!(result.trim(), expected);
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
    
    // Expected output: both types extracted and sorted alphabetically.
    // "TypeInsideEnclosingFunction" comes before "TypeInsideMarker" lexicographically.
    let expected = "TypeInsideEnclosingFunction\nTypeInsideMarker";
    assert_eq!(result.trim(), expected);
    Ok(())
}
