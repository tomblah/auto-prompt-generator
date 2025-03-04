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
    // Internally use the finder struct.
    let finder = PromptInstructionFinder::new(search_dir, verbose);
    finder.find()
}

// === Private Implementation === //

struct PromptInstructionFinder<'a> {
    search_dir: &'a str,
    verbose: bool,
    allowed_extensions: &'static [&'static str],
    todo_marker: &'static str,
}

impl<'a> PromptInstructionFinder<'a> {
    fn new(search_dir: &'a str, verbose: bool) -> Self {
        Self {
            search_dir,
            verbose,
            allowed_extensions: &["swift", "h", "m", "js"],
            todo_marker: "// TODO: - ",
        }
    }

    fn find(&self) -> io::Result<PathBuf> {
        // Collect matching files using iterator combinators.
        let matching_files: Vec<PathBuf> = WalkDir::new(self.search_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .filter(|path| {
                // Check if the file has an allowed extension.
                path.extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| self.allowed_extensions.contains(&ext))
                    .unwrap_or(false)
            })
            .filter(|path| {
                // Open the file and check if any line contains the TODO marker.
                if let Ok(file) = fs::File::open(path) {
                    let reader = io::BufReader::new(file);
                    reader
                        .lines()
                        .filter_map(Result::ok)
                        .any(|line| line.contains(self.todo_marker))
                } else {
                    false
                }
            })
            .collect();

        if matching_files.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("No files found containing '{}'", self.todo_marker),
            ));
        }

        // Choose the file with the most recent modification time.
        let chosen_file = matching_files
            .iter()
            .max_by(|a, b| {
                let mod_a = fs::metadata(a)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let mod_b = fs::metadata(b)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                mod_a.cmp(&mod_b)
            })
            .expect("At least one file exists")
            .clone();

        if self.verbose {
            eprintln!("[VERBOSE] {} matching file(s) found.", matching_files.len());
            if matching_files.len() > 1 {
                eprintln!("[VERBOSE] Ignoring the following files:");
                for file in matching_files.iter().filter(|&f| f != &chosen_file) {
                    let basename = file
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("<unknown>");
                    let todo_line = extract_first_todo_line(file, self.todo_marker)
                        .unwrap_or_else(|| "<no TODO line found>".to_string());
                    eprintln!("  - {}: {}", basename, todo_line.trim());
                    eprintln!("--------------------------------------------------");
                }
                eprintln!(
                    "[VERBOSE] Chosen file: {}",
                    chosen_file.display()
                );
            } else {
                eprintln!(
                    "[VERBOSE] Only one matching file found: {}",
                    chosen_file.display()
                );
            }
        }

        Ok(chosen_file)
    }
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

// --- Additional internal tests for find_prompt_instruction ---

#[cfg(test)]
mod internal_tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write;
    use std::path::Path;
    use tempfile::{tempdir, NamedTempFile};
    use filetime::{set_file_mtime, FileTime};
    use std::time::SystemTime;

    // Test the helper that extracts the first TODO line.
    #[test]
    fn test_extract_first_todo_line_found() {
        // Create a temporary file with a TODO marker.
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line one").unwrap();
        writeln!(temp_file, "// TODO: - This is the todo line").unwrap();
        writeln!(temp_file, "Line three").unwrap();

        let path = temp_file.path();
        let result = extract_first_todo_line(path, "// TODO: - ");
        assert!(result.is_some(), "Expected to find a TODO line");
        let line = result.unwrap();
        assert!(line.contains("This is the todo line"), "Unexpected TODO line: {}", line);
    }

    #[test]
    fn test_extract_first_todo_line_not_found() {
        // Create a file that does not include the marker.
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line one").unwrap();
        writeln!(temp_file, "Line two without marker").unwrap();

        let path = temp_file.path();
        let result = extract_first_todo_line(path, "// TODO: - ");
        assert!(result.is_none(), "Did not expect a TODO line to be found");
    }

    // Directly test the PromptInstructionFinder's internal logic.
    #[test]
    fn test_prompt_instruction_finder_with_multiple_files() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("file1.swift");
        let file2 = dir.path().join("file2.swift");

        // Write content with the TODO marker in both files.
        fs::write(&file1, "Some content\n// TODO: - First todo\nMore content").unwrap();
        fs::write(&file2, "Other content\n// TODO: - Second todo\nExtra content").unwrap();

        // Set explicit modification times so file2 is more recent.
        let ft1 = FileTime::from_unix_time(1000, 0);
        let ft2 = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&file1, ft1).unwrap();
        set_file_mtime(&file2, ft2).unwrap();

        // Create a finder instance manually.
        let finder = PromptInstructionFinder::new(dir.path().to_str().unwrap(), false);
        let chosen_file = finder.find().expect("Expected to find a valid file");
        // Since file2 is more recent, it should be chosen.
        assert_eq!(chosen_file, file2, "Expected file2 to be chosen as the most recent file");
    }

    // Test that files with disallowed extensions are ignored.
    #[test]
    fn test_finder_excludes_disallowed_extension() {
        let dir = tempdir().unwrap();
        let file_txt = dir.path().join("ignored.txt"); // .txt is not an allowed extension.
        fs::write(&file_txt, "Text content\n// TODO: - This should not be picked\nMore text").unwrap();

        let finder = PromptInstructionFinder::new(dir.path().to_str().unwrap(), false);
        let result = finder.find();
        assert!(result.is_err(), "Expected an error because no allowed files were found");
    }

    // (Optional) Test the behavior when a file is unreadable.
    #[cfg(unix)]
    #[test]
    fn test_finder_skips_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let unreadable = dir.path().join("unreadable.swift");
        let readable = dir.path().join("readable.swift");

        fs::write(&unreadable, "Content\n// TODO: - Unreadable todo\nMore").unwrap();
        fs::write(&readable, "Valid content\n// TODO: - Readable todo\nFooter").unwrap();

        // Remove read permissions for the unreadable file.
        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&unreadable, perms).unwrap();

        // Set modification times so the readable file is more recent.
        let ft_unreadable = FileTime::from_unix_time(1000, 0);
        let ft_readable = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&unreadable, ft_unreadable).unwrap();
        set_file_mtime(&readable, ft_readable).unwrap();

        let finder = PromptInstructionFinder::new(dir.path().to_str().unwrap(), false);
        let chosen_file = finder.find().expect("Expected to pick a valid file");
        // Since the unreadable file should be skipped, the readable file is expected.
        assert_eq!(chosen_file, readable, "Expected the readable file to be chosen");

        // Restore permissions for cleanup.
        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&unreadable, perms).unwrap();
    }
}
