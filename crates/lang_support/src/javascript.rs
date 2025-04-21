//! JavaScript implementation of `LanguageSupport`.
//!
//! The goals mirror the Swift support you just ported:
//! * **extract_identifiers** – pull out function call‑sites *and* class names
//!   so that helpers referenced by the TODO file are included.
//! * **file_defines_any**   – true if the file contains a matching declaration
//!   (`function foo`, `class Bar`, `exports.foo = …`, etc.).
//! * **resolve_dependency_path** – best‑effort: when we encounter an
//!   `import` or `require` line, resolve the relative path so the caller
//!   can include that file immediately.

use super::LanguageSupport;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};

pub(super) struct JavaScriptSupport;
pub(super) const JS: JavaScriptSupport = JavaScriptSupport;

// ---------------------------------------------------------------------------
//  Regexes
// ---------------------------------------------------------------------------

// Function call (identifier followed by '(' – excludes 'if(', 'for(' …)
static CALL_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b([A-Za-z_][A-Za-z0-9_]*)\s*\(").unwrap());

// Class instantiation `new Foo(` or declaration `class Foo {`
static CLASS_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:new\s+)?([A-Z][A-Za-z0-9_]*)\s*\(").unwrap());

// ---------------------------------------------------------------------------
//  Reserved words we don't want as identifiers
// ---------------------------------------------------------------------------
static RESERVED: &[&str] = &[
    "if", "for", "while", "switch", "catch", "function", "return",
    "class", "new", "await", "async", "const", "let", "var",
];

fn is_reserved(w: &str) -> bool {
    RESERVED.binary_search(&w).is_ok()
}

// ---------------------------------------------------------------------------
//  Trait implementation
// ---------------------------------------------------------------------------

impl LanguageSupport for JavaScriptSupport {
    fn extract_identifiers(&self, src: &str) -> Vec<String> {
        let mut out = Vec::new();

        // Function / method calls
        for cap in CALL_RE.captures_iter(src) {
            let ident = &cap[1];
            if !is_reserved(ident) && ident.chars().next().unwrap_or(' ').is_ascii_lowercase() {
                if !out.contains(&ident.to_string()) {
                    out.push(ident.to_string());
                }
            }
        }

        // Capitalised class names
        for cap in CLASS_RE.captures_iter(src) {
            let ident = &cap[1];
            if !out.contains(&ident.to_string()) {
                out.push(ident.to_string());
            }
        }

        out
    }

    fn file_defines_any(&self, content: &str, idents: &[String]) -> bool {
        for ident in idents {
            // 1. Traditional function declaration
            let fn_decl = format!(r"\bfunction\s+{}\b", regex::escape(ident));
            if Regex::new(&fn_decl).unwrap().is_match(content) {
                return true;
            }

            // 2. const/let/var foo = function | async function | () =>
            let assign = format!(
                r"\b(?:const|let|var)\s+{}\s*=\s*(?:async\s+)?(?:function\b|\()",
                regex::escape(ident)
            );
            if Regex::new(&assign).unwrap().is_match(content) {
                return true;
            }

            // 3. ES module export
            let es_export_fn =
                format!(r"\bexport\s+(?:async\s+)?function\s+{}\b", regex::escape(ident));
            if Regex::new(&es_export_fn).unwrap().is_match(content) {
                return true;
            }
            let es_export_name =
                format!(r"\bexport\s+\{{[^}}]*\b{}\b[^}}]*\}}", regex::escape(ident));
            if Regex::new(&es_export_name).unwrap().is_match(content) {
                return true;
            }

            // 4. CommonJS exports
            let cjs_default =
                format!(r"\bmodule\.exports\s*=\s*{}\b", regex::escape(ident));
            if Regex::new(&cjs_default).unwrap().is_match(content) {
                return true;
            }
            let cjs_named =
                format!(r"\bexports\.{}\s*=", regex::escape(ident));
            if Regex::new(&cjs_named).unwrap().is_match(content) {
                return true;
            }

            // 5. Class declaration
            let class_decl =
                format!(r"\bclass\s+{}\b", regex::escape(ident));
            if Regex::new(&class_decl).unwrap().is_match(content) {
                return true;
            }
        }
        false
    }

    fn resolve_dependency_path(
        &self,
        line: &str,
        current_dir: &Path,
    ) -> Option<PathBuf> {
        // import Foo from './foo.js'
        if let Some(cap) = Regex::new(r#"from\s+['"]([^'"]+)['"]"#)
            .unwrap()
            .captures(line)
        {
            return Some(current_dir.join(cap[1].to_string()));
        }

        // const foo = require('./foo')
        if let Some(cap) = Regex::new(r#"require\(\s*['"]([^'"]+)['"]\s*\)"#)
            .unwrap()
            .captures(line)
        {
            return Some(current_dir.join(cap[1].to_string()));
        }

        None
    }
}
