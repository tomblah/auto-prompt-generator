// crates/lang_support/src/objc.rs

//!
//! Placeholder only: returns empty results so the workspace builds. We’ll
//! migrate the real Obj‑C definition/identifier logic here later.

use super::LanguageSupport;
use std::path::{Path, PathBuf};

pub(super) struct ObjCSupport;
pub(super) const OBJC: ObjCSupport = ObjCSupport;

impl LanguageSupport for ObjCSupport {
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
