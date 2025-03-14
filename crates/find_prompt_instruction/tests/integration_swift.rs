// crates/find_prompt_instruction/tests/integration_swift.rs

#[cfg(test)]
mod integration_swift {
    use std::fs;
    use tempfile::tempdir;
    use filetime::{set_file_mtime, FileTime};
    // Import the public API from the find_prompt_instruction crate.
    use find_prompt_instruction::find_prompt_instruction_in_dir;

    /// Test that when there is exactly one Swift file containing the TODO marker,
    /// the function returns that file.
    #[test]
    fn test_find_prompt_instruction_single_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("Instruction.swift");
        let content = "public func example() {}\n// TODO: - Fix the bug\nMore text";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false)
            .expect("Expected to find a file with a TODO marker");
        assert_eq!(result, file_path);
    }

    /// Test that when multiple files contain the TODO marker,
    /// the file with the most recent modification time is chosen.
    #[test]
    fn test_find_prompt_instruction_multiple_files() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("OldInstruction.swift");
        let file2 = dir.path().join("NewInstruction.swift");
        let content1 = "public func old() {}\n// TODO: - Old fix\n";
        let content2 = "public func new() {}\n// TODO: - New fix\n";

        fs::write(&file1, content1).unwrap();
        fs::write(&file2, content2).unwrap();

        // Set file1 to an older modification time and file2 to a newer one.
        let ft1 = FileTime::from_unix_time(1000, 0);
        let ft2 = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&file1, ft1).unwrap();
        set_file_mtime(&file2, ft2).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false)
            .expect("Expected to choose the most recently modified file");
        assert_eq!(result, file2);
    }

    /// Test that if no file in the directory contains the TODO marker, an error is returned.
    #[test]
    fn test_find_prompt_instruction_no_file_found() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("NoTodo.swift");
        let content = "public func example() {}\n// No TODO here\n";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false);
        assert!(result.is_err(), "Expected an error when no file contains the TODO marker");
    }

    /// Test that files with disallowed extensions (e.g. .txt) are ignored.
    #[test]
    fn test_find_prompt_instruction_ignores_disallowed_extension() {
        let dir = tempdir().unwrap();
        // Create a file with a .txt extension that contains the TODO marker.
        let file_path = dir.path().join("Ignored.txt");
        let content = "Some text\n// TODO: - This should be ignored\n";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false);
        assert!(result.is_err(), "Expected error because files with disallowed extensions are ignored");
    }

    /// Test that enabling verbose mode does not change the returned file.
    #[test]
    fn test_find_prompt_instruction_verbose_mode() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("Instruction.swift");
        let content = "func test() {}\n// TODO: - Verbose fix\n";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), true)
            .expect("Expected to find a file even with verbose enabled");
        assert_eq!(result, file_path);
    }
    
    /// Test that if the most recent file contains multiple TODO markers,
    /// the function returns an error and the error message includes the trimmed marker lines.
    #[test]
    fn test_find_prompt_instruction_error_on_ambiguous_marker_in_most_recent() {
        let dir = tempdir().unwrap();
        let ambiguous_file = dir.path().join("AmbiguousInstruction.swift");
        let unambiguous_file = dir.path().join("CleanInstruction.swift");

        // ambiguous_file (most recent) has two markers.
        let ambiguous_content = "\
public func ambiguous() {}\n\
// TODO: - First marker\n\
Some intermediate text\n\
// TODO: - Second marker\n\
Extra text";
        fs::write(&ambiguous_file, ambiguous_content).unwrap();

        // unambiguous_file (older) has one marker.
        let unambiguous_content = "\
public func clean() {}\n\
// TODO: - Only marker\n\
Extra text";
        fs::write(&unambiguous_file, unambiguous_content).unwrap();

        // Set modification times: ambiguous_file is more recent.
        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&unambiguous_file, older_time).unwrap();
        set_file_mtime(&ambiguous_file, newer_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false);
        assert!(result.is_err(), "Expected error due to multiple markers in most recent file");

        if let Err(e) = result {
            let err_msg = e.to_string();
            assert!(err_msg.to_lowercase().contains("ambiguous"), "Error message should indicate ambiguity");
            assert!(err_msg.contains("// TODO: - First marker"), "Error message should include the first marker");
            assert!(err_msg.contains("// TODO: - Second marker"), "Error message should include the second marker");
        }
    }

    /// Test that if an older file is ambiguous but the most recent file is unambiguous,
    /// the unambiguous file is returned.
    #[test]
    fn test_find_prompt_instruction_selects_most_recent_unambiguous_file() {
        let dir = tempdir().unwrap();
        let ambiguous_file = dir.path().join("AmbiguousInstruction.swift");
        let unambiguous_file = dir.path().join("CleanInstruction.swift");

        // ambiguous_file (older) has two markers.
        let ambiguous_content = "\
public func ambiguous() {}\n\
// TODO: - First marker\n\
Some text\n\
// TODO: - Second marker\n\
Extra text";
        fs::write(&ambiguous_file, ambiguous_content).unwrap();

        // unambiguous_file (most recent) has one marker.
        let unambiguous_content = "\
public func clean() {}\n\
// TODO: - Only marker\n\
Extra text";
        fs::write(&unambiguous_file, unambiguous_content).unwrap();

        // Set modification times: ambiguous_file older, unambiguous_file more recent.
        let older_time = FileTime::from_unix_time(1000, 0);
        let newer_time = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&ambiguous_file, older_time).unwrap();
        set_file_mtime(&unambiguous_file, newer_time).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false)
            .expect("Expected to select the most recent unambiguous file");
        assert_eq!(result, unambiguous_file, "Expected the unambiguous file to be selected");
    }
}
