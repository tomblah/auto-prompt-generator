// crates/lang_support/src/swift.rs

//!
//! * `extract_identifiers` – very similar to the old `TypeExtractor`: grabs
//!   capitalised type names **and** unqualified function calls so that helper
//!   methods (`foo()` → `func foo`) are pulled in.
//! * `file_defines_any`    – mirrors the old `SwiftMatcher`: reports *true* if
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
    Regex::new(
        r"\b(?:class|struct|enum|protocol|typealias)\s+([A-Z][A-Za-z0-9_]*)",
    )
    .unwrap()
});

// Matches a *call‑site* that looks like `identifier(`
static CALL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(").unwrap());

// Reserved words & common keywords we don’t want as identifiers
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

        // 1️⃣  Type & protocol declarations *inside* the TODO file itself
        for cap in DECL_RE.captures_iter(src) {
            let ident = &cap[1];
            if !out.contains(&ident.to_string()) {
                out.push(ident.to_string());
            }
        }

        // 2️⃣  Function calls – this is what lets us pull helper files in
        for cap in CALL_RE.captures_iter(src) {
            let ident = &cap[1];
            if !is_reserved(ident) && ident.chars().next().map(|c| c.is_ascii_lowercase()).unwrap_or(false)
            {
                if !out.contains(&ident.to_string()) {
                    out.push(ident.to_string());
                }
            }
        }

        out
    }

    /// Returns *true* if the file defines **any** of the requested identifiers.
    fn file_defines_any(&self, file_content: &str, idents: &[String]) -> bool {
        for ident in idents {
            // look for `class Foo`, `struct Foo`, `enum Foo`, `protocol Foo`
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

            // also treat a free‑standing `func foo(` as a definition
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

    // Swift doesn’t use explicit import paths the way JS does, so we leave
    // `resolve_dependency_path` as the default `None`.
}
