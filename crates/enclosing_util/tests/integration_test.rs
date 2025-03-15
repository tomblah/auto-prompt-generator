// crates/enclosing_util/tests/integration_test.rs

use enclosing_util::{extract_context, ProgrammingLanguage};

#[test]
fn integration_extract_context_found() {
    let content = r#"
        fn main() {
            // Some introductory code.
            println!("Hello, world!");
            {
                let x = 10;
                println!("{}", x);
            }
            // End of function.
        }
    "#;
    // Use a token that's known to be inside the inner block.
    let result = extract_context(content, "let x = 10;", ProgrammingLanguage::Rust);
    assert!(result.is_some(), "Should extract a block containing the token");
    let block = result.unwrap();
    assert!(block.contains("let x = 10;"), "The extracted block should contain the expected code");
    // Optionally, print the block for visual confirmation during testing:
    println!("Extracted block: {}", block);
}

#[test]
fn integration_extract_context_not_found() {
    let content = r#"
        // This file has no braces.
        println!("Hello, world!");
    "#;
    // Use a token that is present.
    let result = extract_context(content, "println!(\"Hello, world!\")", ProgrammingLanguage::Rust);
    assert!(result.is_none(), "No block should be extracted when token isn't inside braces");
}
