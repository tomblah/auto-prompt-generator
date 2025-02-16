use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use tempfile::NamedTempFile;

/// Filters the list of candidate file paths for slim mode.
/// Always includes the `todo_file` and excludes any candidate whose
/// basename contains one of the disallowed keywords.
pub fn filter_files_for_slim_mode(todo_file: &str, candidate_files: Vec<String>) -> Vec<String> {
    let mut filtered = Vec::new();
    // Always include the TODO file.
    filtered.push(todo_file.to_string());

    // Define the exclusion patterns.
    let exclude_patterns = [
        "ViewController",
        "Manager",
        "Presenter",
        "Router",
        "Interactor",
        "Configurator",
        "DataSource",
        "Delegate",
        "View",
    ];

    for line in candidate_files {
        // Skip if it's exactly the TODO file.
        if line == todo_file {
            continue;
        }
        // Check the basename for exclusion patterns.
        if let Some(basename) = Path::new(&line).file_name().and_then(|s| s.to_str()) {
            if exclude_patterns.iter().any(|pat| basename.contains(pat)) {
                continue;
            }
        }
        filtered.push(line);
    }

    filtered
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Expect exactly two arguments: <todo_file> and <found_files_file>
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <todo_file> <found_files_file>", args[0]);
        std::process::exit(1);
    }
    let todo_file = &args[1];
    let found_files_file = &args[2];

    // Read candidate file paths from the provided file.
    let file = File::open(found_files_file)?;
    let reader = BufReader::new(file);
    let candidate_files: Vec<String> = reader.lines().collect::<Result<_, _>>()?;

    // Filter the list.
    let filtered = filter_files_for_slim_mode(todo_file, candidate_files);

    // Write the filtered list to a temporary file.
    let mut temp_file = NamedTempFile::new()?;
    for file in &filtered {
        writeln!(temp_file, "{}", file)?;
    }
    let temp_path = temp_file.into_temp_path().keep()?;
    println!("{}", temp_path.display());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_includes_todo_file_always() {
        let todo = "Test.swift";
        let candidates = vec!["Test.swift".to_string(), "Another.swift".to_string()];
        let filtered = filter_files_for_slim_mode(todo, candidates);
        assert!(filtered.contains(&"Test.swift".to_string()));
    }

    #[test]
    fn test_excludes_disallowed_files() {
        let todo = "Test.swift";
        let candidates = vec![
            "Test.swift".to_string(),
            "Manager.swift".to_string(),
            "Normal.swift".to_string(),
            "PresenterHelper.swift".to_string(),
            "Extra.swift".to_string(),
        ];
        let filtered = filter_files_for_slim_mode(todo, candidates);
        // "Manager.swift" and "PresenterHelper.swift" should be excluded.
        assert!(filtered.contains(&"Test.swift".to_string()));
        assert!(filtered.contains(&"Normal.swift".to_string()));
        assert!(filtered.contains(&"Extra.swift".to_string()));
        assert!(!filtered.contains(&"Manager.swift".to_string()));
        assert!(!filtered.contains(&"PresenterHelper.swift".to_string()));
    }

    #[test]
    fn test_does_not_duplicate_todo_file() {
        let todo = "Test.swift";
        let candidates = vec![
            "Test.swift".to_string(),
            "Test.swift".to_string(),
            "Normal.swift".to_string(),
        ];
        let filtered = filter_files_for_slim_mode(todo, candidates);
        // The todo file should appear only once.
        let count = filtered.iter().filter(|&s| s == todo).count();
        assert_eq!(count, 1);
    }
}
