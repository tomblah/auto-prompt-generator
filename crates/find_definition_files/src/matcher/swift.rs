// crates/find_definition_files/src/matcher/swift.rs

use regex::Regex;
use crate::matcher::DefinitionMatcher;

pub struct SwiftMatcher;

impl DefinitionMatcher for SwiftMatcher {
    fn matches_definition(&self, file_content: &str, type_name: &str) -> bool {
        let pattern = format!(
            r"\b(?:class|struct|enum|protocol|typealias)\s+{}\b",
            regex::escape(type_name)
        );
        Regex::new(&pattern).map_or(false, |re| re.is_match(file_content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swift_matcher_class_definition() {
        let content = "public class MyClass { }";
        let matcher = SwiftMatcher;
        assert!(matcher.matches_definition(content, "MyClass"));
    }

    #[test]
    fn test_swift_matcher_struct_definition() {
        let content = "struct MyStruct { }";
        let matcher = SwiftMatcher;
        assert!(matcher.matches_definition(content, "MyStruct"));
    }

    #[test]
    fn test_swift_matcher_enum_definition() {
        let content = "enum MyEnum { case one, two }";
        let matcher = SwiftMatcher;
        assert!(matcher.matches_definition(content, "MyEnum"));
    }

    #[test]
    fn test_swift_matcher_protocol_definition() {
        let content = "protocol MyProtocol { func doSomething() }";
        let matcher = SwiftMatcher;
        assert!(matcher.matches_definition(content, "MyProtocol"));
    }

    #[test]
    fn test_swift_matcher_typealias_definition() {
        let content = "typealias MyType = Int";
        let matcher = SwiftMatcher;
        assert!(matcher.matches_definition(content, "MyType"));
    }

    #[test]
    fn test_swift_matcher_negative_case() {
        let content = "var something = 123";
        let matcher = SwiftMatcher;
        // There is no definition for "MyClass" here so it should return false.
        assert!(!matcher.matches_definition(content, "MyClass"));
    }
}
