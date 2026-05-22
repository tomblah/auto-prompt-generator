// crates/lang_support/src/objc.rs

use super::LanguageSupport;
use once_cell::sync::Lazy;
use regex::Regex;
use std::path::{Path, PathBuf};

pub(super) struct ObjCSupport;
pub(super) const OBJC: ObjCSupport = ObjCSupport;

static OBJC_METHOD_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^\s*[-+]\s*\([^)]*\)\s*[a-zA-Z_][a-zA-Z0-9_]*(?::\s*\([^)]*\)\s*[a-zA-Z_][a-zA-Z0-9_]*)*\s*\{"#).unwrap()
});

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

    fn is_function_candidate(&self, line: &str) -> bool {
        OBJC_METHOD_RE.is_match(line)
    }

    fn resolve_dependency_path(&self, _line: &str, _current_dir: &Path) -> Option<PathBuf> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn function_candidate_instance_method() {
        assert!(OBJC.is_function_candidate("- (void)myMethod:(NSString *)arg {"));
    }

    #[test]
    fn function_candidate_class_method() {
        assert!(OBJC.is_function_candidate("+ (instancetype)sharedInstance {"));
    }

    #[test]
    fn function_candidate_no_params() {
        assert!(OBJC.is_function_candidate("- (void)viewDidLoad {"));
    }

    #[test]
    fn function_candidate_rejects_plain_code() {
        assert!(!OBJC.is_function_candidate("NSLog(@\"hello\");"));
    }
}
