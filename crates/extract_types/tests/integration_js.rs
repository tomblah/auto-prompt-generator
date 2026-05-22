// crates/extract_types/tests/integration_js.rs

use std::collections::BTreeSet;

use anyhow::Result;
use extract_types::extract_types_from_file;
use std::io::Write;
use tempfile::NamedTempFile;

fn types(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
#[ignore] // Ignored until full JavaScript support is implemented.
fn integration_extract_types_javascript_class() -> Result<()> {
    // JavaScript content that defines a class.
    // When tokenized, the line "class MyComponent extends React.Component {" becomes:
    //   ["class", "MyComponent", "extends", "React", "Component"]
    // Only tokens starting with an uppercase letter will be kept:
    //   "MyComponent", "React", and "Component"
    // Since a BTreeSet is used, the tokens are sorted alphabetically.
    let js_content = r#"
        import React from 'react';

        class MyComponent extends React.Component {
            render() {
                return <div>Hello</div>;
            }
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", js_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert_eq!(result, types(&["Component", "MyComponent", "React"]));
    Ok(())
}

#[test]
fn integration_extract_types_javascript_no_types() -> Result<()> {
    // JavaScript content with no valid type declarations.
    let js_content = r#"
        import something from 'somewhere';
        const x = 5;
        function doSomething() {
            return x;
        }
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", js_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert!(result.is_empty());
    Ok(())
}

#[test]
fn integration_extract_types_javascript_trigger_comment() -> Result<()> {
    // JavaScript content using a trigger comment to explicitly indicate a type.
    let js_content = r#"
        // TODO: - TriggeredType
    "#;
    let mut temp_file = NamedTempFile::new()?;
    write!(temp_file, "{}", js_content)?;

    let result = extract_types_from_file(temp_file.path())?;
    assert_eq!(result, types(&["TriggeredType"]));
    Ok(())
}
