use std::env;
use std::fs;
use std::io::{self, Write};

/// Processes a file’s content by outputting only the text between substring markers.
/// The markers are defined as:
///   - An opening marker: a line that, when trimmed, is exactly "// v"
///   - A closing marker: a line that, when trimmed, is exactly "// ^"
/// Lines outside these markers are omitted (replaced by a placeholder).
fn filter_substring_markers(content: &str) -> String {
    let mut output = String::new();
    let mut in_block = false;
    let mut last_was_placeholder = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "// v" {
            if !last_was_placeholder {
                output.push_str("\n// ...\n");
                last_was_placeholder = true;
            }
            in_block = true;
            continue;
        }
        if trimmed == "// ^" {
            in_block = false;
            if !last_was_placeholder {
                output.push_str("\n// ...\n");
                last_was_placeholder = true;
            }
            continue;
        }
        if in_block {
            output.push_str(line);
            output.push('\n');
            last_was_placeholder = false;
        }
    }
    output
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        writeln!(io::stderr(), "Usage: {} <file_path>", args[0]).unwrap();
        std::process::exit(1);
    }
    let file_path = &args[1];
    let content = fs::read_to_string(file_path).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", file_path, err);
        std::process::exit(1);
    });
    let filtered = filter_substring_markers(&content);
    print!("{}", filtered);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_markers() {
        let input = "Just some text\nwithout any markers.";
        let expected = "";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_single_block() {
        let input = "\
Header text
// v
Inside block line 1
Inside block line 2
// ^
Footer text";
        let expected = "\n// ...\nInside block line 1\nInside block line 2\n\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_only_start_marker() {
        let input = "\
Some text before
// v
Block content line 1
Block content line 2";
        let expected = "\n// ...\nBlock content line 1\nBlock content line 2\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_only_end_marker() {
        let input = "\
Intro text
// ^
Conclusion text";
        let expected = "\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_consecutive_markers() {
        let input = "\
// v
Line 1
// v
Line 2
// ^
Line 3
// ^";
        let expected = "\n// ...\nLine 1\n\n// ...\nLine 2\n\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_markers_with_extra_whitespace() {
        let input = "   // v\nBlock content line\n   // ^  ";
        let expected = "\n// ...\nBlock content line\n\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }
}
