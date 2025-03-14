// crates/find_referencing_files/tests/integration_js.rs

#[cfg(test)]
mod integration_js {
    use std::fs;
    use tempfile::tempdir;
    use find_referencing_files::find_files_referencing;

    /// Test that when files with allowed extensions reference a target type in JavaScript,
    /// only those files (and not files in excluded directories or with disallowed extensions)
    /// are returned.
    #[test]
    fn test_find_referencing_files_basic_js() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory as the search root.
        let temp_dir = tempdir()?;

        // Create a JavaScript file in the root that references "MyJSClass".
        let file1_path = temp_dir.path().join("file1.js");
        fs::write(
            &file1_path,
            "function example() { return new MyJSClass(); }",
        )?;

        // Create another JavaScript file that does not reference "MyJSClass".
        let file2_path = temp_dir.path().join("file2.js");
        fs::write(&file2_path, "var x = 42;")?;

        // Create a JavaScript file that also references "MyJSClass".
        let file3_path = temp_dir.path().join("file3.js");
        fs::write(&file3_path, "const obj = new MyJSClass();")?;

        // Create a file with a disallowed extension (.txt) that references "MyJSClass".
        let file4_path = temp_dir.path().join("file4.txt");
        fs::write(
            &file4_path,
            "This is a reference: new MyJSClass();",
        )?;

        // Create a JavaScript file inside a "Pods" subdirectory that references "MyJSClass" (should be excluded).
        let pods_dir = temp_dir.path().join("Pods");
        fs::create_dir_all(&pods_dir)?;
        let file5_path = pods_dir.join("file5.js");
        fs::write(&file5_path, "let a = MyJSClass;")?;

        // Create a JavaScript file inside a ".build" subdirectory that references "MyJSClass" (should be excluded).
        let build_dir = temp_dir.path().join(".build");
        fs::create_dir_all(&build_dir)?;
        let file6_path = build_dir.join("file6.js");
        fs::write(&file6_path, "console.log(MyJSClass);")?;

        // Call the function under test.
        let mut result = find_files_referencing("MyJSClass", temp_dir.path().to_str().unwrap())?;
        result.sort();

        // We expect only file1.js and file3.js to be returned.
        let mut expected: Vec<String> = vec![
            file1_path.to_string_lossy().into_owned(),
            file3_path.to_string_lossy().into_owned(),
        ];
        expected.sort();

        assert_eq!(
            result, expected,
            "The returned file list did not match expected files."
        );
        Ok(())
    }

    /// Test that if no JavaScript file in the search root contains the target type,
    /// an empty vector is returned.
    #[test]
    fn test_find_referencing_files_no_matches_js() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;

        // Create a JavaScript file that does not reference "NonExistentJSClass".
        let file_path = temp_dir.path().join("file.js");
        fs::write(&file_path, "function test() { console.log('No match here'); }")?;

        let result = find_files_referencing("NonExistentJSClass", temp_dir.path().to_str().unwrap())?;
        assert!(
            result.is_empty(),
            "Expected no matches for 'NonExistentJSClass'"
        );
        Ok(())
    }

    /// Test that when multiple JavaScript files reference a given target,
    /// all are returned and that references embedded within longer words are not falsely matched.
    #[test]
    fn test_find_referencing_files_multiple_references_js() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;

        // Create several JavaScript files that reference "TargetJS".
        let file1_path = temp_dir.path().join("a.js");
        fs::write(&file1_path, "class TargetJS {} \n// usage: new TargetJS();")?;

        let file2_path = temp_dir.path().join("b.js");
        fs::write(&file2_path, "var obj = new TargetJS();")?;

        let file3_path = temp_dir.path().join("c.js");
        fs::write(&file3_path, "function doSomething() { return TargetJS; }")?;

        // Create a file that contains "NotTargetJSExtra" which should not match.
        let file4_path = temp_dir.path().join("d.js");
        fs::write(&file4_path, "let dummy = 'NotTargetJSExtra';")?;

        let mut result = find_files_referencing("TargetJS", temp_dir.path().to_str().unwrap())?;
        result.sort();

        let mut expected: Vec<String> = vec![
            file1_path.to_string_lossy().into_owned(),
            file2_path.to_string_lossy().into_owned(),
            file3_path.to_string_lossy().into_owned(),
        ];
        expected.sort();

        assert_eq!(
            result, expected,
            "Expected only files that reference 'TargetJS'"
        );
        Ok(())
    }
}
