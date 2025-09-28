// crates/generate_prompt/src/services.rs

#![allow(dead_code)]
//
// Commit 2: service interfaces for side-effects (not wired yet).
// - Clipboard: copy text to system clipboard
// - DiffProvider: get a file’s diff vs a branch
//
// NOTE: Introduced without behavior changes. In a later commit the CLI
// will construct and inject these instead of env/process-wide side-effects.

use anyhow::{anyhow, Result};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

/// Copy text to the clipboard.
pub trait Clipboard {
    fn copy(&self, text: &str) -> Result<()>;
}

/// macOS clipboard via `pbcopy`.
pub struct MacClipboard;

impl Clipboard for MacClipboard {
    fn copy(&self, text: &str) -> Result<()> {
        let mut child = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .map_err(|e| anyhow!("Error running pbcopy: {}", e))?;

        child
            .stdin
            .take()
            .ok_or_else(|| anyhow!("Failed to open pbcopy stdin"))?
            .write_all(text.as_bytes())
            .map_err(|e| anyhow!("Failed to write to pbcopy: {}", e))?;

        let status = child.wait().map_err(|e| anyhow!("Failed to wait on pbcopy: {}", e))?;
        if !status.success() {
            return Err(anyhow!("pbcopy exited with status {}", status));
        }
        Ok(())
    }
}

/// Provide diffs for a file vs some branch (e.g. "main").
pub trait DiffProvider {
    /// Returns Ok(Some(diff)) if there is a diff, Ok(None) if clean, Err on command failure.
    fn diff_file(&self, file_path: &Path) -> Result<Option<String>>;
}

/// Git-backed diff provider.
pub struct GitDiffProvider {
    pub branch: String,
}

impl DiffProvider for GitDiffProvider {
    fn diff_file(&self, file_path: &Path) -> Result<Option<String>> {
        let file_str = file_path.to_string_lossy().to_string();

        // Ensure file is tracked
        let ls_status = Command::new("git")
            .args(["ls-files", "--error-unmatch", &file_str])
            .stderr(Stdio::null())
            .status()
            .map_err(|e| anyhow!("Error executing git ls-files: {}", e))?;
        if !ls_status.success() {
            return Ok(None);
        }

        // Diff vs branch
        let out = Command::new("git")
            .args(["diff", &self.branch, "--", &file_str])
            .stderr(Stdio::null())
            .output()
            .map_err(|e| anyhow!("Error executing git diff: {}", e))?;

        let trimmed = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if trimmed.is_empty() { Ok(None) } else { Ok(Some(trimmed)) }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::{self, File};
    use std::io::Write as IoWrite;
    use std::path::{Path, PathBuf};
    use std::process::Command;
    use tempfile::TempDir;

    // ---------- helpers ----------
    fn new_git_repo() -> TempDir {
        let td = TempDir::new().expect("tempdir");
        assert!(Command::new("git")
            .arg("-c").arg("init.defaultBranch=main")
            .arg("init")
            .current_dir(td.path())
            .status().expect("git init").success());
        assert!(Command::new("git")
            .args(["config", "user.name", "Testy McTestface"])
            .current_dir(td.path())
            .status().unwrap().success());
        assert!(Command::new("git")
            .args(["config", "user.email", "test@example.com"])
            .current_dir(td.path())
            .status().unwrap().success());
        td
    }

    fn write_file(dir: &Path, rel: &str, contents: &str) -> PathBuf {
        let p = dir.join(rel);
        if let Some(parent) = p.parent() { fs::create_dir_all(parent).unwrap(); }
        let mut f = File::create(&p).unwrap();
        f.write_all(contents.as_bytes()).unwrap();
        p
    }

    fn git(args: &[&str], cwd: &Path) {
        let ok = Command::new("git").args(args).current_dir(cwd).status()
            .expect("spawn git").success();
        assert!(ok, "git {:?} failed in {}", args, cwd.display());
    }

    // ---------- GitDiffProvider paths ----------
    #[test]
    fn diff_none_for_untracked_file() {
        let repo = new_git_repo();
        let file_path = write_file(repo.path(), "src/foo.txt", "hello\n");
        let dp = GitDiffProvider { branch: "main".to_string() };

        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo.path()).unwrap();
        let res = dp.diff_file(&file_path).expect("diff_file ok");
        std::env::set_current_dir(cwd).unwrap();

        assert!(res.is_none());
    }

