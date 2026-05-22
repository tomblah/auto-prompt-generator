// crates/find_prompt_instruction/src/lib.rs

use anyhow::{anyhow, Context, Result};
use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use todo_marker::TODO_MARKER_WS;
use walkdir::WalkDir;

/// Searches the given directory (and its subdirectories) for files with allowed extensions
/// that contain the TODO marker. If multiple files are found, returns the one with the most
/// recent modification time. If verbose is true, logs extra details.
///
/// Allowed extensions are: `swift`, `h`, `m`, and `js`.
/// The marker searched for is `todo_marker::TODO_MARKER_WS`.
pub fn find_prompt_instruction_in_dir(search_dir: &Path, verbose: bool) -> Result<PathBuf> {
    let finder = PromptInstructionFinder::new(search_dir, verbose);
    finder.find()
}

// === Private Implementation === //

struct PromptInstructionFinder<'a> {
    search_dir: &'a Path,
    verbose: bool,
    allowed_extensions: &'static [&'static str],
    todo_marker: &'static str,
}

impl<'a> PromptInstructionFinder<'a> {
    fn new(search_dir: &'a Path, verbose: bool) -> Self {
        Self {
            search_dir,
            verbose,
            allowed_extensions: &["swift", "h", "m", "js"],
            todo_marker: TODO_MARKER_WS,
        }
    }

    fn find(&self) -> Result<PathBuf> {
        let matching_files: Vec<PathBuf> = WalkDir::new(self.search_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .filter(|path| {
                path.extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| self.allowed_extensions.contains(&ext))
                    .unwrap_or(false)
            })
            .filter(|path| {
                if let Ok(file) = fs::File::open(path) {
                    let reader = io::BufReader::new(file);
                    reader
                        .lines()
                        .map_while(Result::ok)
                        .any(|line| line.contains(self.todo_marker))
                } else {
                    false
                }
            })
            .collect();

        if matching_files.is_empty() {
            return Err(anyhow!("No files found containing '{}'", self.todo_marker));
        }

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
            .cloned()
            .ok_or_else(|| anyhow!("No files found containing '{}'", self.todo_marker))?;

        let content = fs::read_to_string(&chosen_file)
            .with_context(|| format!("Failed to read {}", chosen_file.display()))?;
        let marker_lines: Vec<String> = content
            .lines()
            .filter(|line| line.contains(self.todo_marker))
            .map(|line| line.trim().to_string())
            .collect();
        let marker_count = marker_lines.len();
        if marker_count > 1 {
            return Err(anyhow!(
                "Ambiguous TODO marker: file {} contains {} markers:\n{}",
                chosen_file.display(),
                marker_count,
                marker_lines.join("\n")
            ));
        }

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
                eprintln!("[VERBOSE] Chosen file: {}", chosen_file.display());
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
        for line in reader.lines().map_while(Result::ok) {
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
        let file_path = dir.path().join("dummy.swift");
        fs::write(&file_path, "Some random content without marker").unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false);
        let err = result.expect_err("expected missing TODO marker to return an error");
        assert!(err.to_string().contains(TODO_MARKER_WS));
    }

    #[test]
    fn test_finder_uses_shared_todo_marker() {
        let dir = tempdir().unwrap();
        let finder = PromptInstructionFinder::new(dir.path(), false);

        assert_eq!(finder.todo_marker, todo_marker::TODO_MARKER_WS);
    }

