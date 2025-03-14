// crates/generate_prompt/src/clipboard.rs

use std::process::{Command, Stdio};
use std::io::Write;
use unescape_newlines::unescape_newlines;

/// Copies the provided prompt to the clipboard using the `pbcopy` command.
/// If the environment variable `DISABLE_PBCOPY` is set, the function logs a message and skips copying.
pub fn copy_to_clipboard(final_prompt: &str) {
    if std::env::var("DISABLE_PBCOPY").is_err() {
        let mut pbcopy = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| {
                eprintln!("Error running pbcopy: {}", err);
                std::process::exit(1);
            });
        {
            let pb_stdin = pbcopy.stdin.as_mut().expect("Failed to open pbcopy stdin");
            pb_stdin
                .write_all(unescape_newlines(final_prompt).as_bytes())
                .expect("Failed to write to pbcopy");
        }
        pbcopy.wait().expect("Failed to wait on pbcopy");
    } else {
        eprintln!("DISABLE_PBCOPY is set; skipping clipboard copy.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use std::fs;
    use std::io::Read;
    use tempfile::tempdir;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn test_copy_to_clipboard_disabled() {
        // Set DISABLE_PBCOPY so that the clipboard copy is skipped.
        env::set_var("DISABLE_PBCOPY", "1");

        // Call the function. This branch should not attempt to run pbcopy.
        copy_to_clipboard("Test\\nPrompt");

        // Clean up the environment variable.
        env::remove_var("DISABLE_PBCOPY");
    }

    #[test]
    fn test_copy_to_clipboard_with_fake_pbcopy() {
        // Ensure DISABLE_PBCOPY is not set.
        env::remove_var("DISABLE_PBCOPY");

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
        let script_content = "@echo off\r\nsetlocal\r\nset OUTPUT=%FAKE_PBCOPY_OUTPUT%\r\nmore > %OUTPUT%\r\n";

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
        copy_to_clipboard("Test\\nPrompt");

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
}
