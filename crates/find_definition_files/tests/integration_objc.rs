// crates/find_definition_files/tests/integration_objc.rs

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;
use find_definition_files::find_definition_files;

mod integration_objc {
    use super::*;

    #[test]
    fn test_find_definition_files_basic_objc() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        // Create a types file listing the type "MyType".
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyType\n")?;

        // Create Objective‑C files:
        // - A header file defining MyType.
        let header_path = dir.path().join("MyType.h");
        fs::write(&header_path, "@interface MyType : NSObject @end")?;
        // - An implementation file defining MyType.
        let impl_path = dir.path().join("MyType.m");
        fs::write(&impl_path, "@implementation MyType @end")?;

        // Create a non-matching Objective‑C file.
        let non_match = dir.path().join("OtherType.h");
        fs::write(&non_match, "@interface OtherType : NSObject @end")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(header_path);
        expected.insert(impl_path);

        // Verify that only the header and implementation files for MyType are returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_with_subdirectories_objc() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyType\n")?;

        // Create a header file in the root.
        let root_header = dir.path().join("MyType.h");
        fs::write(&root_header, "@interface MyType : NSObject @end")?;

        // Create an allowed subdirectory "Sub1" with an implementation file for MyType.
        let sub1 = dir.path().join("Sub1");
        fs::create_dir_all(&sub1)?;
        let sub1_impl = sub1.join("MyType.m");
        fs::write(&sub1_impl, "@implementation MyType @end")?;

        // Create an excluded subdirectory "Pods" with a matching definition.
        let pods_dir = dir.path().join("Pods");
        fs::create_dir_all(&pods_dir)?;
        let pods_file = pods_dir.join("Ignored.h");
        fs::write(&pods_file, "@interface MyType : NSObject @end")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(root_header);
        expected.insert(sub1_impl);

        // Verify that only files in the root and allowed subdirectories are returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_non_objc_files_ignored() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        // Create a types file searching for "MyType".
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "MyType\n")?;

        // Create a matching Objective‑C file.
        let objc_file = dir.path().join("MyType.h");
        fs::write(&objc_file, "@interface MyType : NSObject @end")?;

        // Create a file with a different extension that should be ignored.
        let txt_file = dir.path().join("b.txt");
        fs::write(&txt_file, "@interface MyType : NSObject @end")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(objc_file);

        // Verify that only the Objective‑C file is returned.
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_no_matches_objc() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;
        // Create a types file with a type name that will not be found.
        let types_path = dir.path().join("types.txt");
        fs::write(&types_path, "NonExistentType\n")?;

        // Create an Objective‑C file that defines a different type.
        let file = dir.path().join("MyType.h");
        fs::write(&file, "@interface MyType : NSObject @end")?;

        // Read the types file content and run the public API.
        let types_content = fs::read_to_string(&types_path)?;
        let result = find_definition_files(types_content.as_str(), dir.path())?;
        let expected: BTreeSet<PathBuf> = BTreeSet::new();

        // Verify that the returned set is empty.
        assert_eq!(result, expected);
        Ok(())
    }
}
