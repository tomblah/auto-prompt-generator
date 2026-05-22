// crates/find_definition_files/tests/integration_objc.rs

use find_definition_files::find_definition_files;
use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

fn types(items: &[&str]) -> BTreeSet<String> {
    items.iter().map(|s| s.to_string()).collect()
}

mod integration_objc {
    use super::*;

    #[test]
    fn test_find_definition_files_basic_objc() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let header_path = dir.path().join("MyType.h");
        fs::write(&header_path, "@interface MyType : NSObject @end")?;
        let impl_path = dir.path().join("MyType.m");
        fs::write(&impl_path, "@implementation MyType @end")?;

        let non_match = dir.path().join("OtherType.h");
        fs::write(&non_match, "@interface OtherType : NSObject @end")?;

        let types = types(&["MyType"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(header_path);
        expected.insert(impl_path);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_with_subdirectories_objc(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let root_header = dir.path().join("MyType.h");
        fs::write(&root_header, "@interface MyType : NSObject @end")?;

        let sub1 = dir.path().join("Sub1");
        fs::create_dir_all(&sub1)?;
        let sub1_impl = sub1.join("MyType.m");
        fs::write(&sub1_impl, "@implementation MyType @end")?;

        let pods_dir = dir.path().join("Pods");
        fs::create_dir_all(&pods_dir)?;
        let pods_file = pods_dir.join("Ignored.h");
        fs::write(&pods_file, "@interface MyType : NSObject @end")?;

        let types = types(&["MyType"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(root_header);
        expected.insert(sub1_impl);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_non_objc_files_ignored() -> Result<(), Box<dyn std::error::Error>>
    {
        let dir = tempdir()?;

        let objc_file = dir.path().join("MyType.h");
        fs::write(&objc_file, "@interface MyType : NSObject @end")?;

        let txt_file = dir.path().join("b.txt");
        fs::write(&txt_file, "@interface MyType : NSObject @end")?;

        let types = types(&["MyType"]);
        let result = find_definition_files(&types, dir.path())?;
        let mut expected = BTreeSet::new();
        expected.insert(objc_file);

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_no_matches_objc() -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let file = dir.path().join("MyType.h");
        fs::write(&file, "@interface MyType : NSObject @end")?;

        let types = types(&["NonExistentType"]);
        let result = find_definition_files(&types, dir.path())?;
        let expected: BTreeSet<PathBuf> = BTreeSet::new();

        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_objc_whitespace_variation(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let header_path = dir.path().join("Message.h");
        fs::write(&header_path, "   @interface   Message   : NSObject")?;

        let types = types(&["Message"]);
        let result = find_definition_files(&types, dir.path())?;

        let mut expected = BTreeSet::new();
        expected.insert(header_path);
        assert_eq!(result, expected);
        Ok(())
    }

    #[test]
    fn test_find_definition_files_objc_rejects_partial_match(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let dir = tempdir()?;

        let header_path = dir.path().join("MessageExtra.h");
        fs::write(&header_path, "@interface MessageExtra : NSObject @end")?;
        let impl_path = dir.path().join("MessageExtra.m");
        fs::write(&impl_path, "@implementation MessageExtra @end")?;

        let types = types(&["Message"]);
        let result = find_definition_files(&types, dir.path())?;
        let expected: BTreeSet<PathBuf> = BTreeSet::new();

        assert_eq!(result, expected);
        Ok(())
    }
}
