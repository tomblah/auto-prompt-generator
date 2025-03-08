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
}
