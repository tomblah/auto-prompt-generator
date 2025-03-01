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

        // Set the Git root override so that generate_prompt uses our fake Git root.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Dummy commands.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create the TODO file with expected content.
        // For singular mode, we use a simple token; its content here is "   // TODO: - FixIssue"
        fs::write(&todo_file, "   // TODO: - FixIssue").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - FixIssue");
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

        // Check for key output markers.
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Singular mode enabled: only including the TODO file"))
            .stdout(predicate::str::contains("// TODO: - FixIssue"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
    }

    /// Test that when GET_GIT_ROOT is set and --force-global is passed, the global context is used.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_force_global() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set the Git root override.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Dummy commands.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create the TODO file with expected content.
        fs::write(&todo_file, "   // TODO: - ForceGlobalTest").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        // For force-global, get_package_root is irrelevant.
        create_dummy_executable(&temp_dir, "get_package_root", "NonEmptyValue");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - ForceGlobalTest");
        // Create a dummy types file.
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeForce").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        // Create a real definition file.
        let def_file = fake_git_root.path().join("Definition.swift");
        fs::write(&def_file, "class TypeForce {}").unwrap();
        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Prepend temp_dir to PATH and disable clipboard.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--force-global");

        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Force global enabled: using Git root for context"));
    }

    /// Test a normal (non‑singular) run.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_normal_mode() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set the Git root override.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create the TODO file with expected content.
        // Here we use a token "TypeA" that our definition files will match.
        fs::write(&todo_file, "   // TODO: - TypeA").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - TypeA");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Create a dummy types file that contains "TypeA".
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());

        // Create two real definition files in the fake Git root that contain valid definitions for TypeA.
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeA {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "struct TypeA {}").unwrap();

        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Prepend temp_dir to PATH and disable clipboard.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Found exactly one instruction in"))
            .stdout(predicate::str::contains("Instruction content: // TODO: - TypeA"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"))
            .stdout(predicate::str::contains("Success:"))
            .stdout(predicate::str::contains("Prompt has been copied to clipboard."));
    }

    /// Test that when exclusion flags are provided the exclusion branch is taken.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_exclude() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set the Git root override.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Basic dummy commands.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);
        let todo_file = format!("{}/TODO.swift", fake_git_root_path);
        // Create the TODO file with expected content.
        // Use "TypeExclude" as token so that our definition files can match.
        fs::write(&todo_file, "   // TODO: - TypeExclude").unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &todo_file);
        create_dummy_executable(&temp_dir, "get_package_root", "");
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - TypeExclude");

        // Create a dummy types file containing "TypeExclude".
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "TypeExclude").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        
        // Create two real definition files in the fake Git root that contain valid definitions for TypeExclude.
        let def_file1 = fake_git_root.path().join("Definition1.swift");
        fs::write(&def_file1, "class TypeExclude {}").unwrap();
        let def_file2 = fake_git_root.path().join("Definition2.swift");
        fs::write(&def_file2, "struct TypeExclude {}").unwrap();

        create_dummy_executable(&temp_dir, "assemble_prompt", "dummy");

        // Force the instruction file override.
        env::set_var("GET_INSTRUCTION_FILE", &todo_file);

        // Prepend temp_dir to PATH and disable clipboard copying.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        // Pass multiple exclusion flags.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.args(&["--exclude", "ExcludePattern", "--exclude", "AnotherPattern"]);

        // Now the final output should include our definition file names.
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("Excluding files matching:"))
            .stdout(predicate::str::contains("Definition1.swift"))
            .stdout(predicate::str::contains("Definition2.swift"));
    }

    /// Test that generate_prompt exits with an error when multiple markers are present in the assembled prompt.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_multiple_markers() {
        let temp_dir = TempDir::new().unwrap();
        let fake_git_root = TempDir::new().unwrap();
        let fake_git_root_path = fake_git_root.path().to_str().unwrap();

        // Set the Git root override.
        env::set_var("GET_GIT_ROOT", fake_git_root_path);

        // Dummy "get_git_root" returns our fake Git root.
        create_dummy_executable(&temp_dir, "get_git_root", fake_git_root_path);

        // Create a dummy TODO instruction file.
        let instruction_path = format!("{}/Instruction.swift", fake_git_root_path);
        // Create the file with content that includes multiple markers.
        fs::write(
            &instruction_path,
            "   // TODO: - Marker One\nSome content here\n   // TODO: - Marker Two\nMore content here\n   // TODO: -",
        )
        .unwrap();
        create_dummy_executable(&temp_dir, "find_prompt_instruction", &instruction_path);

        // Dummy "get_package_root" returns an empty string.
        create_dummy_executable(&temp_dir, "get_package_root", "");

        // Dummy "extract_instruction_content" returns the content with markers.
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Marker One\nSome content here\n   // TODO: - Marker Two\nMore content here\n   // TODO: -");

        // Create a dummy types file.
        let types_file = temp_dir.path().join("types.txt");
        fs::write(&types_file, "TypeA").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file.to_str().unwrap());

        // Add a dummy for find_definition_files so that it succeeds (we won’t actually use it).
        fs::write(fake_git_root.path().join("Definition.swift"), "class TypeA {}").unwrap();

        // For this test we want the final prompt to have multiple markers.
        create_dummy_executable(&temp_dir, "assemble_prompt", "\
The contents of Instruction.swift is as follows:\n\n\
 // TODO: - Marker One\nSome content here\n\n\
 // TODO: - Marker Two\nMore content here\n\n\
 // TODO: -\n");

        // Dummy for filter_files_singular.
        create_dummy_executable(&temp_dir, "filter_files_singular", &instruction_path);

        // Prepend our temporary directory to PATH and disable clipboard copying.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.assert().failure()
            .stderr(predicate::str::contains("Multiple // TODO: - markers found. Exiting."));
    }
}

#[cfg(test)]
mod additional_tests {
    // You can include any further tests here as needed.
}
