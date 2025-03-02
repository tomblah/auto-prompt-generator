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
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

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

    // --- New tests begin here ---

    #[test]
    fn test_extension_filtering() {
        let dir = tempdir().unwrap();
        // Create a file with a disallowed extension (.txt) that contains the marker.
        let file_path = dir.path().join("ignored.txt");
        fs::write(&file_path, "Some text\n// TODO: - This should be ignored\nMore text").unwrap();

        // Since the file has a .txt extension (not one of the allowed extensions), the function should not pick it.
        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false);
        assert!(result.is_err(), "Expected error because no valid files found");
    }

    #[test]
    fn test_recursive_search() {
        let dir = tempdir().unwrap();
        // Create a nested directory structure.
        let nested_dir = dir.path().join("nested/subdir");
        fs::create_dir_all(&nested_dir).unwrap();
        let file_path = nested_dir.join("nested.swift");
        fs::write(&file_path, "Header\n// TODO: - Nested todo\nFooter").unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false).unwrap();
        assert_eq!(result, file_path);
    }

    #[test]
    fn test_verbose_output_with_multiple_files() {
        // Even though we won't capture the verbose output from stderr,
        // we can ensure that the function returns the correct file.
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("file1.swift");
        let file2 = dir.path().join("file2.swift");
        fs::write(&file1, "Content\n// TODO: - Todo in file1\nMore").unwrap();
        fs::write(&file2, "Other content\n// TODO: - Todo in file2\nExtra").unwrap();

        // Set modification times so that file2 is more recent.
        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&file1, older_time).unwrap();
        set_file_mtime(&file2, newer_time).unwrap();

        // Call function with verbose output enabled.
        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), true).unwrap();
        // Expect that the most recent file is chosen.
        assert_eq!(result, file2);
    }

    #[test]
    fn test_same_modification_time_tie() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("tie1.swift");
        let file2 = dir.path().join("tie2.swift");
        fs::write(&file1, "Alpha\n// TODO: - Tie todo 1\nBeta").unwrap();
        fs::write(&file2, "Gamma\n// TODO: - Tie todo 2\nDelta").unwrap();

        // Set both files to have the same modification time.
        let same_time = FileTime::from_unix_time(1500, 0);
        set_file_mtime(&file1, same_time).unwrap();
        set_file_mtime(&file2, same_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false).unwrap();
        // The result should be one of the two files.
        assert!(result == file1 || result == file2, "Result should be either tie1.swift or tie2.swift");
    }

    #[cfg(unix)]
    #[test]
    fn test_unreadable_file_is_skipped() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let unreadable = dir.path().join("unreadable.swift");
        let readable = dir.path().join("readable.swift");
        fs::write(&unreadable, "Some text\n// TODO: - Unreadable todo\nMore text").unwrap();
        fs::write(&readable, "Valid content\n// TODO: - Readable todo\nFooter").unwrap();

        // Make the unreadable file have no read permissions.
        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&unreadable, perms).unwrap();

        // Set modification times so that the readable file is more recent.
        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&unreadable, older_time).unwrap();
        set_file_mtime(&readable, newer_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false).unwrap();
        // The unreadable file should be skipped, so the returned file should be the readable one.
        assert_eq!(result, readable);

        // Restore permissions for cleanup.
        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&unreadable, perms).unwrap();
    }
}

