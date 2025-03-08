#[cfg(test)]
mod integration_swift {
    use assemble_prompt::assemble_prompt;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::NamedTempFile;

    /// Test assembling a prompt from a single Swift file.
    #[test]
    fn test_assemble_prompt_single_file() {
        // Create a temporary Swift file with some content.
        let mut swift_file = NamedTempFile::new().expect("Failed to create temp Swift file");
        let swift_content = "public class MyClass {\n    func test() {}\n}\n";
        write!(swift_file, "{}", swift_content).expect("Failed to write to Swift file");
        let swift_path = swift_file.path().to_str().unwrap().to_string();

        // Create a temporary found_files list that contains the Swift file path.
        let mut found_files = NamedTempFile::new().expect("Failed to create found files temp file");
        writeln!(found_files, "{}", swift_path).expect("Failed to write to found files file");
        let found_files_path = found_files
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        // Call assemble_prompt with an arbitrary instruction (ignored by the function).
        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        // Bind the file name to an owned String so it lives long enough.
        let binding = PathBuf::from(&swift_path);
        let file_name = binding
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        // Check that the output contains a header for the Swift file and the file content.
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name)),
            "Output should include the file header for {}",
            file_name
        );
        assert!(
            output.contains("public class MyClass"),
            "Output should contain the Swift file content"
        );
        // Verify that the fixed instruction (or a characteristic substring) is appended.
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    /// Test that multiple Swift files (including duplicates) are processed correctly.
    #[test]
    fn test_assemble_prompt_multiple_files_deduplicated() {
        // Create two temporary Swift files.
        let mut swift_file1 = NamedTempFile::new().expect("Failed to create Swift file 1");
        let swift_content1 = "public struct StructOne {}\n";
        write!(swift_file1, "{}", swift_content1).expect("Failed to write to Swift file 1");
        let swift_path1 = swift_file1.path().to_str().unwrap().to_string();

        let mut swift_file2 = NamedTempFile::new().expect("Failed to create Swift file 2");
        let swift_content2 = "public enum EnumTwo {}\n";
        write!(swift_file2, "{}", swift_content2).expect("Failed to write to Swift file 2");
        let swift_path2 = swift_file2.path().to_str().unwrap().to_string();

        // Create a temporary found_files list that includes both files and a duplicate.
        let mut found_files = NamedTempFile::new().expect("Failed to create found files file");
        writeln!(found_files, "{}", swift_path1).expect("Failed to write to found files file");
        writeln!(found_files, "{}", swift_path2).expect("Failed to write to found files file");
        // Duplicate swift_file1.
        writeln!(found_files, "{}", swift_path1).expect("Failed to write duplicate entry");
        let found_files_path = found_files
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        // Bind file names to owned Strings.
        let binding1 = PathBuf::from(&swift_path1);
        let file_name1 = binding1.file_name().unwrap().to_string_lossy().into_owned();
        let binding2 = PathBuf::from(&swift_path2);
        let file_name2 = binding2.file_name().unwrap().to_string_lossy().into_owned();

        // Check that the prompt includes headers for both files.
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name1)),
            "Output should include the header for {}",
            file_name1
        );
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name2)),
            "Output should include the header for {}",
            file_name2
        );
        // Ensure the duplicate entry does not result in repeated content.
        let occurrences = output.matches(file_name1.as_str()).count();
        assert_eq!(
            occurrences, 1,
            "The header for {} should appear only once",
            file_name1
        );
        // Verify both file contents are present.
        assert!(
            output.contains("public struct StructOne"),
            "Output should contain the content from Swift file 1"
        );
        assert!(
            output.contains("public enum EnumTwo"),
            "Output should contain the content from Swift file 2"
        );
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    /// Test that if a found file does not exist, it is skipped.
    #[test]
    fn test_assemble_prompt_with_missing_file() {
        // Create a valid Swift file.
        let mut swift_file = NamedTempFile::new().expect("Failed to create Swift file");
        let swift_content = "public class MissingTest {}\n";
        write!(swift_file, "{}", swift_content).expect("Failed to write to Swift file");
        let swift_path = swift_file.path().to_str().unwrap().to_string();

        // Create a found_files list that includes one valid and one non-existent file.
        let mut found_files = NamedTempFile::new().expect("Failed to create found files file");
        writeln!(found_files, "{}", swift_path).expect("Failed to write valid file path");
        writeln!(found_files, "/path/to/nonexistent/file.swift")
            .expect("Failed to write non-existent file path");
        let found_files_path = found_files
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        let binding = PathBuf::from(&swift_path);
        let file_name = binding.file_name().unwrap().to_string_lossy().into_owned();

        // Verify that output contains header and content for the valid file.
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name)),
            "Output should include header for the valid Swift file"
        );
        assert!(
            output.contains("public class MissingTest"),
            "Output should contain the valid Swift file content"
        );
        // Ensure that no reference to the missing file appears.
        assert!(
            !output.contains("nonexistent"),
            "Output should not reference the missing file"
        );
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    /// Test that an empty found_files list results in a prompt containing only the fixed instruction.
    #[test]
    fn test_assemble_prompt_empty_found_files() {
        // Create an empty found_files temporary file.
        let found_files = NamedTempFile::new().expect("Failed to create empty found files file");
        let found_files_path = found_files
            .into_temp_path()
            .keep()
            .expect("Failed to persist empty found files list");

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        let trimmed_output = output.trim();
        assert!(
            trimmed_output.starts_with("Can you do the TODO:- in the above code?"),
            "Output should start with the fixed instruction when no files are provided, got: {}",
            trimmed_output
        );
        assert!(
            trimmed_output.ends_with("doesn't have the hyphen"),
            "Output should end with the fixed instruction when no files are provided, got: {}",
            trimmed_output
        );
    }
}
