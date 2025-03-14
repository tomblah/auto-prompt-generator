// crates/find_referencing_files/tests/integration_swift.rs

#[cfg(test)]
mod integration_swift {
    use std::fs;
    use tempfile::tempdir;
    use find_referencing_files::find_files_referencing;

    /// Test that when files with allowed extensions reference a target type,
    /// only those files (and not files in excluded directories or with disallowed extensions)
    /// are returned.
    #[test]
    fn test_find_referencing_files_basic_swift() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory as the search root.
        let temp_dir = tempdir()?;
        
        // Create a Swift file in the root that references "MyType".
        let file1_path = temp_dir.path().join("file1.swift");
        fs::write(&file1_path, "class MyType {}\nfunc foo() { MyType() }")?;
        
        // Create another Swift file that does not reference "MyType".
        let file2_path = temp_dir.path().join("file2.swift");
        fs::write(&file2_path, "struct OtherType {}")?;
        
        // Create a JavaScript file that references "MyType".
        let file3_path = temp_dir.path().join("file3.js");
        fs::write(&file3_path, "function test() { return MyType; }")?;
        
        // Create a file with a disallowed extension (.txt) that references "MyType".
        let file4_path = temp_dir.path().join("file4.txt");
        fs::write(&file4_path, "This is MyType reference in a text file")?;
        
        // Create a Swift file inside a "Pods" subdirectory that references "MyType" (should be excluded).
        let pods_dir = temp_dir.path().join("Pods");
        fs::create_dir_all(&pods_dir)?;
        let file5_path = pods_dir.join("file5.swift");
        fs::write(&file5_path, "class MyType {}")?;
        
        // Create a Swift file inside a ".build" subdirectory that references "MyType" (should be excluded).
        let build_dir = temp_dir.path().join(".build");
        fs::create_dir_all(&build_dir)?;
        let file6_path = build_dir.join("file6.swift");
        fs::write(&file6_path, "class MyType {}")?;
        
        // Call find_files_referencing on the temporary directory.
        let mut result = find_files_referencing("MyType", temp_dir.path().to_str().unwrap())?;
        result.sort();
        
        // Expect only file1.swift and file3.js to be returned.
        let mut expected: Vec<String> = vec![
            file1_path.to_string_lossy().into_owned(),
            file3_path.to_string_lossy().into_owned(),
        ];
        expected.sort();
        
        assert_eq!(result, expected, "The returned file list did not match expected files.");
        Ok(())
    }

    /// Test that if no file in the search root contains the target type, an empty vector is returned.
    #[test]
    fn test_find_referencing_files_no_matches() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        
        // Create a Swift file that does not reference "NonExistentType".
        let file_path = temp_dir.path().join("file.swift");
        fs::write(&file_path, "class SomeOtherType {}")?;
        
        let result = find_files_referencing("NonExistentType", temp_dir.path().to_str().unwrap())?;
        assert!(result.is_empty(), "Expected no matches for 'NonExistentType'");
        Ok(())
    }

    /// Test that multiple files referencing a given type are all returned,
    /// and that references embedded within longer words are not falsely matched.
    #[test]
    fn test_find_referencing_files_multiple_references() -> Result<(), Box<dyn std::error::Error>> {
        let temp_dir = tempdir()?;
        
        // Create several files with allowed extensions that reference "TargetType".
        let file1_path = temp_dir.path().join("a.swift");
        fs::write(&file1_path, "class TargetType {}\n// usage: TargetType")?;
        
        let file2_path = temp_dir.path().join("b.h");
        fs::write(&file2_path, "typedef struct TargetType TargetType;")?;
        
        let file3_path = temp_dir.path().join("c.m");
        fs::write(&file3_path, "extern TargetType *obj;")?;
        
        // Create a file that contains "NotTargetTypology" which should not match.
        let file4_path = temp_dir.path().join("d.swift");
        fs::write(&file4_path, "class NotTargetTypology {}")?;
        
        let mut result = find_files_referencing("TargetType", temp_dir.path().to_str().unwrap())?;
        result.sort();
        
        let mut expected: Vec<String> = vec![
            file1_path.to_string_lossy().into_owned(),
            file2_path.to_string_lossy().into_owned(),
            file3_path.to_string_lossy().into_owned(),
        ];
        expected.sort();
        
        assert_eq!(result, expected, "Expected only files that reference 'TargetType'");
        Ok(())
    }
}
