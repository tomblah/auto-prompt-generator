use std::env;
use std::io::Write;
use std::process::{Command, exit, Stdio};
use anyhow::{Context, Result};
use unescape_newlines::unescape_newlines;

// Use the library API from lib.rs.
use assemble_prompt;

fn main() -> Result<()> {
    // Expect exactly two arguments: <found_files_file> and <instruction_content>
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <found_files_file> <instruction_content>", args[0]);
        exit(1);
    }
    let found_files_file = &args[1];
    let instruction_content = &args[2];

    // Call the library function to assemble the prompt.
    let final_prompt = assemble_prompt::assemble_prompt(found_files_file, instruction_content)
        .context("Failed to assemble prompt")?;

    // Copy final prompt to clipboard if DISABLE_PBCOPY is not set.
    if env::var("DISABLE_PBCOPY").is_err() {
        let mut pbcopy = Command::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .unwrap_or_else(|err| {
                eprintln!("Error running pbcopy: {}", err);
                exit(1);
            });
        {
            let pb_stdin = pbcopy.stdin.as_mut().expect("Failed to open pbcopy stdin");
            pb_stdin
                .write_all(unescape_newlines(&final_prompt).as_bytes())
                .expect("Failed to write to pbcopy");
        }
        pbcopy.wait().expect("Failed to wait on pbcopy");
    } else {
        eprintln!("DISABLE_PBCOPY is set; skipping clipboard copy.");
    }

    // Print the final prompt to stdout.
    println!("{}", final_prompt);

    Ok(())
}
