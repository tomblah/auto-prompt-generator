// crates/lang_support/tests/javascript.rs

use lang_support::for_extension;

#[test]
fn extract_identifiers_finds_calls_and_classes() {
    let js = for_extension("js").unwrap();
    let src = r#"
        function top() {}
        async function ignored() {}
        const foo = () => {}
        new Foo();
        bar();
        Baz();
        if (condition) { whileLoop(); }
    "#;

    let mut idents = js.extract_identifiers(src);
    idents.sort_unstable();

    let must_have = ["bar", "Baz", "Foo"];
    for name in &must_have {
        assert!(idents.contains(&name.to_string()), "missing {}", name);
    }
}

#[test]
fn file_defines_any_matches_various_forms() {
    let js = for_extension("js").unwrap();
    let want = vec!["helper".to_string(), "Whatever".to_string()];

    let cases = [
        "function helper() {}",
        "const helper = () => {}",
        "export function helper() {}",
        "exports.helper = () => {}",
        "module.exports = helper",
        "class Whatever {}",
    ];

    for snippet in &cases {
        assert!(
            js.file_defines_any(snippet, &want),
            "Did not match snippet: {:?}",
            snippet
        );
    }
}

#[test]
fn for_extension_dispatches() {
    assert!(for_extension("JSx").is_some());
    assert!(for_extension("swift").is_some());
    assert!(for_extension("h").is_some());   // Obj‑C
    assert!(for_extension("unknown").is_none());
}

#[test]
fn resolve_dependency_path_handles_import_and_require() {
    let js = for_extension("js").unwrap();

    // temp dir to act as "current_dir"
    let dir = tempfile::TempDir::new().unwrap();
    let cur   = dir.path();

    // 1️⃣  ES-module import ------------------------------------------
    let line_import = r#"import Foo from './foo.js';"#;
    let p = js
        .resolve_dependency_path(line_import, cur)
        .expect("should match import");
    assert_eq!(p, cur.join("foo.js"));

    // 2️⃣  CommonJS require ------------------------------------------
    let line_require = r#"const baz = require('./bar');"#;
    let p2 = js
        .resolve_dependency_path(line_require, cur)
        .expect("should match require");
    assert_eq!(p2, cur.join("bar"));

    // 3️⃣  Negative case ---------------------------------------------
    let none_line = "console.log('no dep here');";
    assert!(js.resolve_dependency_path(none_line, cur).is_none());
}
