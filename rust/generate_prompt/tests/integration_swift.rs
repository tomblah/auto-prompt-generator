#[cfg(test)]
mod integration_tests {
    use assert_cmd::Command;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;
    use filetime::{set_file_mtime, FileTime};

    /// Sets up a dummy Git project that is inside a Swift package,
    /// but with one extra file outside the package.
    ///
    /// Dummy Project Structure:
    ///
    /// git_root/
    /// ├── my_package/
    /// │   ├── Package.swift           // Marks this directory as a Swift package.
    /// │   ├── Instruction.swift       // Contains the main instruction.
    /// │   │                           // It has a TODO trigger for "TriggerCommentType"
    /// │   │                           // and a non-trigger comment mentioning "CommentReferencedType".
    /// │   ├── OldTodo.swift           // Contains an old TODO marker.
    /// │   ├── Ref.swift               // A referencing file that should be excluded.
    /// │   └── Sources/
    /// │       ├── Definition1.swift   // Defines DummyType1.
    /// │       ├── Definition2.swift   // Defines DummyType2.
    /// │       ├── TriggerCommentReferenced.swift   // Defines TriggerCommentType.
    /// │       └── CommentReferenced.swift            // Defines CommentReferencedType.
    /// └── Outside.swift               // (Outside the package) Defines DummyType3.
    ///
    /// The Instruction.swift file (inside my_package) contains:
    ///     class SomeClass {
    ///         var foo: DummyType1? = nil
    ///         var bar: DummyType2? = nil
    ///         var dummy: DummyType3? = nil
    ///     }
    ///     // TODO: - Let's fix TriggerCommentType
    ///     // Note: CommentReferencedType is mentioned here but is not a trigger.
    ///
    /// Returns a tuple (git_root_dir, instruction_file_path) where git_root_dir is the TempDir for the project,
    /// and instruction_file_path points to my_package/Instruction.swift.
    fn setup_dummy_project() -> (TempDir, PathBuf) {
        // Create the git root (which is not a Swift package by itself)
        let git_root_dir = TempDir::new().unwrap();
        let git_root_path = git_root_dir.path();

        // Set GET_GIT_ROOT to the git root.
        env::set_var("GET_GIT_ROOT", git_root_path.to_str().unwrap());

        // Create the package directory inside the git root.
        let package_dir = git_root_path.join("my_package");
        fs::create_dir_all(&package_dir).unwrap();

        // Create Package.swift inside the package directory.
        let package_file_path = package_dir.join("Package.swift");
        fs::write(&package_file_path, "// swift package").unwrap();

        // Create the main Instruction.swift file inside the package.
        // Note: The TODO trigger now directly references "TriggerCommentType".
        // Also, a non-trigger comment mentions "CommentReferencedType".
        let instruction_file_path = package_dir.join("Instruction.swift");
        fs::write(
            &instruction_file_path,
            "class SomeClass {
    var foo: DummyType1? = nil
    var bar: DummyType2? = nil
    var dummy: DummyType3? = nil
}
 // TODO: - Let's fix TriggerCommentType
 // Note: CommentReferencedType is mentioned here but should not trigger inclusion.",
        )
        .unwrap();

        // Create an extra file with an old TODO marker inside the package.
        let old_todo_path = package_dir.join("OldTodo.swift");
        fs::write(&old_todo_path, "class OldClass { } // TODO: - Old marker").unwrap();
        set_file_mtime(&old_todo_path, FileTime::from_unix_time(1000, 0)).unwrap();

        // Create a Sources directory inside the package with definition files.
        let sources_dir = package_dir.join("Sources");
        fs::create_dir_all(&sources_dir).unwrap();
        let def1_path = sources_dir.join("Definition1.swift");
        let def2_path = sources_dir.join("Definition2.swift");
        fs::write(&def1_path, "class DummyType1 { }").unwrap();
        fs::write(&def2_path, "class DummyType2 { }").unwrap();

        let trigger_file = sources_dir.join("TriggerCommentReferenced.swift");
        fs::write(&trigger_file, "class TriggerCommentType { }").unwrap();

        let comment_file = sources_dir.join("CommentReferenced.swift");
        fs::write(&comment_file, "class CommentReferencedType { }").unwrap();

