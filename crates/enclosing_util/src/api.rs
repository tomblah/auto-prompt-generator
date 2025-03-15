// crates/enclosing_util/src/api.rs

use crate::factory::{create_context_extractor, ProgrammingLanguage};

/// Extracts the enclosing context for the given content, token, and programming language.
pub fn extract_context(content: &str, token: &str, lang: ProgrammingLanguage) -> Option<String> {
    let extractor = create_context_extractor(lang);
    extractor.extract_enclosing_context(content, token)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::factory::ProgrammingLanguage;

    #[test]
    fn test_extract_context_returns_block() {
        let content = r#"
            fn example() {
                let x = 42;
                {
                    let y = 100;
                    println!("{}", y);
                }
            }
        "#;
        // Using ProgrammingLanguage::Rust (or any variant) should work the same right now.
        let result = extract_context(content, "println!", ProgrammingLanguage::Rust);
        assert!(result.is_some(), "Should find an enclosing block for the token");
        let block = result.unwrap();
        // The extracted block should contain the inner block's content.
        assert!(block.contains("let y = 100;"), "Block should contain the inner block code");
    }

    #[test]
    fn test_extract_context_returns_none_when_no_braces() {
        let content = "This is a plain text with token: token";
        let result = extract_context(content, "token", ProgrammingLanguage::Rust);
        // Since there are no braces, the default simple extractor should return None.
        assert!(result.is_none(), "No block should be extracted when no braces exist");
    }

    #[test]
    fn test_extract_context_fallback_for_unknown_language() {
        let content = r#"
            fn example() {
                let x = 42;
                {
                    let y = 100;
                    println!("{}", y);
                }
            }
        "#;
        // Using the Unknown variant should fall back to the simple extractor.
        let result = extract_context(content, "println!", ProgrammingLanguage::Unknown);
        assert!(result.is_some(), "Fallback extractor should work for Unknown language variant");
    }
}
