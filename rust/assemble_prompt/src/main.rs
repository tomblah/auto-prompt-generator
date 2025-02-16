use assemble_prompt::assemble_prompt;
use std::env;
use std::process;
use std::io::Write;

fn main() {
    // Expect two arguments: a found_files file and an instruction_content string.
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <found_files> <instruction_content>", args[0]);
        process::exit(1);
    }
    let found_files = &args[1];
    let instruction_content = &args[2];

    match assemble_prompt(found_files, instruction_content) {
        Ok(prompt) => {
            // Copy to clipboard via pbcopy.
            let mut pbcopy = process::Command::new("pbcopy")
                .stdin(process::Stdio::piped())
                .spawn()
                .expect("Failed to spawn pbcopy");
            {
                let stdin = pbcopy.stdin.as_mut().expect("Failed to open pbcopy stdin");
                stdin
                    .write_all(prompt.as_bytes())
                    .expect("Failed to write to pbcopy");
            }
            pbcopy.wait().expect("Failed to wait on pbcopy");
            println!("{}", prompt);
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
