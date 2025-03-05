use regex::Regex;
use std::fs;
use std::path::{Component, Path};
use walkdir::WalkDir;
use anyhow::{Result, Context};

// Import tree-sitter and the Swift language.
use tree_sitter::{Parser, Node};
use tree_sitter_swift;

pub fn find_files_referencing(
    type_name: &str,
    search_root: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Allowed file extensions remain unchanged.
    let allowed_extensions = ["swift", "h", "m", "js"];
    let mut results = Vec::new();

    // Recursively traverse the directory.
    for entry in WalkDir::new(search_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();

        // Determine file extension.
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e.to_lowercase(),
            None => continue,
        };
        if !allowed_extensions.contains(&ext.as_str()) {
            continue;
        }

        // Skip files in directories named "Pods" or ".build".
        if path.components().any(|comp| {
            if let Component::Normal(os_str) = comp {
                let s = os_str.to_string_lossy();
                s == "Pods" || s == ".build"
            } else {
                false
            }
        }) {
            continue;
        }

        // Read the file's contents.
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // For Swift files, try using tree-sitter; otherwise fall back to regex.
        let found = if ext == "swift" {
            match contains_type_reference_swift(&content, type_name) {
                Ok(found) => found,
                Err(_) => {
                    // Fall back to regex matching if parsing fails.
                    let pattern = format!(r"\b{}\b", regex::escape(type_name));
                    let re = Regex::new(&pattern)?;
                    re.is_match(&content)
                }
            }
        } else {
            let pattern = format!(r"\b{}\b", regex::escape(type_name));
            let re = Regex::new(&pattern)?;
            re.is_match(&content)
        };

        if found {
            results.push(path.display().to_string());
        }
    }

    Ok(results)
}

/// Uses tree-sitter to parse Swift file content and searches the AST for
/// a node whose text exactly matches `type_name`. In addition to checking
/// nodes of kind "identifier", it now also checks "type_identifier".
fn contains_type_reference_swift(content: &str, type_name: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let mut parser = Parser::new();
    parser.set_language(unsafe { std::mem::transmute(tree_sitter_swift::LANGUAGE) })
        .context("Failed to set tree-sitter language")?;
    
    let tree = parser.parse(content, None)
        .ok_or("Tree-sitter failed to parse content")?;
    let root_node = tree.root_node();

    // Iterate over all named descendant nodes.
    for node in get_named_descendants(root_node) {
        let kind = node.kind();
        if kind == "identifier" || kind == "type_identifier" {
            if let Ok(text) = node.utf8_text(content.as_bytes()) {
                if text == type_name {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

/// Recursively collects all named descendant nodes of the given node.
fn get_named_descendants<'a>(node: Node<'a>) -> Vec<Node<'a>> {
    let mut result = Vec::new();
    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child(i) {
            result.push(child);
            result.extend(get_named_descendants(child));
        }
    }
    result
}
