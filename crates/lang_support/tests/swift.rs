// crates/lang_support/tests/swift.rs

use lang_support::for_extension;
use std::path::Path;

#[test]
fn swift_extracts_class_name() {
    let swift = for_extension("swift").unwrap();

    let idents = swift.extract_identifiers("class Foo {}");
    assert!(
        idents.contains(&"Foo".to_string()),
        "expected to find 'Foo'"
    );

    // file_defines_any should also succeed for the same snippet
    assert!(swift.file_defines_any("class Foo {}", &["Foo".into()]));
}

#[test]
fn swift_resolve_dependency_is_none() {
    let swift = for_extension("swift").unwrap();

    // default implementation should always return None
    assert!(swift
        .resolve_dependency_path("import Foo from './foo.js'", Path::new("."))
        .is_none());
}

#[test]
fn swift_extracts_function_call_identifiers() {
    let swift = for_extension("swift").unwrap();

    let idents = swift.extract_identifiers(
        r#"
class Widget {}

func load() {
    fetchData()
    renderWidget()
}
"#,
    );

    assert!(idents.contains(&"Widget".to_string()));
    assert!(idents.contains(&"fetchData".to_string()));
    assert!(idents.contains(&"renderWidget".to_string()));
}

#[test]
fn swift_skips_reserved_words_as_call_identifiers() {
    let swift = for_extension("swift").unwrap();

    let idents = swift.extract_identifiers(
        r#"
if ready {
    guard isValid() else { return }
    switch state { default: break }
}
"#,
    );

    assert!(!idents.contains(&"if".to_string()));
    assert!(!idents.contains(&"guard".to_string()));
    assert!(!idents.contains(&"return".to_string()));
    assert!(idents.contains(&"isValid".to_string()));
}

#[test]
fn swift_file_defines_free_function_identifier() {
    let swift = for_extension("swift").unwrap();

    assert!(swift.file_defines_any("func fetchData() -> Data { Data() }", &["fetchData".into()]));
    assert!(!swift.file_defines_any(
        "func fetchData() -> Data { Data() }",
        &["renderWidget".into()]
    ));
}
