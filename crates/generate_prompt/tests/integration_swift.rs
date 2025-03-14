// crates/generate_prompt/tests/integration_swift.rs

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

        // Remove any diff branch setting to avoid unwanted branch verification.
        env::remove_var("DIFF_WITH_BRANCH");

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
            "public final class SomeClass<T: Codable, U: Equatable> {
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
        env::remove_var("DIFF_WITH_BRANCH");
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
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
        env::remove_var("DIFF_WITH_BRANCH");
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
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
        env::remove_var("DIFF_WITH_BRANCH");
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
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
        env::remove_var("DIFF_WITH_BRANCH");
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
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
        env::remove_var("DIFF_WITH_BRANCH");
        let (_project_dir, instruction_file_path) = setup_dummy_project();

        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
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
        env::remove_var("DIFF_WITH_BRANCH");
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
        env::remove_var("DIFF_WITH_BRANCH");
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

#[cfg(test)]
mod integration_tests_substring_markers {
    use assert_cmd::Command;
    use std::env;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[cfg(unix)]
    // NB: substring markers and Swift aren't really working too well, will not support it for the time being
    fn test_generate_prompt_swift_enclosing_function_outside_markers() {
        env::remove_var("DIFF_WITH_BRANCH");
        // Create a temporary directory for our dummy Swift project.
        let temp_dir = TempDir::new().unwrap();
        let main_swift_path = temp_dir.path().join("main.swift");

        // Write a Swift file that contains:
        // - A function 'unimportantFunction' that should not appear in the final prompt.
        // - A substring markers block (only content between "// v" and "// ^" is normally included)
        //   that wraps the function 'importantFunction'.
        // - A function 'anotherUnimportantFunction' that should not appear in the final prompt.
        // - A function 'enclosingFunction' that is not inside any markers and which contains a TODO marker.
        //
        // Because the TODO marker ("// TODO: - Correct the computation here") appears
        // outside of any marker block, the entire 'enclosingFunction' is automatically
        // appended as an enclosing context to the final prompt.
        let main_swift_content = r#"
import Foundation

public func unimportantFunction<T: Collection, U: Numeric>(
    input: T,
    transform: (T.Element) throws -> U
) async rethrows -> [U] where T.Element: CustomStringConvertible {
    print("This is not inside markers.")
    return try input.map { try transform($0) }
}

// v
// This content is included via substring markers.
public func importantFunction<T: Collection>(with data: T) async rethrows -> [T.Element] where T.Element: Numeric {
    print("This is inside markers.")
}
// ^

public func anotherUnimportantFunction<T: Decodable, U: Encodable>(
    input: T,
    transform: (T) throws -> U
) rethrows -> U {
    print("This is outside markers.")
    return try transform(input)
}

public func enclosingFunction<V: Equatable, W: Codable>(input: V) -> W? {
    print("This is not inside markers normally.")
    // TODO: - Correct the computation here
    print("Computation ends.")
    return nil
}
"#;
        fs::write(&main_swift_path, main_swift_content).unwrap();

        // Set environment variables so generate_prompt uses our Swift file.
        env::set_var("GET_INSTRUCTION_FILE", main_swift_path.to_str().unwrap());
        env::set_var("GET_GIT_ROOT", temp_dir.path().to_str().unwrap());

        // Set up a dummy pbcopy executable that writes its stdin to a temporary clipboard file.
        let pbcopy_dir = TempDir::new().unwrap();
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display())
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Ensure that clipboard copying is enabled.
        env::remove_var("DISABLE_PBCOPY");

        // Run the generate_prompt binary in singular mode.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the output from the dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read dummy clipboard file");

        // Assert that the final prompt includes the content from the substring markers.
        assert!(
            clipboard_content.contains("This content is included via substring markers."),
            "Expected marker content to appear in prompt; got:\n{}",
            clipboard_content
        );

        // Assert that the final prompt includes the function name 'importantFunction'.
        assert!(
            clipboard_content.contains("importantFunction"),
            "Expected the prompt to include 'importantFunction'; got:\n{}",
            clipboard_content
        );

        // Assert that the final prompt includes the function definition of 'enclosingFunction'
        // and the TODO marker comment.
        assert!(
            clipboard_content.contains("enclosingFunction"),
            "Expected the prompt to include the function 'enclosingFunction'; got:\n{}",
            clipboard_content
        );
        assert!(
            clipboard_content.contains("// TODO: - Correct the computation here"),
            "Expected the prompt to include the TODO comment; got:\n{}",
            clipboard_content
        );

