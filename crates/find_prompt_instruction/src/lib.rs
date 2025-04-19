// crates/find_prompt_instruction/src/lib.rs

use std::fs;
use std::io::{self, BufRead};
use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// Searches the given directory (and its subdirectories) for files with allowed
/// extensions that contain the TODO marker. If multiple files are found, returns
/// the one with the most recent modification time. If `verbose` is true, logs
/// extra details.
///
/// Allowed extensions are: `swift`, `h`, `m`, and `js`.
/// The marker searched for is: `// TODO: - `
pub fn find_prompt_instruction_in_dir(search_dir: &str, verbose: bool) -> io::Result<PathBuf> {
    // Internally use the finder struct.
    let finder = PromptInstructionFinder::new(search_dir, verbose);
    finder.find()
}

// === Private Implementation === //

struct PromptInstructionFinder<'a> {
    search_dir: &'a str,
    verbose: bool,
    allowed_extensions: &'static [&'static str],
    todo_marker: &'static str,
}

impl<'a> PromptInstructionFinder<'a> {
    fn new(search_dir: &'a str, verbose: bool) -> Self {
        Self {
            search_dir,
            verbose,
            allowed_extensions: &["swift", "h", "m", "js"],
            todo_marker: "// TODO: - ",
        }
    }

    fn find(&self) -> io::Result<PathBuf> {
        // Collect matching files using iterator combinators.
        let matching_files: Vec<PathBuf> = WalkDir::new(self.search_dir)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.into_path())
            .filter(|path| {
                // Check if the file has an allowed extension.
                path.extension()
                    .and_then(|s| s.to_str())
                    .map(|ext| self.allowed_extensions.contains(&ext))
                    .unwrap_or(false)
            })
            .filter(|path| {
                // Open the file and check if any line contains the TODO marker.
                if let Ok(file) = fs::File::open(path) {
                    let reader = io::BufReader::new(file);
                    reader.lines()
                          .filter_map(Result::ok)
                          .any(|line| line.contains(self.todo_marker))
                } else {
                    false
                }
            })
            .collect();

        if matching_files.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("No files found containing '{}'", self.todo_marker),
            ));
        }

        // Choose the file with the most recent modification time.
        let chosen_file = matching_files
            .iter()
            .max_by(|a, b| {
                let mod_a = fs::metadata(a)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                let mod_b = fs::metadata(b)
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                mod_a.cmp(&mod_b)
            })
            .expect("At least one file exists")
            .clone();

        // Check the chosen file: if it has more than one marker, error out.
        let content = fs::read_to_string(&chosen_file)?;
        let marker_lines: Vec<String> = content
            .lines()
            .filter(|line| line.contains(self.todo_marker))
            .map(|line| line.trim().to_string())
            .collect();
        let marker_count = marker_lines.len();
        if marker_count > 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "Ambiguous TODO marker: file {} contains {} markers:\n{}",
                    chosen_file.display(),
                    marker_count,
                    marker_lines.join("\n")
                ),
            ));
        }

        if self.verbose {
            // Handled by `log::debug!` instead of stderr
            log::debug!("[VERBOSE] {} matching file(s) found.", matching_files.len());
            if matching_files.len() > 1 {
                log::debug!("[VERBOSE] Ignoring the following files:");
                for file in matching_files.iter().filter(|&f| f != &chosen_file) {
                    let basename = file
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or("<unknown>");
                    let todo_line = extract_first_todo_line(file, self.todo_marker)
                        .unwrap_or_else(|| "<no TODO line found>".to_string());
                    log::debug!("  - {}: {}", basename, todo_line.trim());
                    log::debug!("--------------------------------------------------");
                }
                log::debug!("[VERBOSE] Chosen file: {}", chosen_file.display());
            } else {
                log::debug!(
                    "[VERBOSE] Only one matching file found: {}",
                    chosen_file.display()
                );
            }
        }

        Ok(chosen_file)
    }
}

/// Private helper: extracts the first line in the file that contains the given marker.
fn extract_first_todo_line(path: &Path, marker: &str) -> Option<String> {
    if let Ok(file) = fs::File::open(path) {
        let reader = io::BufReader::new(file);
        for line in reader.lines().filter_map(Result::ok) {
            if line.contains(marker) {
                return Some(line);
            }
        }
    }
    None
}
