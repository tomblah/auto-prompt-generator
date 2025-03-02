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

    /// Helper: clear GET_GIT_ROOT so tests that don't need it won't use a stale value.
    fn clear_git_root() {
        env::remove_var("GET_GIT_ROOT");
    }

    /// In singular mode, only the TODO file should be included.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode() {
        clear_git_root();
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

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Prepend temp_dir to PATH and disable clipboard.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Singular mode enabled: only including the TODO file"))
            .stdout(predicate::str::contains("// TODO: - Fix issue"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));

        env::remove_var("GET_GIT_ROOT");
    }

    /// If --include-references is used but the TODO file isn’t a Swift file, we should error out.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_error_for_non_swift() {
        clear_git_root();
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        // TODO file with .js extension.
        let todo_file = format!("{}/TODO.js", fake_git_root_path);
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

        clear_git_root();
    }

    /// Test a normal (non‑singular) run.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode() {
        // Use a fake Git root by setting GET_GIT_ROOT.
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set GET_GIT_ROOT so generate_prompt uses our fake Git root.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Write a TODO file that includes a type declaration so that extract_types_from_file
        // extracts "TypeFixBug".
        fs::write(&todo_file, "class TypeFixBug {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // (Optional) Create a dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeFixBug").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

        // Now create actual definition files in the fake Git root.
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeFixBug {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TypeFixBug {}").unwrap();

        // No dummy executable for find_definition_files is needed now.

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

        // Clean up GET_GIT_ROOT so it doesn't affect other tests.
        env::remove_var("GET_GIT_ROOT");
    }

    /// Test inclusion of referencing files for Swift.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_for_swift() {
        clear_git_root();
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Include a type declaration so that the extractor finds "MyType".
        fs::write(&todo_file, "class MyType {}\n   // TODO: - Fix bug").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix bug");

        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_file);
        create_dummy_executable(&temp_dir, "filter_files_singular", &todo_file);
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

        clear_git_root();
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

    /// Test that when the --force-global flag is passed the global context is used.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_force_global() {
        // This test requires its own fake Git root.
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set GET_GIT_ROOT to our fake_git_root.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Include a type declaration for extraction.
        fs::write(&todo_file, "class TypeForce {}\n   // TODO: - Force global test").unwrap();
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "NonEmptyValue");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Force global test");
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeForce").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_files_output = format!("{}/Definition.swift", fake_git_root_path);
        create_dummy_executable(&temp_dir, "find_definition_files", &def_files_output);
        create_dummy_executable(&temp_dir, "filter_excluded_files", &def_files_output);
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Also force GET_INSTRUCTION_FILE so that extraction works.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--force-global");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Force global enabled: using Git root for context"));

        env::remove_var("GET_GIT_ROOT");
    }

    /// Test that when exclusion flags are provided the exclusion branch is taken.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_exclude() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        fs::write(&todo_file, "class TypeExclude {}\n   // TODO: - Exclude test").unwrap();
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Exclude test");
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeExclude").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeExclude {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "class TypeExclude {}").unwrap();
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.args(&["--exclude", "ExcludePattern", "--exclude", "AnotherPattern"]);

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Excluding files matching:"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"));

        env::remove_var("GET_GIT_ROOT");
    }
    
    /// Test that generate_prompt exits with an error when multiple markers are present.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_multiple_markers() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        let instruction_path = format!("{}/Instruction.swift", fake_git_root_path);
        let multi_marker_content = "\
            // TODO: - Marker One\n\
            Some content here\n\
            // TODO: - Marker Two\n\
            More content here\n\
            // TODO: -\n";
        fs::write(&instruction_path, multi_marker_content).unwrap();
        env::set_var("GET_INSTRUCTION_FILE", &instruction_path);
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &instruction_path);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        let types_file = temp_dir.path().join("types.txt");
        fs::write(&types_file, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file.to_str().unwrap());
        create_dummy_executable(&temp_dir, "find_definition_files", "dummy_definitions");
        create_dummy_executable(&temp_dir, "filter_files_singular", &instruction_path);

        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("Multiple // TODO: - markers found. Exiting."));

        env::remove_var("GET_GIT_ROOT");
    }
}
