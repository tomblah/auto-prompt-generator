// crates/lang_support/src/lib.rs

//! workspace free of giant `match ext { … }` chains.
//!
//!  * **Zero business-logic deps** – the crate only knows about source
//!    text, file paths and `regex`.
//!  * **One trait** – `LanguageSupport` – implemented once per language
//!    (Swift, JavaScript, Obj-C …).  Adding a new language means adding
//!    a single file in this crate.
//!  * **Thin adapter API** – other crates call `lang_support::for_extension()`
//!    and forward the work.

use std::path::{Path, PathBuf};

/// Abstracts the minimum the rest of the tool-chain needs from a language-
/// specific helper.
pub trait LanguageSupport: Sync + Send {
    /// Extracts *candidate* identifiers from a chunk of source code.
    fn extract_identifiers(&self, source: &str) -> Vec<String>;

    /// Returns `true` if `file_content` defines **any** of the identifiers.
    fn file_defines_any(&self, file_content: &str, idents: &[String]) -> bool;

    /// Walk the **direct, relative dependencies** of `file_path` and return
    /// absolute paths to files that should be pulled into the prompt.
    ///
    /// * Implementations may normalise or filter paths, but everything returned
    ///   **must** live inside `search_root` and point to an actual file.
    /// * The default impl returns an empty list so languages that don’t embed
    ///   import paths (Swift / Obj-C) don’t need to override it.
    fn walk_dependencies(
        &self,
        _file_path: &Path,
        _search_root: &Path,
    ) -> Vec<PathBuf> {
        Vec::new()
    }

    /// Best-effort extraction of a dependency path from a single source line.
    /// Languages like JavaScript override this; others can ignore it.
    fn resolve_dependency_path(&self, _line: &str, _current_dir: &Path) -> Option<PathBuf> {
        None
    }
}

/// Returns the language helper for a given file extension.
pub fn for_extension(ext: &str) -> Option<&'static dyn LanguageSupport> {
    match ext.to_lowercase().as_str() {
        "swift"                     => Some(&swift::SWIFT),
        "js" | "jsx" | "mjs" | "cjs" => Some(&javascript::JS),
        "h" | "m"                   => Some(&objc::OBJC),
        _                           => None,
    }
}

// ---------------------------------------------------------------------------
//  One sub-module per language
// ---------------------------------------------------------------------------
mod swift;
mod javascript;
mod objc;

// Re-export the trait so callers can `use lang_support::LanguageSupport;`
pub use self::LanguageSupport as _;
