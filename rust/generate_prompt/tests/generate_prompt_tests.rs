use assert_cmd::Command;
use predicates::prelude::*;
use std::env;
use std::fs;
use std::io::Write;
use tempfile::TempDir;

/// On Unix systems, creates a dummy executable (a shell script) in the given temporary directory.
/// The script simply echoes the provided output.
#[cfg(unix)]
fn create_dummy_executable(dir: &TempDir, name: &str, output: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, format!("#!/bin/sh\necho \"{}\"", output)).unwrap();
    use std::os::unix::fs::PermissionsExt;
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_cmd::prelude::*;
    use std::process::Command;

    /// In singular mode, only the TODO file should be included.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Dummy commands.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create the TODO file with the expected content.
        fs::write(&todo_file, "   // TODO: - Fix issue").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix issue");
        // In singular mode, we expect the file list to contain only the TODO file.
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        // Dummy assemble_prompt.
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // IMPORTANT: Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Prepend temp_dir to PATH and disable clipboard.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");

        // Check for key output markers.
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Singular mode enabled: only including the TODO file"))
            .stdout(predicate::str::contains("// TODO: - Fix issue"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
    }

    /// If --include-references is used but the TODO file isn’t a Swift file, we should error out.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_error_for_non_swift() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        // TODO file with .js extension.
        let todo_file = format!("{}/TODO.js", fake_git_root_path);
        // Create the file so extract_instruction_content can read it.
        fs::write(&todo_file, "   // TODO: - Fix issue").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix issue");
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "DummyType").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("--include-references is only supported for Swift files"));
    }

    /// Test a normal (non‑singular) run.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create the TODO file with expected content.
        fs::write(&todo_file, "   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        // IMPORTANT: Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Create a dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

        // Simulate two definition files.
        let def_files_output = format!("{}/Definition1.swift\n{}/Definition2.swift", fake_git_root_path, fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
        // Dummy filter_excluded_files is no longer used (we now use the library), so this dummy can be present or omitted.
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Instruction content: // TODO: - Fix bug"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"))
            .stdout(predicate::str::contains("Success:"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
    }

    /// Test inclusion of referencing files for Swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_for_swift() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Update: Include a type declaration so the library can extract "MyType".
        fs::write(&todo_file, "class MyType {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        // IMPORTANT: Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);

        // Note: We no longer need a dummy for "extract_enclosing_type" because the library is used.
        // Simulate that find_referencing_files returns one referencing file.
        create_dummy_executable(&temp_dir, "find_referencing_files", &format!("{}/Ref1.swift", fake_git_root_path));
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Including files that reference the enclosing type"))
            .stdout(predicate::str::contains("Enclosing type: MyType"))
            .stdout(predicate::str::contains("Searching for files referencing MyType"))
            .stdout(predicate::str::contains("Warning: The --include-references option is experimental."))
            .stdout(predicate::str::contains("Success:"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
    }
}

#[cfg(test)]
mod additional_tests {
    use super::*;
    use assert_cmd::prelude::*;
    use std::env;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

/*
    /// Test that when DIFF_WITH_BRANCH is set and the dummy "diff_with_branch" command returns a diff,
    /// the final prompt includes the diff report.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_diff_inclusion() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Basic dummy commands.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Diff test");
        // Dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeDiff").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        // Simulate a definition file.
        let def_files_output = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
        // Dummy filter_excluded_files (just echo back).
        create_dummy_executable(&temp_dir, "filter_excluded_files", &def_files_output);
        // Dummy assemble_prompt.
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");
        // Dummy diff_with_branch returns a fixed diff message.
        create_dummy_executable(&temp_dir, "diff_with_branch", "Diff output: changed");

        // Update PATH and disable clipboard.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");
        env::set_var("DIFF_WITH_BRANCH", "dummy-branch");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("The diff for"))
            .stdout(predicate::str::contains("against branch dummy-branch"))
            .stdout(predicate::str::contains("Diff output: changed"));
    }
*/

    /// Test that when the --force-global flag is passed the global context is used.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_force_global() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Create dummy commands. Here, get_package_root returns a non-empty value,
        // but with --force-global we expect that to be overridden.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create the TODO file with expected content.
        fs::write(&todo_file, "   // TODO: - Force global test").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "NonEmptyValue");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Force global test");
        // Dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeForce").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        // Simulate a definition file.
        let def_files_output = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
        create_dummy_executable(&temp_dir, "filter_excluded_files", &def_files_output);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--force-global");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Force global enabled: using Git root for context"));
    }

    /// Test that when exclusion flags are provided the exclusion branch is taken.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_exclude() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Basic dummy commands.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create the TODO file with expected content.
        fs::write(&todo_file, "   // TODO: - Exclude test").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Exclude test");
        // Dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeExclude").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        // Simulate two definition files.
        let def_files_output = format!("{}/Definition1.swift\n{}/Definition2.swift", fake_git_root_path, fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
        // With the new filtering logic using the library, we now expect that the original definition names remain.
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        // Pass multiple exclusion flags.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.args(&["--exclude", "ExcludePattern", "--exclude", "AnotherPattern"]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Excluding files matching:"))
            // Updated expectations: since the library filtering does not transform file names,
            // the definitions remain as "Definition1.swift" and "Definition2.swift".
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"));
    }
    
    /// Test that generate_prompt exits with an error when multiple markers are present in the assembled prompt.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_multiple_markers() {
        // Create a temporary directory for dummy executables.
        let temp_dir = TempDir::new().unwrap();
        // Create a fake Git root.
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Dummy "get_git_root" returns our fake Git root.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);

        // Create a dummy instruction file with multiple markers.
        let instruction_path = format!("{}/Instruction.swift", fake_git_root_path);
        let multi_marker_content = "\
            // TODO: - Marker One\n\
            Some content here\n\
            // TODO: - Marker Two\n\
            More content here\n\
            // TODO: -\n";
        fs::write(&instruction_path, multi_marker_content).unwrap();
        // Set the environment override so generate_prompt picks this file.
        env::set_var("GET_INSTRUCTION_FILE", &instruction_path);

        // Dummy "find_prompt_instruction" returns the instruction path.
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &instruction_path);
        // Dummy "get_package_root" returns an empty string.
        create_dummy_executable(&temp_dir, "get_package_root", "");
        // We allow extract_instruction_content to read directly from the file.
        // Create a dummy types file.
        let types_file = temp_dir.path().join("types.txt");
        fs::write(&types_file, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file.to_str().unwrap());
        // Dummy for find_definition_files.
        create_dummy_executable(&temp_dir, "find_definition_files", "dummy_definitions");
        // Dummy for filter_files_singular – simply echo back the instruction file path.
        create_dummy_executable(&temp_dir, "filter_files_singular", &instruction_path);

        // Prepend our temporary directory to PATH and disable clipboard copying.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        // Run the generate_prompt binary.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Multiple // TODO: - markers found. Exiting."));
    }
}