    #[test]
    fn test_single_file_found() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.swift");
        let content = "Some content\n// TODO: - Fix something\nOther content";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(result, file_path);
    }

    #[test]
    fn test_multiple_files_choose_most_recent() {
        let dir = tempdir().unwrap();
        let older_file = dir.path().join("older.swift");
        let newer_file = dir.path().join("newer.swift");

        fs::write(&older_file, "Content\n// TODO: - Old todo\nMore content").unwrap();
        fs::write(
            &newer_file,
            "Other content\n// TODO: - New todo\nExtra content",
        )
        .unwrap();

        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&older_file, older_time).unwrap();
        set_file_mtime(&newer_file, newer_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(result, newer_file);
    }

    #[test]
    fn test_extension_filtering() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("ignored.txt");
        fs::write(
            &file_path,
            "Some text\n// TODO: - This should be ignored\nMore text",
        )
        .unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false);
        assert!(
            result.is_err(),
            "Expected error because no valid files found"
        );
    }

    #[test]
    fn test_recursive_search() {
        let dir = tempdir().unwrap();
        let nested_dir = dir.path().join("nested/subdir");
        fs::create_dir_all(&nested_dir).unwrap();
        let file_path = nested_dir.join("nested.swift");
        fs::write(&file_path, "Header\n// TODO: - Nested todo\nFooter").unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(result, file_path);
    }

    #[test]
    fn test_verbose_output_with_multiple_files() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("file1.swift");
        let file2 = dir.path().join("file2.swift");
        fs::write(&file1, "Content\n// TODO: - Todo in file1\nMore").unwrap();
        fs::write(&file2, "Other content\n// TODO: - Todo in file2\nExtra").unwrap();

        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&file1, older_time).unwrap();
        set_file_mtime(&file2, newer_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), true).unwrap();
        assert_eq!(result, file2);
    }

    #[test]
    fn test_verbose_output_with_single_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("single.swift");
        fs::write(&file_path, "Header\n// TODO: - Single verbose todo\nFooter").unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), true).unwrap();

        assert_eq!(result, file_path);
    }

    #[test]
    fn test_same_modification_time_tie() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("tie1.swift");
        let file2 = dir.path().join("tie2.swift");
        fs::write(&file1, "Alpha\n// TODO: - Tie todo 1\nBeta").unwrap();
        fs::write(&file2, "Gamma\n// TODO: - Tie todo 2\nDelta").unwrap();

        let same_time = FileTime::from_unix_time(1500, 0);
        set_file_mtime(&file1, same_time).unwrap();
        set_file_mtime(&file2, same_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert!(
            result == file1 || result == file2,
            "Result should be either tie1.swift or tie2.swift"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_unreadable_file_is_skipped() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let unreadable = dir.path().join("unreadable.swift");
        let readable = dir.path().join("readable.swift");
        fs::write(
            &unreadable,
            "Some text\n// TODO: - Unreadable todo\nMore text",
        )
        .unwrap();
        fs::write(&readable, "Valid content\n// TODO: - Readable todo\nFooter").unwrap();

        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&unreadable, older_time).unwrap();
        set_file_mtime(&readable, newer_time).unwrap();

        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&unreadable, perms).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(result, readable);

        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&unreadable, perms).unwrap();
    }
}

#[cfg(test)]
mod internal_tests {
    use super::*;
    use filetime::{set_file_mtime, FileTime};
    use std::fs::{self};
    use std::io::Write;
    use tempfile::{tempdir, NamedTempFile};

    #[test]
    fn test_extract_first_todo_line_found() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line one").unwrap();
        writeln!(temp_file, "// TODO: - This is the todo line").unwrap();
        writeln!(temp_file, "Line three").unwrap();

