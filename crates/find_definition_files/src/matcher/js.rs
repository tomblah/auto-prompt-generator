use regex::Regex;
use crate::matcher::DefinitionMatcher;

pub struct JSMatcher;

impl DefinitionMatcher for JSMatcher {
    fn matches_definition(&self, file_content: &str, type_name: &str) -> bool {
        // Look for a JS class declaration using the "class" keyword.
        let pattern = format!(r"\bclass\s+{}\b", regex::escape(type_name));
        Regex::new(&pattern).map_or(false, |re| re.is_match(file_content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_js_matcher_class_definition() {
        let content = "class MyClass { constructor() {} }";
        let matcher = JSMatcher;
        assert!(matcher.matches_definition(content, "MyClass"));
    }

    #[test]
    fn test_js_matcher_class_definition_with_extends() {
        let content = "class MySubClass extends MyClass { }";
        let matcher = JSMatcher;
        assert!(matcher.matches_definition(content, "MySubClass"));
    }

    #[test]
    fn test_js_matcher_negative_case() {
        let content = "function notAClass() {}";
        let matcher = JSMatcher;
        assert!(!matcher.matches_definition(content, "MyClass"));
    }

    #[test]
    fn test_js_matcher_partial_match() {
        let content = "class MyClassExtra { }";
        let matcher = JSMatcher;
        // This should return false because "MyClass" is not the full match.
        assert!(!matcher.matches_definition(content, "MyClass"));
    }
}
