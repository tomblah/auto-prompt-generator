// rust/assemble_prompt/src/lib.rs

use std::error::Error;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Fixed instruction that is always appended.
pub const FIXED_INSTRUCTION: &str = "Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen";

/// Assemble a prompt given a file containing a list of file paths (found_files)
/// and an instruction_content string (currently ignored, but passed for compatibility).
///
/// This function simulates the behavior of your Bash assemble-prompt script.
/// In production it calls external binaries (e.g. for filtering markers, diffing, etc.)
/// but here we allow an override (via the `get_rust_binary` function) so that tests can
/// simulate or bypass those calls.
pub fn assemble_prompt(
    found_files_path: &str,
    _instruction_content: &str,
) -> Result<String, Box<dyn Error>> {
    // Read and deduplicate the list of file paths.
    let file = fs::File::open(found_files_path)?;
    let reader = io::BufReader::new(file);
    let mut file_paths: Vec<String> = reader.lines().filter_map(Result::ok).collect();
    file_paths.sort();
    file_paths.dedup();

    let mut clipboard_content = String::new();

    // For each file, read its content and append it with a header.
    for file_path in file_paths.iter() {
        let path = Path::new(file_path);
        let file_basename = path.file_name()
            .and_then(|s| s.to_str())
            .ok_or("Failed to determine file basename")?
            .to_string();

        // For this example, we simulate the “marker” processing and diff calls.
        // In production these would call external binaries.
        let content = fs::read_to_string(file_path)?;
        clipboard_content.push_str(&format!(
            "\nThe contents of {} is as follows:\n\n{}\n\n",
            file_basename, content
        ));

        // (For simplicity, we are not simulating diff output here.)
        clipboard_content.push_str("\n--------------------------------------------------\n");
    }

    // Append the fixed instruction.
    clipboard_content.push_str("\n\n");
    clipboard_content.push_str(FIXED_INSTRUCTION);

    // Optionally, simulate the unescaping step. (Here we simply return the content.)
    let final_content = clipboard_content;

    Ok(final_content)
}

/// A dummy implementation of get_rust_binary used in production to locate external binaries.
/// In unit tests, you can override or bypass calls to external processes.
#[allow(dead_code)]
pub fn get_rust_binary(binary_name: &str) -> Result<String, Box<dyn Error>> {
    // In production, you might call:
    // let exe_dir = std::env::current_exe()?.parent().ok_or("Cannot get exe dir")?;
    // let candidate = exe_dir.join(binary_name);
    // if candidate.exists() { Ok(candidate.to_string_lossy().into_owned()) } else { Err("Not found".into()) }
    // For testing, we simply return the binary name.
    Ok(binary_name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_assemble_prompt_includes_fixed_instruction() -> Result<(), Box<dyn Error>> {
        // Create a temporary directory to simulate files.
        let dir = tempdir()?;
        let test_file_path = dir.path().join("Test.swift");
        // Write some content to the file.
        let mut file = File::create(&test_file_path)?;
        writeln!(file, "TestClass")?;

        // Create a "found_files" file that lists the path.
        let found_files_path = dir.path().join("found_files.txt");
        let mut ff_file = File::create(&found_files_path)?;
        writeln!(ff_file, "{}", test_file_path.to_str().unwrap())?;

        // Call our assemble_prompt function.
        let assembled = assemble_prompt(found_files_path.to_str().unwrap(), "// TODO: - dummy")?;
        // Check that the fixed instruction is present.
        assert!(assembled.contains(FIXED_INSTRUCTION));
        // Check that the file header and content are present.
        assert!(assembled.contains("The contents of Test.swift is as follows:"));
        assert!(assembled.contains("TestClass"));

        Ok(())
    }

    #[test]
    fn test_assemble_prompt_multiple_files() -> Result<(), Box<dyn Error>> {
        // Create a temporary directory to simulate files.
        let dir = tempdir()?;
        // Create two files.
        let file1 = dir.path().join("A.swift");
        let file2 = dir.path().join("B.swift");
        File::create(&file1)?.write_all(b"ContentA")?;
        File::create(&file2)?.write_all(b"ContentB")?;

        // Create a found_files file listing both.
        let found_files_path = dir.path().join("found_files.txt");
        let mut ff_file = File::create(&found_files_path)?;
        writeln!(ff_file, "{}", file1.to_str().unwrap())?;
        writeln!(ff_file, "{}", file2.to_str().unwrap())?;

        let assembled = assemble_prompt(found_files_path.to_str().unwrap(), "ignored")?;
        // Expect both file contents to be present.
        assert!(assembled.contains("The contents of A.swift is as follows:"));
        assert!(assembled.contains("ContentA"));
        assert!(assembled.contains("The contents of B.swift is as follows:"));
        assert!(assembled.contains("ContentB"));
        // And fixed instruction.
        assert!(assembled.contains(FIXED_INSTRUCTION));
        Ok(())
    }
}
