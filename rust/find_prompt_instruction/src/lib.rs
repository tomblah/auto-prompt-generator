// rust/find_prompt_instruction/src/lib.rs

use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Searches the given directory (and its subdirectories) for files with allowed extensions
/// that contain the TODO marker. If multiple files are found, returns the one with the most
/// recent modification time. If verbose is true, logs extra details.
///
/// Allowed extensions are: `swift`, `h`, `m`, and `js`.
/// The marker searched for is: "// TODO: - "
pub fn find_prompt_instruction_in_dir(search_dir: &str, verbose: bool) -> io::Result<PathBuf> {
    let allowed_extensions = ["swift", "h", "m", "js"];
    let todo_marker = "// TODO: - ";
    let mut matching_files: Vec<PathBuf> = Vec::new();

    for entry in WalkDir::new(search_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.into_path();
        if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
            if !allowed_extensions.contains(&ext) {
                continue;
            }
        } else {
            continue;
        }
        if let Ok(file) = fs::File::open(&path) {
            let reader = io::BufReader::new(file);
            if reader
                .lines()
                .filter_map(Result::ok)
                .any(|line| line.contains(todo_marker))
            {
                matching_files.push(path);
            }
        }
    }

    if matching_files.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("No files found containing '{}'", todo_marker),
        ));
    }

    if matching_files.len() == 1 {
        if verbose {
            eprintln!(
                "[VERBOSE] Only one matching file found: {}",
                matching_files[0].display()
            );
        }
        return Ok(matching_files[0].clone());
    }

    // Choose the file with the most recent modification time.
    let mut chosen_file = matching_files[0].clone();
    let mut chosen_mod_time = fs::metadata(&chosen_file)
        .and_then(|meta| meta.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);

    for file in &matching_files {
        if let Ok(meta) = fs::metadata(file) {
            if let Ok(mod_time) = meta.modified() {
                if mod_time > chosen_mod_time {
                    chosen_file = file.clone();
                    chosen_mod_time = mod_time;
                }
            }
        }
    }

    if verbose {
        eprintln!("[VERBOSE] Multiple matching files found. Ignoring the following files:");
        for file in matching_files.iter().filter(|&f| f != &chosen_file) {
            let basename = file
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("<unknown>");
            let todo_line = extract_first_todo_line(file, todo_marker)
                .unwrap_or_else(|| "<no TODO line found>".to_string());
            eprintln!("  - {}: {}", basename, todo_line.trim());
            eprintln!("--------------------------------------------------");
        }
        eprintln!("[VERBOSE] Chosen file: {}", chosen_file.display());
    }

    Ok(chosen_file)
}

/// Private helper: extracts the first line in the file that contains the given marker.
fn extract_first_todo_line(path: &Path, marker: &str) -> Option<String> {
    if let Ok(file) = fs::File::open(path) {
        let reader = io::BufReader::new(file);
        for line in reader.lines().filter_map(Result::ok) {
            if line.contains(marker) {
                return Some(line);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::{set_file_mtime, FileTime};
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_no_files_found() {
        let dir = tempdir().unwrap();
        // Create a file that does not contain the TODO marker.
        let file_path = dir.path().join("dummy.swift");
        fs::write(&file_path, "Some random content without marker").unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_single_file_found() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.swift");
        let content = "Some content\n// TODO: - Fix something\nOther content";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false).unwrap();
        assert_eq!(result, file_path);
    }

    #[test]
    fn test_multiple_files_choose_most_recent() {
        let dir = tempdir().unwrap();
        let older_file = dir.path().join("older.swift");
        let newer_file = dir.path().join("newer.swift");

        // Write content with the TODO marker into both files.
        fs::write(&older_file, "Content\n// TODO: - Old todo\nMore content").unwrap();
        fs::write(&newer_file, "Other content\n// TODO: - New todo\nExtra content").unwrap();

        // Set modification times explicitly: older_file gets an older timestamp.
        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&older_file, older_time).unwrap();
        set_file_mtime(&newer_file, newer_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false).unwrap();
        assert_eq!(result, newer_file);
    }
}
