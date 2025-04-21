//! `lang_support` — per‑language helpers that keep the rest of the
//! workspace free of giant `match ext { … }` chains.
//!
//!  * **Zero business‑logic deps** – the crate only knows about source
//!    text, file paths and `regex`.
//!  * **One trait** – `LanguageSupport` – implemented once per language
//!    (Swift, JavaScript, Obj‑C …).  Adding a new language means adding
//!    a single file in this crate.
//!  * **Thin adapter API** – other crates call `lang_support::for_ext()`
//!    and forward the work.

use std::path::{Path, PathBuf};

/// Abstracts the minimum the rest of the tool‑chain needs from a language‑
/// specific helper.
pub trait LanguageSupport: Sync + Send {
    /// Extracts *candidate* identifiers from a chunk of source code.
    ///
    /// Implementations should be cheap – a few regex scans is fine – we do
    /// not need a full parser here.
    fn extract_identifiers(&self, source: &str) -> Vec<String>;

    /// Returns `true` if `file_content` contains a *definition* for **any** of
    /// the supplied identifiers.
    fn file_defines_any(&self, file_content: &str, idents: &[String]) -> bool;

    /// Best‑effort extraction of an import / include path from one line of
    /// source.  If the line does not contain a dependency path understood by
    /// the language, return `None`.
    fn resolve_dependency_path(&self, _line: &str, _current_dir: &Path) -> Option<PathBuf> {
        None
    }
}

/// Returns the [`LanguageSupport`] implementation matching the file extension
/// (e.g. "swift" → Swift support).  Extensions are matched case‑insensitively.
pub fn for_extension(ext: &str) -> Option<&'static dyn LanguageSupport> {
    match ext.to_lowercase().as_str() {
        "swift" => Some(&swift::SWIFT),
        // .js, .jsx, .mjs, .cjs all share the JavaScript rules
        "js" | "jsx" | "mjs" | "cjs" => Some(&javascript::JS),
        // Objective‑C headers and impl files share one matcher
        "h" | "m" => Some(&objc::OBJC),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
//  Sub‑modules (one per language)
// ---------------------------------------------------------------------------

mod swift;
mod javascript;
mod objc;

// Re‑export the trait so call‑sites can `use lang_support::LanguageSupport;`
pub use self::LanguageSupport as _; // underscores to suppress the unused‑import lint
