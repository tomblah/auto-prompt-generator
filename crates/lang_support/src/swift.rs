use super::LanguageSupport;

pub(super) struct SwiftSupport;
pub(super) const SWIFT: SwiftSupport = SwiftSupport;

impl LanguageSupport for SwiftSupport {
    fn extract_identifiers(&self, _src: &str) -> Vec<String> { Vec::new() }
    fn file_defines_any(&self, _file: &str, _ids: &[String]) -> bool { false }
}