        // Also assert that the function body of 'enclosingFunction' is included (e.g. the final print statement).
        assert!(
            clipboard_content.contains("Computation ends."),
            "Expected the prompt to include the body of 'enclosingFunction'; got:\n{}",
            clipboard_content
        );

        // Assert that the final prompt includes an appended enclosing function context.
        // (Typically indicated by a string like "Enclosing function context:" in the output.)
        assert!(
            clipboard_content.contains("Enclosing function context:"),
            "Expected the prompt to include the enclosing function context; got:\n{}",
            clipboard_content
        );

        // Assert that the unimportant function does not appear in the prompt.
        assert!(
            !clipboard_content.contains("unimportantFunction"),
            "Expected the prompt to not include 'unimportantFunction'; got:\n{}",
            clipboard_content
        );
        
        // Assert that 'anotherUnimportantFunction' does not appear in the final prompt.
        assert!(
            !clipboard_content.contains("anotherUnimportantFunction"),
            "Expected the prompt to not include 'anotherUnimportantFunction'; got:\n{}",
            clipboard_content
        );
    }
}

#[cfg(test)]
mod integration_diff {
    use assert_cmd::Command;
    use std::env;
    use std::fs;
    use std::path::PathBuf;
    use std::process::Command as StdCommand;
    use tempfile::{TempDir};
    use predicates::prelude::*;