        // Create a referencing file in the package root.
        let ref_file_path = package_dir.join("Ref.swift");
        fs::write(&ref_file_path, "let instance = SomeClass()").unwrap();

        // Now add a file outside the package (at the Git root) that defines another type.
        let outside_file = git_root_path.join("Outside.swift");
        fs::write(&outside_file, "class DummyType3 { }").unwrap();

        (git_root_dir, instruction_file_path)
    }

    /// Sets up a dummy pbcopy executable that writes its stdin to a temporary file.
    /// Returns a tuple (pbcopy_dir, clipboard_file) where pbcopy_dir is the TempDir
    /// containing the dummy pbcopy and clipboard_file is the PathBuf to the file where output is captured.
    fn setup_dummy_pbcopy() -> (TempDir, PathBuf) {
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display()),
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        (pbcopy_dir, clipboard_file)
    }

    /// Integration test for normal mode.
    /// Expects that generate_prompt (without --singular or --include-references)
    /// will include the Instruction.swift file and both definition files,
    /// while excluding the referencing file (Ref.swift) and OldTodo.swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode_includes_all_files() {
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        assert!(
            clipboard_content.contains("The contents of Instruction.swift is as follows:"),
            "Expected clipboard to include the Instruction.swift file header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected clipboard to include Definition1.swift header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected clipboard to include Definition2.swift header"
        );
        assert!(
            clipboard_content.contains("class DummyType1 { }"),
            "Expected clipboard to contain the declaration of DummyType1"
        );
        assert!(
            clipboard_content.contains("class DummyType2 { }"),
            "Expected clipboard to contain the declaration of DummyType2"
        );
        assert!(
            clipboard_content.contains("DummyType1"),
            "Expected the Instruction.swift file to reference DummyType1"
        );
        assert!(
            clipboard_content.contains("DummyType2"),
            "Expected the Instruction.swift file to reference DummyType2"
        );
        assert!(
            clipboard_content.contains("// TODO: - Let's fix TriggerCommentType"),
            "Expected the TODO comment to appear in the prompt"
        );
        assert!(
            !clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Did not expect Ref.swift to be included in the prompt"
        );
        assert!(
            !clipboard_content.contains("The contents of OldTodo.swift is as follows:"),
            "Did not expect OldTodo.swift to be included in the prompt"
        );
        assert!(
            !clipboard_content.contains("Old marker"),
            "Did not expect the old TODO marker to appear in the prompt"
        );
    }

    /// Integration test for singular mode.
    /// Expects that generate_prompt (with --singular) will include only the Instruction.swift file,
    /// excluding definition files, Ref.swift, and OldTodo.swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode_includes_only_todo_file() {
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        assert!(
            clipboard_content.contains("The contents of Instruction.swift is as follows:"),
            "Expected clipboard to include the Instruction.swift file header"
        );
        assert!(
            !clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected Definition1.swift header to be absent in singular mode"
        );
        assert!(
            !clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected Definition2.swift header to be absent in singular mode"
        );
        assert!(
            clipboard_content.contains("DummyType1"),
            "Expected the Instruction.swift file to reference DummyType1"
        );
        assert!(
            clipboard_content.contains("DummyType2"),
            "Expected the Instruction.swift file to reference DummyType2"
        );
        assert!(
            clipboard_content.contains("// TODO: - Let's fix TriggerCommentType"),
            "Expected the TODO comment to appear in the prompt"
        );
        assert!(
            !clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Did not expect Ref.swift to be included in singular mode"
        );
        assert!(
            !clipboard_content.contains("The contents of OldTodo.swift is as follows:"),
            "Did not expect OldTodo.swift to be included in singular mode"
        );
        assert!(
            !clipboard_content.contains("Old marker"),
            "Did not expect the old TODO marker to appear in the prompt"
        );
    }

    /// Integration test for include-references mode.
    /// Expects that generate_prompt (with --include-references) will include Ref.swift,
    /// in addition to the Instruction.swift and definition files, while still excluding OldTodo.swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_includes_ref_file() {
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        assert!(
            clipboard_content.contains("The contents of Instruction.swift is as follows:"),
            "Expected clipboard to include the Instruction.swift file header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected clipboard to include Definition1.swift header"
        );
        assert!(
            clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected clipboard to include Definition2.swift header"
        );
        assert!(
            clipboard_content.contains("// TODO: - Let's fix TriggerCommentType"),
            "Expected the TODO comment to appear in the prompt"
        );
        assert!(
            clipboard_content.contains("The contents of Ref.swift is as follows:"),
            "Expected Ref.swift to be included with --include-references"
        );
        assert!(
            clipboard_content.contains("let instance = SomeClass()"),
            "Expected the content of Ref.swift to appear in the prompt"
        );
        assert!(
            !clipboard_content.contains("The contents of OldTodo.swift is as follows:"),
            "Did not expect OldTodo.swift to be included in the prompt"
        );
        assert!(
            !clipboard_content.contains("Old marker"),
            "Did not expect the old TODO marker to appear in the prompt"
        );
    }
    
    /// Integration test for exclusion flags.
    /// Here we run generate_prompt with the --exclude flag for "Definition1.swift".
    /// We expect that the final prompt includes the Instruction.swift file and Definition2.swift,
    /// but does NOT include Definition1.swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_excludes_definition1() {
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt with the exclusion flag for "Definition1.swift"
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--exclude").arg("Definition1.swift");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the prompt still includes the Instruction.swift file header.
        assert!(
            clipboard_content.contains("The contents of Instruction.swift is as follows:"),
            "Expected clipboard to include the Instruction.swift file header"
        );
        // Assert that the prompt does NOT include the header for Definition1.swift.
        assert!(
            !clipboard_content.contains("The contents of Definition1.swift is as follows:"),
            "Expected Definition1.swift to be excluded"
        );
        // Assert that the prompt still includes the header for Definition2.swift.
        assert!(
            clipboard_content.contains("The contents of Definition2.swift is as follows:"),
            "Expected Definition2.swift to be included"
        );
        // Verify that the Instruction.swift file's content (including the TODO comment) is present.
        assert!(
            clipboard_content.contains("// TODO: - Let's fix TriggerCommentType"),
            "Expected the TODO comment to appear in the prompt"
        );
    }

    /// Integration test for force-global mode.
    /// Expects that generate_prompt (with --force-global) will include the file Outside.swift,
    /// which defines DummyType3, in addition to the Instruction.swift and definition files.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_force_global_includes_outside_file() {
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap()); // FIXME: hack workaround
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--force-global");
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Verify that the global search has included Outside.swift.
        assert!(
            clipboard_content.contains("The contents of Outside.swift is as follows:"),
            "Expected clipboard to include the Outside.swift file header in force-global mode"
        );
        assert!(
            clipboard_content.contains("class DummyType3 { }"),
            "Expected clipboard to contain the declaration of DummyType3"
        );
    }
    
    /// New Integration test to verify that when the TODO trigger references "TriggerCommentType",
    /// the file TriggerCommentReferenced.swift (which defines TriggerCommentType) is included in the final prompt.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_includes_trigger_referenced_file() {
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        // Set the GET_INSTRUCTION_FILE to point to Instruction.swift.
        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt (normal mode).
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Verify that TriggerCommentReferenced.swift (defining TriggerCommentType) is included in the final prompt.
        assert!(
            clipboard_content.contains("The contents of TriggerCommentReferenced.swift is as follows:"),
            "Expected TriggerCommentReferenced.swift header in prompt. Got:\n{}",
            clipboard_content
        );
        assert!(
            clipboard_content.contains("class TriggerCommentType { }"),
            "Expected TriggerCommentType declaration in prompt. Got:\n{}",
            clipboard_content
        );
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_excludes_comment_referenced_file() {
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        // Set the GET_INSTRUCTION_FILE to point to Instruction.swift.
        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt (normal mode).
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().success();

        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that CommentReferenced.swift is NOT included in the final prompt.
        assert!(
            !clipboard_content.contains("The contents of CommentReferenced.swift is as follows:"),
            "Expected CommentReferenced.swift to be excluded from prompt, but it was found."
        );
        assert!(
            !clipboard_content.contains("class CommentReferencedType { }"),
            "Expected CommentReferencedType declaration to be excluded from prompt, but it was found."
        );
    }
}
