// crates/generate_prompt/src/prompt_generator.rs

use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};

use crate::file_selector;
use crate::search_root;
use extract_instruction_content::extract_instruction_content;

#[derive(Debug, Clone, Default)]
pub struct GeneratePromptOptions {
    pub singular: bool,
    pub force_global: bool,
    pub include_references: bool,
    pub excludes: Vec<String>,
    pub diff_branch: Option<String>,
    pub targeted: bool,
}

pub fn generate_prompt_with_options(
    git_root: &str,
    file_path: &str,
    options: &GeneratePromptOptions,
) -> Result<()> {
    let todo_file_basename = Path::new(file_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_string();

    // Check file type compatibility.
    if options.include_references && !file_path.ends_with(".swift") {
        return Err(anyhow!(
            "--include-references is only supported for Swift files"
        ));
    }

    // Determine package scope.
    let base_dir = if options.force_global {
        println!("Force global enabled: using Git root for context");
        PathBuf::from(git_root)
    } else {
        PathBuf::from(git_root)
    };

    let search_root_path = if options.force_global {
        base_dir.clone()
    } else {
        search_root::determine_search_root(&base_dir, file_path)
    };
    println!("Search root: {}", search_root_path.display());

    // Extract the instruction content.
    let instruction_content =
        extract_instruction_content(file_path).context("Failed to extract instruction content")?;
    println!("Instruction content: {}", instruction_content.trim());
    println!("--------------------------------------------------");

    // Determine the list of files to include.
    let found_files = file_selector::determine_files_to_include_with_options(
        file_path,
        options.singular,
        &search_root_path,
        &options.excludes,
        &file_selector::FileSelectionOptions {
            include_references: options.include_references,
            targeted: options.targeted,
        },
    )?;

    // Assemble the final prompt.
    let assembly_options = assemble_prompt::AssemblyOptions {
        todo_file_basename: Some(todo_file_basename),
        diff_branch: options.diff_branch.clone(),
    };
    let assembled_prompt =
        assemble_prompt::assemble_prompt_with_options(&found_files, &assembly_options)
            .context("Failed to assemble prompt")?;

    let diff_enabled = options.diff_branch.is_some();

    // Post-process the prompt.
    let final_prompt = post_processing::scrub_extra_todo_markers(
        &assembled_prompt,
        diff_enabled,
        instruction_content.trim(),
    )
    .map_err(|err| anyhow!("Error during post-processing: {err}"))?;

    // Validate the marker count.
    crate::prompt_validation::validate_marker_count(&final_prompt, diff_enabled)
        .map_err(|err| anyhow!("Prompt marker validation failed: {err}"))?;

    println!("--------------------------------------------------");
    println!("Success:\n");
    println!("{}", instruction_content.trim());
    println!("--------------------------------------------------\n");
    println!("Prompt has been copied to clipboard.");

    // Copy the final prompt to the clipboard.
    crate::clipboard::copy_to_clipboard(&final_prompt)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs::{self, File};
    use std::io::Write;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use tempfile::tempdir;

    /// Helper to write a file with the given contents.
    fn write_temp_file(dir: &Path, filename: &str, contents: &str) -> PathBuf {
        let file_path = dir.join(filename);
        let mut file = File::create(&file_path).expect("Failed to create temp file");
        file.write_all(contents.as_bytes())
            .expect("Failed to write to temp file");
        file_path
    }

    #[test]
    #[cfg(unix)]
    fn test_generate_prompt_with_options_targeted_ignores_env() {
        env::remove_var("TARGETED");
        let original_path = env::var("PATH").unwrap_or_default();
        let original_disable_pbcopy = env::var("DISABLE_PBCOPY").ok();

        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();
        let clipboard_file = temp_dir.path().join("clipboard.txt");
        let fake_pbcopy_path = temp_dir.path().join("pbcopy");
        fs::write(
            &fake_pbcopy_path,
            format!("#!/bin/sh\ncat > \"{}\"\n", clipboard_file.display()),
        )
        .expect("Failed to write fake pbcopy");
        let mut perms = fs::metadata(&fake_pbcopy_path)
            .expect("Failed to read fake pbcopy metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_pbcopy_path, perms)
            .expect("Failed to make fake pbcopy executable");

        env::set_var(
            "PATH",
            format!("{}:{}", temp_dir.path().display(), original_path),
        );
        env::remove_var("DISABLE_PBCOPY");

        let instruction_file = write_temp_file(
            temp_dir.path(),
            "Instruction.swift",
            r#"
class OuterType {}
func testFunction() {
    class InnerType {}
    // TODO: - Perform action
}
"#,
        );
        let outer_def_path = write_temp_file(
            temp_dir.path(),
            "OuterDefinition.swift",
            "class OuterType {}\n",
        );
        let inner_def_path = write_temp_file(
            temp_dir.path(),
            "InnerDefinition.swift",
            "class InnerType {}\n",
        );

        let result = generate_prompt_with_options(
            git_root,
            instruction_file.to_str().unwrap(),
            &GeneratePromptOptions {
                singular: false,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: true,
            },
        );

        env::set_var("PATH", original_path);
        if let Some(value) = original_disable_pbcopy {
            env::set_var("DISABLE_PBCOPY", value);
        } else {
            env::remove_var("DISABLE_PBCOPY");
        }

        assert!(
            result.is_ok(),
            "Expected explicit targeted prompt generation"
        );
        let clipboard_content =
            fs::read_to_string(&clipboard_file).expect("Failed to read fake clipboard");
        assert!(clipboard_content
            .contains(inner_def_path.file_name().and_then(|s| s.to_str()).unwrap()));
        assert!(!clipboard_content
            .contains(outer_def_path.file_name().and_then(|s| s.to_str()).unwrap()));
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
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.txt", file_content);
        let file_path_str = instruction_file.to_str().unwrap();

        // Set an environment variable to disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        let result = generate_prompt_with_options(
            git_root,
            file_path_str,
            &GeneratePromptOptions {
                singular: true,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );
        assert!(
            result.is_ok(),
            "Expected generate_prompt to succeed in singular mode"
        );
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
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.swift", file_content);
        let file_path_str = instruction_file.to_str().unwrap();

        // Disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        let result = generate_prompt_with_options(
            git_root,
            file_path_str,
            &GeneratePromptOptions {
                singular: false,
                force_global: false,
                include_references: true,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );
        assert!(
            result.is_ok(),
            "Expected generate_prompt to succeed for Swift file with references"
        );
    }

    /// Regression test for the desired library behavior: unsupported reference lookup should return an error.
    #[test]
    fn test_generate_prompt_include_references_non_swift_returns_error() {
        let temp_dir = tempdir().expect("Failed to create temp dir");
        let git_root = temp_dir.path().to_str().unwrap();
        let file_content = r#"
// Some JS code
// TODO: - JS Test instruction
"#;
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.js", file_content);
        let file_path_str = instruction_file.to_str().unwrap();
        let result = generate_prompt_with_options(
            git_root,
            file_path_str,
            &GeneratePromptOptions {
                singular: false,
                force_global: false,
                include_references: true,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );

        let err = result.expect_err("Expected non-Swift include_references to return an error");
        assert!(
            err.to_string()
                .contains("--include-references is only supported for Swift files"),
            "Unexpected error: {err}"
        );
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
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.txt", file_content);
        let file_path_str = instruction_file.to_str().unwrap();

        // Disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        let result = generate_prompt_with_options(
            git_root,
            file_path_str,
            &GeneratePromptOptions {
                singular: true,
                force_global: true,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );
        assert!(
            result.is_ok(),
            "Expected generate_prompt to succeed in force global mode"
        );
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
        let instruction_file = write_temp_file(temp_dir.path(), "instruction.js", file_content);
        let file_path_str = instruction_file.to_str().unwrap();

        // Disable clipboard copying.
        env::set_var("DISABLE_PBCOPY", "1");

        let result = generate_prompt_with_options(
            git_root,
            file_path_str,
            &GeneratePromptOptions {
                singular: false,
                force_global: false,
                include_references: false,
                excludes: vec![],
                diff_branch: None,
                targeted: false,
            },
        );
        assert!(
            result.is_ok(),
            "Expected generate_prompt to succeed for a JS file (with warning)"
        );
    }
}
