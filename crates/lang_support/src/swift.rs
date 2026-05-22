// crates/lang_support/src/swift.rs

//!
//! * `extract_identifiers` -- very similar to the old `TypeExtractor`: grabs
//!   capitalised type names **and** unqualified function calls so that helper
//!   methods (`foo()` -> `func foo`) are pulled in.
//! * `file_defines_any`    -- mirrors the old `SwiftMatcher`: reports *true* if
//!   the file declares **any** of the requested identifiers.

use super::LanguageSupport;
use once_cell::sync::Lazy;
use regex::Regex;

pub(super) struct SwiftSupport;
pub(super) const SWIFT: SwiftSupport = SwiftSupport;

// ---------------------------------------------------------------------------
//  Regexes
// ---------------------------------------------------------------------------

// Matches `class Foo`, `struct Bar`, `enum Baz`, `protocol Qux`, `typealias Zap`
static DECL_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:class|struct|enum|protocol|typealias)\s+([A-Z][A-Za-z0-9_]*)").unwrap()
});

static SWIFT_FUNCTION_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"^\s*(?:(?:public|private|internal|fileprivate)\s+)?func\s+\w+(?:<[^>]+>)?\s*\([^)]*\)\s*(?:->\s*\S+)?\s*\{"#,
    )
    .unwrap()
});

static SWIFT_CLASS_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^\s*class\s+\w+.*\{"#).unwrap());

static SWIFT_ENUM_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r#"^\s*enum\s+\w+.*\{"#).unwrap());

static TYPE_NAME_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(class|struct|enum)\s+(\w+)").unwrap());

// Matches a *call-site* that looks like `identifier(`
static CALL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(").unwrap());

// Reserved words & common keywords we don't want as identifiers
static RESERVED: &[&str] = &[
    "if", "for", "while", "switch", "guard", "return", "catch", "throw", "init", "deinit",
];

fn is_reserved(word: &str) -> bool {
    RESERVED.binary_search(&word).is_ok()
}

// ---------------------------------------------------------------------------
//  Trait impl
// ---------------------------------------------------------------------------

impl LanguageSupport for SwiftSupport {
    /// Collects type names *and* free function / static method identifiers.
    fn extract_identifiers(&self, src: &str) -> Vec<String> {
        let mut out = Vec::new();

        for cap in DECL_RE.captures_iter(src) {
            let ident = &cap[1];
            if !out.contains(&ident.to_string()) {
                out.push(ident.to_string());
            }
        }

        for cap in CALL_RE.captures_iter(src) {
            let ident = &cap[1];
            if !is_reserved(ident)
                && ident
                    .chars()
                    .next()
                    .map(|c| c.is_ascii_lowercase())
                    .unwrap_or(false)
                && !out.contains(&ident.to_string())
            {
                out.push(ident.to_string());
            }
        }

        out
    }

    /// Returns *true* if the file defines **any** of the requested identifiers.
    fn file_defines_any(&self, file_content: &str, idents: &[String]) -> bool {
        for ident in idents {
            let pattern = format!(
                r"\b(?:class|struct|enum|protocol|typealias)\s+{}\b",
                regex::escape(ident)
            );
            if Regex::new(&pattern)
                .map(|re| re.is_match(file_content))
                .unwrap_or(false)
            {
                return true;
            }

            let func_pat = format!(r"\bfunc\s+{}\s*\(", regex::escape(ident));
            if Regex::new(&func_pat)
                .map(|re| re.is_match(file_content))
                .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }

    fn is_function_candidate(&self, line: &str) -> bool {
        SWIFT_FUNCTION_RE.is_match(line)
    }

    fn is_type_candidate(&self, line: &str) -> bool {
        SWIFT_CLASS_RE.is_match(line) || SWIFT_ENUM_RE.is_match(line)
    }

    fn extract_type_name(&self, line: &str) -> Option<String> {
        TYPE_NAME_RE
            .captures(line)
            .and_then(|caps| caps.get(2))
            .map(|m| m.as_str().to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_candidate_plain() {
        assert!(SWIFT.is_function_candidate("func doSomething() {"));
    }

    #[test]
    fn function_candidate_public() {
        assert!(SWIFT.is_function_candidate("public func doSomething() {"));
    }

    #[test]
    fn function_candidate_with_return() {
        assert!(SWIFT.is_function_candidate("func doSomething() -> Bool {"));
    }

    #[test]
    fn function_candidate_with_generics() {
        assert!(SWIFT.is_function_candidate("func doSomething<T>(value: T) {"));
    }

    #[test]
    fn function_candidate_rejects_class() {
        assert!(!SWIFT.is_function_candidate("class MyClass {"));
    }

    #[test]
    fn type_candidate_class() {
        assert!(SWIFT.is_type_candidate("class MyClass {"));
    }

    #[test]
    fn type_candidate_enum() {
        assert!(SWIFT.is_type_candidate("enum MyEnum {"));
    }

    #[test]
    fn type_candidate_rejects_struct() {
        assert!(!SWIFT.is_type_candidate("struct MyStruct {"));
    }

    #[test]
    fn type_candidate_rejects_func() {
        assert!(!SWIFT.is_type_candidate("func doSomething() {"));
    }

    #[test]
    fn extract_type_name_class() {
        assert_eq!(
            SWIFT.extract_type_name("class MyClass {"),
            Some("MyClass".to_string())
        );
    }

    #[test]
    fn extract_type_name_struct() {
        assert_eq!(
            SWIFT.extract_type_name("struct MyStruct {"),
            Some("MyStruct".to_string())
        );
    }

    #[test]
    fn extract_type_name_enum() {
        assert_eq!(
            SWIFT.extract_type_name("enum MyEnum {"),
            Some("MyEnum".to_string())
        );
    }

    #[test]
    fn extract_type_name_none_for_func() {
        assert_eq!(SWIFT.extract_type_name("func doSomething() {"), None);
    }
}
