// crates/extract_enclosing_type/src/lib.rs

//
// Finds the type (class/struct/enum) that encloses the shared TODO marker in a
// Swift source file.  Uses a tree‑sitter pass first, then falls back to a regex
// scan if the parser is unavailable or fails.

use regex::Regex;
use std::fs;
use std::mem;
use std::path::Path;
use tree_sitter::{Node, Parser};
use tree_sitter_swift;

use todo_marker::{TODO_MARKER, TODO_MARKER_WS};

/// ---------------------------------------------------------------------------
///  Parser abstraction
/// ---------------------------------------------------------------------------

/// Something that can parse Swift source and give back a simplified tree.
pub trait SwiftParser {
    fn parse_content(&mut self, content: &str) -> Option<SwiftParseTree>;
}

/// Minimal tree wrapper.
#[derive(Clone)]
pub struct SwiftParseTree {
    pub root: SwiftNode,
}

/// Minimal node wrapper.
#[derive(Clone)]
pub struct SwiftNode {
    pub kind: String,
    pub start_byte: usize,
    pub children: Vec<SwiftNode>,
    pub name: Option<String>,
}

impl SwiftNode {
    fn from_tree_sitter_node(node: Node, content: &str) -> Self {
        let kind = node.kind().to_string();
        let start_byte = node.start_byte();
        let name = node
            .child_by_field_name("name")
            .and_then(|n| n.utf8_text(content.as_bytes()).ok())
            .map(|s| s.to_string());

        let mut children = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                children.push(SwiftNode::from_tree_sitter_node(child, content));
            }
        }
        SwiftNode {
            kind,
            start_byte,
            children,
            name,
        }
    }
}

/// Real parser backed by tree‑sitter‑swift.
pub struct RealSwiftParser {
    parser: Parser,
}

impl RealSwiftParser {
    pub fn new() -> Option<Self> {
        let mut parser = Parser::new();
        // SAFETY: tree_sitter_swift::LANGUAGE matches the ABI expected by tree‑sitter.
        let lang: tree_sitter::Language = unsafe { mem::transmute(tree_sitter_swift::LANGUAGE) };
        parser.set_language(&lang).ok()?;
        Some(Self { parser })
    }
}

impl SwiftParser for RealSwiftParser {
    fn parse_content(&mut self, content: &str) -> Option<SwiftParseTree> {
        let tree = self.parser.parse(content, None)?;
        Some(SwiftParseTree {
            root: SwiftNode::from_tree_sitter_node(tree.root_node(), content),
        })
    }
}

/// ---------------------------------------------------------------------------
///  Tree walk helpers
/// ---------------------------------------------------------------------------

fn find_last_type_declaration(node: &SwiftNode, todo_offset: usize) -> Option<String> {
    let mut candidate = None;

    if matches!(
        node.kind.as_str(),
        "class_declaration" | "struct_declaration" | "enum_declaration"
    ) && node.start_byte <= todo_offset
    {
        candidate = node.name.clone();
    }

    for child in &node.children {
        if let Some(found) = find_last_type_declaration(child, todo_offset) {
            candidate = Some(found);
        }
    }
    candidate
}

pub fn extract_enclosing_type_with_parser(
    content: &str,
    todo_offset: usize,
    parser: &mut impl SwiftParser,
) -> Option<String> {
    let tree = parser.parse_content(content)?;
    find_last_type_declaration(&tree.root, todo_offset)
}

/// ---------------------------------------------------------------------------
///  Public API – fall back to regex if parser fails
/// ---------------------------------------------------------------------------

