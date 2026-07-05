// crates/lang_support/src/lib.rs

//! workspace free of giant `match ext { … }` chains.
//!
//!  * **Zero business‑logic deps** – the crate only knows about source
//!    text, file paths and `regex`.
//!  * **One trait** – `LanguageSupport` – implemented once per language
//!    (Swift, JavaScript, Obj‑C …).  Adding a new language means adding
//!    a single file in this crate.
//!  * **Thin adapter API** – other crates call `lang_support::for_ext()`
//!    and forward the work.

use once_cell::sync::Lazy;
use regex::Regex;
use std::fs;
use std::path::{Component, Path, PathBuf};
use todo_marker::TODO_MARKER;
use walkdir::WalkDir;

/// Abstracts the minimum the rest of the tool‑chain needs from a language‑
/// specific helper.
pub trait LanguageSupport: Sync + Send {
    /// Extracts *candidate* identifiers from a chunk of source code.
    fn extract_identifiers(&self, source: &str) -> Vec<String>;

    /// Returns `true` if `file_content` defines **any** of the identifiers.
    fn file_defines_any(&self, file_content: &str, idents: &[String]) -> bool;

    /// Best‑effort extraction of a dependency path from a source line.
    fn resolve_dependency_path(&self, _line: &str, _current_dir: &Path) -> Option<PathBuf> {
        None
    }

    /// Returns `true` when `line` looks like a function or method declaration
    /// that could serve as the "enclosing block" for a TODO marker.
    fn is_function_candidate(&self, _line: &str) -> bool {
        false
    }

    /// Returns `true` when `line` looks like a type declaration (class, enum,
    /// struct) that could serve as an enclosing block for type extraction.
    fn is_type_candidate(&self, _line: &str) -> bool {
        false
    }

    /// Best-effort extraction of an enclosing type name from a source line.
    ///
    /// Returns the name if the line contains a type declaration like
    /// `class Foo`, `struct Bar`, or `enum Baz`.
    fn extract_type_name(&self, _line: &str) -> Option<String> {
        None
    }
}

/// Returns the language helper for a given file extension.
pub fn for_extension(ext: &str) -> Option<&'static dyn LanguageSupport> {
    match ext.to_lowercase().as_str() {
        "swift" => Some(&swift::SWIFT),
        "js" | "jsx" | "mjs" | "cjs" => Some(&javascript::JS),
        "h" | "m" => Some(&objc::OBJC),
        _ => None,
    }
}

/// All file extensions recognised by `for_extension`.
pub fn supported_extensions() -> &'static [&'static str] {
    &["swift", "js", "jsx", "mjs", "cjs", "h", "m"]
}

/// Returns `true` if `line` matches any language's function-candidate pattern.
///
/// Use when the file extension is unknown or when checking across all languages.
pub fn is_function_candidate_any_lang(line: &str) -> bool {
    static ALL: &[&dyn LanguageSupport] = &[&swift::SWIFT, &javascript::JS, &objc::OBJC];
    ALL.iter().any(|lang| lang.is_function_candidate(line))
}

// Matches a bare PascalCase token such as `MyType`.
static GENERIC_SIMPLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"^[A-Z][A-Za-z0-9]+$").unwrap());

/// Extracts language-agnostic, capitalized type-name candidates from source text.
///
/// This is the generic token pass shared across every language: it collects
/// PascalCase tokens while skipping imports and non-TODO comments. It is the
/// sole source of identifiers for languages whose `extract_identifiers` yields
/// nothing (e.g. Obj-C) and for files with an unrecognized extension.
///
/// Bracketed forms such as `[MyType]` are handled implicitly: `generic_tokens`
/// strips the brackets so the inner name is matched as a plain PascalCase token.
///
/// Results are returned in order of first appearance with duplicates removed.
pub fn extract_generic_identifiers(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in source.lines() {
        for token in generic_tokens(line) {
            if GENERIC_SIMPLE_RE.is_match(&token) && !out.contains(&token) {
                out.push(token);
            }
        }
    }
    out
}

/// Splits a single source line into whitespace-separated tokens, after
/// stripping non-alphanumeric characters. Import directives and non-TODO
/// comments produce no tokens; a `TODO_MARKER` prefix is removed so the text
/// following it is still scanned.
fn generic_tokens(line: &str) -> Vec<String> {
    let trimmed = line.trim();

    if trimmed.is_empty()
        || trimmed.starts_with("import ")
        || trimmed.starts_with("#import")
        || trimmed.starts_with("#include")
        || (trimmed.starts_with("//") && !trimmed.starts_with(TODO_MARKER))
    {
        return Vec::new();
    }

    let content = if trimmed.starts_with(TODO_MARKER) {
        trimmed.trim_start_matches(TODO_MARKER).trim_start()
    } else {
        trimmed
    };

    let cleaned: String = content
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { ' ' })
        .collect();

    cleaned.split_whitespace().map(String::from).collect()
}

pub struct SourceFile {
    pub path: PathBuf,
    pub content: String,
    pub language: &'static dyn LanguageSupport,
}

/// Walks a root directory and returns readable files supported by `lang_support`.
///
/// Files inside generated/vendor directories are skipped so definition and
/// reference searches share the same source-file policy.
pub fn walk_source_files(root: impl AsRef<Path>) -> Vec<SourceFile> {
    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter_map(|entry| {
            let path = entry.into_path();
            if has_ignored_component(&path) {
                return None;
            }

            let ext = path.extension().and_then(|s| s.to_str())?;
            let language = for_extension(ext)?;
            let content = fs::read_to_string(&path).ok()?;

            Some(SourceFile {
                path,
                content,
                language,
            })
        })
        .collect()
}