        let path = temp_file.path();
        let result = extract_first_todo_line(path, "// TODO: - ");
        assert!(result.is_some(), "Expected to find a TODO line");
        let line = result.unwrap();
        assert!(
            line.contains("This is the todo line"),
            "Unexpected TODO line: {}",
            line
        );
    }

    #[test]
    fn test_extract_first_todo_line_not_found() {
        let mut temp_file = NamedTempFile::new().unwrap();
        writeln!(temp_file, "Line one").unwrap();
        writeln!(temp_file, "Line two without marker").unwrap();

        let path = temp_file.path();
        let result = extract_first_todo_line(path, "// TODO: - ");
        assert!(result.is_none(), "Did not expect a TODO line to be found");
    }

    #[test]
    fn test_prompt_instruction_finder_with_multiple_files() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("file1.swift");
        let file2 = dir.path().join("file2.swift");

        fs::write(&file1, "Some content\n// TODO: - First todo\nMore content").unwrap();
        fs::write(
            &file2,
            "Other content\n// TODO: - Second todo\nExtra content",
        )
        .unwrap();

        let ft1 = FileTime::from_unix_time(1000, 0);
        let ft2 = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&file1, ft1).unwrap();
        set_file_mtime(&file2, ft2).unwrap();

        let finder = PromptInstructionFinder::new(dir.path(), false);
        let chosen_file = finder.find().expect("Expected to find a valid file");
        assert_eq!(
            chosen_file, file2,
            "Expected file2 to be chosen as the most recent file"
        );
    }

    #[test]
    fn test_finder_excludes_disallowed_extension() {
        let dir = tempdir().unwrap();
        let file_txt = dir.path().join("ignored.txt");
        fs::write(
            &file_txt,
            "Text content\n// TODO: - This should not be picked\nMore text",
        )
        .unwrap();

        let finder = PromptInstructionFinder::new(dir.path(), false);
        let result = finder.find();
        assert!(
            result.is_err(),
            "Expected an error because no allowed files were found"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_finder_skips_unreadable_file() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir().unwrap();
        let unreadable = dir.path().join("unreadable.swift");
        let readable = dir.path().join("readable.swift");

        fs::write(&unreadable, "Content\n// TODO: - Unreadable todo\nMore").unwrap();
        fs::write(&readable, "Valid content\n// TODO: - Readable todo\nFooter").unwrap();

        let ft_unreadable = FileTime::from_unix_time(1000, 0);
        let ft_readable = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&unreadable, ft_unreadable).unwrap();
        set_file_mtime(&readable, ft_readable).unwrap();

        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&unreadable, perms).unwrap();

        let finder = PromptInstructionFinder::new(dir.path(), false);
        let chosen_file = finder.find().expect("Expected to pick a valid file");
        assert_eq!(
            chosen_file, readable,
            "Expected the readable file to be chosen"
        );

        let mut perms = fs::metadata(&unreadable).unwrap().permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&unreadable, perms).unwrap();
    }

    #[test]
    fn test_multiple_markers_in_most_recent_file() {
        let dir = tempdir().unwrap();
        let ambiguous_file = dir.path().join("ambiguous.swift");
        let unambiguous_file = dir.path().join("clean.swift");

        fs::write(
            &ambiguous_file,
            "Content\n// TODO: - First marker\nSome intermediate text\n// TODO: - Second marker\nExtra",
        )
        .unwrap();

        fs::write(&unambiguous_file, "Content\n// TODO: - Only marker\nExtra").unwrap();

        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&unambiguous_file, older_time).unwrap();
        set_file_mtime(&ambiguous_file, newer_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false);
        assert!(
            result.is_err(),
            "Expected error due to multiple markers in most recent file"
        );

        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(
                err_msg.to_lowercase().contains("ambiguous"),
                "Error message should indicate ambiguity"
            );
            assert!(
                err_msg.contains("// TODO: - First marker"),
                "Expected first marker line in error"
            );
            assert!(
                err_msg.contains("// TODO: - Second marker"),
                "Expected second marker line in error"
            );
        }
    }

    #[test]
    fn test_most_recent_unambiguous_over_ambiguous_older() {
        let dir = tempdir().unwrap();
        let ambiguous_file = dir.path().join("ambiguous.swift");
        let unambiguous_file = dir.path().join("clean.swift");

        fs::write(
            &ambiguous_file,
            "Content\n// TODO: - First marker\nSome text\n// TODO: - Second marker\nExtra",
        )
        .unwrap();

        fs::write(
            &unambiguous_file,
            "Other content\n// TODO: - Only marker\nExtra",
        )
        .unwrap();

        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&ambiguous_file, older_time).unwrap();
        set_file_mtime(&unambiguous_file, newer_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(
            result, unambiguous_file,
            "Expected the most recent unambiguous file to be chosen"
        );
    }
}

#[cfg(test)]
mod extension_characterization_tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn write_todo_file(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, "Content\n// TODO: - Test instruction\nMore").unwrap();
        path
    }

    #[test]
    fn test_swift_extension_accepted() {
        let dir = tempdir().unwrap();
        let file = write_todo_file(dir.path(), "test.swift");
        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn test_h_extension_accepted() {
        let dir = tempdir().unwrap();
        let file = write_todo_file(dir.path(), "test.h");
        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn test_m_extension_accepted() {
        let dir = tempdir().unwrap();
        let file = write_todo_file(dir.path(), "test.m");
        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn test_js_extension_accepted() {
        let dir = tempdir().unwrap();
        let file = write_todo_file(dir.path(), "test.js");
        let result = find_prompt_instruction_in_dir(dir.path(), false).unwrap();
        assert_eq!(result, file);
    }

    #[test]
    fn test_txt_extension_rejected() {
        let dir = tempdir().unwrap();
        write_todo_file(dir.path(), "test.txt");
        let result = find_prompt_instruction_in_dir(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_ts_extension_rejected() {
        let dir = tempdir().unwrap();
        write_todo_file(dir.path(), "test.ts");
        let result = find_prompt_instruction_in_dir(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_jsx_extension_rejected() {
        let dir = tempdir().unwrap();
        write_todo_file(dir.path(), "test.jsx");
        let result = find_prompt_instruction_in_dir(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_mjs_extension_rejected() {
        let dir = tempdir().unwrap();
        write_todo_file(dir.path(), "test.mjs");
        let result = find_prompt_instruction_in_dir(dir.path(), false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cjs_extension_rejected() {
        let dir = tempdir().unwrap();
        write_todo_file(dir.path(), "test.cjs");
        let result = find_prompt_instruction_in_dir(dir.path(), false);
        assert!(result.is_err());
    }
}
