// crates/extract_types/tests/integration_js.rs

use extract_types::extract_types_from_file;
use std::io::Write;
use tempfile::NamedTempFile;
use anyhow::Result;

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

    // Directly get the extracted types as a String.
    let result = extract_types_from_file(temp_file.path())?;
    let expected = "Component\nMyComponent\nReact";
    assert_eq!(result.trim(), expected);
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

    // Directly get the extracted types as a String.
    let result = extract_types_from_file(temp_file.path())?;
    // Since there are no tokens starting with an uppercase letter,
    // we expect no types to be extracted.
    assert!(result.trim().is_empty());
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

    // Directly get the extracted types as a String.
    let result = extract_types_from_file(temp_file.path())?;
    let expected = "TriggeredType";
    assert_eq!(result.trim(), expected);
    Ok(())
}
