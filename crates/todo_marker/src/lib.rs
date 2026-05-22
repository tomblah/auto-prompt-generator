// crates/todo_marker/src/lib.rs

//! Shared TODO marker constants and position‑analysis helpers used
//! throughout the prompt‑generation tool‑chain.

/// Exact form **without** the trailing space.
pub const TODO_MARKER: &str = "// TODO: -";

/// Exact form **with** a trailing space (the version most
/// parsers look for when scanning file content).
pub const TODO_MARKER_WS: &str = "// TODO: - ";

/// Zero‑based index of the first line that contains `TODO_MARKER_WS`.
/// Returns `None` if the marker is not present.
pub fn todo_index(content: &str) -> Option<usize> {
    content
        .lines()
        .position(|line| line.contains(TODO_MARKER_WS))
}

/// Returns `true` if the TODO marker at `todo_idx` lives inside an active
/// substring‑marker block (`// v` … `// ^`).
pub fn is_todo_inside_markers(content: &str, todo_idx: usize) -> bool {
    let lines: Vec<&str> = content.lines().collect();
    let mut marker_depth = 0;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed == "// v" {
            marker_depth += 1;
        } else if trimmed == "// ^" && marker_depth > 0 {
            marker_depth -= 1;
        }
        if i == todo_idx {
            break;
        }
    }
    marker_depth > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_todo_marker_constants_match_expected_strings() {
        assert_eq!(TODO_MARKER, "// TODO: -");
        assert_eq!(TODO_MARKER_WS, "// TODO: - ");
    }

    #[test]
    fn test_todo_index() {
        let content = "Line1\nLine2 // TODO: - Fix issue\nLine3";
        let idx = todo_index(content);
        assert!(idx.is_some());
        let lines: Vec<&str> = content.lines().collect();
        let index = idx.unwrap();
        assert!(lines[index].contains("// TODO: -"));
    }

    #[test]
    fn test_todo_index_none_when_absent() {
        let content = "Line1\nLine2\nLine3";
        assert!(todo_index(content).is_none());
    }

    #[test]
    fn test_is_todo_inside_markers_true() {
        let content = "\
Line1
// v
// TODO: - inside markers
// ^
Line after";
        let idx = todo_index(content).unwrap();
        assert!(is_todo_inside_markers(content, idx));
    }

    #[test]
    fn test_is_todo_inside_markers_false() {
        let content = "\
Line1
// TODO: - outside markers
// v
Marker start
// ^
More text";
        let idx = todo_index(content).unwrap();
        assert!(!is_todo_inside_markers(content, idx));
    }
}
