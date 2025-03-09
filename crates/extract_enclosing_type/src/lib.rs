use regex::Regex;
use std::fs;
use std::path::Path;
use std::mem;
use tree_sitter::{Node, Parser};
use tree_sitter_swift;

/// ---
/// # Production Code: Using a Parser Abstraction
/// ---

/// The trait that abstracts Swift parsing functionality.
pub trait SwiftParser {
    /// Parses the given content and returns a SwiftParseTree on success.
    fn parse_content(&mut self, content: &str) -> Option<SwiftParseTree>;
}

/// A simplified parse tree.
#[derive(Clone)]
pub struct SwiftParseTree {
    pub root: SwiftNode,
}

/// A simplified node extracted from the parse tree.
#[derive(Clone)]
pub struct SwiftNode {
    pub kind: String,
    pub start_byte: usize,
    pub children: Vec<SwiftNode>,
    /// The text of the node’s “name” child (if any). In production this is filled by
    /// examining the tree‑sitter node’s child by field “name”.
    pub name: Option<String>,
}

impl SwiftNode {
    /// Convert a tree_sitter::Node into our simplified SwiftNode.
    fn from_tree_sitter_node(node: Node, content: &str) -> Self {
        let kind = node.kind().to_string();
        let start_byte = node.start_byte();
        let name = node.child_by_field_name("name")
            .and_then(|n| n.utf8_text(content.as_bytes()).ok())
            .map(|s| s.to_string());
        let mut children = Vec::new();
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.is_named() {
                children.push(SwiftNode::from_tree_sitter_node(child, content));
            }
        }
        SwiftNode { kind, start_byte, children, name }
    }
}

/// A real implementation of SwiftParser that uses tree-sitter.
pub struct RealSwiftParser {
    parser: Parser,
}

impl RealSwiftParser {
    pub fn new() -> Option<Self> {
        let mut parser = Parser::new();
        // SAFETY: transmute tree_sitter_swift::LANGUAGE into tree_sitter::Language.
        let lang: tree_sitter::Language = unsafe { mem::transmute(tree_sitter_swift::LANGUAGE) };
        parser.set_language(&lang).ok()?;
        Some(Self { parser })
    }
}

impl SwiftParser for RealSwiftParser {
    fn parse_content(&mut self, content: &str) -> Option<SwiftParseTree> {
        let tree = self.parser.parse(content, None)?;
        let root = SwiftNode::from_tree_sitter_node(tree.root_node(), content);
        Some(SwiftParseTree { root })
    }
}

/// Given a parse tree (via our SwiftParser abstraction), traverse the tree to find
/// the last type declaration ("class_declaration", "struct_declaration", or "enum_declaration")
/// whose start_byte is at or before the TODO marker. Returns the name if available.
fn find_last_type_declaration(node: &SwiftNode, todo_offset: usize) -> Option<String> {
    let mut candidate = None;
    if (node.kind == "class_declaration" || node.kind == "struct_declaration" || node.kind == "enum_declaration")
        && node.start_byte <= todo_offset
    {
        candidate = node.name.clone();
    }
    for child in &node.children {
        if let Some(child_candidate) = find_last_type_declaration(child, todo_offset) {
            candidate = Some(child_candidate);
        }
    }
    candidate
}

/// A helper function that uses any SwiftParser to extract the enclosing type name.
pub fn extract_enclosing_type_with_parser(
    content: &str,
    todo_offset: usize,
    parser: &mut impl SwiftParser,
) -> Option<String> {
    let parse_tree = parser.parse_content(content)?;
    find_last_type_declaration(&parse_tree.root, todo_offset)
}

/// ---
/// # Public API
/// ---

/// Extracts the enclosing type (class, struct, or enum) from a Swift file.
/// Scans until a line containing "// TODO: - " is encountered (or the end of the file if none is found).
/// Returns the last type declaration encountered before the cutoff. If none is found,
/// falls back to returning the file’s basename (without extension).
pub fn extract_enclosing_type(file_path: &str) -> Result<String, String> {
    // Read file content.
    let content = fs::read_to_string(file_path)
        .map_err(|err| format!("Error reading file {}: {}", file_path, err))?;
    
    // Determine the cutoff (TODO marker position).
    let todo_offset = content.find("// TODO: - ").unwrap_or(content.len());

    // First, try to extract using our SwiftParser abstraction.
    if let Some(mut parser) = RealSwiftParser::new() {
        if let Some(ty) = extract_enclosing_type_with_parser(&content, todo_offset, &mut parser) {
            return Ok(ty);
        }
    }
    
    // Fallback: use a regex-based scan over lines until the TODO marker.
    let re = Regex::new(r"(class|struct|enum)\s+(\w+)")
        .map_err(|err| format!("Regex error: {}", err))?;
    let mut last_type: Option<String> = None;
    for line in content.lines() {
        if line.contains("// TODO: -") {
            break;
        }
        if let Some(caps) = re.captures(line) {
            if let Some(type_name) = caps.get(2) {
                last_type = Some(type_name.as_str().to_string());
            }
        }
    }
    
    if let Some(found_type) = last_type {
        Ok(found_type)
    } else {
        // Fallback to the file's basename (without extension).
        let path = Path::new(file_path);
        let fallback = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Unknown")
            .to_string();
        Ok(fallback)
    }
}

/// ---
/// # Legacy Functions (No Longer Used)
/// ---

/*
fn extract_enclosing_type_tree_sitter(content: &str, todo_offset: usize) -> Option<String> { ... }

fn get_named_descendants<'a>(node: Node<'a>) -> Vec<Node<'a>> { ... }
*/

/// ---
/// # Tests
/// ---

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
