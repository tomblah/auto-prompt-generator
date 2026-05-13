// crates/lang_support/tests/objc.rs

use lang_support::for_extension;

#[test]
fn objc_file_defines_any_matches_interface_and_implementation() {
    let objc = for_extension("h").unwrap();
    let want = vec!["Message".to_string()];

    let cases = [
        r#"
            #import <Foundation/Foundation.h>
            @interface Message : NSObject
            @end
        "#,
        r#"
            #import "Message.h"
            @implementation Message
            @end
        "#,
    ];

    for snippet in cases {
        assert!(
            objc.file_defines_any(snippet, &want),
            "Did not match snippet: {snippet:?}"
        );
    }
}

#[test]
fn objc_file_defines_any_rejects_partial_match() {
    let objc = for_extension("m").unwrap();
    let want = vec!["Message".to_string()];
    let content = r#"
        @interface MessageExtra : NSObject
        @end
        @implementation MessageExtra
        @end
    "#;

    assert!(!objc.file_defines_any(content, &want));
}

#[test]
fn objc_file_defines_any_handles_whitespace_variation() {
    let objc = for_extension("h").unwrap();

    assert!(objc.file_defines_any(
        "   @interface   Message   : NSObject",
        &["Message".to_string()]
    ));
}

#[test]
fn objc_extract_identifiers_is_empty() {
    let objc = for_extension("m").unwrap();

    assert!(objc
        .extract_identifiers("@interface Message : NSObject")
        .is_empty());
}

#[test]
fn objc_resolve_dependency_path_is_none() {
    let objc = for_extension("h").unwrap();

    assert!(objc
        .resolve_dependency_path("#import \"Message.h\"", std::path::Path::new("."))
        .is_none());
}
