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

use std::fs;
use std::path::{Component, Path, PathBuf};
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
