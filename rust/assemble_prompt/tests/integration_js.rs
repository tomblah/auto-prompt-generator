#[cfg(test)]
mod integration_js {
    use assemble_prompt::assemble_prompt;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::{tempdir, NamedTempFile};

    /// Test assembling a prompt from a single JavaScript file.
    #[test]
    fn test_assemble_prompt_single_file() {
        // Create a temporary directory and a JS file within it.
        let dir = tempdir().expect("Failed to create temporary directory");
        let js_path = dir.path().join("script.js");
        let js_content = "function myFunction() {\n    console.log('Hello from JS');\n}\n";
        fs::write(&js_path, js_content).expect("Failed to write JS file");

        // Create a temporary found_files list that contains the JS file path.
        let mut found_files = NamedTempFile::new().expect("Failed to create found files file");
        writeln!(found_files, "{}", js_path.to_string_lossy()).expect("Failed to write to found files file");
        let found_files_path = found_files
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        // Call assemble_prompt with an arbitrary instruction.
        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        // Bind the file name to an owned String.
        let binding = PathBuf::from(&js_path);
        let file_name = binding
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();

        // Check that the output contains the file header, the JS content, and the fixed instruction.
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name)),
            "Output should include the file header for {}",
            file_name
        );
        assert!(
            output.contains("function myFunction()"),
            "Output should contain the JS file content"
        );
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    /// Test that multiple JavaScript files (with a duplicate) are processed correctly.
    #[test]
    fn test_assemble_prompt_multiple_files_deduplicated() {
        let dir = tempdir().expect("Failed to create temporary directory");

        // Create two JS files.
        let js_path1 = dir.path().join("first.js");
        let js_content1 = "const a = 1;\n";
        fs::write(&js_path1, js_content1).expect("Failed to write first JS file");

        let js_path2 = dir.path().join("second.js");
        let js_content2 = "const b = 2;\n";
        fs::write(&js_path2, js_content2).expect("Failed to write second JS file");

        // Create a found_files list including both files and a duplicate of the first.
        let mut found_files = NamedTempFile::new().expect("Failed to create found files file");
        writeln!(found_files, "{}", js_path1.to_string_lossy()).expect("Failed to write first file path");
        writeln!(found_files, "{}", js_path2.to_string_lossy()).expect("Failed to write second file path");
        writeln!(found_files, "{}", js_path1.to_string_lossy()).expect("Failed to write duplicate entry");
        let found_files_path = found_files
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        // Bind file names to owned strings.
        let binding1 = PathBuf::from(&js_path1);
        let file_name1 = binding1.file_name().unwrap().to_string_lossy().into_owned();
        let binding2 = PathBuf::from(&js_path2);
        let file_name2 = binding2.file_name().unwrap().to_string_lossy().into_owned();

        // Verify that headers for both files appear and that duplicate entries are deduplicated.
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
        let occurrences = output.matches(file_name1.as_str()).count();
        assert_eq!(
            occurrences, 1,
            "The header for {} should appear only once",
            file_name1
        );
        assert!(
            output.contains("const a = 1;"),
            "Output should contain the content from the first JS file"
        );
        assert!(
            output.contains("const b = 2;"),
            "Output should contain the content from the second JS file"
        );
        assert!(
            output.contains("Can you do the TODO:- in the above code?"),
            "Output should contain the fixed instruction"
        );
    }

    /// Test that if a found file does not exist, it is skipped.
    #[test]
    fn test_assemble_prompt_with_missing_file() {
        let dir = tempdir().expect("Failed to create temporary directory");

        // Create a valid JS file.
        let js_path = dir.path().join("existent.js");
        let js_content = "console.log('Existing JS');\n";
        fs::write(&js_path, js_content).expect("Failed to write existent JS file");

        // Create a found_files list including one valid and one non-existent file.
        let mut found_files = NamedTempFile::new().expect("Failed to create found files file");
        writeln!(found_files, "{}", js_path.to_string_lossy()).expect("Failed to write valid file path");
        writeln!(found_files, "/path/to/nonexistent/script.js")
            .expect("Failed to write non-existent file path");
        let found_files_path = found_files
            .into_temp_path()
            .keep()
            .expect("Failed to persist found files list");

        let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
            .expect("assemble_prompt failed");

        let binding = PathBuf::from(&js_path);
        let file_name = binding.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            output.contains(&format!("The contents of {} is as follows:", file_name)),
            "Output should include header for the existent JS file"
        );
        assert!(
            output.contains("console.log('Existing JS')"),
            "Output should contain the content of the existent JS file"
        );
        assert!(
            !output.contains("nonexistent"),
            "Output should not reference the non-existent file"
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
