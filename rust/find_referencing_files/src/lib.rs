// rust/find_referencing_files/src/lib.rs

use regex::Regex;
use std::fs;
use std::path::Component;
use walkdir::WalkDir;

/// Searches the given directory (and its subdirectories) for files with allowed
/// extensions that contain the given type name as a whole word.
/// Files inside directories named "Pods" or ".build" are skipped.
///
/// # Arguments
///
/// * `type_name` - The type name to search for (as a whole word).
/// * `search_root` - The root directory to begin the search.
///
/// # Returns
///
/// A `Result` containing a vector of matching file paths as `String` on success,
/// or an error if something goes wrong.
///
/// # Examples
///
/// ```rust
/// use find_referencing_files::find_files_referencing;
///
/// let files = find_files_referencing("MyType", "/path/to/search").unwrap();
/// for file in files {
///     println!("{}", file);
/// }
/// ```
pub fn find_files_referencing(
    type_name: &str,
    search_root: &str,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    // Build a regex that matches the type name as a whole word.
    let pattern = format!(r"\b{}\b", regex::escape(type_name));
    let re = Regex::new(&pattern)?;

    // Allowed file extensions.
    let allowed_extensions = ["swift", "h", "m", "js"];
    let mut matches = Vec::new();

    // Recursively traverse the search_root directory.
    for entry in WalkDir::new(search_root).into_iter().filter_map(Result::ok) {
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();

        // Check if the file has one of the allowed extensions.
        let ext = match path.extension().and_then(|s| s.to_str()) {
            Some(e) => e.to_lowercase(),
            None => continue,
        };
        if !allowed_extensions.contains(&ext.as_str()) {
            continue;
        }

        // Skip files that are in directories named "Pods" or ".build".
        if path.components().any(|comp| match comp {
            Component::Normal(os_str) => {
                let s = os_str.to_string_lossy();
                s == "Pods" || s == ".build"
            }
            _ => false,
        }) {
            continue;
        }

        // Read the file contents.
        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // If the file contains the type name (as a whole word), add its path to the list.
        if re.is_match(&content) {
            matches.push(path.display().to_string());
        }
    }

    Ok(matches)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_find_files_referencing_basic() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        let dir_path = dir.path();

        // Create a file that references the type.
        let file1_path = dir_path.join("match.swift");
        let mut file1 = fs::File::create(&file1_path)?;
        writeln!(file1, "class MySpecialClass {{}}")?;
        writeln!(file1, "let instance = MySpecialClass()")?;

        // Create a file that does not reference the type.
        let file2_path = dir_path.join("nomatch.swift");
        let mut file2 = fs::File::create(&file2_path)?;
        writeln!(file2, "print(\"Nothing here\")")?;

        // Call our function.
        let results = find_files_referencing("MySpecialClass", dir_path.to_str().unwrap())?;
        let results_str: Vec<String> = results;
        let file1_str = file1_path.to_string_lossy().to_string();
        let file2_str = file2_path.to_string_lossy().to_string();

        assert!(results_str.contains(&file1_str));
        assert!(!results_str.contains(&file2_str));

        Ok(())
    }

    #[test]
    fn test_excludes_directories() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        let dir_path = dir.path();

        // Create a subdirectory named "Pods" and a file inside it.
        let pods_dir = dir_path.join("Pods");
        fs::create_dir(&pods_dir)?;
        let file_in_pods = pods_dir.join("match.swift");
        let mut f = fs::File::create(&file_in_pods)?;
        writeln!(f, "class MySpecialClass {{}}")?;
        writeln!(f, "let instance = MySpecialClass()")?;

        // Create a file in the root that references the type.
        let root_file = dir_path.join("match2.swift");
        let mut f2 = fs::File::create(&root_file)?;
        writeln!(f2, "class MySpecialClass {{}}")?;
        writeln!(f2, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path.to_str().unwrap())?;
        let results_str: Vec<String> = results;
        let root_file_str = root_file.to_string_lossy().to_string();
        let file_in_pods_str = file_in_pods.to_string_lossy().to_string();

        // The result should contain the root file but not the file in Pods.
        assert!(results_str.contains(&root_file_str));
        assert!(!results_str.contains(&file_in_pods_str));

        Ok(())
    }

    #[test]
    fn test_allowed_extensions() -> Result<(), Box<dyn std::error::Error>> {
        // Create a temporary directory.
        let dir = tempdir()?;
        let dir_path = dir.path();

        // Create a file with a disallowed extension.
        let file_txt = dir_path.join("file.txt");
        let mut f_txt = fs::File::create(&file_txt)?;
        writeln!(f_txt, "class MySpecialClass {{}}")?;
        writeln!(f_txt, "let instance = MySpecialClass()")?;

        // Create a file with an allowed extension.
        let file_js = dir_path.join("file.js");
        let mut f_js = fs::File::create(&file_js)?;
        writeln!(f_js, "class MySpecialClass {{}}")?;
        writeln!(f_js, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path.to_str().unwrap())?;
        let results_str: Vec<String> = results;
        let file_js_str = file_js.to_string_lossy().to_string();
        let file_txt_str = file_txt.to_string_lossy().to_string();

        // Should include the JS file but not the txt file.
        assert!(results_str.contains(&file_js_str));
        assert!(!results_str.contains(&file_txt_str));

        Ok(())
    }
}
