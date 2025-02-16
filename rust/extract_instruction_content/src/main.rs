use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process;

fn main() {
    // Expect exactly one argument: the path to the Swift file.
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <swift_file>", args[0]);
        process::exit(1);
    }
    let file_path = &args[1];

    // Attempt to open the file.
    let file = File::open(file_path).unwrap_or_else(|err| {
        eprintln!("Error opening file {}: {}", file_path, err);
        process::exit(1);
    });
    let reader = BufReader::new(file);
    let marker = "// TODO: - ";

    // Iterate through the file's lines, looking for the marker.
    for line_result in reader.lines() {
        match line_result {
            Ok(line) => {
                if line.contains(marker) {
                    // Trim leading whitespace and print the line.
                    println!("{}", line.trim_start());
                    return;
                }
            }
            Err(err) => {
                eprintln!("Error reading file {}: {}", file_path, err);
                process::exit(1);
            }
        }
    }

    eprintln!("Error: No valid TODO instruction found in {}", file_path);
    process::exit(1);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    // Helper function to simulate reading from a string.
    fn extract_instruction_from_content(content: &str) -> Option<String> {
        let reader = BufReader::new(Cursor::new(content));
        let marker = "// TODO: - ";
        for line_result in reader.lines() {
            if let Ok(line) = line_result {
                if line.contains(marker) {
                    return Some(line.trim_start().to_string());
                }
            }
        }
        None
    }

    #[test]
    fn test_extract_instruction_found() {
        let content = "Some initial text\n    // TODO: - Fix the bug\nMore text";
        let extracted = extract_instruction_from_content(content);
        assert_eq!(extracted, Some("// TODO: - Fix the bug".to_string()));
    }

    #[test]
    fn test_no_instruction_found() {
        let content = "No relevant line here\nAnother line";
        let extracted = extract_instruction_from_content(content);
        assert_eq!(extracted, None);
    }
}
