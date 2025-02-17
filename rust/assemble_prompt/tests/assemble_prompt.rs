use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;
use std::env;
use std::io::Write;

#[test]
fn test_fixed_instruction_appended() {
    // Create a temporary file listing one file to process.
    let mut found_files = NamedTempFile::new().unwrap();
    let mut file1 = NamedTempFile::new().unwrap();
    writeln!(file1, "class Dummy {{}}").unwrap();
    let file1_path = file1.path().to_str().unwrap();
    writeln!(found_files, "{}", file1_path).unwrap();

    // For this test, override the prompt processor to simply "cat" the file.
    env::set_var("RUST_PROMPT_FILE_PROCESSOR", "cat");
    env::remove_var("DIFF_WITH_BRANCH"); // ensure diff is not added

    let mut cmd = Command::cargo_bin("assemble_prompt").unwrap();
    cmd.arg(found_files.path().to_str().unwrap())
       .arg("ignored");

    // Verify that the output contains the file header and the fixed instruction.
    cmd.assert().success()
       .stdout(predicate::str::contains("The contents of")
       .and(predicate::str::contains("Can you do the TODO:- in the above code? But ignoring all FIXMEs")));
}

#[test]
fn test_diff_inclusion() {
    // Create temporary file for found_files and a file to process.
    let mut found_files = NamedTempFile::new().unwrap();
    let mut file_diff = NamedTempFile::new().unwrap();
    writeln!(file_diff, "class DummyDiff {{}}").unwrap();
    let file_diff_path = file_diff.path().to_str().unwrap();
    writeln!(found_files, "{}", file_diff_path).unwrap();

    // Override prompt processor to 'cat' (so it returns the file contents).
    env::set_var("RUST_PROMPT_FILE_PROCESSOR", "cat");
    // Activate diff logic.
    env::set_var("DIFF_WITH_BRANCH", "dummy-branch");

    // Create a dummy diff_with_branch script that returns a fixed diff message.
    let temp_dir = tempfile::tempdir().unwrap();
    let dummy_diff = temp_dir.path().join("diff_with_branch");
    std::fs::write(&dummy_diff, "#!/bin/sh\necho \"Dummy diff output for $(basename \\\"$1\\\")\"")
        .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dummy_diff).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dummy_diff, perms).unwrap();
    }
    // Prepend the temp directory to PATH.
    let current_path = env::var("PATH").unwrap();
    let new_path = format!("{}:{}", temp_dir.path().to_str().unwrap(), current_path);
    env::set_var("PATH", new_path);

    let mut cmd = Command::cargo_bin("assemble_prompt").unwrap();
    cmd.arg(found_files.path().to_str().unwrap())
       .arg("ignored");

    cmd.assert().success()
       .stdout(predicate::str::contains("Dummy diff output for")
       .and(predicate::str::contains("against branch dummy-branch")));
}

#[test]
fn test_substring_marker_filtering() {
    use std::path::PathBuf;
    
    // Create a file with substring markers.
    let mut found_files = tempfile::NamedTempFile::new().unwrap();
    let mut marked_file = tempfile::NamedTempFile::new().unwrap();
    writeln!(marked_file, "import Foundation").unwrap();
    writeln!(marked_file, "// v").unwrap();
    writeln!(marked_file, "func secretFunction() {{").unwrap();
    writeln!(marked_file, "    print(\"This is inside the markers.\")").unwrap();
    writeln!(marked_file, "}}").unwrap();
    writeln!(marked_file, "// ^").unwrap();
    writeln!(marked_file, "func publicFunction() {{").unwrap();
    writeln!(marked_file, "    print(\"This is outside the markers.\")").unwrap();
    writeln!(marked_file, "}}").unwrap();
    let marked_file_path = marked_file.path().to_str().unwrap();
    writeln!(found_files, "{}", marked_file_path).unwrap();

    // Prepare a temporary directory for our dummy filter script.
    let temp_dir = tempfile::tempdir().unwrap();
    let dummy_filter_path = temp_dir.path().join("filter_substring_markers");
    std::fs::write(
        &dummy_filter_path,
        "#!/bin/sh\necho \"func secretFunction() {\\n    print(\\\"This is inside the markers.\\\")\\n}\"",
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dummy_filter_path).unwrap().permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dummy_filter_path, perms).unwrap();
    }
    
    // Build the command and set per-command environment overrides.
    let mut cmd = assert_cmd::Command::cargo_bin("assemble_prompt").unwrap();
    cmd.arg(found_files.path().to_str().unwrap())
       .arg("ignored")
       // Override the prompt processor to force failure.
       .env("RUST_PROMPT_FILE_PROCESSOR", "false_command")
       // Set our dummy filter command.
       .env("RUST_FILTER_SUBSTRING_MARKERS", dummy_filter_path.to_str().unwrap());

    // Optionally, you can override PATH locally if needed:
    // let current_path = std::env::var("PATH").unwrap();
    // cmd.env("PATH", format!("{}:{}", temp_dir.path().to_str().unwrap(), current_path));

    let output = cmd.assert().success().get_output().stdout.clone();
    let output_str = String::from_utf8_lossy(&output);
    
    // Check that content outside the markers (publicFunction) is not included.
    assert!(!output_str.contains("func publicFunction() {"),
            "Output should not contain publicFunction, but got:\n{}", output_str);
    // Check that the secretFunction content is included.
    assert!(output_str.contains("func secretFunction() {"),
            "Output should contain secretFunction, but got:\n{}", output_str);
}