    #[test]
    fn diff_none_when_clean_vs_branch() {
        let repo = new_git_repo();
        let repo_path = repo.path();

        let file_path = write_file(repo_path, "README.md", "v1\n");
        git(&["add", "."], repo_path);
        git(&["commit", "-m", "init"], repo_path);

        let dp = GitDiffProvider { branch: "main".to_string() };

        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo_path).unwrap();
        let res = dp.diff_file(&file_path).expect("diff_file ok");
        std::env::set_current_dir(cwd).unwrap();

        assert!(res.is_none());
    }

    #[test]
    fn diff_some_when_modified_vs_branch() {
        let repo = new_git_repo();
        let repo_path = repo.path();

        let file_path = write_file(repo_path, "README.md", "v1\n");
        git(&["add", "."], repo_path);
        git(&["commit", "-m", "v1"], repo_path);

        // robust overwrite
        use std::fs::OpenOptions;
        {
            let mut f = OpenOptions::new().create(true).write(true).truncate(true)
                .open(&file_path).unwrap();
            use std::io::Write as _;
            f.write_all(b"v2\n").unwrap();
        }

        let dp = GitDiffProvider { branch: "main".to_string() };

        let cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(repo_path).unwrap();
        let res = dp.diff_file(&file_path).expect("diff_file ok when modified");
        std::env::set_current_dir(cwd).unwrap();

        let diff = res.expect("expected some diff");
        assert!(diff.contains("-v1") || diff.contains("@@"), "unexpected diff:\n{diff}");
    }

    // ---------- MacClipboard paths ----------
    #[test]
    fn mac_clipboard_spawn_error_when_pbcopy_missing() {
        let prev = std::env::var("PATH").ok();
        std::env::set_var("PATH", "");
        let cb = MacClipboard;
        let err = cb.copy("hello").unwrap_err().to_string();
        if let Some(p) = prev { std::env::set_var("PATH", p); } else { std::env::remove_var("PATH"); }
        let e = err.to_lowercase();
        assert!(e.contains("pbcopy") || e.contains("error"));
    }

    #[test]
    fn mac_clipboard_success_with_fake_pbcopy() {
        let td = TempDir::new().unwrap();
        let bin = td.path();
        let out_file = td.path().join("captured.txt");
        let script = bin.join("pbcopy");
        fs::write(&script, r#"#!/bin/sh
set -euo pipefail
cat - > "${FAKE_PBCOPY_OUT}"
exit 0
"#).unwrap();
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", bin.display(), old_path));
        std::env::set_var("FAKE_PBCOPY_OUT", &out_file);

        let cb = MacClipboard;
        cb.copy("hello clipboard").expect("pbcopy ok");

        if old_path.is_empty() { std::env::remove_var("PATH"); } else { std::env::set_var("PATH", old_path); }
        let captured = fs::read_to_string(&out_file).unwrap();
        assert_eq!(captured, "hello clipboard");
    }

    #[test]
    fn mac_clipboard_nonzero_exit_with_fake_pbcopy() {
        let td = TempDir::new().unwrap();
        let bin = td.path();
        let script = bin.join("pbcopy");
        fs::write(&script, r#"#!/bin/sh
set -euo pipefail
cat - >/dev/null || true
exit 1
"#).unwrap();
        #[cfg(unix)] {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script, perms).unwrap();
        }
        let old_path = std::env::var("PATH").unwrap_or_default();
        std::env::set_var("PATH", format!("{}:{}", bin.display(), old_path));

        let cb = MacClipboard;
        let err = cb.copy("text").unwrap_err().to_string();

        if old_path.is_empty() { std::env::remove_var("PATH"); } else { std::env::set_var("PATH", old_path); }
        let e = err.to_lowercase();
        assert!(e.contains("status") || e.contains("failed") || e.contains("pbcopy"));
    }
}
