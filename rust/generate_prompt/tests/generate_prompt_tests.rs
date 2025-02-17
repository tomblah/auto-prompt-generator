// tests/generate_prompt_tests.rs

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

/*
    /// This test simulates a successful run in singular mode.
    /// We create dummy external binaries (e.g. get_git_root, find_prompt_instruction, etc.)
    /// that return fixed outputs. We also disable pbcopy by setting DISABLE_PBCOPY.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_singular_mode() {
        // Create a temporary directory to hold our fake binaries.
        let temp_dir = TempDir::new().unwrap();

        // Create dummy executables that return fixed outputs:
        // get_git_root: returns a fake git root.
        create_dummy_executable(&temp_dir, "get_git_root", "/fake/git/root");
        // find_prompt_instruction: returns a fake TODO file (a Swift file).
        create_dummy_executable(&temp_dir, "find_prompt_instruction", "/fake/git/root/TODO.swift");
        // get_package_root: returns an empty string (so that global scope is used).
        create_dummy_executable(&temp_dir, "get_package_root", "");
        // extract_instruction_content: returns a dummy instruction.
        create_dummy_executable(&temp_dir, "extract_instruction_content", "   // TODO: - Fix issue");
        // extract_types: returns a fake file path (simulate a types file).
        let types_file_path = temp_dir.path().join("types.txt");
        fs::write(&types_file_path, "DummyType").unwrap();
        create_dummy_executable(&temp_dir, "extract_types", types_file_path.to_str().unwrap());
        // find_definition_files: returns a fake definitions file path.
        let def_file_path = temp_dir.path().join("definitions.txt");
        fs::write(&def_file_path, "/fake/git/root/Definition.swift").unwrap();
        create_dummy_executable(&temp_dir, "find_definition_files", def_file_path.to_str().unwrap());
        // filter_files_singular: returns a file list containing only the TODO file.
        let singular_file_path = temp_dir.path().join("singular.txt");
        fs::write(&singular_file_path, "/fake/git/root/TODO.swift").unwrap();
        create_dummy_executable(&temp_dir, "filter_files_singular", singular_file_path.to_str().unwrap());
        // assemble_prompt: echoes a fixed assembled prompt.
        create_dummy_executable(&temp_dir, "assemble_prompt", "Assembled prompt");

        // Prepend our temporary directory to PATH so our dummy commands are found.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        // Disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        // Run the generate_prompt binary with the --singular flag.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--singular");

        // Assert that the command succeeds and prints "Success:" in its output.
        cmd.assert().success().stdout(predicate::str::contains("Success:"));
    }
*/

/*
    /// This test checks that if the --include-references flag is used but the
    /// TODO file is not a Swift file, an error is printed and the process exits with failure.
    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_include_references_error_for_non_swift() {
        let temp_dir = TempDir::new().unwrap();
        // Create dummy binaries with minimal output:
        create_dummy_executable(&temp_dir, "get_git_root", "/fake/git/root");
        // Here the TODO file is a JavaScript file.
        create_dummy_executable(&temp_dir, "find_prompt_instruction", "/fake/git/root/TODO.js");

        // Prepend our temporary directory to PATH.
        let original_path = env::var("PATH").unwrap();
        env::set_var("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), original_path));
        env::set_var("DISABLE_PBCOPY", "1");

        // Run the binary with --include-references; this should fail because TODO.js is not a Swift file.
        let mut cmd = Command::cargo_bin("generate_prompt").unwrap();
        cmd.arg("--include-references");

        cmd.assert()
            .failure()
            .stderr(predicate::str::contains("--include-references is only supported for Swift files"));
    }
*/

}