fn has_ignored_component(path: &Path) -> bool {
    path.components().any(|component| match component {
        Component::Normal(name) => {
            let name = name.to_string_lossy();
            name == ".build" || name == "Pods"
        }
        _ => false,
    })
}

// ---------------------------------------------------------------------------
//  One sub‑module per language
// ---------------------------------------------------------------------------
mod javascript;
mod objc;
mod swift;

// Re‑export the trait so callers can `use lang_support::LanguageSupport;`
pub use self::LanguageSupport as _;

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn ignored_component_policy_matches_source_searches() {
        assert!(has_ignored_component(Path::new("Root/.build/File.swift")));
        assert!(has_ignored_component(Path::new("Root/Pods/File.swift")));
        assert!(!has_ignored_component(Path::new("Root/Sources/File.swift")));
    }

    #[test]
    fn walk_source_files_returns_supported_readable_files() {
        let dir = tempdir().expect("Failed to create temp dir");
        let root = dir.path();

        fs::write(root.join("Model.swift"), "class Model {}\n").expect("Failed to write Swift");
        fs::write(root.join("Component.jsx"), "class Component {}\n").expect("Failed to write JSX");
        fs::write(root.join("README.txt"), "class Ignored {}\n").expect("Failed to write text");

        let build_dir = root.join(".build");
        fs::create_dir(&build_dir).expect("Failed to create .build");
        fs::write(build_dir.join("Generated.swift"), "class Generated {}\n")
            .expect("Failed to write generated file");

        let files = walk_source_files(root);
        let names: BTreeSet<_> = files
            .iter()
            .map(|source_file| {
                source_file
                    .path
                    .file_name()
                    .expect("Expected filename")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();

        assert_eq!(
            names,
            BTreeSet::from(["Component.jsx".to_string(), "Model.swift".to_string()])
        );
        assert!(files.iter().any(|source_file| source_file
            .language
            .file_defines_any(&source_file.content, &["Model".to_string()])));
    }
}

/// Tests for the language-agnostic generic identifier extractor. These were
/// relocated from `extract_types`'s `TypeExtractor` when that logic moved here,
/// so the generic token pass has a single owner.
#[cfg(test)]
mod generic_identifier_tests {
    use super::*;
    use std::collections::BTreeSet;

    fn as_set(idents: Vec<String>) -> BTreeSet<String> {
        idents.into_iter().collect()
    }

    #[test]
    fn simple_pascalcase_token_is_extracted() {
        assert_eq!(extract_generic_identifiers("MyType"), vec!["MyType"]);
    }

    #[test]
    fn generic_tokens_returns_empty_for_non_eligible_lines() {
        assert!(generic_tokens("").is_empty());
        assert!(generic_tokens("   ").is_empty());
        assert!(generic_tokens("import Foundation").is_empty());
        assert!(generic_tokens("#import <Foundation/Foundation.h>").is_empty());
        assert!(generic_tokens("#include <stdio.h>").is_empty());
        assert!(generic_tokens("// comment").is_empty());
    }

    #[test]
    fn generic_tokens_splits_and_cleans_input() {
        assert_eq!(
            generic_tokens("MyClass,struct MyStruct"),
            vec!["MyClass", "struct", "MyStruct"]
        );
    }

    #[test]
    fn generic_tokens_strips_todo_marker_prefix() {
        assert_eq!(
            generic_tokens("// TODO: - MyTriggeredType"),
            vec!["MyTriggeredType"]
        );
    }

    #[test]
    fn extracts_type_names_from_declarations() {
        let source = "class MyClass {}\nstruct MyStruct {}\nenum MyEnum {}";
        assert_eq!(
            as_set(extract_generic_identifiers(source)),
            BTreeSet::from([
                "MyClass".to_string(),
                "MyEnum".to_string(),
                "MyStruct".to_string()
            ])
        );
    }

    #[test]
    fn extracts_type_name_from_bracket_notation() {
        assert_eq!(
            extract_generic_identifiers("let array: [CustomType] = []"),
            vec!["CustomType"]
        );
    }

    #[test]
    fn extracts_mixed_tokens_on_one_line() {
        assert_eq!(
            as_set(extract_generic_identifiers(
                "class MyClass, struct MyStruct; enum MyEnum."
            )),
            BTreeSet::from([
                "MyClass".to_string(),
                "MyEnum".to_string(),
                "MyStruct".to_string()
            ])
        );
    }

    #[test]
    fn deduplicates_repeated_type_names() {
        let source = "class DuplicateType {}\nstruct DuplicateType {}\nenum DuplicateType {}";
        assert_eq!(
            extract_generic_identifiers(source),
            vec!["DuplicateType".to_string()]
        );
    }

    #[test]
    fn splits_underscored_names_and_drops_lowercase_tokens() {
        // `My_Class` splits on the underscore into `My` and `Class`; the joined
        // `My_Class` token never survives, and lowercase words are dropped.
        let result = as_set(extract_generic_identifiers("class My_Class {}"));
        assert!(!result.contains("My_Class"));
        assert_eq!(
            result,
            BTreeSet::from(["My".to_string(), "Class".to_string()])
        );
        assert!(extract_generic_identifiers("let x = 5").is_empty());
    }
}
