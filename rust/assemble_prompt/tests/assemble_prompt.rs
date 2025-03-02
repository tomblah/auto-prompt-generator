use assemble_prompt::assemble_prompt;
use tempfile::tempdir;
use std::fs;
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

/// Returns the fixed instruction string appended by the library.
fn fixed_instruction() -> &'static str {
    "Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"
}

#[test]
fn test_formatting_output_with_fixed_instruction() {
    // Create two temporary source files.
    let mut file1 = NamedTempFile::new().expect("Failed to create file1");
    let file1_content = "class MyClass {\n    // TODO: - Do something important\n}\n";
    file1
        .write_all(file1_content.as_bytes())
        .expect("Failed to write file1");
    let file1_path = file1.path().to_owned();

    let mut file2 = NamedTempFile::new().expect("Failed to create file2");
    let file2_content = "struct MyStruct {}\n";
    file2
        .write_all(file2_content.as_bytes())
        .expect("Failed to write file2");
    let file2_path = file2.path().to_owned();

    // Create a temporary found_files list that includes file1, file2 and a duplicate of file1.
    let mut found_files = NamedTempFile::new().expect("Failed to create found_files file");
    writeln!(found_files, "{}", file1_path.display()).unwrap();
    writeln!(found_files, "{}", file2_path.display()).unwrap();
    writeln!(found_files, "{}", file1_path.display()).unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    // Use an arbitrary instruction content (which is now ignored by the library).
    let instruction_content = "This instruction content is ignored.";

    // Call the library function to assemble the prompt.
    let output = assemble_prompt(found_files_path.to_str().unwrap(), instruction_content)
        .expect("assemble_prompt failed");

    // Check that headers for both files are present.
    assert!(output.contains(&format!(
        "The contents of {} is as follows:",
        file1_path.file_name().unwrap().to_string_lossy()
    )));
    assert!(output.contains(&format!(
        "The contents of {} is as follows:",
        file2_path.file_name().unwrap().to_string_lossy()
    )));

    // Check that file contents are included.
    assert!(output.contains("class MyClass {"));
    assert!(output.contains("struct MyStruct {}"));

    // Check that the fixed instruction is appended at the end.
    assert!(output.contains(fixed_instruction()));
}

