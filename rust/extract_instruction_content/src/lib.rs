use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;
use anyhow::{Context, Result};

/// Reads the given Swift file and returns the first line that contains the TODO marker.
/// The returned string is trimmed of any leading whitespace.
///
/// # Arguments
///
/// * `file_path` - Path to the Swift file.
///
/// # Errors
///
/// Returns an error if the file cannot be opened, read, or if no valid TODO instruction is found.
pub fn extract_instruction_content<P: AsRef<Path>>(file_path: P) -> Result<String> {
    let file_path_ref = file_path.as_ref();
    let file = File::open(file_path_ref)
        .with_context(|| format!("Error opening file {}", file_path_ref.display()))?;
    let reader = BufReader::new(file);
    let marker = "// TODO: - ";

    for line in reader.lines() {
        let line = line.with_context(|| format!("Error reading file {}", file_path_ref.display()))?;
        if line.contains(marker) {
            return Ok(line.trim_start().to_string());
        }
    }

    anyhow::bail!("No valid TODO instruction found in {}", file_path_ref.display());
}
