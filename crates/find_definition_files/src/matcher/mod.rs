pub mod swift;
pub mod js;

pub use swift::SwiftMatcher;
pub use js::JSMatcher;

/// Trait for language-specific definition matching.
pub trait DefinitionMatcher {
    fn matches_definition(&self, file_content: &str, type_name: &str) -> bool;
}

/// Helper to select the appropriate matcher based on file extension.
pub fn get_matcher_for_extension(ext: &str) -> Option<Box<dyn DefinitionMatcher>> {
    match ext.to_lowercase().as_str() {
        "swift" => Some(Box::new(SwiftMatcher)),
        "js" => Some(Box::new(JSMatcher)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_matcher_for_extension_swift() {
        let matcher = get_matcher_for_extension("swift");
        assert!(matcher.is_some(), "Expected a Swift matcher for 'swift' extension");
        // Use some Swift content to verify the matcher works.
        let content = "public class MyClass { }";
        assert!(matcher.unwrap().matches_definition(content, "MyClass"));
    }

    #[test]
    fn test_get_matcher_for_extension_js() {
        let matcher = get_matcher_for_extension("js");
        assert!(matcher.is_some(), "Expected a JS matcher for 'js' extension");
        let content = "class MyClass { constructor() {} }";
        assert!(matcher.unwrap().matches_definition(content, "MyClass"));
    }

    #[test]
    fn test_get_matcher_for_extension_unknown() {
        let matcher = get_matcher_for_extension("py");
        assert!(matcher.is_none(), "Expected no matcher for unsupported extension 'py'");
    }

    #[test]
    fn test_get_matcher_for_extension_case_insensitivity() {
        let matcher_swift = get_matcher_for_extension("SWIFT");
        assert!(matcher_swift.is_some(), "Expected a Swift matcher regardless of case");
        let matcher_js = get_matcher_for_extension("Js");
        assert!(matcher_js.is_some(), "Expected a JS matcher regardless of case");
    }
}
