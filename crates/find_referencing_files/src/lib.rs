// crates/find_referencing_files/src/lib.rs

use anyhow::Result;
use lang_support::{walk_source_files, SourceFile};
use regex::Regex;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

/// Filters a pre-walked set of source files for those whose content contains
/// `type_name` as a whole word. Use when the caller has already materialised
/// the source collection and wants to avoid a redundant filesystem walk.
pub fn find_files_referencing_from_sources(
    type_name: &str,
    sources: &[SourceFile],
) -> Result<BTreeSet<PathBuf>> {
    let pattern = format!(r"\b{}\b", regex::escape(type_name));
    let re = Regex::new(&pattern)?;

    Ok(sources
        .iter()
        .filter(|sf| re.is_match(&sf.content))
        .map(|sf| sf.path.clone())
        .collect())
}

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
/// A `Result` containing a vector of matching file paths on success,
/// or an error if something goes wrong.
///
/// # Examples
///
/// ```rust
/// use find_referencing_files::find_files_referencing;
/// use std::path::Path;
///
/// let files = find_files_referencing("MyType", Path::new("/path/to/search")).unwrap();
/// for file in files {
///     println!("{}", file.display());
/// }
/// ```
pub fn find_files_referencing(type_name: &str, search_root: &Path) -> Result<Vec<PathBuf>> {
    let pattern = format!(r"\b{}\b", regex::escape(type_name));
    let re = Regex::new(&pattern)?;

    let mut matches = Vec::new();

    for source_file in walk_source_files(search_root) {
        if re.is_match(&source_file.content) {
            matches.push(source_file.path);
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
    fn test_find_files_referencing_basic() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let file1_path = dir_path.join("match.swift");
        let mut file1 = fs::File::create(&file1_path)?;
        writeln!(file1, "class MySpecialClass {{}}")?;
        writeln!(file1, "let instance = MySpecialClass()")?;

        let file2_path = dir_path.join("nomatch.swift");
        let mut file2 = fs::File::create(&file2_path)?;
        writeln!(file2, "print(\"Nothing here\")")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        assert!(results.contains(&file1_path));
        assert!(!results.contains(&file2_path));

        Ok(())
    }

    #[test]
    fn test_excludes_directories() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let pods_dir = dir_path.join("Pods");
        fs::create_dir(&pods_dir)?;
        let file_in_pods = pods_dir.join("match.swift");
        let mut f = fs::File::create(&file_in_pods)?;
        writeln!(f, "class MySpecialClass {{}}")?;
        writeln!(f, "let instance = MySpecialClass()")?;

        let root_file = dir_path.join("match2.swift");
        let mut f2 = fs::File::create(&root_file)?;
        writeln!(f2, "class MySpecialClass {{}}")?;
        writeln!(f2, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        assert!(results.contains(&root_file));
        assert!(!results.contains(&file_in_pods));

        Ok(())
    }

    #[test]
    fn test_allowed_extensions() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let file_txt = dir_path.join("file.txt");
        let mut f_txt = fs::File::create(&file_txt)?;
        writeln!(f_txt, "class MySpecialClass {{}}")?;
        writeln!(f_txt, "let instance = MySpecialClass()")?;

        let file_js = dir_path.join("file.js");
        let mut f_js = fs::File::create(&file_js)?;
        writeln!(f_js, "class MySpecialClass {{}}")?;
        writeln!(f_js, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        assert!(results.contains(&file_js));
        assert!(!results.contains(&file_txt));

        Ok(())
    }

    #[test]
    fn test_supported_language_extensions_are_searched() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let supported_files = [
            dir_path.join("reference.swift"),
            dir_path.join("reference.h"),
            dir_path.join("reference.m"),
            dir_path.join("reference.js"),
            dir_path.join("reference.jsx"),
            dir_path.join("reference.mjs"),
            dir_path.join("reference.cjs"),
        ];
        for path in &supported_files {
            let mut file = fs::File::create(path)?;
            writeln!(file, "let instance = MySpecialClass()")?;
        }

        let unsupported_path = dir_path.join("reference.txt");
        let mut unsupported_file = fs::File::create(&unsupported_path)?;
        writeln!(unsupported_file, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        for path in &supported_files {
            assert!(results.contains(path));
        }
        assert!(!results.contains(&unsupported_path));

        Ok(())
    }

    #[test]
    fn test_excludes_build_directory() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let build_dir = dir_path.join(".build");
        fs::create_dir(&build_dir)?;
        let file_in_build = build_dir.join("match.swift");
        let mut f = fs::File::create(&file_in_build)?;
        writeln!(f, "class MySpecialClass {{}}")?;
        writeln!(f, "let instance = MySpecialClass()")?;

        let root_file = dir_path.join("match2.swift");
        let mut f2 = fs::File::create(&root_file)?;
        writeln!(f2, "class MySpecialClass {{}}")?;
        writeln!(f2, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        assert!(results.contains(&root_file));
        assert!(!results.contains(&file_in_build));

        Ok(())
    }

    #[test]
    fn test_whole_word_matching() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let file_partial = dir_path.join("partial.swift");
        let mut f_partial = fs::File::create(&file_partial)?;
        writeln!(f_partial, "class MySpecialClassExtra {{}}")?;
        writeln!(f_partial, "let instance = MySpecialClassExtra()")?;

        let file_exact = dir_path.join("exact.swift");
        let mut f_exact = fs::File::create(&file_exact)?;
        writeln!(f_exact, "class MySpecialClass {{}}")?;
        writeln!(f_exact, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        assert!(results.contains(&file_exact));
        assert!(!results.contains(&file_partial));

        Ok(())
    }

    #[test]
    fn test_case_insensitive_extension() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let file_upper = dir_path.join("upper.SWIFT");
        let mut f_upper = fs::File::create(&file_upper)?;
        writeln!(f_upper, "class MySpecialClass {{}}")?;
        writeln!(f_upper, "let instance = MySpecialClass()")?;

        let file_mixed = dir_path.join("mixed.Js");
        let mut f_mixed = fs::File::create(&file_mixed)?;
        writeln!(f_mixed, "class MySpecialClass {{}}")?;
        writeln!(f_mixed, "let instance = MySpecialClass()")?;

        let file_lower = dir_path.join("lower.swift");
        let mut f_lower = fs::File::create(&file_lower)?;
        writeln!(f_lower, "class MySpecialClass {{}}")?;
        writeln!(f_lower, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        assert!(results.contains(&file_upper));
        assert!(results.contains(&file_mixed));
        assert!(results.contains(&file_lower));

        Ok(())
    }

    #[test]
    fn test_file_with_missing_extension() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let dir_path = dir.path();

        let file_no_ext = dir_path.join("no_extension");
        let mut f_no_ext = fs::File::create(&file_no_ext)?;
        writeln!(f_no_ext, "class MySpecialClass {{}}")?;
        writeln!(f_no_ext, "let instance = MySpecialClass()")?;

        let file_allowed = dir_path.join("allowed.swift");
        let mut f_allowed = fs::File::create(&file_allowed)?;
        writeln!(f_allowed, "class MySpecialClass {{}}")?;
        writeln!(f_allowed, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        assert!(results.contains(&file_allowed));
        assert!(!results.contains(&file_no_ext));

        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn test_unreadable_file() -> anyhow::Result<()> {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempdir()?;
        let dir_path = dir.path();

        let file_unreadable = dir_path.join("unreadable.swift");
        let mut f = fs::File::create(&file_unreadable)?;
        writeln!(f, "class MySpecialClass {{}}")?;
        writeln!(f, "let instance = MySpecialClass()")?;

        let mut perms = fs::metadata(&file_unreadable)?.permissions();
        perms.set_mode(0o000);
        fs::set_permissions(&file_unreadable, perms)?;

        let file_normal = dir_path.join("normal.swift");
        let mut f2 = fs::File::create(&file_normal)?;
        writeln!(f2, "class MySpecialClass {{}}")?;
        writeln!(f2, "let instance = MySpecialClass()")?;

        let results = find_files_referencing("MySpecialClass", dir_path)?;

        assert!(results.contains(&file_normal));
        assert!(!results.contains(&file_unreadable));

        let mut perms = fs::metadata(&file_unreadable)?.permissions();
        perms.set_mode(0o644);
        fs::set_permissions(&file_unreadable, perms)?;

        Ok(())
    }
}

#[cfg(test)]
mod pathbuf_characterization_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use std::path::PathBuf;
    use tempfile::tempdir;

    /// Characterizes that returned paths match the original `PathBuf` values
    /// used to create the temp files.
    #[test]
    fn test_returned_paths_match_original_pathbufs() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let file_path = dir.path().join("Roundtrip.swift");
        let mut f = fs::File::create(&file_path)?;
        writeln!(f, "class RoundtripType {{}}")?;

        let results = find_files_referencing("RoundtripType", dir.path())?;
        assert_eq!(results.len(), 1);
        assert_eq!(results[0], file_path);

        Ok(())
    }

    /// Characterizes that multiple matching files are all returned with paths
    /// matching the originals.
    #[test]
    fn test_multiple_results_match_original_pathbufs() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let paths: Vec<PathBuf> =
            vec![dir.path().join("First.swift"), dir.path().join("Second.js")];
        for p in &paths {
            let mut f = fs::File::create(p)?;
            writeln!(f, "let x = SharedType()")?;
        }

        let results = find_files_referencing("SharedType", dir.path())?;

        for p in &paths {
            assert!(results.contains(p), "Expected {:?} in results", p);
        }

        Ok(())
    }
}

#[cfg(test)]
mod from_sources_tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_from_sources_filters_references() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let match_path = dir.path().join("match.swift");
        let mut f = fs::File::create(&match_path)?;
        writeln!(f, "let x = Widget()")?;

        let nomatch_path = dir.path().join("nomatch.swift");
        let mut f2 = fs::File::create(&nomatch_path)?;
        writeln!(f2, "let y = 42")?;

        let sources = walk_source_files(dir.path());
        let results = find_files_referencing_from_sources("Widget", &sources)?;

        assert!(results.contains(&match_path));
        assert!(!results.contains(&nomatch_path));
        Ok(())
    }

    #[test]
    fn test_from_sources_returns_btreeset() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let path = dir.path().join("ref.swift");
        let mut f = fs::File::create(&path)?;
        writeln!(f, "let x = Gadget()")?;

        let sources = walk_source_files(dir.path());
        let results = find_files_referencing_from_sources("Gadget", &sources)?;

        assert_eq!(results.len(), 1);
        assert!(results.contains(&path));
        Ok(())
    }
}
