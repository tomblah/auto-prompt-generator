use regex::Regex;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Component, Path, PathBuf};
use tempfile::NamedTempFile;
use walkdir::WalkDir;

/// Mimics the behavior of the get_search_roots binary.
/// If the provided root contains a "Package.swift" file, returns just that root.
/// Otherwise, returns the root (if not named ".build") plus any subdirectories
/// that contain a "Package.swift" (excluding any under a ".build" directory).
fn get_search_roots(root: &Path, verbose: bool) -> io::Result<Vec<PathBuf>> {
    if root.join("Package.swift").is_file() {
        if verbose {
            eprintln!("[VERBOSE] Root {} contains Package.swift", root.display());
        }
        return Ok(vec![root.to_path_buf()]);
    }
    let mut found_dirs = BTreeSet::new();
    if root.file_name().and_then(|s| s.to_str()) != Some(".build") {
        found_dirs.insert(root.to_path_buf());
    }
    for entry in WalkDir::new(root) {
        let entry = entry?;
        if entry.file_type().is_file() && entry.file_name() == "Package.swift" {
            if entry
                .path()
                .components()
                .any(|comp| comp.as_os_str() == ".build")
            {
                continue;
            }
            if let Some(parent) = entry.path().parent() {
                found_dirs.insert(parent.to_path_buf());
            }
        }
    }
    let roots: Vec<PathBuf> = found_dirs.into_iter().collect();
    if verbose {
        let roots_str = roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
            .join(", ");
        eprintln!("[VERBOSE] Search roots: {}", roots_str);
    }
    Ok(roots)
}

fn main() -> io::Result<()> {
    // Expect exactly two arguments: <types_file> and <root>
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <types_file> <root>", args[0]);
        std::process::exit(1);
    }
    let types_file = &args[1];
    let root_str = &args[2];
    let root = Path::new(root_str);

    // Enable verbose logging if the environment variable VERBOSE is set to "true".
    let verbose = env::var("VERBOSE")
        .map(|v| v.to_lowercase() == "true")
        .unwrap_or(false);

    // Read the types file and build a combined regex by joining type names with '|'
    let types_content = fs::read_to_string(types_file)?;
    let type_names: Vec<String> = types_content
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let combined_regex = type_names.join("|");
    if verbose {
        eprintln!("[VERBOSE] Combined regex from {}: {}", types_file, combined_regex);
    }

    // Build the final regex pattern.
    // It will match a word boundary, then one of the keywords,
    // some whitespace, then one of the type names, then a word boundary.
    let pattern = format!(
        r"\b(?:class|struct|enum|protocol|typealias)\s+(?:{})\b",
        combined_regex
    );
    let re = Regex::new(&pattern).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("Error compiling regex '{}': {}", pattern, e),
        )
    })?;
    if verbose {
        eprintln!("[VERBOSE] Final regex pattern: {}", pattern);
    }

    // Determine search roots.
    let search_roots = get_search_roots(root, verbose)?;

    // Define allowed file extensions.
    let allowed_exts = ["swift", "h", "m", "js"];

    // We'll collect matching file paths in a sorted set to deduplicate them.
    let mut result_files = BTreeSet::new();

    // Walk each search root.
    for sr in search_roots {
        if verbose {
            eprintln!("[VERBOSE] Searching in directory: {}", sr.display());
        }
        for entry in WalkDir::new(&sr) {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    if verbose {
                        eprintln!("[VERBOSE] Error reading entry: {}", e);
                    }
                    continue;
                }
            };
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();

            // Check the file extension.
            if let Some(ext) = path.extension().and_then(|s| s.to_str()) {
                if !allowed_exts.contains(&ext) {
                    continue;
                }
            } else {
                continue;
            }

            // Skip files if any path component is ".build" or "Pods".
            let mut skip = false;
            for comp in path.components() {
                if let Component::Normal(os_str) = comp {
                    if let Some(comp_str) = os_str.to_str() {
                        if comp_str == ".build" || comp_str == "Pods" {
                            skip = true;
                            break;
                        }
                    }
                }
            }
            if skip {
                continue;
            }

            // Open the file and search for a matching definition line.
            if let Ok(file) = fs::File::open(path) {
                let reader = BufReader::new(file);
                if reader
                    .lines()
                    .filter_map(Result::ok)
                    .any(|line| re.is_match(&line))
                {
                    result_files.insert(path.to_path_buf());
                    if verbose {
                        eprintln!("[VERBOSE] Found matching definition in: {}", path.display());
                    }
                }
            }
        }
    }

    if verbose {
        eprintln!(
            "[VERBOSE] Total files found (before deduplication): {}",
            result_files.len()
        );
    }

    // Write the sorted unique file paths to a temporary file.
    let mut temp_file = NamedTempFile::new()?;
    for file_path in &result_files {
        writeln!(temp_file, "{}", file_path.display())?;
    }
    // Persist the temporary file (so it isn’t deleted when dropped)
    let temp_path = temp_file.into_temp_path().keep()?;
    if verbose {
        eprintln!(
            "[VERBOSE] Written results to temporary file: {}",
            temp_path.display()
        );
    }

    // Finally, print the path to the temporary file.
    println!("{}", temp_path.display());

    Ok(())
}