    /// Sets up a dummy pbcopy executable that writes its stdin to a temporary file.
    /// Returns a tuple (pbcopy_dir, clipboard_file) where pbcopy_dir is the TempDir
    /// containing the dummy pbcopy and clipboard_file is the PathBuf to the file where output is captured.
    fn setup_dummy_pbcopy() -> (TempDir, PathBuf) {
        let pbcopy_dir = TempDir::new().expect("Failed to create dummy pbcopy directory");
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display()),
        )
        .expect("Failed to write dummy pbcopy script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms).unwrap();
        }
        (pbcopy_dir, clipboard_file)
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_with_diff_option() {
        // Create a temporary directory to act as the Git repository root.
        let git_root_dir = TempDir::new().expect("Failed to create Git root temp dir");
        let git_root_path = git_root_dir.path();

        // Initialize a Git repository.
        let init_status = StdCommand::new("git")
            .arg("init")
            .current_dir(&git_root_path)
            .status()
            .expect("Failed to initialize git repository");
        assert!(init_status.success(), "Git init failed");

        // Create the Swift package directory.
        let package_dir = git_root_path.join("my_package");
        fs::create_dir_all(&package_dir).expect("Failed to create package directory");

        // Create Package.swift to mark this as a Swift package.
        let package_file_path = package_dir.join("Package.swift");
        fs::write(&package_file_path, "// swift package").expect("Failed to write Package.swift");

        // Create Instruction.swift with initial content.
        let instruction_file_path = package_dir.join("Instruction.swift");
        let initial_content = "public final class SomeClass {\n    var x: Int = 0\n}\n// TODO: - Fix SomeClass\n";
        fs::write(&instruction_file_path, initial_content).expect("Failed to write Instruction.swift");

        // Add and commit Instruction.swift.
        let add_status = StdCommand::new("git")
            .args(&["add", "Instruction.swift"])
            .current_dir(&package_dir)
            .status()
            .expect("Failed to git add Instruction.swift");
        assert!(add_status.success(), "Git add failed");

        let commit_status = StdCommand::new("git")
            .args(&["commit", "-m", "Initial commit"])
            .current_dir(&package_dir)
            .status()
            .expect("Failed to git commit");
        assert!(commit_status.success(), "Git commit failed");

        // Modify Instruction.swift to create a diff.
        let modified_content = "public final class SomeClass {\n    var x: Int = 0\n    func diffFunc() {}\n}\n// TODO: - Fix SomeClass\n";
        fs::write(&instruction_file_path, modified_content).expect("Failed to modify Instruction.swift");

        // Set environment variables so generate_prompt picks up our dummy project.
        env::set_var("GET_GIT_ROOT", git_root_path.to_str().unwrap());
        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        // Set up dummy pbcopy so that clipboard output is captured.
        let (pbcopy_dir, clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Enable diff reporting by setting DIFF_WITH_BRANCH (using "HEAD" here).
        env::set_var("DIFF_WITH_BRANCH", "HEAD");

        // Set TODO_FILE_BASENAME so that the diff output is appended.
        let file_basename = instruction_file_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_string();
        env::set_var("TODO_FILE_BASENAME", &file_basename);

        // Run the generate_prompt binary.
        let mut cmd = Command::cargo_bin("generate_prompt").expect("Failed to find generate_prompt binary");
        cmd.assert().success();

        // Read the content from our dummy clipboard file.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read clipboard content");

        // Assert that the prompt includes a diff section.
        assert!(
            clipboard_content.contains("The diff for"),
            "Expected diff section header in prompt; got:\n{}",
            clipboard_content
        );
        // Check that the diff output contains the newly added function.
        assert!(
            clipboard_content.contains("func diffFunc()"),
            "Expected diff output to mention the added function; got:\n{}",
            clipboard_content
        );
        // Also, a diff marker such as 'diff --git' should be present.
        assert!(
            clipboard_content.contains("diff --git"),
            "Expected diff markers in output; got:\n{}",
            clipboard_content
        );
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_scrubs_extra_todo_markers() {
        // Ensure diff mode is disabled so that no extra diff markers are appended.
        env::remove_var("DIFF_WITH_BRANCH");
        // Create a temporary directory to simulate a Git repository.
        let git_root = TempDir::new().expect("Failed to create temp git root");
        let git_root_path = git_root.path();
        env::set_var("GET_GIT_ROOT", git_root_path.to_str().unwrap());

        // Create a package directory with a Package.swift to mark it as a Swift package.
        let package_dir = git_root_path.join("test_package");
        fs::create_dir_all(&package_dir).expect("Failed to create package directory");
        fs::write(package_dir.join("Package.swift"), "// swift package")
            .expect("Failed to write Package.swift");

        // Create Instruction.swift with multiple "// TODO: -" markers.
        // The first marker exactly matches the primary (CTA) marker.
        // Any extra markers in between should be scrubbed.
        // The final marker is not taken from the file but replaced by the fixed instruction
        // appended by the prompt assembler.
        let instruction_file = package_dir.join("Instruction.swift");
        let instruction_content = r#"
public class Dummy {
    func doSomething() {
        // some code here
    }
}
// TODO: - Primary TODO marker
// TODO: - Extra marker that should be scrubbed
// TODO: - Another marker that should be scrubbed
"#;
        fs::write(&instruction_file, instruction_content)
            .expect("Failed to write Instruction.swift");

        // Set the environment variable so generate_prompt uses this instruction file.
        env::set_var("GET_INSTRUCTION_FILE", instruction_file.to_str().unwrap());
        env::remove_var("DISABLE_PBCOPY");

        // Set up a dummy pbcopy executable to capture clipboard output.
        let pbcopy_dir = TempDir::new().expect("Failed to create dummy pbcopy dir");
        let clipboard_file = pbcopy_dir.path().join("clipboard.txt");
        let dummy_pbcopy_path = pbcopy_dir.path().join("pbcopy");
        fs::write(
            &dummy_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"", clipboard_file.display()),
        )
        .expect("Failed to write dummy pbcopy script");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&dummy_pbcopy_path)
                .expect("Failed to get metadata")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&dummy_pbcopy_path, perms)
                .expect("Failed to set permissions on dummy pbcopy");
        }
        // Prepend the dummy pbcopy directory to the PATH.
        let original_path = env::var("PATH").expect("PATH not found");
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt in singular mode so that only the instruction file is processed.
        let mut cmd = Command::cargo_bin("generate_prompt")
            .expect("Failed to find generate_prompt binary");
        cmd.arg("--singular");
        cmd.assert().success();

        // Read the final prompt from the clipboard output.
        let clipboard_content = fs::read_to_string(&clipboard_file)
            .expect("Failed to read clipboard file");

        // Count the occurrences of marker lines.
        let marker_count = clipboard_content.matches("// TODO: -").count();
        // We expect exactly 2 markers: the primary marker from the instruction file
        // and the fixed instruction (acting as the final marker) appended by the prompt assembler.
        assert_eq!(
            marker_count, 2,
            "Expected exactly 2 TODO markers in final prompt, found {}",
            marker_count
        );

        // Verify that the primary marker appears.
        assert!(
            clipboard_content.contains("// TODO: - Primary TODO marker"),
            "Primary marker not found in final prompt"
        );
        // Verify that the fixed instruction (serving as the final marker) appears.
        assert!(
            clipboard_content.contains("Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"),
            "Fixed instruction final marker not found in final prompt"
        );
        // Ensure that extra markers have been scrubbed.
        assert!(
            !clipboard_content.contains("Extra marker that should be scrubbed"),
            "Extra marker was not scrubbed from final prompt"
        );
        assert!(
            !clipboard_content.contains("Another marker that should be scrubbed"),
            "Another extra marker was not scrubbed from final prompt"
        );
    }
    
    /// New integration test that asserts failure when a branch specified by DIFF_WITH_BRANCH is not found.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_diff_with_nonexistent_branch_integration() {
        // Create a temporary directory to act as the Git repository root.
        let git_root_dir = TempDir::new().expect("Failed to create Git root temp dir");
        let git_root_path = git_root_dir.path();

        // Initialize a Git repository (without any commits so HEAD does not exist).
        let init_status = StdCommand::new("git")
            .arg("init")
            .current_dir(&git_root_path)
            .status()
            .expect("Failed to initialize git repository");
        assert!(init_status.success(), "Git init failed");

        // Create the Swift package directory.
        let package_dir = git_root_path.join("my_package");
        fs::create_dir_all(&package_dir).expect("Failed to create package directory");

        // Create Package.swift to mark this as a Swift package.
        let package_file_path = package_dir.join("Package.swift");
        fs::write(&package_file_path, "// swift package").expect("Failed to write Package.swift");

        // Create Instruction.swift with some content.
        let instruction_file_path = package_dir.join("Instruction.swift");
        let content = "public final class SomeClass { var x: Int = 0 } \n// TODO: - Fix SomeClass\n";
        fs::write(&instruction_file_path, content).expect("Failed to write Instruction.swift");

        // Set environment variables.
        env::set_var("GET_GIT_ROOT", git_root_path.to_str().unwrap());
        env::set_var("GET_INSTRUCTION_FILE", instruction_file_path.to_str().unwrap());
        // Set DIFF_WITH_BRANCH to a branch that doesn't exist.
        env::set_var("DIFF_WITH_BRANCH", "nonexistent");
        env::remove_var("DISABLE_PBCOPY");

        // Set up dummy pbcopy so that if any output were produced, it would be captured.
        let (pbcopy_dir, _clipboard_file) = setup_dummy_pbcopy();
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", pbcopy_dir.path().to_str().unwrap(), original_path));

        // Run generate_prompt and assert that it fails with the expected error.
        let mut cmd = Command::cargo_bin("generate_prompt").expect("Failed to find generate_prompt binary");
        cmd.assert()
           .failure()
           .stderr(predicate::str::contains("Error: Branch 'nonexistent' does not exist."));
    }
}

