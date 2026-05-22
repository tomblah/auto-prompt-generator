// crates/assemble_prompt/tests/integration_js.rs

#[cfg(test)]
mod integration_js {
    use assemble_prompt::{assemble_prompt, AssemblyOptions};
    use std::fs;
    use std::path::PathBuf;
    use tempfile::tempdir;

    /// Test assembling a prompt from a single JavaScript file.
    #[test]
    fn test_assemble_prompt_single_file() {
        // Create a temporary directory and a JS file within it.
        let dir = tempdir().expect("Failed to create temporary directory");
        let js_path = dir.path().join("script.js");
        let js_content = "function myFunction() {\n    console.log('Hello from JS');\n}\n";
        fs::write(&js_path, js_content).expect("Failed to write JS file");

        let found_files_vec = vec![js_path.clone()];

        // Call assemble_prompt with the in‑memory list.
        let output = assemble_prompt(&found_files_vec, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        let file_name = js_path.file_name().unwrap().to_string_lossy().into_owned();

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

        // Build found_files list including a duplicate of js_path1, then sort and dedup.
        let mut found_files_vec: Vec<PathBuf> =
            vec![js_path1.clone(), js_path2.clone(), js_path1.clone()];
        found_files_vec.sort();
        found_files_vec.dedup();

        let output = assemble_prompt(&found_files_vec, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        let file_name1 = js_path1.file_name().unwrap().to_string_lossy().into_owned();
        let file_name2 = js_path2.file_name().unwrap().to_string_lossy().into_owned();

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

        let found_files_vec: Vec<PathBuf> = vec![
            js_path.clone(),
            PathBuf::from("/path/to/nonexistent/script.js"),
        ];

        let output = assemble_prompt(&found_files_vec, &AssemblyOptions::default())
            .expect("assemble_prompt failed");

        let file_name = js_path.file_name().unwrap().to_string_lossy().into_owned();
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
        let found_files: Vec<PathBuf> = Vec::new();

        let output = assemble_prompt(&found_files, &AssemblyOptions::default())
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
