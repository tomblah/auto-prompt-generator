// crates/enclosing_util/src/traits.rs

/// around a given token in the source content.
pub trait EnclosingContextExtractor {
    /// Given the source `content` and a `token`, returns the smallest block of code,
    /// delimited by matching braces (`{` ... `}`), that encloses the token.
    /// Returns `None` if no such block is found.
    fn extract_enclosing_context(&self, content: &str, token: &str) -> Option<String>;
}

/// Trait for extracting an enclosing function (or method) that contains a token.
/// This is language-specific. The default implementation here returns `None`,
/// and language-specific implementations can override this method.
pub trait EnclosingFunctionExtractor {
    /// Given the source `content` and a `token`, returns the function (or method) block
    /// that encloses the token. Returns `None` if not found.
    fn extract_enclosing_function(&self, content: &str, token: &str) -> Option<String>;
}
