// crates/generate_prompt/src/main.rs

use clap::{Arg, Command};
use anyhow::Result;
use generate_prompt::{generate_prompt, PromptConfig};
use std::env;
use std::process::{Command as ProcessCommand, Stdio};
use std::io::Write;
use unescape_newlines::unescape_newlines;

fn main() -> Result<()> {
    // Parse command-line arguments.
    let matches = Command::new("generate_prompt")
        .version("0.1.0")
        .about("Generates an AI prompt by delegating to existing Rust libraries and binaries")
        .arg(
            Arg::new("singular")
                .long("singular")
                .help("Only include the TODO file")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("force_global")
                .long("force-global")
                .help("Force global context inclusion")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("include_references")
                .long("include-references")
                .help("Include files that reference the enclosing type")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .arg(
            Arg::new("diff_with")
                .long("diff-with")
                .num_args(1)
                .help("Include diff report against the specified branch"),
        )
        .arg(
            Arg::new("exclude")
                .long("exclude")
                .action(clap::ArgAction::Append)
                .help("Exclude file(s) whose basename match the given name"),
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .help("Enable verbose logging")
                .action(clap::ArgAction::SetTrue)
                .default_value("false"),
        )
        .get_matches();

    let singular = *matches.get_one::<bool>("singular").unwrap();
    let force_global = *matches.get_one::<bool>("force_global").unwrap();
    let include_references = *matches.get_one::<bool>("include_references").unwrap();
    let diff_with = matches.get_one::<String>("diff_with").cloned();
    let excludes: Vec<String> = matches
        .get_many::<String>("exclude")
        .unwrap_or_default()
        .map(|s| s.to_string())
        .collect();
    let verbose = *matches.get_one::<bool>("verbose").unwrap();

    // Build the configuration for prompt generation.
    let config = PromptConfig {
        singular,
        force_global,
        include_references,
        diff_with,
        excludes,
        verbose,
    };

    // Generate the prompt using the library function.
    let prompt = generate_prompt(config)?;

    // Optionally copy the prompt to the clipboard unless DISABLE_PBCOPY is set.
    if env::var("DISABLE_PBCOPY").is_err() {
        let mut pbcopy = ProcessCommand::new("pbcopy")
            .stdin(Stdio::piped())
            .spawn()
            .expect("Error running pbcopy");
        {
            let pb_stdin = pbcopy.stdin.as_mut().expect("Failed to open pbcopy stdin");
            pb_stdin
                .write_all(unescape_newlines(&prompt).as_bytes())
                .expect("Failed to write to pbcopy");
        }
        pbcopy.wait().expect("Failed to wait on pbcopy");
    } else {
        eprintln!("DISABLE_PBCOPY is set; skipping clipboard copy.");
    }

    // Print the prompt to the console.
    println!("--------------------------------------------------");
    println!("Success:\n");
    println!("{}", prompt);
    println!("--------------------------------------------------\n");
    println!("Prompt has been copied to clipboard.");

    Ok(())
}
