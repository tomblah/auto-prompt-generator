// crates/substring_marker_snippet_extractor/src/utils/marker_utils.rs

//! Helper utilities for working with **substring markers** (`// v` / `// ^`)
//! *and* the single shared “TODO marker” that drives the prompt‑generation
//! pipeline.

use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
//  Public API – marker filtering
// ---------------------------------------------------------------------------

/// Returns only the text between `// v` … `// ^` blocks.  Everything else is
/// replaced by `placeholder`.  Multiple blocks are concatenated with a blank
/// line between them.
pub fn filter_substring_markers(content: &str, placeholder: &str) -> String {
    let mut output = String::new();
    let mut state = "omitted";
    let mut omitted_line_count = 0;
    let mut last_was_closing = false;

    for line in content.lines() {
        let trimmed = line.trim();
        match trimmed {
            "// v" => {
                if omitted_line_count > 0 {
                    output.push_str("\n\n");
                    output.push_str(placeholder);
                    output.push_str("\n\n");
                }
                omitted_line_count = 0;
                state = "included";
                last_was_closing = false;
            }
            "// ^" => {
                state = "omitted";
                omitted_line_count = 0;
                last_was_closing = true;
            }
            _ => match state {
                "included" => {
                    output.push_str(line);
                    output.push('\n');
                    last_was_closing = false;
                }
                "omitted" => {
                    omitted_line_count += 1;
                    last_was_closing = false;
                }
                _ => unreachable!(),
            },
        }
    }

    if state == "omitted" && (omitted_line_count > 0 || last_was_closing) {
        output.push_str("\n\n");
        output.push_str(placeholder);
        output.push_str("\n\n");
    }
    output
}

/// `true` if the file uses both `// v` *and* `// ^`.
pub fn file_uses_markers(content: &str) -> bool {
    let has_open = content.lines().any(|line| line.trim() == "// v");
    let has_close = content.lines().any(|line| line.trim() == "// ^");
    has_open && has_close
}

use todo_marker::{is_todo_inside_markers, todo_index};

fn is_candidate_line(line: &str) -> bool {
    lang_support::is_function_candidate_any_lang(line)
}

// ---------------------------------------------------------------------------
//  FileAnalysis – unified file context for marker and enclosing-block logic
// ---------------------------------------------------------------------------

/// Pre-computed analysis of a source file's marker structure and TODO position.
///
/// Consumers that need to filter markers, extract enclosing blocks, or check
/// TODO position should construct a `FileAnalysis` once and call its methods,
/// rather than re-implementing gating logic independently.
pub struct FileAnalysis<'a> {
    content: &'a str,
    has_markers: bool,
    todo_idx: Option<usize>,
    todo_inside_markers: bool,
}

impl<'a> FileAnalysis<'a> {
    /// Analyse the content once.
    pub fn new(content: &'a str) -> Self {
        let has_markers = file_uses_markers(content);
        let todo_idx = todo_index(content);
        let todo_inside_markers =
            has_markers && todo_idx.is_some_and(|idx| is_todo_inside_markers(content, idx));
        Self {
            content,
            has_markers,
            todo_idx,
            todo_inside_markers,
        }
    }

    pub fn has_markers(&self) -> bool {
        self.has_markers
    }

    pub fn todo_inside_markers(&self) -> bool {
        self.todo_inside_markers
    }

    /// The zero-based line index of the TODO marker, if present.
    pub fn todo_idx(&self) -> Option<usize> {
        self.todo_idx
    }

    /// Filter content through substring markers (delegates to `filter_substring_markers`).
    pub fn filtered_content(&self, placeholder: &str) -> String {
        filter_substring_markers(self.content, placeholder)
    }

    /// Extract the enclosing block around the TODO marker, using an optional
    /// additional candidate predicate.
    ///
    /// Returns `None` if:
    /// - There is no TODO marker in the content
    /// - The TODO is inside markers (gating)
    /// - No candidate line is found before the TODO
    ///
    /// The `additional_candidate` predicate extends the built-in function/method
    /// candidates. Pass `None` for function-only matching (assembly path), or
    /// supply a type-candidate predicate for class/enum matching (extract_types path).
    pub fn enclosing_block(
        &self,
        additional_candidate: Option<&dyn Fn(&str) -> bool>,
    ) -> Option<String> {
        if !self.has_markers {
            return None;
        }
        if self.todo_inside_markers {
            return None;
        }
        extract_enclosing_block_from_content(self.content, additional_candidate)
    }
}