// NB: these tests are currently for when we find a bug and want to reproduce it
mod strict_end_to_end_tests {
    use assert_cmd::Command;           // for running your binary via cargo_bin
    use std::env;                      // for env::set_var etc.
    use assert_fs::prelude::*;         // for methods like child(), which requires the PathChild trait
    use assert_fs::fixture::PathChild; // explicitly bring the PathChild trait into scope
    use std::process::Command as StdCommand;
    use assert_cmd::assert::OutputAssertExt;
    use predicates::boolean::PredicateBooleanExt;

    #[test]
    fn test_generate_prompt_runs_in_git_repo() {
        // Create a temporary directory and set it up as a git repo.
        let temp = assert_fs::TempDir::new().unwrap();

        // Create a dummy Swift file with a TODO marker.
        let test_swift = temp.child("Test.swift");
        test_swift
            .write_str("// TODO: - Implement feature\nfn main() {}\n")
            .unwrap();

        // Set GET_INSTRUCTION_FILE to force generate_prompt to use our file.
        env::set_var("GET_INSTRUCTION_FILE", test_swift.path().to_str().unwrap());

        // Initialize the temporary directory as a git repository.
        StdCommand::new("git")
            .current_dir(temp.path())
            .args(&["init"])
            .assert()
            .success();

        // Set GET_GIT_ROOT explicitly to the canonicalized path of the temp directory.
        let canonical_git_root = temp.path().canonicalize().unwrap();
        env::set_var("GET_GIT_ROOT", canonical_git_root.to_str().unwrap());

        // Optionally, disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        // Run the generate_prompt binary from the temporary git repo.
        Command::cargo_bin("generate_prompt")
            .unwrap()
            .current_dir(temp.path())
            .assert()
            .success()
            .stdout(predicates::str::contains("Success:"));

        // Cleanup.
        temp.close().unwrap();
    }
    
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_with_swift_input() {
        // Remove any diff branch setting.
        env::remove_var("DIFF_WITH_BRANCH");

        // Create a temporary directory to simulate the Git repository.
        let temp = assert_fs::TempDir::new().unwrap();

        // Create a Swift source file ("Example.swift") with the input content.
        // The content is as similar as possible to the Objective-C snippet.
        let example_swift = temp.child("Example.swift");
        example_swift
            .write_str(
r#"// v
// ^

func exampleFunction() {
    let foo = FooBar()
    // TODO: - example
}
"#,
            )
            .unwrap();

        // Force generate_prompt to use this file as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", example_swift.path().to_str().unwrap());

        // Set GET_GIT_ROOT to the canonicalized temporary directory.
        let canonical_git_root = temp.path().canonicalize().unwrap();
        env::set_var("GET_GIT_ROOT", canonical_git_root.to_str().unwrap());

        // Disable clipboard copying during testing.
        env::set_var("DISABLE_PBCOPY", "1");

        // Initialize a Git repository in the temp directory.
        StdCommand::new("git")
            .current_dir(temp.path())
            .args(&["init"])
            .assert()
            .success();

        // Run the generate_prompt binary from the temporary directory.
        Command::cargo_bin("generate_prompt")
            .unwrap()
            .current_dir(temp.path())
            .assert()
            .success()
            .stdout(
                predicates::str::contains("Success:")
                    .and(predicates::str::contains("example")),
            );

        // Cleanup.
        temp.close().unwrap();
    }
}

