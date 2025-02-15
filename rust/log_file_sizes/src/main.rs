use std::env;
use std::fs;
use std::path::Path;
use std::process;

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
        eprintln!("Usage: {} <file_list>", args[0]);
        process::exit(1);
    }
    let file_list_path = &args[1];

    let file_list = fs::read_to_string(file_list_path).unwrap_or_else(|err| {
        eprintln!("Error reading file list: {}", err);
        process::exit(1);
    });

    // Store (basename, size) pairs.
    let mut sizes: Vec<(String, usize)> = file_list
        .lines()
        .map(|file_path| {
            let content = fs::read_to_string(file_path)
                .unwrap_or_else(|_| String::new());

            // If the file contains a line exactly matching "// v", apply filtering.
            let filtered_content = if content
                .lines()
                .any(|line| line.trim() == "// v")
            {
                filter_substring_markers(&content)
            } else {
                content
            };

            let size = filtered_content.chars().count();
            let basename = Path::new(file_path)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
            (basename, size)
        })
        .collect();

    // Sort descending by size.
    sizes.sort_by(|a, b| b.1.cmp(&a.1));

    for (basename, size) in sizes {
        println!("{} ({})", basename, size);
    }
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
        let input = r#"Header text
// v
Inside block line 1
Inside block line 2
// ^
Footer text"#;
        let expected = "\n// ...\nInside block line 1\nInside block line 2\n\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_only_start_marker() {
        let input = r#"Some text before
// v
Block content line 1
Block content line 2"#;
        let expected = "\n// ...\nBlock content line 1\nBlock content line 2\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_only_end_marker() {
        let input = r#"Intro text
// ^
Conclusion text"#;
        let expected = "\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_consecutive_markers() {
        let input = r#"// v
Line 1
// v
Line 2
// ^
Line 3
// ^"#;
        let expected = "\n// ...\nLine 1\n\n// ...\nLine 2\n\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }

    #[test]
    fn test_markers_with_extra_whitespace() {
        let input = r#"   // v
Block content line
   // ^  "#;
        let expected = "\n// ...\nBlock content line\n\n// ...\n";
        let output = filter_substring_markers(input);
        assert_eq!(output, expected);
    }
}