// ---------------------------------------------------------------------------
//  Extract the enclosing block around the TODO marker
// ---------------------------------------------------------------------------

/// Extracts the enclosing block around the TODO marker from file content.
///
/// Scans lines before the TODO marker for the last candidate line (function,
/// cloud function, ObjC method, or diff-candidate heuristic). An optional
/// `additional_candidate` predicate allows callers to extend which lines count
/// as candidates (e.g. class/enum declarations for type extraction).
///
/// Returns the brace-delimited block starting from that candidate, or `None`
/// if no candidate is found.
pub fn extract_enclosing_block_from_content(
    content: &str,
    additional_candidate: Option<&dyn Fn(&str) -> bool>,
) -> Option<String> {
    let todo_idx = todo_index(content)?;

    let lines: Vec<&str> = content.lines().collect();
    let mut candidate_index = None;
    for i in 0..todo_idx {
        let line = lines[i];
        let diff_candidate = (line.trim_start().starts_with('-')
            || line.trim_start().starts_with('+'))
            && i + 1 < todo_idx
            && lines[i + 1].contains('{');
        let extra_match = additional_candidate.as_ref().is_some_and(|pred| pred(line));
        if is_candidate_line(line) || diff_candidate || extra_match {
            candidate_index = Some(i);
        }
    }

    let start_index = candidate_index?;
    let mut brace_count = 0;
    let mut found_open = false;
    let mut extracted_lines = Vec::new();

    for line in &lines[start_index..] {
        if !found_open {
            if line.contains('{') {
                found_open = true;
                brace_count += line.matches('{').count();
                brace_count = brace_count.saturating_sub(line.matches('}').count());
            }
            extracted_lines.push(*line);
        } else {
            extracted_lines.push(*line);
            brace_count += line.matches('{').count();
            brace_count = brace_count.saturating_sub(line.matches('}').count());
            if brace_count == 0 {
                break;
            }
        }
    }

    Some(extracted_lines.join("\n"))
}

