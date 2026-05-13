// crates/lang_support/src/objc.rs

use super::LanguageSupport;
use regex::Regex;
use std::path::{Path, PathBuf};

pub(super) struct ObjCSupport;
pub(super) const OBJC: ObjCSupport = ObjCSupport;

impl LanguageSupport for ObjCSupport {
    fn extract_identifiers(&self, _src: &str) -> Vec<String> {
        Vec::new()
    }

    fn file_defines_any(&self, file_content: &str, idents: &[String]) -> bool {
        idents.iter().any(|ident| {
            let interface_pattern = format!(r"@interface\s+{}\b", regex::escape(ident));
            let implementation_pattern = format!(r"@implementation\s+{}\b", regex::escape(ident));

            Regex::new(&interface_pattern).is_ok_and(|re| re.is_match(file_content))
                || Regex::new(&implementation_pattern).is_ok_and(|re| re.is_match(file_content))
        })
    }

    fn resolve_dependency_path(&self, _line: &str, _current_dir: &Path) -> Option<PathBuf> {
        None
    }
}
