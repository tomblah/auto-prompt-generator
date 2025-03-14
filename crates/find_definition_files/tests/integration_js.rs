// crates/find_definition_files/tests/integration_js.rs

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use find_definition_files::find_definition_files;

mod integration_javascript {
    use super::*;

    #[test]
    fn test_find_definition_files_basic_js() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        // Create a types file listing two JavaScript type names.
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyClass\nOtherClass\n")?;

        // Create dummy JavaScript files:
        // - file1.js defines MyClass (should be included).
        let file1_path = dir.path().join("file1.js");
        fs::write(&file1_path, "class MyClass {}")?;
        // - file2.js defines OtherClass (should be included).
        let file2_path = dir.path().join("file2.js");
        fs::write(&file2_path, "class OtherClass {}")?;
        // - file3.js defines an unrelated function (should not be included).
        let file3_path = dir.path().join("file3.js");
        fs::write(&file3_path, "function notAMatch() {}")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(file1_path);
        expected.insert(file2_path);

        // Verify that only file1.js and file2.js are returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_with_subdirectories_js() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory with nested subdirectories.
        let dir = tempdir()?;
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyClass\n")?;

        // Create a JS file in the root that defines MyClass.
        let root_file = dir.path().join("root.js");
        fs::write(&root_file, "class MyClass {}")?;

        // Create a subdirectory "Sub1" with a JS file that defines MyClass.
        let sub1 = dir.path().join("Sub1");
        fs::create_dir_all(&sub1)?;
        let sub1_file = sub1.join("sub1.js");
        fs::write(&sub1_file, "class MyClass {}")?;

        // Create an excluded subdirectory "Pods" with a matching definition.
        let pods_dir = dir.path().join("Pods");
        fs::create_dir_all(&pods_dir)?;
        let pods_file = pods_dir.join("ignored.js");
        fs::write(&pods_file, "class MyClass {}")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(root_file);
        expected.insert(sub1_file);

        // Verify that only files in the root and Sub1 are returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_non_js_files_ignored() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        // Create a types file looking for "MyClass".
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyClass\n")?;

        // Create a matching JS file.
        let js_file = dir.path().join("a.js");
        fs::write(&js_file, "class MyClass {}")?;
        // Create a file with a different extension that should be ignored.
        let txt_file = dir.path().join("b.txt");
        fs::write(&txt_file, "class MyClass {}")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(js_file);

        // Verify that only the JavaScript file is returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_empty_types_file_js() {
        // Create a temporary directory.
        let dir = tempdir().unwrap();
        // Create an empty types file.
        let types_path = dir.path().join("empty_types.txt");
        fs::write(&types_path, "").unwrap();

        // Read the types file content.
        let types_content = fs::read_to_string(&types_path).unwrap();
        // Calling find_definition_files should return an error.
        let result = find_definition_files(types_content.as_str(), dir.path());
        assert!(result.is_err(), "Expected error for empty types file");
    }

    #[test]
    fn test_find_definition_files_no_matches_js() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        // Create a types file with a type name that won't be found.
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "NonExistentClass\n")?;

        // Create a JS file that defines a different class.
        let file = dir.path().join("file.js");
        fs::write(&file, "class SomeOtherClass {}")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let expected: BTreeSet<PathBuf> = BTreeSet::new();

        // Verify that the returned set is empty.
        assert_eq!(result, expected);
        Ok(())
    }
}
