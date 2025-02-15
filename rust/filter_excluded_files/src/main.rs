use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// Filters a list of file paths by excluding those whose basenames exactly match any of the provided exclusion patterns.
/// Also, lines that end with a slash (i.e. directories) are excluded.
///
/// # Arguments
///
/// * `lines` - A vector of file paths (each as a `String`).
/// * `exclusions` - A slice of strings representing exclusion patterns.
///
/// # Returns
///
/// A vector of file paths that are not excluded.
pub fn filter_excluded_files_lines(lines: Vec<String>, exclusions: &[String]) -> Vec<String> {
    lines
        .into_iter()
        .filter(|line| {
            let trimmed_line = line.trim();
            // Skip empty lines or lines that end with a slash (directories).
            if trimmed_line.is_empty() || trimmed_line.ends_with('/') {
                return false;
            }
            let path = Path::new(trimmed_line);
            // Attempt to extract the basename.
            let basename = match path.file_name() {
                Some(name) => name.to_string_lossy().trim().to_string(),
                None => return false,
            };
            // Exclude if the basename exactly matches any of the provided patterns.
            !exclusions.iter().any(|pattern| &basename == pattern)
        })
        .collect()
}

fn main() {
    // Expect at least two arguments: the found_files file and one or more exclusion patterns.
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <found_files_file> <exclusion1> [<exclusion2> ...]",
            args[0]
        );
        process::exit(1);
    }
    let found_files_file = &args[1];
    let exclusion_patterns: Vec<String> = args[2..].to_vec();

    // Open the input file (which contains the list of file paths).
    let input_file = match File::open(found_files_file) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error opening input file {}: {}", found_files_file, e);
            process::exit(1);
        }
    };
    let reader = BufReader::new(input_file);
    let lines: Vec<String> = reader.lines().filter_map(Result::ok).collect();

    // Filter the lines using our library function.
    let filtered_lines = filter_excluded_files_lines(lines, &exclusion_patterns);

    // Create a temporary file to write the filtered list.
    let mut temp_path = env::temp_dir();
    let pid = process::id();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let filename = format!("filter_excluded_files_{}_{}.tmp", pid, now.as_nanos());
    temp_path.push(filename);

    let output_file = match File::create(&temp_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error creating temporary file {}: {}", temp_path.display(), e);
            process::exit(1);
        }
    };
    let mut writer = BufWriter::new(output_file);

    for line in filtered_lines {
        if let Err(e) = writeln!(writer, "{}", line) {
            eprintln!("Error writing to output file: {}", e);
            process::exit(1);
        }
    }
    if let Err(e) = writer.flush() {
        eprintln!("Error flushing output file: {}", e);
        process::exit(1);
    }

    // Print the path to the temporary file containing the filtered list.
    println!("{}", temp_path.display());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_exclusions() {
        let lines = vec![
            "/path/to/FileA.swift".to_string(),
            "/path/to/FileB.swift".to_string(),
            "/path/to/FileC.swift".to_string(),
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines.clone(), &exclusions);
        assert_eq!(filtered, lines);
    }

    #[test]
    fn test_exclude_one() {
        let lines = vec![
            "/path/to/FileA.swift".to_string(),
            "/path/to/FileB.swift".to_string(),
            "/path/to/FileC.swift".to_string(),
        ];
        let exclusions = vec!["FileB.swift".to_string()];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        assert_eq!(
            filtered,
            vec![
                "/path/to/FileA.swift".to_string(),
                "/path/to/FileC.swift".to_string(),
            ]
        );
    }

    #[test]
    fn test_exclude_multiple() {
        let lines = vec![
            "/path/to/FileA.swift".to_string(),
            "/another/path/FileB.swift".to_string(),
            "/yet/another/path/FileC.swift".to_string(),
            "/different/FileD.swift".to_string(),
        ];
        let exclusions = vec!["FileB.swift".to_string(), "FileD.swift".to_string()];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        assert_eq!(
            filtered,
            vec![
                "/path/to/FileA.swift".to_string(),
                "/yet/another/path/FileC.swift".to_string(),
            ]
        );
    }

    #[test]
    fn test_empty_lines() {
        let lines = vec![
            "".to_string(),
            "   ".to_string(),
            "/path/to/FileA.swift".to_string(),
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        assert_eq!(filtered, vec!["/path/to/FileA.swift".to_string()]);
    }

    #[test]
    fn test_no_basename() {
        // Test lines that are directories or root, which should be excluded.
        let lines = vec![
            "/".to_string(),             // root, should be excluded
            "/path/to/".to_string(),       // directory with trailing slash, should be excluded
            "/path/to/FileA.swift".to_string(),
        ];
        let exclusions: Vec<String> = vec![];
        let filtered = filter_excluded_files_lines(lines, &exclusions);
        // Only the file with a valid basename should be included.
        assert_eq!(filtered, vec!["/path/to/FileA.swift".to_string()]);
    }
}