/// File-path-based wrapper: reads the file, checks marker/TODO gating, then
/// delegates to `extract_enclosing_block_from_content`.
pub fn extract_enclosing_block(file_path: &Path) -> Option<String> {
    let content = fs::read_to_string(file_path).ok()?;
    if !file_uses_markers(&content) {
        return None;
    }

    let todo_idx = todo_index(&content)?;
    if is_todo_inside_markers(&content, todo_idx) {
        return None;
    }

    extract_enclosing_block_from_content(&content, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_filter_substring_markers() {
        let input = "\
Line before
// v
content line 1
content line 2
// ^
Line after";
        // The expected output has two newlines before and after the placeholder,
        // and due to the included region ending with a newline, an extra newline appears.
        let expected = "\n\n// ...\n\ncontent line 1\ncontent line 2\n\n\n// ...\n\n";
        let result = filter_substring_markers(input, "// ...");
        assert_eq!(result, expected);
    }

    #[test]
    fn test_file_uses_markers_true() {
        let content = "Some text\n// v\nmarker content\n// ^\nMore text";
        assert!(file_uses_markers(content));
    }

    #[test]
    fn test_file_uses_markers_false() {
        let content = "Some text\n// v\nmarker content\nMore text";
        assert!(!file_uses_markers(content));
    }

    #[test]
    fn test_extract_enclosing_block_success() {
        // Create a temporary file with a candidate declaration, markers, and a TODO.
        let content = "\
Some irrelevant text
func myFunction() {
    let x = 10;
}
Other text
// v
Marker content
// ^
More text
// TODO: - Do something";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("func myFunction() {"));
        assert!(block_str.contains("let x = 10;"));
        assert!(block_str.contains("}"));
    }

    #[test]
    fn test_extract_enclosing_block_no_markers() {
        // Create a temporary file without both markers.
        let content = "\
func myFunction() {
    let x = 10;
}";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path());
        assert!(block.is_none());
    }

    #[test]
    fn test_extract_enclosing_block_parse_cloud_success() {
        // Create a temporary file where substring markers wrap header content
        // and a Parse.Cloud.beforeSave function (which contains the TODO marker)
        // appears after the markers. The "Footer text" outside the function should be omitted.
        let content = "\
Header text
// v
Header content inside markers
// ^
Parse.Cloud.beforeSave(\"Message\", async (request) => {
    console.log(\"Setup\");
    // TODO: - Do something important
    console.log(\"Teardown\");
});
Footer text that should be omitted";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path());
        assert!(block.is_some());
        let block_str = block.unwrap();
        // Verify that the extracted block is the entire Parse.Cloud function
        assert!(block_str.contains("Parse.Cloud.beforeSave(\"Message\", async (request) => {"));
        assert!(block_str.contains("// TODO: - Do something important"));
        assert!(block_str.contains("console.log(\"Teardown\");"));
        // Ensure that footer text is not included
        assert!(!block_str.contains("Footer text"));
    }

    #[test]
    fn test_extract_enclosing_block_after_save_success() {
        // Test a Parse.Cloud.afterSave function with a quoted first argument.
        let content = "\
Header text
// v
Header section that is not part of the function
// ^
Parse.Cloud.afterSave(\"Message\", async (request) => {
    console.log(\"AfterSave Setup\");
    // TODO: - Handle after save logic
    console.log(\"AfterSave Teardown\");
});
Some trailing footer text";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("Parse.Cloud.afterSave(\"Message\", async (request) => {"));
        assert!(block_str.contains("// TODO: - Handle after save logic"));
        assert!(block_str.contains("console.log(\"AfterSave Teardown\");"));
        assert!(!block_str.contains("trailing footer text"));
    }

    #[test]
    fn test_extract_enclosing_block_before_save_parse_user_success() {
        // Test a Parse.Cloud.beforeSave function with Parse.User as the first argument.
        let content = "\
Some header information
// v
Ignored header details
// ^
Parse.Cloud.beforeSave(Parse.User, async (request) => {
    console.log(\"BeforeSave Init\");
    // TODO: - Process user before save
    console.log(\"BeforeSave Complete\");
});
Extra text that should be omitted";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("Parse.Cloud.beforeSave(Parse.User, async (request) => {"));
        assert!(block_str.contains("// TODO: - Process user before save"));
        assert!(block_str.contains("console.log(\"BeforeSave Complete\");"));
        assert!(!block_str.contains("Extra text"));
    }

    #[test]
    fn test_extract_enclosing_block_after_save_parse_user_success() {
        // Test a Parse.Cloud.afterSave function with Parse.User as the first argument.
        let content = "\
Introductory header
// v
Header content that is not part of the function
// ^
Parse.Cloud.afterSave(Parse.User, async (request) => {
    console.log(\"AfterSave Start\");
    // TODO: - Process user after save
    console.log(\"AfterSave End\");
});
Irrelevant footer";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("Parse.Cloud.afterSave(Parse.User, async (request) => {"));
        assert!(block_str.contains("// TODO: - Process user after save"));
        assert!(block_str.contains("console.log(\"AfterSave End\");"));
        assert!(!block_str.contains("Irrelevant footer"));
    }

    #[test]
    fn test_objc_method_candidate_line() {
        let objc_line = " - (void)myMethod:(NSString *)arg {";
        assert!(is_candidate_line(objc_line));
    }

    #[test]
    fn test_extract_enclosing_block_objc_success() {
        // Create a temporary file with an Objective-C method declaration,
        // markers, and a TODO marker after the markers.
        let content = "\
Some header info
// v
// ^
- (void)myMethod:(NSString *)arg {
    NSLog(@\"Start\");
    // TODO: - Do something in ObjC
    NSLog(@\"End\");
}";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)myMethod:(NSString *)arg {"));
        assert!(block_str.contains("// TODO: - Do something in ObjC"));
        assert!(block_str.contains("NSLog(@\"End\");"));
    }

    #[test]
    fn test_extract_enclosing_block_objc_split_declaration_success() {
        // Test an Objective-C method declaration split across two lines:
        // The first line has the method signature without the opening brace,
        // and the following line contains the opening brace.
        let content = "\
Some header info
// v
Header details that are not part of the method
// ^
- (void)myMethod:(NSString *)arg
{
    NSLog(@\"Start split\");
    // TODO: - Do something split
    NSLog(@\"End split\");
}";
        let mut temp_file = NamedTempFile::new().unwrap();
        write!(temp_file, "{}", content).unwrap();
        let block = extract_enclosing_block(temp_file.path());
        assert!(block.is_some());
        let block_str = block.unwrap();
        assert!(block_str.contains("- (void)myMethod:(NSString *)arg"));
        assert!(block_str.contains("{"));
        assert!(block_str.contains("// TODO: - Do something split"));
        assert!(block_str.contains("NSLog(@\"End split\");"));
    }
}

#[cfg(test)]
mod candidate_line_characterization_tests {
    use super::*;

    // --- Swift function candidates ---

    #[test]
    fn test_swift_plain_func() {
        assert!(is_candidate_line("func doSomething() {"));
    }

    #[test]
    fn test_swift_func_with_params() {
        assert!(is_candidate_line("func doSomething(x: Int, y: String) {"));
    }

    #[test]
    fn test_swift_public_func() {
        assert!(is_candidate_line("public func doSomething() {"));
    }

    #[test]
    fn test_swift_private_func() {
        assert!(is_candidate_line("private func doSomething() {"));
    }

    #[test]
    fn test_swift_internal_func() {
        assert!(is_candidate_line("internal func doSomething() {"));
    }

    #[test]
    fn test_swift_fileprivate_func() {
        assert!(is_candidate_line("fileprivate func doSomething() {"));
    }

    #[test]
    fn test_swift_func_with_return_type() {
        assert!(is_candidate_line("func doSomething() -> Bool {"));
    }

    #[test]
    fn test_swift_func_with_generics() {
        assert!(is_candidate_line("func doSomething<T>(value: T) {"));
    }

    #[test]
    fn test_swift_indented_func() {
        assert!(is_candidate_line("    func doSomething() {"));
    }

    // --- JavaScript candidates ---

    #[test]
    fn test_js_const_function_assignment() {
        assert!(is_candidate_line("const handler = function() {"));
    }

    #[test]
    fn test_js_var_function_assignment() {
        assert!(is_candidate_line("var handler = function() {"));
    }

    #[test]
    fn test_js_let_function_assignment() {
        assert!(is_candidate_line("let handler = function() {"));
    }

    #[test]
    fn test_js_bare_function_assignment() {
        assert!(is_candidate_line("handler = function() {"));
    }

    #[test]
    fn test_js_named_function() {
        assert!(is_candidate_line("function myFunction() {"));
    }

    #[test]
    fn test_js_async_function() {
        assert!(is_candidate_line("async function myFunction() {"));
    }

    // --- Parse.Cloud candidates ---

    #[test]
    fn test_parse_cloud_define_quoted() {
        assert!(is_candidate_line(
            "Parse.Cloud.define(\"myFunc\", async (request) => {"
        ));
    }

    #[test]
    fn test_parse_cloud_before_save_quoted() {
        assert!(is_candidate_line(
            "Parse.Cloud.beforeSave(\"Message\", async (request) => {"
        ));
    }

    #[test]
    fn test_parse_cloud_after_save_quoted() {
        assert!(is_candidate_line(
            "Parse.Cloud.afterSave(\"Message\", async (request) => {"
        ));
    }

    #[test]
    fn test_parse_cloud_before_save_dotted() {
        assert!(is_candidate_line(
            "Parse.Cloud.beforeSave(Parse.User, async (request) => {"
        ));
    }

    #[test]
    fn test_parse_cloud_define_sync() {
        assert!(is_candidate_line(
            "Parse.Cloud.define(\"myFunc\", (request) => {"
        ));
    }

    // --- ObjC candidates ---

    #[test]
    fn test_objc_instance_method() {
        assert!(is_candidate_line("- (void)myMethod:(NSString *)arg {"));
    }

    #[test]
    fn test_objc_class_method() {
        assert!(is_candidate_line("+ (instancetype)sharedInstance {"));
    }

    #[test]
    fn test_objc_no_params() {
        assert!(is_candidate_line("- (void)viewDidLoad {"));
    }

    // --- Negative cases ---

    #[test]
    fn test_plain_assignment_not_candidate() {
        assert!(!is_candidate_line("let x = 10;"));
    }

    #[test]
    fn test_comment_not_candidate() {
        assert!(!is_candidate_line("// func doSomething() {"));
    }

    #[test]
    fn test_class_declaration_not_candidate() {
        assert!(!is_candidate_line("class MyClass {"));
    }

    #[test]
    fn test_enum_declaration_not_candidate() {
        assert!(!is_candidate_line("enum MyEnum {"));
    }

    #[test]
    fn test_struct_declaration_not_candidate() {
        assert!(!is_candidate_line("struct MyStruct {"));
    }

    #[test]
    fn test_empty_line_not_candidate() {
        assert!(!is_candidate_line(""));
    }

    #[test]
    fn test_import_not_candidate() {
        assert!(!is_candidate_line("import Foundation"));
    }

    #[test]
    fn test_arrow_function_not_candidate() {
        assert!(!is_candidate_line("const x = () => {"));
    }

    #[test]
    fn test_swift_func_missing_brace_not_candidate() {
        assert!(!is_candidate_line("func doSomething()"));
    }
}

#[cfg(test)]
mod predicate_extension_characterization_tests {
    use super::*;

    fn type_candidate_predicate(line: &str) -> bool {
        lang_support::for_extension("swift").is_some_and(|lang| lang.is_type_candidate(line))
    }

    /// With no additional predicate, a class declaration is NOT recognized as a
    /// candidate — only functions/methods are. This is the path used by
    /// `assemble_prompt::DefaultFileProcessor`.
    #[test]
    fn char_class_not_found_without_predicate() {
        let content = "\
class MyWidget {\n\
    var name: String\n\
    // TODO: - Add initializer\n\
}";
        let result = extract_enclosing_block_from_content(content, None);
        assert!(
            result.is_none(),
            "Without type predicate, class should not be a candidate"
        );
    }

    /// With the type-candidate predicate, a class declaration IS recognized.
    /// This is the path used by `extract_types`.
    #[test]
    fn char_class_found_with_type_predicate() {
        let content = "\
class MyWidget {\n\
    var name: String\n\
    // TODO: - Add initializer\n\
}";
        let result = extract_enclosing_block_from_content(content, Some(&type_candidate_predicate));
        assert!(result.is_some(), "With type predicate, class should match");
        let block = result.unwrap();
        assert!(block.contains("class MyWidget {"));
        assert!(block.contains("// TODO: - Add initializer"));
    }

    /// Enum declarations are only found with the type predicate.
    #[test]
    fn char_enum_not_found_without_predicate() {
        let content = "\
enum MyState {\n\
    case loading\n\
    // TODO: - Add error case\n\
}";
        assert!(extract_enclosing_block_from_content(content, None).is_none());
    }

    #[test]
    fn char_enum_found_with_type_predicate() {
        let content = "\
enum MyState {\n\
    case loading\n\
    // TODO: - Add error case\n\
}";
        let result = extract_enclosing_block_from_content(content, Some(&type_candidate_predicate));
        assert!(result.is_some());
        assert!(result.unwrap().contains("enum MyState {"));
    }

    /// Functions are found by both paths (no predicate needed).
    #[test]
    fn char_function_found_with_and_without_predicate() {
        let content = "\
func doWork() {\n\
    let x = 42\n\
    // TODO: - Fix calculation\n\
}";
        let without = extract_enclosing_block_from_content(content, None);
        let with = extract_enclosing_block_from_content(content, Some(&type_candidate_predicate));
        assert!(without.is_some());
        assert!(with.is_some());
        assert_eq!(
            without, with,
            "Both paths should produce identical output for functions"
        );
    }

    /// When both a function and a class precede the TODO, the last candidate wins.
    /// The algorithm picks the last candidate LINE before the TODO regardless of
    /// brace closure. Without type predicate: last function is the candidate.
    /// With type predicate: last of (function OR type) is the candidate.
    #[test]
    fn char_last_candidate_wins_divergence() {
        let content = "\
func earlyFunc() {\n\
    let a = 1\n\
}\n\
class LateClass {\n\
    var b = 2\n\
    // TODO: - Do something\n\
}";
        let without = extract_enclosing_block_from_content(content, None);
        let with = extract_enclosing_block_from_content(content, Some(&type_candidate_predicate));

        // Without: earlyFunc is still the last function-candidate line before TODO.
        // The algorithm extracts the brace block starting from that candidate.
        assert!(
            without.is_some(),
            "earlyFunc is a candidate line before TODO"
        );
        assert!(
            without.as_ref().unwrap().contains("func earlyFunc()"),
            "Should extract from earlyFunc"
        );

        // With: class LateClass is the last candidate (later than earlyFunc).
        assert!(with.is_some());
        assert!(
            with.as_ref().unwrap().contains("class LateClass {"),
            "Type predicate makes class the last candidate"
        );
    }

    /// Documents behavior when function appears AFTER class but before TODO.
    #[test]
    fn char_function_after_class_before_todo() {
        let content = "\
class Container {\n\
    func innerMethod() {\n\
        // TODO: - Implement\n\
    }\n\
}";
        let without = extract_enclosing_block_from_content(content, None);
        let with = extract_enclosing_block_from_content(content, Some(&type_candidate_predicate));

        // Both should find innerMethod (last candidate before TODO in both modes)
        assert!(without.is_some());
        assert!(with.is_some());
        assert!(without.unwrap().contains("func innerMethod()"));
        assert!(with.unwrap().contains("func innerMethod()"));
    }
}
