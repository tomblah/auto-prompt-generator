// crates/extract_types/src/lib.rs

use anyhow::{Context, Result};
use regex::Regex;
use std::collections::BTreeSet;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use lang_support::for_extension;
use substring_marker_snippet_extractor::FileAnalysis;
use todo_marker::{TODO_MARKER, TODO_MARKER_WS};

fn is_type_candidate_line(line: &str) -> bool {
    for_extension("swift").is_some_and(|lang| lang.is_type_candidate(line))
}

/// ---------------------------------------------------------------------------
///  Token extraction helper (legacy logic)
/// ---------------------------------------------------------------------------
struct TypeExtractor {
    re_simple: Regex,
    re_bracket: Regex,
}

impl TypeExtractor {
    fn new() -> Result<Self, regex::Error> {
        Ok(Self {
            re_simple: Regex::new(r"^[A-Z][A-Za-z0-9]+$")?,
            re_bracket: Regex::new(r"^\[([A-Z][A-Za-z0-9]+)\]$")?,
        })
    }

    fn extract_tokens(&self, line: &str) -> Option<Vec<String>> {
        let trimmed = line.trim();

        if trimmed.is_empty()
            || trimmed.starts_with("import ")
            || trimmed.starts_with("#import")
            || trimmed.starts_with("#include")
            || (trimmed.starts_with("//") && !trimmed.starts_with(TODO_MARKER))
        {
            return None;
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
        let cleaned = cleaned.trim();

        Some(cleaned.split_whitespace().map(String::from).collect())
    }

    fn extract_types<I>(&self, lines: I) -> BTreeSet<String>
    where
        I: Iterator<Item = String>,
    {
        let mut types = BTreeSet::new();
        for line in lines {
            if let Some(tokens) = self.extract_tokens(&line) {
                for token in tokens {
                    if self.re_simple.is_match(&token) {
                        types.insert(token);
                    } else if let Some(caps) = self.re_bracket.captures(&token) {
                        if let Some(inner) = caps.get(1) {
                            types.insert(inner.as_str().to_string());
                        }
                    }
                }
            }
        }
        types
    }
}

/// ---------------------------------------------------------------------------
///  Public API
/// ---------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, Default)]
pub struct ExtractTypesOptions {
    pub targeted: bool,
}

pub fn extract_types_from_file<P: AsRef<Path>>(swift_file: P) -> Result<BTreeSet<String>> {
    extract_types_from_file_with_options(swift_file, &ExtractTypesOptions::default())
}

pub fn extract_types_from_file_with_options<P: AsRef<Path>>(
    swift_file: P,
    options: &ExtractTypesOptions,
) -> Result<BTreeSet<String>> {
    let full_content = fs::read_to_string(&swift_file)
        .with_context(|| format!("Failed to open file {}", swift_file.as_ref().display()))?;

    // Decide which slice of the file to analyse
    let content_slice = if options.targeted {
        if let Some(inner) = extract_inner_block_from_content(&full_content) {
            inner
        } else {
            full_content.clone()
        }
    } else {
        let analysis = FileAnalysis::new(&full_content);
        if analysis.has_markers() {
            let mut filtered = analysis.filtered_content("");
            if !filtered.contains(TODO_MARKER) {
                if let Some(enclosing) = analysis.enclosing_block(Some(&is_type_candidate_line)) {
                    filtered.push('\n');
                    filtered.push_str(&enclosing);
                }
            }
            filtered
        } else {
            full_content.clone()
        }
    };

    // 1️⃣  Legacy TypeExtractor on the slice
    let reader = BufReader::new(content_slice.as_bytes());
    let extractor = TypeExtractor::new()?;
    let mut all_types: BTreeSet<String> =
        extractor.extract_types(reader.lines().map_while(Result::ok));

    // 2️⃣  Language‑specific extraction on the SAME slice
    if let Some(ext) = swift_file.as_ref().extension().and_then(|s| s.to_str()) {
        if let Some(lang) = for_extension(ext) {
            for ident in lang.extract_identifiers(&content_slice) {
                all_types.insert(ident);
            }
        }
    }

    Ok(all_types)
}

/// ---------------------------------------------------------------------------
///  Helper: extract inner block (targeted mode, unchanged)
/// ---------------------------------------------------------------------------
fn extract_inner_block_from_content(content: &str) -> Option<String> {
    let pos = content.find(TODO_MARKER_WS)?;
    let mut stack = Vec::new();

    for (i, ch) in content[..pos].char_indices() {
        if ch == '{' {
            stack.push(i);
        } else if ch == '}' {
            stack.pop();
        }
    }
    let open_brace = stack.pop()?;

    let mut brace_count = 1;
    let mut index = open_brace + 1;
    let bytes = content.as_bytes();
    while index < content.len() && brace_count > 0 {
        match bytes[index] {
            b'{' => brace_count += 1,
            b'}' => brace_count -= 1,
            _ => {}
        }
        index += 1;
    }

    if brace_count == 0 {
        Some(content[open_brace + 1..index - 1].to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::collections::BTreeSet;
    use std::env;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn types(items: &[&str]) -> BTreeSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_extract_inner_block_success() {
        let content = r#"
            func foo() {
                class OuterType {}
                {
                    class InnerType {}
                    // TODO: - Do something important
                }
            }
        "#;
        let inner = extract_inner_block_from_content(content);
        assert!(inner.is_some());
        let inner_str = inner.unwrap();
        // Ensure that the extracted inner block contains the inner declaration and the TODO marker,
        // and that it does not include "OuterType".
        assert!(
            inner_str.contains("class InnerType"),
            "Extracted block: {}",
            inner_str
        );
        assert!(
            inner_str.contains("// TODO: -"),
            "Extracted block: {}",
            inner_str
        );
        assert!(
            !inner_str.contains("OuterType"),
            "Extracted block should not contain OuterType: {}",
            inner_str
        );
    }

    #[test]
    fn test_extract_inner_block_no_marker() {
        let content = "func foo() { class InnerType {} }";
        assert!(extract_inner_block_from_content(content).is_none());
    }

    #[test]
    fn test_extract_types_returns_empty_for_file_with_no_capitalized_words() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import foundation\nlet x = 5")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_extract_types_returns_error_for_missing_file() {
        let result = extract_types_from_file("missing.swift");
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_types_extracts_capitalized_words() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(
            swift_file,
            "import Foundation\nclass MyClass {{}}\nstruct MyStruct {{}}\nenum MyEnum {{}}"
        )?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result, types(&["MyClass", "MyEnum", "MyStruct"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_extracts_type_names_from_bracket_notation() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import UIKit\nlet array: [CustomType] = []")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result, types(&["CustomType"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_deduplicates_type_names() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class DuplicateType {{}}")?;
        writeln!(swift_file, "struct DuplicateType {{}}")?;
        writeln!(swift_file, "enum DuplicateType {{}}")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result, types(&["DuplicateType"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_mixed_tokens_in_one_line() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class MyClass, struct MyStruct; enum MyEnum.")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result, types(&["MyClass", "MyEnum", "MyStruct"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_with_underscores() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "class My_Class {{}}")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert!(
            !result.contains("My_Class"),
            "Found token 'My_Class', which should have been split."
        );
        Ok(())
    }

    #[test]
    fn test_extract_types_trailing_punctuation() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "enum MyEnum.")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result, types(&["MyEnum"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_includes_trigger_comment() -> Result<()> {
        let mut swift_file = NamedTempFile::new()?;
        writeln!(swift_file, "import Foundation\n// TODO: - TriggeredType")?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result, types(&["TriggeredType"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_with_substring_markers() -> Result<()> {
        let swift_content = r#"
            // This type is outside the markers and should be ignored.
            class OutsideType {}
            // v
            // Only this section should be processed:
            class InsideType {}
            // ^
            // This type is also outside and should be ignored.
            class OutsideType2 {}
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result, types(&["InsideType"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_with_markers_and_enclosing_todo() -> Result<()> {
        let swift_content = r#"
            // v
            let foo = TypeThatIsInsideMarker()
            // ^
            
            let bar = TypeThatIsOutSideMarker()
            
            func hello() {
                let hi = TypeThatIsInsideEnclosingFunction()
                // TODO: - example
            }
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(
            result,
            types(&[
                "TypeThatIsInsideEnclosingFunction",
                "TypeThatIsInsideMarker"
            ])
        );
        Ok(())
    }

    #[test]
    fn test_extract_types_targeted_mode() -> Result<()> {
        let swift_content = r#"
            class OuterType {}
            func testFunction() {
                class InnerType {}
                // TODO: - Perform action
            }
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file_with_options(
            swift_file.path(),
            &ExtractTypesOptions { targeted: true },
        )?;
        assert_eq!(result, types(&["InnerType", "Perform"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_targeted_mode_no_enclosing_block() -> Result<()> {
        let swift_content = r#"
            class OuterType {}
            // TODO: - Some todo
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file_with_options(
            swift_file.path(),
            &ExtractTypesOptions { targeted: true },
        )?;
        assert_eq!(result, types(&["OuterType", "Some"]));
        Ok(())
    }

    #[test]
    fn test_extract_types_default_ignores_targeted_env() -> Result<()> {
        env::set_var("TARGETED", "1");
        let swift_content = r#"
            class OuterType {}
            func testFunction() {
                class InnerType {}
                // TODO: - Perform action
            }
        "#;
        let mut swift_file = NamedTempFile::new()?;
        write!(swift_file, "{}", swift_content)?;
        let result = extract_types_from_file(swift_file.path())?;
        assert_eq!(result, types(&["InnerType", "OuterType", "Perform"]));
        env::remove_var("TARGETED");
        Ok(())
    }

    #[test]
    fn test_extract_enclosing_block_skips_todo_inside_markers() {
        let content = "\
// v\n\
func markedFunction() {\n\
    let value = MarkedType()\n\
    // TODO: - Ignore marked block\n\
}\n\
// ^";

        let analysis = FileAnalysis::new(content);
        assert!(analysis
            .enclosing_block(Some(&is_type_candidate_line))
            .is_none());
    }

    #[test]
    fn test_extract_inner_block_returns_none_for_unclosed_block() {
        let content = "\
func testFunction() {\n\
    class InnerType {}\n\
    // TODO: - Missing close\n";

        assert!(extract_inner_block_from_content(content).is_none());
    }
}

#[cfg(test)]
mod type_extractor_tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_type_extractor_new() {
        let extractor = TypeExtractor::new().expect("Failed to create TypeExtractor");
        assert!(extractor.re_simple.is_match("MyType"));
    }

    #[test]
    fn test_extract_tokens_returns_none_for_empty_or_non_eligible_lines() {
        let extractor = TypeExtractor::new().unwrap();
        assert!(extractor.extract_tokens("").is_none());
        assert!(extractor.extract_tokens("   ").is_none());
        assert!(extractor.extract_tokens("import Foundation").is_none());
        assert!(extractor.extract_tokens("// comment").is_none());
    }

    #[test]
    fn test_extract_tokens_splits_and_cleans_input() {
        let extractor = TypeExtractor::new().unwrap();
        let tokens = extractor.extract_tokens("MyClass,struct MyStruct").unwrap();
        assert_eq!(tokens, vec!["MyClass", "struct", "MyStruct"]);
    }

    #[test]
    fn test_extract_tokens_for_trigger_comment() {
        let extractor = TypeExtractor::new().unwrap();
        let tokens = extractor
            .extract_tokens("// TODO: - MyTriggeredType")
            .unwrap();
        assert_eq!(tokens, vec!["MyTriggeredType"]);
    }

    #[test]
    fn test_extract_types_basic() {
        let extractor = TypeExtractor::new().unwrap();
        let lines = vec![
            "class MyClass {}".to_string(),
            "struct MyStruct {}".to_string(),
            "enum MyEnum {}".to_string(),
        ];
        let types = extractor.extract_types(lines.into_iter());
        let expected: BTreeSet<String> = ["MyClass", "MyEnum", "MyStruct"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(types, expected);
    }

    #[test]
    fn test_extract_types_with_bracket_notation() {
        let extractor = TypeExtractor::new().unwrap();
        let lines = vec!["let array: [CustomType] = []".to_string()];
        let types = extractor.extract_types(lines.into_iter());
        let expected: BTreeSet<String> = ["CustomType"].iter().map(|s| s.to_string()).collect();
        assert_eq!(types, expected);
    }

    #[test]
    fn test_extract_types_mixed_tokens() {
        let extractor = TypeExtractor::new().unwrap();
        let lines = vec!["class MyClass, struct MyStruct; enum MyEnum.".to_string()];
        let types = extractor.extract_types(lines.into_iter());
        let expected: BTreeSet<String> = ["MyClass", "MyEnum", "MyStruct"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(types, expected);
    }

    #[test]
    fn test_extract_types_deduplication() {
        let extractor = TypeExtractor::new().unwrap();
        let lines = vec![
            "class DuplicateType {}".to_string(),
            "struct DuplicateType {}".to_string(),
            "enum DuplicateType {}".to_string(),
        ];
        let types = extractor.extract_types(lines.into_iter());
        let expected: BTreeSet<String> = ["DuplicateType"].iter().map(|s| s.to_string()).collect();
        assert_eq!(types, expected);
    }
}

#[cfg(test)]
mod objc_tests {
    use super::*;
    use substring_marker_snippet_extractor::extract_enclosing_block_from_content;

    fn enclosing_block_with_type_predicate(content: &str) -> Option<String> {
        extract_enclosing_block_from_content(content, Some(&is_type_candidate_line))
    }

    #[test]
    fn test_extract_enclosing_block_with_objc_method_split_lines() {
        let content = "\
- (void)MyObjCMethod\n\
{\n\
    // method implementation\n\
}\n\
void anotherFunction() {\n\
    // TODO: - Fix issue\n\
}";
        let block = enclosing_block_with_type_predicate(content);
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)MyObjCMethod"));
        assert!(block_str.contains("{"));
    }

    #[test]
    fn test_extract_enclosing_block_with_objc_method_single_line() {
        let content = "\
- (void)MyObjCMethod { // implementation start\n\
    // method body\n\
}\n\
void someFunction() {\n\
    // TODO: - Address bug\n\
}";
        let block = enclosing_block_with_type_predicate(content);
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)MyObjCMethod {"));
    }
}

#[cfg(test)]
mod candidate_detection_tests {
    use super::*;

    #[test]
    fn test_type_candidate_line_swift_class() {
        let swift_class = "class MyInnerClass {";
        assert!(
            is_type_candidate_line(swift_class),
            "Swift class declaration should be detected as a type candidate"
        );
    }

    #[test]
    fn test_type_candidate_line_swift_enum() {
        let swift_enum = "enum MyEnum {";
        assert!(
            is_type_candidate_line(swift_enum),
            "Swift enum declaration should be detected as a type candidate"
        );
    }

    #[test]
    fn test_type_candidate_line_non_candidate() {
        let non_candidate = "let x = 10;";
        assert!(
            !is_type_candidate_line(non_candidate),
            "Non-declaration line should not be detected as a type candidate"
        );
    }

    #[test]
    fn test_type_candidate_line_function_not_matched() {
        let func = "func testAsyncFunction(foo: Int) async {";
        assert!(
            !is_type_candidate_line(func),
            "Function declarations are handled by the shared marker_utils, not the type predicate"
        );
    }
}

#[cfg(test)]
mod enclosing_block_characterization_tests {
    use super::*;
    use std::io::Write;
    use substring_marker_snippet_extractor::extract_enclosing_block_from_content;
    use tempfile::NamedTempFile;

    fn enclosing_block_with_type_predicate(content: &str) -> Option<String> {
        extract_enclosing_block_from_content(content, Some(&is_type_candidate_line))
    }

    /// Characterizes that extract_types recognizes ObjC split-line declarations
    /// (method signature on one line, `{` on the next) as candidates via the
    /// diff-candidate heuristic, matching the behavior in marker_utils.
    #[test]
    fn test_diff_candidate_objc_split_line() {
        let content = "\
- (void)myMethod:(NSString *)arg\n\
{\n\
    NSLog(@\"Start\");\n\
    // TODO: - Do something in ObjC\n\
    NSLog(@\"End\");\n\
}";
        let block = enclosing_block_with_type_predicate(content);
        assert!(
            block.is_some(),
            "Should recognize ObjC split-line as a diff candidate"
        );
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)myMethod:(NSString *)arg"));
        assert!(block_str.contains("// TODO: - Do something in ObjC"));
        assert!(block_str.contains("NSLog(@\"End\");"));
    }

    /// Characterizes that extract_types recognizes class declarations as
    /// candidates (via SWIFT_CLASS_RE), which marker_utils intentionally does NOT.
    /// This documents the intentional divergence between the two implementations.
    #[test]
    fn test_class_recognized_as_candidate_divergence() {
        let content = "\
class MyEnclosingClass {\n\
    let value = 42\n\
    // TODO: - Implement feature\n\
}";
        let block = enclosing_block_with_type_predicate(content);
        assert!(
            block.is_some(),
            "extract_types should recognize class as a candidate"
        );
        let block_str = block.unwrap();
        assert!(block_str.contains("class MyEnclosingClass {"));
        assert!(block_str.contains("// TODO: - Implement feature"));
    }

    /// Characterizes that extract_types recognizes enum declarations as
    /// candidates (via SWIFT_ENUM_RE), which marker_utils intentionally does NOT.
    #[test]
    fn test_enum_recognized_as_candidate_divergence() {
        let content = "\
enum MyState {\n\
    case loading\n\
    case loaded\n\
    // TODO: - Add error case\n\
}";
        let block = enclosing_block_with_type_predicate(content);
        assert!(
            block.is_some(),
            "extract_types should recognize enum as a candidate"
        );
        let block_str = block.unwrap();
        assert!(block_str.contains("enum MyState {"));
        assert!(block_str.contains("// TODO: - Add error case"));
    }

    /// Cross-crate equivalence: for content with only function-level candidates,
    /// both extract_types's local wrapper and marker_utils's file-path-based
    /// extract_enclosing_block produce the same output (when the file has markers
    /// and TODO is outside them).
    #[test]
    fn test_cross_crate_equivalence_for_function_candidates() {
        let content = "\
Some preamble\n\
// v\n\
Header content\n\
// ^\n\
func sharedFunction() {\n\
    let x = 10;\n\
    // TODO: - Cross-crate test\n\
    let y = 20;\n\
}";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();

        let local_result = enclosing_block_with_type_predicate(content);

        let file_result =
            substring_marker_snippet_extractor::utils::marker_utils::extract_enclosing_block(
                temp_file.path(),
            );

        assert_eq!(
            local_result, file_result,
            "Both implementations should produce identical output for function candidates"
        );
    }

    /// Cross-crate divergence: for content with a class candidate, extract_types
    /// finds the block but marker_utils does not (since it only matches functions).
    #[test]
    fn test_cross_crate_divergence_for_class_candidates() {
        let content = "\
Some preamble\n\
// v\n\
Header content\n\
// ^\n\
class MyWidget {\n\
    var name: String\n\
    // TODO: - Add initializer\n\
}";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();

        let local_result = enclosing_block_with_type_predicate(content);

        let file_result =
            substring_marker_snippet_extractor::utils::marker_utils::extract_enclosing_block(
                temp_file.path(),
            );

        assert!(
            local_result.is_some(),
            "extract_types should find the class block"
        );
        assert!(
            file_result.is_none(),
            "marker_utils should NOT find a class-only candidate"
        );
    }
}

#[cfg(test)]
mod type_candidate_characterization_tests {
    use super::*;

    #[test]
    fn test_swift_class_is_type_candidate() {
        assert!(is_type_candidate_line("class MyClass {"));
    }

    #[test]
    fn test_swift_class_indented() {
        assert!(is_type_candidate_line("    class MyClass {"));
    }

    #[test]
    fn test_swift_class_with_inheritance() {
        assert!(is_type_candidate_line("class MyClass: BaseClass {"));
    }

    #[test]
    fn test_swift_enum_is_type_candidate() {
        assert!(is_type_candidate_line("enum MyEnum {"));
    }

    #[test]
    fn test_swift_enum_indented() {
        assert!(is_type_candidate_line("    enum MyEnum {"));
    }

    #[test]
    fn test_swift_enum_with_raw_type() {
        assert!(is_type_candidate_line("enum MyEnum: String {"));
    }

    #[test]
    fn test_struct_not_type_candidate() {
        assert!(!is_type_candidate_line("struct MyStruct {"));
    }

    #[test]
    fn test_func_not_type_candidate() {
        assert!(!is_type_candidate_line("func doSomething() {"));
    }

    #[test]
    fn test_let_not_type_candidate() {
        assert!(!is_type_candidate_line("let x = 10;"));
    }

    #[test]
    fn test_protocol_not_type_candidate() {
        assert!(!is_type_candidate_line("protocol MyProtocol {"));
    }

    #[test]
    fn test_empty_not_type_candidate() {
        assert!(!is_type_candidate_line(""));
    }

    #[test]
    fn test_class_without_brace_not_type_candidate() {
        assert!(!is_type_candidate_line("class MyClass"));
    }
}