/// Returns the enclosing type’s name, or (as a last resort) the file’s stem.
pub fn extract_enclosing_type(file_path: &str) -> Result<String, String> {
    let content = fs::read_to_string(file_path)
        .map_err(|e| format!("Error reading file {}: {}", file_path, e))?;

    // Where in the buffer is the TODO marker?
    let todo_offset = content.find(TODO_MARKER_WS).unwrap_or(content.len());

    // 1️⃣  Try the tree‑sitter path first.
    if let Some(mut parser) = RealSwiftParser::new() {
        if let Some(ty) = extract_enclosing_type_with_parser(&content, todo_offset, &mut parser) {
            return Ok(ty);
        }
    }

    // 2️⃣  Regex fallback – scan line by line up to the marker.
    let re = Regex::new(r"(class|struct|enum)\s+(\w+)")
        .map_err(|e| format!("Regex error: {}", e))?;

    let mut last_type: Option<String> = None;
    for line in content.lines() {
        if line.contains(TODO_MARKER) {
            break;
        }
        if let Some(caps) = re.captures(line) {
            if let Some(name) = caps.get(2) {
                last_type = Some(name.as_str().to_string());
            }
        }
    }

    // 3️⃣  Fallback to file name if nothing found.
    if let Some(found) = last_type {
        Ok(found)
    } else {
        Path::new(file_path)
            .file_stem()
            .and_then(|s| s.to_str())
            .map(|s| s.to_string())
            .ok_or_else(|| "Unknown".to_string())
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile;

    #[test]
    fn test_extract_from_file_with_type_before_todo() {
        let content = "\
class MyAwesomeClass {
    // Some code here
}
// Another type definition
struct HelperStruct {
    // TODO: - Implement something
}";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("Test.swift");
        fs::write(&file_path, content).unwrap();

        // Expect that the last type encountered before the TODO is "HelperStruct".
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "HelperStruct");
    }

    #[test]
    fn test_extract_fallback_to_basename() {
        let content = "\
func doSomething() {
    // Some code here
}
// No type declaration before TODO:
 // TODO: - Fix something";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("FallbackTest.swift");
        fs::write(&file_path, content).unwrap();

        // Since no type was found, it should fall back to "FallbackTest".
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "FallbackTest");
    }

    #[test]
    fn test_ignore_types_after_todo() {
        let content = "\
class EarlyClass {
    // Some code here
}
// TODO: - Do something
struct LateStruct {
    // Some code here
}";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("AfterTodo.swift");
        fs::write(&file_path, content).unwrap();

        // Should return "EarlyClass" because the type after the TODO is ignored.
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "EarlyClass");
    }

    #[test]
    fn test_empty_file_fallback() {
        let content = "";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("Empty.swift");
        fs::write(&file_path, content).unwrap();

        // With no content, it should fallback to "Empty".
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "Empty");
    }

    #[test]
    fn test_no_todo_marker_returns_last_type() {
        let content = "\
class FirstClass {
    // Some code here
}
struct SecondStruct {
    // Some code here
}
enum ThirdEnum {
    // Some code here
}";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("NoTodo.swift");
        fs::write(&file_path, content).unwrap();

        // Expect the last type ("ThirdEnum") to be returned.
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "ThirdEnum");
    }

    #[test]
    fn test_type_on_same_line_as_todo_marker() {
        let content = "class MyClass { // TODO: - Do something important }";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("SameLine.swift");
        fs::write(&file_path, content).unwrap();

        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        // Expect fallback to the file stem "SameLine" rather than "MyClass".
        assert_eq!(extracted, "SameLine");
    }

    #[test]
    fn test_nonexistent_file_error() {
        let file_path = "/path/to/nonexistent/file.swift";
        let result = extract_enclosing_type(file_path);
        assert!(result.is_err());
        let err_msg = result.err().unwrap();
        assert!(err_msg.contains("Error reading file"));
    }
    
    #[test]
    fn test_regex_fallback_with_invalid_swift() {
        let content = "\
    This is not valid Swift code.
    struct ShouldNotBeFound {
        // TODO: - A marker that comes too early
    }";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("InvalidSwift.swift");
        fs::write(&file_path, content).unwrap();

        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "ShouldNotBeFound");
    }
    
    #[test]
    fn test_no_type_declaration() {
        let content = r#"
            // Just some Swift comments and code.
            // TODO: - marker
        "#;
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("NoType.swift");
        fs::write(&file_path, content).unwrap();
        
        let result = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        // Since no type is found, we expect it to fall back to the file stem ("NoType").
        assert_eq!(result, "NoType");
    }
    
    #[test]
    fn test_regex_fallback_with_invalid_swift_variant() {
        let content = "\
    This is not valid Swift code.
    struct ShouldNotBeFound {
        // TODO: - A marker that comes too early
    }";
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("InvalidSwift.swift");
        fs::write(&file_path, content).unwrap();

        let result = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(result, "ShouldNotBeFound");
    }
    
    #[test]
    fn test_treesitter_finds_class_struct() {
        let content = r#"
        class OuterClass {
            // Some code
        }

        struct NestedStruct {
            // Some code
        }

        // TODO: - marker
        "#;
        let tmp_dir = tempfile::tempdir().unwrap();
        let file_path = tmp_dir.path().join("TreeSitterValid.swift");
        fs::write(&file_path, content).unwrap();

        // Expect the last type before the marker to be "NestedStruct"
        let extracted = extract_enclosing_type(file_path.to_str().unwrap()).unwrap();
        assert_eq!(extracted, "NestedStruct");
    }

    #[derive(Default)]
    struct MockSwiftParserFailure;

    impl SwiftParser for MockSwiftParserFailure {
        fn parse_content(&mut self, _content: &str) -> Option<SwiftParseTree> {
            None
        }
    }

    #[test]
    fn test_extract_enclosing_type_with_parser_returns_none() {
        let content = "irrelevant";
        let mut parser = MockSwiftParserFailure::default();
        let todo_offset = content.len();
        let result = extract_enclosing_type_with_parser(content, todo_offset, &mut parser);
        // This should simulate the failure branch, without invoking unsafe code.
        assert!(result.is_none());
    }

    #[derive(Clone)]
    struct MockParserNoName;

    impl SwiftParser for MockParserNoName {
        fn parse_content(&mut self, _content: &str) -> Option<SwiftParseTree> {
            Some(SwiftParseTree {
                root: SwiftNode {
                    kind: "class_declaration".to_string(),
                    start_byte: 0,
                    name: None, // Force missing name
                    children: vec![],
                },
            })
        }
    }

    #[test]
    fn test_class_declaration_with_no_name() {
        let content = "class { } // missing name, or mock scenario";
        let todo_offset = content.len();
        let mut parser = MockParserNoName;
        let result = extract_enclosing_type_with_parser(content, todo_offset, &mut parser);
        // The parser sees a class_declaration but no name => find_last_type_declaration returns None.
        assert_eq!(result, None);
    }

    // 1) A mock parser that always returns None.
    #[derive(Default)]
    struct MockSwiftParserNone;
    impl SwiftParser for MockSwiftParserNone {
        fn parse_content(&mut self, _content: &str) -> Option<SwiftParseTree> {
            None
        }
    }

    #[test]
    fn test_parse_content_returns_none() {
        let content = "irrelevant";
        let mut parser = MockSwiftParserNone::default();
        let todo_offset = content.len();
        let result = extract_enclosing_type_with_parser(content, todo_offset, &mut parser);
        // We expect None, which covers the path in `extract_enclosing_type_with_parser`
        // where parse_tree = None.
        assert_eq!(result, None);
    }

    // 2) A mock parser with a single node whose start_byte is AFTER the TODO offset.
    struct MockParserStartByteAfterTodo;
    impl SwiftParser for MockParserStartByteAfterTodo {
        fn parse_content(&mut self, _content: &str) -> Option<SwiftParseTree> {
            Some(SwiftParseTree {
                root: SwiftNode {
                    kind: "struct_declaration".to_string(),
                    start_byte: 200,
                    name: Some("LateStruct".to_string()),
                    children: vec![],
                },
            })
        }
    }

    #[test]
    fn test_type_starts_after_todo_offset() {
        let content = "class EarlyClass {} // no TODO yet...";
        // We'll pretend the TODO offset is 100, so the struct at start_byte=200 is skipped.
        let todo_offset = 100;
        let mut parser = MockParserStartByteAfterTodo;
        let result = extract_enclosing_type_with_parser(content, todo_offset, &mut parser);
        // Because the only node is after the offset, find_last_type_declaration should return None.
        assert_eq!(result, None);
    }

    // 3) A mock parser that simulates multiple children, including an unnamed node.
    struct MockParserMultipleChildren;
    impl SwiftParser for MockParserMultipleChildren {
        fn parse_content(&mut self, _content: &str) -> Option<SwiftParseTree> {
            Some(SwiftParseTree {
                root: SwiftNode {
                    kind: "class_declaration".to_string(),
                    start_byte: 0,
                    name: Some("Outer".to_string()),
                    children: vec![
                        // A child that's not recognized as a type
                        SwiftNode {
                            kind: "function_declaration".to_string(),
                            start_byte: 10,
                            name: Some("doSomething".to_string()),
                            children: vec![],
                        },
                        // A nested struct_declaration
                        SwiftNode {
                            kind: "struct_declaration".to_string(),
                            start_byte: 20,
                            name: Some("Inner".to_string()),
                            children: vec![],
                        },
                    ],
                },
            })
        }
    }

    #[test]
    fn test_multiple_children_recursion() {
        let content = "class Outer { func doSomething() {} struct Inner {} }";
        let todo_offset = content.len();
        let mut parser = MockParserMultipleChildren;
        let result = extract_enclosing_type_with_parser(content, todo_offset, &mut parser);
        // The last type encountered is "Inner".
        assert_eq!(result, Some("Inner".to_string()));
    }

    // 4) A mock parser that simulates an invalid UTF-8 scenario
    //    or simply a "class_declaration" but name is None.
    struct MockParserNoNameInvalidUtf8;
    impl SwiftParser for MockParserNoNameInvalidUtf8 {
        fn parse_content(&mut self, _content: &str) -> Option<SwiftParseTree> {
            Some(SwiftParseTree {
                root: SwiftNode {
                    kind: "class_declaration".to_string(),
                    start_byte: 0,
                    // Pretend we tried utf8_text and it failed => name is None
                    name: None,
                    children: vec![],
                },
            })
        }
    }

    #[test]
    fn test_class_declaration_missing_name() {
        let content = "class {} // invalid syntax but let's mock it anyway";
        let todo_offset = content.len();
        let mut parser = MockParserNoNameInvalidUtf8;
        let result = extract_enclosing_type_with_parser(content, todo_offset, &mut parser);
        // We found a class_declaration but there's no name => returns None.
        assert_eq!(result, None);
    }

    struct MockParserSkipType;

    impl SwiftParser for MockParserSkipType {
        fn parse_content(&mut self, _content: &str) -> Option<SwiftParseTree> {
            Some(SwiftParseTree {
                root: SwiftNode {
                    kind: "struct_declaration".to_string(),
                    start_byte: 300,
                    name: Some("LateStruct".to_string()),
                    children: vec![],
                },
            })
        }
    }

    #[test]
    fn test_skip_type_after_todo_offset() {
        let content = "class EarlyClass {} // ...no TODO in content, but let's pretend it's earlier";
        let todo_offset = 100;
        let mut parser = MockParserSkipType;
        let result = extract_enclosing_type_with_parser(content, todo_offset, &mut parser);
        // The single struct_declaration starts at 300, which is after 100 => should be skipped => None
        assert_eq!(result, None);
    }

    struct MockParserInvalidUtf8;

    impl SwiftParser for MockParserInvalidUtf8 {
        fn parse_content(&mut self, _content: &str) -> Option<SwiftParseTree> {
            Some(SwiftParseTree {
                root: SwiftNode {
                    kind: "class_declaration".to_string(),
                    start_byte: 0,
                    name: None, // pretend we tried to parse it and it failed
                    children: vec![],
                },
            })
        }
    }

    #[test]
    fn test_class_declaration_invalid_utf8_name() {
        let mut parser = MockParserInvalidUtf8;
        let content = "class ???";
        let todo_offset = content.len();
        let result = extract_enclosing_type_with_parser(content, todo_offset, &mut parser);
        // No name => None
        assert_eq!(result, None);
    }
}