mod targeted_mode {
    use assert_cmd::Command;
    use assert_fs::prelude::*;
    use assert_fs::fixture::PathChild;
    use predicates::str::contains;
    use std::env;
    use std::process::Command as StdCommand;
    use assert_cmd::assert::OutputAssertExt;

    #[test]
    fn test_generate_prompt_with_targeted_mode() {
        // Create a temporary directory to simulate a Git repository.
        let temp = assert_fs::TempDir::new().unwrap();

        // Create a Swift file ("Targeted.swift") with targeted content.
        // The file contains:
        // - An outer declaration that should be ignored in targeted mode.
        // - A function block (the candidate enclosing block) that declares an inner type and includes a TODO trigger comment.
        //   The trigger comment is: "// TODO: - Perform action"
        //   Since tokens starting with lowercase letters are filtered out, only "Perform" should be extracted.
        let swift_file = temp.child("Targeted.swift");
        swift_file.write_str(
    r#"class OuterType {}
    func fetchData(from urlString: String) async throws -> Data {
        class InnerType {}
        // TODO: - Perform action
    }
    "#,
        ).unwrap();

        // Force generate_prompt to use this file as the instruction file.
        env::set_var("GET_INSTRUCTION_FILE", swift_file.path().to_str().unwrap());

        // Set GET_GIT_ROOT to the canonical path of the temporary directory.
        let canonical_git_root = temp.path().canonicalize().unwrap();
        env::set_var("GET_GIT_ROOT", canonical_git_root.to_str().unwrap());

        // Disable clipboard copying during testing.
        env::set_var("DISABLE_PBCOPY", "1");

        // Initialize the temporary directory as a Git repository.
        StdCommand::new("git")
            .current_dir(temp.path())
            .args(&["init"])
            .assert()
            .success();

        // Run the generate_prompt binary with the new "--tgtd" flag,
        // which enables targeted mode.
        Command::cargo_bin("generate_prompt")
            .unwrap()
            .current_dir(temp.path())
            .arg("--tgtd")
            .assert()
            .success()
            .stdout(contains("Success:"))
            // Verify that the printed "Types found:" section includes only "InnerType" and "Perform"
            .stdout(contains("Types found:\nInnerType\nPerform"))
            // Verify that the final prompt still contains the full instruction.
            .stdout(contains("// TODO: - Perform action"));

        // Cleanup temporary directory.
        temp.close().unwrap();
    }
}
