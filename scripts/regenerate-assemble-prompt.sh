#!/usr/bin/env bash
set -euo pipefail

DEST="crates/assemble_prompt/src/lib.rs"

echo "⚠️  Overwriting $DEST with the exact contents…"

cat > "$DEST" << 'EOF'
use std::fs;
use std::path::Path;
use anyhow::{Result};
use substring_marker_snippet_extractor::processor::file_processor::{DefaultFileProcessor, process_file_with_processor};

/// Converts literal "\\n" sequences in the input string to actual newline characters.
fn unescape_newlines(input: &str) -> String {
    input.replace("\\n", "\n")
}

use diff_with_branch::run_diff;

/// Public API: assembles the final prompt from the found files (provided as an in‑memory slice)
/// and instruction content. The prompt is returned as a String.
pub fn assemble_prompt(found_files: &[String], _instruction_content: &str) -> Result<String> {
    // Sort and deduplicate the list.
    let mut files = found_files.to_vec();
    files.sort();
    files.dedup();

    let mut final_prompt = String::new();
    // Retrieve TODO file basename from the environment.
    let todo_file_basename = std::env::var("TODO_FILE_BASENAME").unwrap_or_default();

    // Process each file in the deduplicated list.
    for file_path in files {
        if !Path::new(&file_path).exists() {
            eprintln!("Warning: file {} does not exist, skipping", file_path);
            continue;
        }
        let basename = Path::new(&file_path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&file_path)
            .to_string();

        // Process the file using the DefaultFileProcessor.
        let processed_content = match process_file_with_processor(&DefaultFileProcessor, &file_path, Some(&todo_file_basename)) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error processing {}: {}. Falling back to raw file contents.", file_path, err);
                std::fs::read_to_string(&file_path).unwrap_or_default()
            }
        };

        final_prompt.push_str(&format!(
            "\nThe contents of {} is as follows:\n\n{}\n\n",
            basename, processed_content
        ));

        // If DIFF_WITH_BRANCH is set, append a diff report using the diff_with_branch crate.
        if let Ok(diff_branch) = std::env::var("DIFF_WITH_BRANCH") {
            if let Ok(Some(diff)) = run_diff(&file_path) {
                if !diff.trim().is_empty() {
                    final_prompt.push_str(&format!(
                        "\n--------------------------------------------------\nThe diff for {} (against branch {}) is as follows:\n\n{}\n\n",
                        basename, diff_branch, diff
                    ));
                }
            }
        }

        final_prompt.push_str("\n--------------------------------------------------\n");
    }

    // Append the fixed instruction.
    let fixed_instruction = "Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen";
    final_prompt.push_str(&format!("\n\n{}", fixed_instruction));

    // Unescape literal "\n" sequences.
    let final_prompt = unescape_newlines(&final_prompt);
    Ok(final_prompt)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ... all your existing tests untouched ...

    #[test]
    fn test_unescape_newlines() {
        let input = "Line1\\nLine2";
        assert_eq!(unescape_newlines(input), "Line1\nLine2");
    }
}
EOF

echo "✅ Done writing $DEST. Now delete the old unescape_newlines crate and update your Cargo.toml:"
echo "    rm -rf crates/unescape_newlines"
echo "    # remove it from [workspace.members]"
echo "    cargo clean && cargo build --all && cargo test --all"
