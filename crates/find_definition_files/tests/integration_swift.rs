// crates/find_definition_files/tests/integration_swift.rs

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use find_definition_files::find_definition_files;

mod integration_swift {
    use super::*;
    
    #[test]
    fn test_find_definition_files_basic() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory for the test.
        let dir = tempdir()?;
        // Create a types file with two type names.
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyType\nOtherType\n")?;

        // Create dummy Swift files:
        // - file1.swift defines MyType (should be included).
        let file1_path = dir.path().join("file1.swift");
        fs::write(&file1_path, "class MyType {}")?;
        // - file2.swift defines OtherType (should be included).
        let file2_path = dir.path().join("file2.swift");
        fs::write(&file2_path, "struct OtherType {}")?;
        // - file3.swift defines an unrelated type (should not be included).
        let file3_path = dir.path().join("file3.swift");
        fs::write(&file3_path, "enum Unmatched {}")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(file1_path);
        expected.insert(file2_path);

        // Verify that only file1.swift and file2.swift are returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_with_subdirectories() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory with a nested structure.
        let dir = tempdir()?;
        // Create a types file with two type names.
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyType\nOtherType\n")?;

        // Create a Swift file in the root that defines MyType.
        let root_file = dir.path().join("root.swift");
        fs::write(&root_file, "class MyType {}")?;

        // Create an allowed subdirectory "Sub1" with a Swift file defining OtherType.
        let sub1 = dir.path().join("Sub1");
        fs::create_dir_all(&sub1)?;
        let sub1_file = sub1.join("sub1.swift");
        fs::write(&sub1_file, "struct OtherType {}")?;

        // Create an excluded subdirectory "Pods" with a matching definition.
        let pods_dir = dir.path().join("Pods");
        fs::create_dir_all(&pods_dir)?;
        let pods_file = pods_dir.join("ignored.swift");
        fs::write(&pods_file, "class MyType {}")?;

        // Create another excluded subdirectory ".build".
        let build_dir = dir.path().join(".build");
        fs::create_dir_all(&build_dir)?;
        let build_file = build_dir.join("ignored.swift");
        fs::write(&build_file, "struct OtherType {}")?;

        // Create a subdirectory "Sub2" with a Swift file that does not match any type.
        let sub2 = dir.path().join("Sub2");
        fs::create_dir_all(&sub2)?;
        let sub2_file = sub2.join("sub2.swift");
        fs::write(&sub2_file, "enum NotRelevant {}")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(root_file);
        expected.insert(sub1_file);

        // Verify that only the files in the root and Sub1 are returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_non_swift_files_ignored() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        // Create a types file that looks for "MyType".
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyType\n")?;

        // Create a matching Swift file.
        let swift_file = dir.path().join("a.swift");
        fs::write(&swift_file, "class MyType {}")?;

        // Create a file with a different extension that should be ignored.
        let txt_file = dir.path().join("b.txt");
        fs::write(&txt_file, "class MyType {}")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(swift_file);

        // Verify that only the Swift file is returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_empty_types_file() {
        // Create a temporary directory.
        let dir = tempdir().unwrap();
        // Create an empty types file.
        let types_path = dir.path().join("empty_types.txt");
        fs::write(&types_path, "").unwrap();

        // Read the types file content.
        let types_content = fs::read_to_string(&types_path).unwrap();

        // Calling find_definition_files should return an empty set.
        let result = find_definition_files(types_content.as_str(), dir.path())
            .expect("Should succeed with empty set for empty types file");
        assert!(result.is_empty(), "Expected an empty set when types file is empty");
    }

    #[test]
    fn test_find_definition_files_no_matches() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        // Create a types file with a type name that will not be found.
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "NonExistentType\n")?;

        // Create a Swift file that defines a different type.
        let file = dir.path().join("file.swift");
        fs::write(&file, "class SomeOtherType {}")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let expected: BTreeSet<PathBuf> = BTreeSet::new();

        // Verify that the returned set is empty.
        assert_eq!(result, expected);
        Ok(())
    }
}
