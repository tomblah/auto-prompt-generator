use regex::Regex;
use crate::matcher::DefinitionMatcher;

pub struct ObjCMatcher;

impl DefinitionMatcher for ObjCMatcher {
    fn matches_definition(&self, file_content: &str, type_name: &str) -> bool {
        // Look for either @interface or @implementation followed by the type name.
        let interface_pattern = format!(r"@interface\s+{}\b", regex::escape(type_name));
        let implementation_pattern = format!(r"@implementation\s+{}\b", regex::escape(type_name));
        let interface_re = Regex::new(&interface_pattern).unwrap();
        let implementation_re = Regex::new(&implementation_pattern).unwrap();
        interface_re.is_match(file_content) || implementation_re.is_match(file_content)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_objc_interface_match() {
        let file_content = r#"
            #import <Foundation/Foundation.h>
            @interface Message : NSObject
            - (void)printMessage;
            @end
        "#;
        let matcher = ObjCMatcher;
        assert!(matcher.matches_definition(file_content, "Message"));
    }

    #[test]
    fn test_objc_implementation_match() {
        let file_content = r#"
            #import "Message.h"
            @implementation Message
            - (void)printMessage {
                NSLog(@"Hello, world!");
            }
            @end
        "#;
        let matcher = ObjCMatcher;
        assert!(matcher.matches_definition(file_content, "Message"));
    }

    #[test]
    fn test_objc_no_match() {
        let file_content = r#"
            #import <Foundation/Foundation.h>
            @interface AnotherClass : NSObject
            - (void)doSomething;
            @end
        "#;
        let matcher = ObjCMatcher;
        assert!(!matcher.matches_definition(file_content, "Message"));
    }

    #[test]
    fn test_objc_partial_match_should_fail() {
        let file_content = r#"
            @interface MessageExtra : NSObject
            @end
            @implementation MessageExtra
            - (void)doSomething {}
            @end
        "#;
        let matcher = ObjCMatcher;
        // Although "MessageExtra" is defined, we should not match when searching for "Message".
        assert!(!matcher.matches_definition(file_content, "Message"));
    }

    #[test]
    fn test_objc_whitespace_variation() {
        let file_content = "   @interface   Message   : NSObject";
        let matcher = ObjCMatcher;
        assert!(matcher.matches_definition(file_content, "Message"));
    }
}
