use std::env;
use std::path::{Path, PathBuf};

/// Starting from `start_path`, traverse upward until a directory containing
/// "Package.swift" is found. If found, returns that directory as a `PathBuf`;
/// otherwise returns `None`.
pub fn get_package_root(start_path: &Path) -> Option<PathBuf> {
    // If start_path is a file, use its parent; if it's already a directory, use it.
    let mut current_dir: PathBuf = if start_path.is_dir() {
        start_path.to_path_buf()
    } else {
        start_path.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("/"))
    };

    // Walk upward until we reach the root.
    while current_dir.as_os_str() != "/" {
        if current_dir.join("Package.swift").is_file() {
            return Some(current_dir);
        }
        if let Some(parent) = current_dir.parent() {
            current_dir = parent.to_path_buf();
        } else {
            break;
        }
    }
    None
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <file_path>", args[0]);
        std::process::exit(1);
    }
    let file_path = Path::new(&args[1]);

    if let Some(pkg_root) = get_package_root(file_path) {
        println!("{}", pkg_root.display());
    } else {
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_get_package_root_file_in_package() {
        // Create a temporary directory structure:
        // temp_dir/
        //   Package.swift
        //   src/
        //     File.swift

        let temp_dir = tempdir().unwrap();
        let pkg_dir = temp_dir.path();

        // Create Package.swift in pkg_dir
        fs::File::create(pkg_dir.join("Package.swift")).unwrap();

        // Create a subdirectory "src"
        let src_dir = pkg_dir.join("src");
        fs::create_dir(&src_dir).unwrap();

        // Create a file File.swift in src
        let file_path = src_dir.join("File.swift");
        let mut file = fs::File::create(&file_path).unwrap();
        writeln!(file, "// Some Swift code").unwrap();

        // Call get_package_root with the file path.
        let result = get_package_root(&file_path);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), pkg_dir);
    }

    #[test]
    fn test_get_package_root_directory_in_package() {
        // Create a temporary directory structure:
        // temp_dir/
        //   Package.swift
        //   src/

        let temp_dir = tempdir().unwrap();
        let pkg_dir = temp_dir.path();

        // Create Package.swift in pkg_dir
        fs::File::create(pkg_dir.join("Package.swift")).unwrap();

        // Create a subdirectory "src"
        let src_dir = pkg_dir.join("src");
        fs::create_dir(&src_dir).unwrap();

        // Call get_package_root with the src directory.
        let result = get_package_root(&src_dir);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), pkg_dir);
    }

    #[test]
    fn test_get_package_root_no_package() {
        // Create a temporary directory structure without a Package.swift:
        // temp_dir/
        //   src/
        //     File.swift

        let temp_dir = tempdir().unwrap();
        let dir = temp_dir.path();

        // Create a subdirectory "src" and a file inside it.
        let src_dir = dir.join("src");
        fs::create_dir(&src_dir).unwrap();
        let file_path = src_dir.join("File.swift");
        fs::File::create(&file_path).unwrap();

        // Call get_package_root with the file path; expect None.
        let result = get_package_root(&file_path);
        assert!(result.is_none());
    }
}
