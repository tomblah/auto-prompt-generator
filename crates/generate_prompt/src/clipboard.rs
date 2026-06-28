// crates/generate_prompt/src/clipboard.rs

use anyhow::{anyhow, Context, Result};
use std::io::Write;
use std::process::{Command, Stdio};
use unescape_newlines::unescape_newlines;

/// Copies the provided prompt to the clipboard using the `pbcopy` command.
///
/// This always performs the copy; whether copying should happen at all is decided by the
/// caller at the binary edge (see `main`).
pub fn copy_to_clipboard(final_prompt: &str) -> Result<()> {
    let mut pbcopy = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .context("Error running pbcopy")?;
    {
        let pb_stdin = pbcopy
            .stdin
            .as_mut()
            .context("Failed to open pbcopy stdin")?;
        pb_stdin
            .write_all(unescape_newlines(final_prompt).as_bytes())
            .context("Failed to write to pbcopy")?;
    }

    let status = pbcopy.wait().context("Failed to wait on pbcopy")?;
    if !status.success() {
        return Err(anyhow!("pbcopy exited with status {status}"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Read;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use tempfile::tempdir;

    #[test]
    fn test_copy_to_clipboard_with_fake_pbcopy() {
        // Create a temporary directory that will contain our fake pbcopy command.
        let temp_dir = tempdir().expect("failed to create temp dir");
        let fake_dir_path = temp_dir.path();

        // Create a temporary file path for the fake pbcopy output.
        let output_file_path = fake_dir_path.join("fake_pbcopy_output.txt");
        // Set an environment variable to let our fake pbcopy know where to write.
        env::set_var("FAKE_PBCOPY_OUTPUT", &output_file_path);

        // Create the fake pbcopy script.
        let fake_pbcopy_path = fake_dir_path.join("pbcopy");
        #[cfg(unix)]
        let script_content = "#!/bin/sh\ncat > \"$FAKE_PBCOPY_OUTPUT\"\n";
        #[cfg(windows)]
        let script_content =
            "@echo off\r\nsetlocal\r\nset OUTPUT=%FAKE_PBCOPY_OUTPUT%\r\nmore > %OUTPUT%\r\n";

        fs::write(&fake_pbcopy_path, script_content).expect("failed to write fake pbcopy script");

        // Make the script executable (on Unix platforms).
        #[cfg(unix)]
        {
            let mut perms = fs::metadata(&fake_pbcopy_path)
                .expect("failed to get metadata")
                .permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&fake_pbcopy_path, perms).expect("failed to set permissions");
        }

        // Prepend our temporary directory to the PATH so that our fake pbcopy is found first.
        let original_path = env::var("PATH").unwrap_or_default();
        let new_path = format!("{}:{}", fake_dir_path.display(), original_path);
        env::set_var("PATH", &new_path);

        // Call copy_to_clipboard with a prompt containing an escaped newline.
        // The unescape_newlines function should convert "Test\\nPrompt" to "Test\nPrompt".
        copy_to_clipboard("Test\\nPrompt").expect("clipboard copy should succeed");

        // Read the contents of the file where our fake pbcopy wrote the data.
        let mut output = String::new();
        fs::File::open(&output_file_path)
            .expect("failed to open fake pbcopy output file")
            .read_to_string(&mut output)
            .expect("failed to read fake pbcopy output file");

        // Verify that the output contains an actual newline.
        assert_eq!(output, "Test\nPrompt");

        // Restore the original PATH and clean up the FAKE_PBCOPY_OUTPUT variable.
        env::set_var("PATH", original_path);
        env::remove_var("FAKE_PBCOPY_OUTPUT");
    }

    #[test]
    #[cfg(unix)]
    fn test_copy_to_clipboard_returns_error_when_pbcopy_fails() {
        let temp_dir = tempdir().expect("failed to create temp dir");
        let fake_pbcopy_path = temp_dir.path().join("pbcopy");
        fs::write(&fake_pbcopy_path, "#!/bin/sh\nexit 42\n")
            .expect("failed to write fake pbcopy script");

        let mut perms = fs::metadata(&fake_pbcopy_path)
            .expect("failed to get metadata")
            .permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&fake_pbcopy_path, perms).expect("failed to set permissions");

        let original_path = env::var("PATH").unwrap_or_default();
        env::set_var(
            "PATH",
            format!("{}:{}", temp_dir.path().display(), original_path),
        );

        let err = copy_to_clipboard("Test\\nPrompt")
            .expect_err("expected failed pbcopy status to be returned");
        assert!(
            err.to_string().contains("pbcopy exited with status"),
            "Unexpected error: {err}"
        );

        env::set_var("PATH", original_path);
    }
}
