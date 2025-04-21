//! JavaScript implementation of the `LanguageSupport` trait.
//!
//! Currently a placeholder: all methods return empty results so that the
//! crate builds.  We’ll port the real identifier‑extraction and definition‑
//! matching logic here next.

use super::LanguageSupport;
use std::path::{Path, PathBuf};

pub(super) struct JavaScriptSupport;
pub(super) const JS: JavaScriptSupport = JavaScriptSupport;

impl LanguageSupport for JavaScriptSupport {
    fn extract_identifiers(&self, _src: &str) -> Vec<String> {
        Vec::new()
    }

    fn file_defines_any(&self, _file_content: &str, _idents: &[String]) -> bool {
        false
    }

    fn resolve_dependency_path(
        &self,
        _line: &str,
        _current_dir: &Path,
    ) -> Option<PathBuf> {
        None
    }
}