#[test]
fn test_process_files_with_substring_markers() {
    // Create a temporary Swift file with substring markers.
    let mut marked_file = NamedTempFile::new().expect("Failed to create MarkedFile.swift");
    let marked_content = "\
import Foundation
// v
func secretFunction() {
    print(\"This is inside the markers.\")
}
// ^
func publicFunction() {
    print(\"This is outside the markers.\")
}
";
    marked_file
        .write_all(marked_content.as_bytes())
        .expect("Failed to write marked file");
    let marked_file_path = marked_file.path().to_owned();

    // Create a temporary found_files list listing this file.
    let mut found_files = NamedTempFile::new().expect("Failed to create found_files file");
    writeln!(found_files, "{}", marked_file_path.display()).unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    // Call assemble_prompt.
    let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
        .expect("assemble_prompt failed");

    // Check that the header is present.
    assert!(output.contains(&format!(
        "The contents of {} is as follows:",
        marked_file_path.file_name().unwrap().to_string_lossy()
    )));
    // Check that the filtered content contains the text between markers.
    assert!(output.contains("func secretFunction() {"));
    assert!(output.contains("print(\"This is inside the markers.\")"));
    // Ensure that content outside the markers is not present.
    assert!(!output.contains("func publicFunction() {"));
}

#[test]
fn test_includes_diff_output_when_diff_with_branch_set() {
    // Create a temporary Swift file.
    let mut file_diff = NamedTempFile::new().expect("Failed to create FileDiff.swift");
    let diff_content = "class Dummy {}";
    file_diff
        .write_all(diff_content.as_bytes())
        .expect("Failed to write diff file");
    let file_diff_path = file_diff.path().to_owned();

    // Create a temporary found_files list.
    let mut found_files = NamedTempFile::new().expect("Failed to create found_files file");
    writeln!(found_files, "{}", file_diff_path.display()).unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    // Set DIFF_WITH_BRANCH to activate diff logic.
    env::set_var("DIFF_WITH_BRANCH", "dummy-branch");

    // Create a temporary directory for dummy external commands.
    let dummy_dir = tempdir().expect("Failed to create dummy dir");
    let dummy_path = dummy_dir.path().join("diff_with_branch");
    // Write a dummy shell script that echoes a fixed diff output.
    #[cfg(unix)]
    {
        let script_content = format!(
            "#!/bin/sh\necho \"Dummy diff output for {}\"\n",
            file_diff_path.file_name().unwrap().to_string_lossy()
        );
        fs::write(&dummy_path, script_content).expect("Failed to write dummy diff script");
        // Make the script executable.
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dummy_path).expect("Failed to get metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dummy_path, perms).expect("Failed to set permissions");
    }

    // Prepend the dummy directory to the PATH.
    let original_path = env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dummy_dir.path().display(), original_path);
    env::set_var("PATH", new_path);

    // Call assemble_prompt.
    let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
        .expect("assemble_prompt failed");

    // Check that the output contains the simulated diff output.
    let expected_diff = format!("Dummy diff output for {}", file_diff_path.file_name().unwrap().to_string_lossy());
    assert!(output.contains(&expected_diff));
    assert!(output.contains("against branch dummy-branch"));

    // Cleanup: unset the DIFF_WITH_BRANCH env var.
    env::remove_var("DIFF_WITH_BRANCH");
    // Restore original PATH.
    env::set_var("PATH", original_path);
}

#[test]
fn test_extracts_enclosing_function_context_for_todo_outside_markers() {
    // Create a temporary JS file with content that has some markers and a TODO outside them.
    let mut file_js = NamedTempFile::new().expect("Failed to create TestFile.js");
    let js_content = "\
const someExampleConstant = 42;

// v

const anotherExampleConstant = 99;

// ^

Parse.Cloud.define(\"getDashboardData\", async (request) => {
    
    // TODO: - helllo
    
    var environment = require(\"./environment.js\");
    var _ = getUnderscore();
    
    var currentUserObjectId = request.params.currentUserObjectId;
    var currentUserGlobal;
    var hiddenPeopleGlobal;
    var timeAgoGlobal = new Date(new Date().getTime() - (24 * 60 * 60 * 1000));
    var resultDictionaryGlobal;
    
});
";
    file_js
        .write_all(js_content.as_bytes())
        .expect("Failed to write JS file");
    let file_js_path = file_js.path().to_owned();

    // Set the environment variable so the library recognizes this as the TODO file.
    env::set_var("TODO_FILE_BASENAME", file_js_path.file_name().unwrap().to_string_lossy().to_string());

    // Create a temporary found_files list containing the JS file.
    let mut found_files = NamedTempFile::new().expect("Failed to create found_files file");
    writeln!(found_files, "{}", file_js_path.display()).unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    // Call assemble_prompt.
    let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored instruction")
        .expect("assemble_prompt failed");

    // Check that the header is present.
    assert!(output.contains(&format!(
        "The contents of {} is as follows:",
        file_js_path.file_name().unwrap().to_string_lossy()
    )));
    // Verify that the function declaration is present.
    assert!(output.contains("Parse.Cloud.define(\"getDashboardData\", async (request) => {"));
    // Verify that the TODO comment is included.
    assert!(output.contains("// TODO: - helllo"));
    // Check that extra context (enclosing function context) was appended.
    assert!(output.contains("// Enclosing function context:"));
    // Ensure that code outside the enclosing block is not included.
    assert!(!output.contains("const someExampleConstant = 42;"));

    // Cleanup: remove the TODO_FILE_BASENAME override.
    env::remove_var("TODO_FILE_BASENAME");
}

/// Test that if the found files list includes a non-existent file, the library
/// skips it gracefully while still appending the fixed instruction.
#[test]
fn test_missing_file_in_found_files() {
    let mut found_files = NamedTempFile::new().unwrap();
    writeln!(found_files, "/path/to/nonexistent/file.swift").unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
        .expect("assemble_prompt failed");

    // The output should not contain any header for the missing file but must include the fixed instruction.
    assert!(!output.contains("nonexistent"));
    assert!(output.contains(fixed_instruction()));
}

/// Test that an empty found files list produces an output that mainly consists
/// of the fixed instruction.
#[test]
fn test_empty_found_files_list() {
    // Create an empty file.
    let found_files = NamedTempFile::new().unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
        .expect("assemble_prompt failed");

    // With no files processed, the final prompt should at least include the fixed instruction.
    assert!(output.trim().ends_with(fixed_instruction()));
}

/// Test fallback behavior: when RUST_PROMPT_FILE_PROCESSOR is set to a non-existent command,
/// the library should fall back to processing the file content using its built‑in logic.
#[test]
fn test_fallback_behavior_when_prompt_processor_fails() {
    // Create a temporary file with some content.
    let mut file = NamedTempFile::new().unwrap();
    let content = "struct FallbackTest {}";
    file.write_all(content.as_bytes()).unwrap();
    let file_path = file.path().to_owned();

    // Create a found files list with this file.
    let mut found_files = NamedTempFile::new().unwrap();
    writeln!(found_files, "{}", file_path.display()).unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    // Set a nonexistent external prompt processor command.
    env::set_var("RUST_PROMPT_FILE_PROCESSOR", "nonexistent_command_xyz");
    env::remove_var("DIFF_WITH_BRANCH");

    let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
        .expect("assemble_prompt failed");

    // Fallback should yield the raw file content.
    assert!(output.contains("struct FallbackTest {}"));

    env::remove_var("RUST_PROMPT_FILE_PROCESSOR");
}

/// Test that when DIFF_WITH_BRANCH is set but the external diff command returns an empty string,
/// no diff block is included in the final prompt.
#[test]
fn test_diff_with_branch_no_diff_output() {
    // Create a temporary file with content.
    let mut file = NamedTempFile::new().unwrap();
    let content = "class NoDiff {}";
    file.write_all(content.as_bytes()).unwrap();
    let file_path = file.path().to_owned();

    // Create a found files list.
    let mut found_files = NamedTempFile::new().unwrap();
    writeln!(found_files, "{}", file_path.display()).unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    // Set DIFF_WITH_BRANCH to activate diff logic.
    env::set_var("DIFF_WITH_BRANCH", "dummy-branch");

    // Create a dummy diff script that returns no output.
    let dummy_dir = tempdir().unwrap();
    let dummy_path = dummy_dir.path().join("diff_with_branch");
    #[cfg(unix)]
    {
        let script_content = "#!/bin/sh\necho \"\"\n";
        fs::write(&dummy_path, script_content).expect("Failed to write dummy diff script");
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&dummy_path).expect("Failed to get metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&dummy_path, perms).expect("Failed to set permissions");
    }
    // Prepend dummy_dir to PATH.
    let original_path = env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", dummy_dir.path().display(), original_path);
    env::set_var("PATH", new_path);

    let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
        .expect("assemble_prompt failed");

    // The output should not include any diff block.
    assert!(!output.contains("against branch dummy-branch"));
    assert!(!output.contains("Dummy diff output"));

    env::remove_var("DIFF_WITH_BRANCH");
    env::set_var("PATH", original_path);
}

/// Test how substring marker filtering behaves when the file has an opening marker ("// v")
/// but is missing the closing marker ("// ^"). The expected behavior may be to process until EOF.
#[test]
fn test_missing_closing_marker_in_substring_markers() {
    let mut file = NamedTempFile::new().unwrap();
    let content = "\
import Foundation
// v
func incompleteMarker() {
    print(\"This is inside an unclosed marker.\")
}
func outsideFunction() {
    print(\"This should not be inside the marker.\")
}
";
    file.write_all(content.as_bytes()).unwrap();
    let file_path = file.path().to_owned();

    // Create a found files list.
    let mut found_files = NamedTempFile::new().unwrap();
    writeln!(found_files, "{}", file_path.display()).unwrap();
    let found_files_path = found_files.into_temp_path().keep().unwrap();

    let output = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")
        .expect("assemble_prompt failed");

    // Check that the output includes the block starting at the opening marker.
    assert!(output.contains("print(\"This is inside an unclosed marker.\")"));
    // Depending on your filtering policy, you might adjust whether or not outside content should be included.
    // For this test we just ensure the inner content is present.
}
