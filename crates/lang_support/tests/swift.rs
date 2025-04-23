// crates/lang_support/tests/swift.rs

use lang_support::for_extension;
use std::path::Path;

#[test]
fn swift_extracts_class_name() {
    let swift = for_extension("swift").unwrap();

    let idents = swift.extract_identifiers("class Foo {}");
    assert!(idents.contains(&"Foo".to_string()), "expected to find 'Foo'");

    // file_defines_any should also succeed for the same snippet
    assert!(swift.file_defines_any("class Foo {}", &["Foo".into()]));
}

#[test]
fn swift_resolve_dependency_is_none() {
    let swift = for_extension("swift").unwrap();

    // default implementation should always return None
    assert!(
        swift
            .resolve_dependency_path("import Foo from './foo.js'", Path::new("."))
            .is_none()
    );
}
