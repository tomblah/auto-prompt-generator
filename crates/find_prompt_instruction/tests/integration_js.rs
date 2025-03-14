// crates/find_prompt_instruction/tests/integration_js.rs

#[cfg(test)]
mod integration_js {
    use std::fs;
    use tempfile::tempdir;
    use filetime::{set_file_mtime, FileTime};
    use find_prompt_instruction::find_prompt_instruction_in_dir;

    /// Test that when there is exactly one JavaScript file containing the TODO marker,
    /// the function returns that file.
    #[test]
    fn test_find_prompt_instruction_single_file_js() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("instruction.js");
        let content = "function example() {}\n// TODO: - Fix the bug in JS code\nconsole.log('Hello');";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false)
            .expect("Expected to find a JS file with a TODO marker");
        assert_eq!(result, file_path);
    }

    /// Test that when multiple JavaScript files contain the TODO marker,
    /// the file with the most recent modification time is chosen.
    #[test]
    fn test_find_prompt_instruction_multiple_files_js() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("old_instruction.js");
        let file2 = dir.path().join("new_instruction.js");
        let content1 = "function old() {}\n// TODO: - Old JS fix\n";
        let content2 = "function new() {}\n// TODO: - New JS fix\n";

        fs::write(&file1, content1).unwrap();
        fs::write(&file2, content2).unwrap();

        // Set file1 to an older modification time and file2 to a newer one.
        let ft1 = FileTime::from_unix_time(1000, 0);
        let ft2 = FileTime::from_unix_time(2000, 0);
        set_file_mtime(&file1, ft1).unwrap();
        set_file_mtime(&file2, ft2).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false)
            .expect("Expected to choose the most recently modified JS file");
        assert_eq!(result, file2);
    }

    /// Test that if no JavaScript file in the directory contains the TODO marker, an error is returned.
    #[test]
    fn test_find_prompt_instruction_no_file_found_js() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("no_todo.js");
        let content = "function example() {}\n// This file has no marker\n";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false);
        assert!(result.is_err(), "Expected an error when no JS file contains the TODO marker");
    }

    /// Test that files with disallowed extensions (e.g. .txt) are ignored even if they contain the marker.
    #[test]
    fn test_find_prompt_instruction_ignores_disallowed_extension_js() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("ignored.txt");
        let content = "Some text\n// TODO: - This should be ignored in JS context\nMore text";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), false);
        assert!(result.is_err(), "Expected error because files with disallowed extensions should be ignored");
    }

    /// Test that enabling verbose mode does not change the returned file.
    #[test]
    fn test_find_prompt_instruction_verbose_mode_js() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("instruction.js");
        let content = "function test() {}\n// TODO: - Verbose JS fix\n";
        fs::write(&file_path, content).unwrap();

        let result = find_prompt_instruction_in_dir(dir.path().to_str().unwrap(), true)
            .expect("Expected to find a JS file even with verbose enabled");
        assert_eq!(result, file_path);
    }
}
