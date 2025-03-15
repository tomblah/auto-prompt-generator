// crates/enclosing_util/src/factory.rs

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgrammingLanguage {
    Rust,
    Swift,
    JavaScript,
    Unknown,
}

// These functions are only visible within the crate.
pub(crate) fn create_context_extractor(lang: ProgrammingLanguage) -> Box<dyn crate::traits::EnclosingContextExtractor> {
    match lang {
        ProgrammingLanguage::Rust => Box::new(crate::default::SimpleEnclosingContextExtractor),
        ProgrammingLanguage::Swift => Box::new(crate::default::SimpleEnclosingContextExtractor), // Placeholder.
        ProgrammingLanguage::JavaScript => Box::new(crate::default::SimpleEnclosingContextExtractor), // Placeholder.
        ProgrammingLanguage::Unknown => Box::new(crate::default::SimpleEnclosingContextExtractor),
    }
}

#[allow(dead_code)]
pub(crate) fn create_function_extractor(lang: ProgrammingLanguage) -> Box<dyn crate::traits::EnclosingFunctionExtractor> {
    match lang {
        ProgrammingLanguage::Rust => Box::new(crate::default::SimpleEnclosingFunctionExtractor),
        ProgrammingLanguage::Swift => Box::new(crate::default::SimpleEnclosingFunctionExtractor), // Placeholder.
        ProgrammingLanguage::JavaScript => Box::new(crate::default::SimpleEnclosingFunctionExtractor), // Placeholder.
        ProgrammingLanguage::Unknown => Box::new(crate::default::SimpleEnclosingFunctionExtractor),
    }
}

#[cfg(test)]
mod tests {
    use crate::factory::{create_context_extractor, create_function_extractor, ProgrammingLanguage};

    #[test]
    fn test_create_context_extractor_for_rust() {
        let content = r#"
            fn example() {
                let x = 42;
                {
                    let y = 100;
                    println!("{}", y);
                }
            }
        "#;
        let extractor = create_context_extractor(ProgrammingLanguage::Rust);
        let result = extractor.extract_enclosing_context(content, "println!");
        assert!(result.is_some(), "Should find an enclosing block for Rust variant");
        let block = result.unwrap();
        assert!(block.contains("let y = 100;"), "Extracted block should contain the inner block");
    }

    #[test]
    fn test_create_context_extractor_for_unknown() {
        let content = r#"
            fn example() {
                let x = 42;
                {
                    let y = 100;
                    println!("{}", y);
                }
            }
        "#;
        let extractor = create_context_extractor(ProgrammingLanguage::Unknown);
        let result = extractor.extract_enclosing_context(content, "println!");
        assert!(result.is_some(), "Should fall back to simple extractor for Unknown variant");
        let block = result.unwrap();
        assert!(block.contains("let y = 100;"), "Extracted block should contain the inner block");
    }

    #[test]
    fn test_create_function_extractor_for_all_variants() {
        // Since the default function extractor returns None, we can test that.
        for lang in [
            ProgrammingLanguage::Rust,
            ProgrammingLanguage::Swift,
            ProgrammingLanguage::JavaScript,
            ProgrammingLanguage::Unknown,
        ]
        .iter()
        {
            let extractor = create_function_extractor(*lang);
            let result = extractor.extract_enclosing_function("fn example() {}", "example");
            assert!(result.is_none(), "Default function extractor should return None");
        }
    }
}
