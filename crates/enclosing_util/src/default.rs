// crates/enclosing_util/src/default.rs

use crate::traits::{EnclosingContextExtractor, EnclosingFunctionExtractor};

/// A simple implementation of `EnclosingContextExtractor` that uses a naive
/// brace-matching algorithm to find the smallest block enclosing the token.
pub struct SimpleEnclosingContextExtractor;

impl SimpleEnclosingContextExtractor {
    /// Helper function that finds the innermost matching pair of braces that
    /// encloses the position given by `token_index`.
    fn find_enclosing_braces(content: &str, token_index: usize) -> Option<(usize, usize)> {
        let bytes = content.as_bytes();
        let mut stack = Vec::new();
        let mut enclosing_range = None;

        for (i, &b) in bytes.iter().enumerate() {
            if b == b'{' {
                stack.push(i);
            } else if b == b'}' {
                if let Some(start) = stack.pop() {
                    if start <= token_index && token_index <= i {
                        match enclosing_range {
                            Some((prev_start, prev_end)) => {
                                // Prefer the innermost block.
                                if (i - start) < (prev_end - prev_start) {
                                    enclosing_range = Some((start, i));
                                }
                            }
                            None => {
                                enclosing_range = Some((start, i));
                            }
                        }
                    }
                }
            }
        }
        enclosing_range
    }
}

impl EnclosingContextExtractor for SimpleEnclosingContextExtractor {
    fn extract_enclosing_context(&self, content: &str, token: &str) -> Option<String> {
        let token_index = content.find(token)?;
        if let Some((start, end)) = Self::find_enclosing_braces(content, token_index) {
            Some(content[start..=end].to_string())
        } else {
            None
        }
    }
}

/// A simple implementation of `EnclosingFunctionExtractor` that does nothing.
/// Language-specific implementations should override this trait.
pub struct SimpleEnclosingFunctionExtractor;

impl EnclosingFunctionExtractor for SimpleEnclosingFunctionExtractor {
    fn extract_enclosing_function(&self, _content: &str, _token: &str) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{EnclosingContextExtractor, EnclosingFunctionExtractor};

    #[test]
    fn test_extract_enclosing_context_simple() {
        let content = r#"
            fn example() {
                let x = 5;
                {
                    let y = 10;
                    println!("{}", y);
                }
            }
        "#;
        let extractor = SimpleEnclosingContextExtractor;
        let result = extractor.extract_enclosing_context(content, "println!");
        assert!(result.is_some());
        let block = result.unwrap();
        assert!(block.contains("let y = 10;"));
    }

    #[test]
    fn test_extract_enclosing_context_not_found() {
        let content = "No braces here, but token exists: token";
        let extractor = SimpleEnclosingContextExtractor;
        let result = extractor.extract_enclosing_context(content, "token");
        assert!(result.is_none());
    }

    #[test]
    fn test_extract_enclosing_function_default() {
        let content = r#"
            fn example() {
                // some code
            }
        "#;
        let extractor = SimpleEnclosingFunctionExtractor;
        let result = extractor.extract_enclosing_function(content, "example");
        assert!(result.is_none());
    }
}
