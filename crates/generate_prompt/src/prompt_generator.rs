// crates/generate_prompt/src/prompt_generator.rs

use anyhow::{Context, Result};
use std::env;
use std::path::{Path, PathBuf};

use extract_instruction_content::extract_instruction_content;
use crate::search_root;
use crate::file_selector;
use assemble_prompt;    // imported as an external crate
use post_processing;    // imported as an external crate

/// Orchestrates the prompt-generation workflow.
///
/// # Parameters
/// - `git_root`: The root directory of the Git repository (as a string slice).
/// - `file_path`: The path to the instruction (TODO) file.
/// - `singular`: If true, only the instruction file is used.
/// - `force_global`: If true, the Git root is used directly as the context.
/// - `include_references`: Whether to include files referencing the enclosing type.
/// - `excludes`: A slice of file basenames to exclude.
/// - `slim_mode`: When true, forces the file to be treated as if it uses substring markers,
///   limiting type extraction to the enclosing function around the TODO marker.
///
/// # Returns
///
/// On success, returns `Ok(())`. On failure, returns an error via `anyhow::Result`.
pub fn generate_prompt(
    git_root: &str,
    file_path: &str,
    singular: bool,
    force_global: bool,
    include_references: bool,
    excludes: &[String],
    slim_mode: bool,
) -> Result<()> {
    // Set the environment variable for the TODO file's basename.
    let todo_file_basename = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();
    env::set_var("TODO_FILE_BASENAME", &todo_file_basename);

    // Check file type compatibility.
    if file_path.ends_with(".js") && !singular {
        eprintln!("WARNING: JavaScript support is beta – enforcing singular mode.");
    }
    if include_references && !file_path.ends_with(".swift") {
        eprintln!("Error: --include-references is only supported for Swift files.");
        std::process::exit(1);
    }

    // Determine package scope.
    let base_dir = if force_global {
        println!("Force global enabled: using Git root for context");
        PathBuf::from(git_root)
    } else {
        PathBuf::from(git_root)
    };

    let search_root_path = if force_global {
        base_dir.clone()
    } else {
        search_root::determine_search_root(&base_dir, file_path)
    };
    println!("Search root: {}", search_root_path.display());

    // Extract the instruction content.
    let instruction_content = extract_instruction_content(file_path)
        .context("Failed to extract instruction content")?;
    println!("Instruction content: {}", instruction_content.trim());
    println!("--------------------------------------------------");

    // Determine the list of files to include.
    let found_files = file_selector::determine_files_to_include(
        file_path,
        singular,
        &search_root_path,
        excludes,
        include_references,
    )?;

    // Assemble the final prompt, passing the slim_mode flag.
    let assembled_prompt = assemble_prompt::assemble_prompt(
        &found_files,
        instruction_content.trim(),
        slim_mode,
    )
    .context("Failed to assemble prompt")?;

    let diff_enabled = env::var("DIFF_WITH_BRANCH").is_ok();

    // Post-process the prompt.
    let final_prompt = post_processing::scrub_extra_todo_markers(
        &assembled_prompt,
        diff_enabled,
        instruction_content.trim(),
    )
    .unwrap_or_else(|err| {
        eprintln!("Error during post-processing: {}", err);
        std::process::exit(1);
    });

    // Validate the marker count.
    crate::prompt_validation::validate_marker_count(&final_prompt, diff_enabled)
        .unwrap_or_else(|err| {
            eprintln!("{}", err);
            std::process::exit(1);
        });

    println!("--------------------------------------------------");
    println!("Success:\n");
    println!("{}", instruction_content.trim());
    if include_references {
        println!("\nWarning: The --include-references option is experimental.");
    }
    println!("--------------------------------------------------\n");
    println!("Prompt has been copied to clipboard.");

    // Copy the final prompt to the clipboard.
    crate::clipboard::copy_to_clipboard(&final_prompt);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use std::path::PathBuf;

    /// Helper to write a file with the given contents.
    fn write_temp_file(dir: &PathBuf, filename: &str, contents: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).expect("Failed to create temp file");
        file.write_all(contents.as_bytes())
            .expect("Failed to write to temp file");
        file_path
    }

    /// Test that generate_prompt succeeds in singular mode with a non‑Swift (e.g. .txt) instruction file.
    #[test]
    fn test_generate_prompt_singular_success() {
        // Create a temporary directory to simulate the Git root.
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        // Create a dummy instruction file with a .txt extension that includes a valid TODO marker.
        let file_content = r#"
// Some initial code
// TODO: - Test instruction
// Some more code
"#;
        let instruction_file = write_temp_file(
            &temp_dir.path().to_path_buf(),
            "instruction.txt",
            file_content,
        );
        let file_path_str = instruction_file.to_str().unwrap();

        // Set an environment variable to disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        // Use singular mode so that only the TODO file is included.
        let singular = true;
        let force_global = false;
        let include_references = false;
        let excludes: Vec<String> = vec![];

        // Pass slim_mode as false.
        let result = generate_prompt(
            git_root,
            file_path_str,
            singular,
            force_global,
            include_references,
            &excludes,
            false
        );
        assert!(result.is_ok(), "Expected generate_prompt to succeed in singular mode");
    }

    /// Test that generate_prompt succeeds in non‑singular mode for a Swift instruction file when include_references is enabled.
    #[test]
    fn test_generate_prompt_include_references_success() {
        // Create a temporary directory to simulate the Git root.
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        // Create a dummy Swift instruction file that includes a type declaration and a valid TODO marker.
        let file_content = r#"
class Dummy {}
// TODO: - Test instruction
"#;
        let instruction_file = write_temp_file(
            &temp_dir.path().to_path_buf(),
            "instruction.swift",
            file_content,
        );
        let file_path_str = instruction_file.to_str().unwrap();

        // Disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        // Non-singular mode with include_references enabled.
        let singular = false;
        let force_global = false;
        let include_references = true;
        let excludes: Vec<String> = vec![];

        let result = generate_prompt(
            git_root,
            file_path_str,
            singular,
            force_global,
            include_references,
            &excludes,
            false
        );
        assert!(result.is_ok(), "Expected generate_prompt to succeed for Swift file with references");
    }

    /// Test that generate_prompt behaves correctly in force global mode.
    #[test]
    fn test_generate_prompt_force_global() {
        // Create a temporary directory to simulate the Git root.
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        // Create a dummy instruction file.
        let file_content = r#"
// Some code
// TODO: - Force global test
"#;
        let instruction_file = write_temp_file(
            &temp_dir.path().to_path_buf(),
            "instruction.txt",
            file_content,
        );
        let file_path_str = instruction_file.to_str().unwrap();

        // Disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        // Use force_global to bypass search_root determination.
        let singular = true;
        let force_global = true;
        let include_references = false;
        let excludes: Vec<String> = vec![];

        let result = generate_prompt(
            git_root,
            file_path_str,
            singular,
            force_global,
            include_references,
            &excludes,
            false
        );
        assert!(result.is_ok(), "Expected generate_prompt to succeed in force global mode");
    }

    /// Test that a JavaScript file triggers the warning but still succeeds.
    #[test]
    fn test_generate_prompt_js_warning() {
        // Create a temporary directory to simulate the Git root.
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        // Create a dummy JavaScript instruction file.
        let file_content = r#"
// Some JS code
// TODO: - JS Test instruction
"#;
        let instruction_file = write_temp_file(
            &temp_dir.path().to_path_buf(),
            "instruction.js",
            file_content,
        );
        let file_path_str = instruction_file.to_str().unwrap();

        // Disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        // Use non-singular mode (this should print a warning but proceed).
        let singular = false;
        let force_global = false;
        let include_references = false;
        let excludes: Vec<String> = vec![];

        let result = generate_prompt(
            git_root,
            file_path_str,
            singular,
            force_global,
            include_references,
            &excludes,
            false
        );
        assert!(result.is_ok(), "Expected generate_prompt to succeed for a JS file (with warning)");
    }

    /// New test: Verify that generate_prompt works in slim mode.
    #[test]
    fn test_generate_prompt_slim_mode_success() {
        // Create a temporary directory to simulate the Git root.
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();

        // Create a dummy Swift instruction file without substring markers
        // so that slim mode forces the use of substring marker filtering.
        let file_content = r#"
func exampleFunction() {
    // Some implementation details.
}
 // TODO: - Perform slim mode test
"#;
        let instruction_file = write_temp_file(
            &temp_dir.path().to_path_buf(),
            "instruction.swift",
            file_content,
        );
        let file_path_str = instruction_file.to_str().unwrap();

        // Disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        // Use singular mode for simplicity.
        let singular = true;
        let force_global = false;
        let include_references = false;
        let excludes: Vec<String> = vec![];

        // Enable slim mode.
        let result = generate_prompt(
            git_root,
            file_path_str,
            singular,
            force_global,
            include_references,
            &excludes,
            true
        );
        assert!(result.is_ok(), "Expected generate_prompt to succeed in slim mode");
    }
}
