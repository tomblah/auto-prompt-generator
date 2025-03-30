# Auto Prompt Generator

Transforms your TODO comments into AI-friendly prompts with relevant contextual code snippets.

*(NB: Swift projects only at the moment)*

## Getting Started (AI Assisted)

1. **Clone the Repository:**  
   Clone the repo to your local machine using:
   ```bash
   git clone https://github.com/tomblah/auto-prompt-generator
   ```

2. **Run the Onboarding Script:**  
   Navigate into the repository directory and run the provided script:
   ```bash
   ./getting-started.sh
   ```

3. **Let AI take over:**  
   Paste the generated "getting started" prompt into your favorite AI tool. The AI will then instruct you from there.

## Getting Started (Traditional)

If you prefer a more hands‑on approach, follow these five steps:

1. **Install Rust Toolchain:**  
   Ensure you have Rust and Cargo installed. You can install them from [rustup.rs](https://rustup.rs). For example:
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```  
   This will set up the Rust toolchain and Cargo on your system.

2. **Clone & Setup Repository:**  
   Clone the repository to your machine:
   ```bash
   git clone https://github.com/tomblah/auto-prompt-generator
   cd auto-prompt-generator
   ```  
   Once inside, perform any initial setup required (such as installing dependencies via Cargo).

3. **Build the Project:**  
   Build all Rust components in release mode by running:
   ```bash
   make build
   ```  
   This compiles the project and generates the required binaries.

4. **Add to PATH:**  
   Locate the `generate_prompt` binary (typically found in `target/release`) and add its directory to your PATH. For example:
   ```bash
   export PATH=$(pwd)/target/release:$PATH
   ```  
   You may add this line to your shell’s startup file (like `.bashrc` or `.zshrc`) for convenience.

5. **Play Around:**  
   Insert a `// TODO: -` comment into one of your Swift files and run the prompt generator:
   ```bash
   generate_prompt
   ```  
   Then copy the generated prompt and paste it into your favorite AI tool. This process shows how the auto prompt generator converts your TODO into a detailed prompt, which the AI can then turn into a helpful answer.
   
## Legacy

Some workplaces won't allow you to build this into a binary. Therefore, use the legacy shell scripts which kinda do the same thing: https://github.com/tomblah/auto-prompt-generator/tree/legacy

## CLI Options

The `generate_prompt` tool accepts several command-line options to customize how the AI-friendly prompt is generated. Below is a description of each:

- **`--singular`**  
  Only include the file containing the TODO marker in the generated prompt. In singular mode, the tool ignores any additional context files and uses only the TODO file itself.

- **`--exclude <pattern>`**  
  Exclude files from being included in the prompt if their basename exactly matches the given pattern. This option can be provided multiple times to specify multiple exclusion patterns.
  
- **`--diff-with <branch>`**  
  Append a diff report to the generated prompt by comparing the current working copy against the specified Git branch. If no branch is provided, the tool defaults to using `main`. This diff helps show what changes have been made relative to that branch.

- **`--include-references`**  
  *(Experimental)* Append additional files that reference the enclosing type of the TODO marker. This option scans for files that mention the type (class, struct, enum, etc.) enclosing the TODO. Note that this is currently supported only for Swift files.
  
- **`--force-global`**  
  Force the inclusion of global context by using the Git repository root as the base for searching context files. This option overrides the default behavior of limiting the search to a package scope (e.g. based on a `Package.swift` file).

- **`--verbose`**  
  Enable verbose logging, which outputs additional details about the prompt generation process (such as which files were found and how they were processed).

- **`--tgtd`**  
  Only consider types from the enclosing block for extraction. This option limits the context gathered to the block immediately surrounding the TODO marker, ensuring that only the most relevant types are included.


## How It Works

The Auto Prompt Generator automates the transformation of your TODO comments into a comprehensive, AI-friendly prompt by gathering both your instruction and all the relevant context from your project. Here’s an overview of the process:

1. **Locate Your TODO Instruction:**  
   The tool scans your project to find the most recent line starting with `// TODO: - `. It extracts this instruction—your question or task—as the central piece of the prompt.

2. **Determine the Context Scope:**  
   Using Git, the generator identifies the root of your repository and looks for Swift package directories (by detecting files like `Package.swift`). This helps it decide the search boundaries for gathering additional context.

3. **Collect Associated Code Snippets:**  
   The tool then examines your project for type definitions (classes, structs, enums, protocols, etc.) that are relevant to your TODO. It extracts and compiles the content from files that define these types, ensuring that your prompt includes the code context needed to understand the issue.  
   - Optionally, with the `--include-references` flag, it can also add files that reference these types.
   - If you specify the `--diff-with <branch>` option, it appends a diff report showing changes relative to that Git branch.

4. **Assemble the Final Prompt:**  
   All the extracted information—the TODO instruction, code definitions, any referencing files, and diff output if applicable—is combined into a single prompt. A fixed instruction is added at the end to direct the AI to focus solely on the marked TODO.

5. **Copy to Clipboard:**  
   The final prompt is formatted (including unescaping literal newline sequences) and automatically copied to your clipboard, ready for you to paste into your favorite AI tool.

This step-by-step process ensures that your prompt is both concise and rich with the exact context required for the AI to provide a relevant and precise answer.


## Caveats

- This was developed almost entirely by AI. As such, there are many uncanny patterns that need to be fixed.
- It currently supports **Swift** projects, with only partial support for JavaScript and Objective-C.
- TODO comments must be written in the exact format `// TODO: - ...` (including the hyphen) to prevent inadvertently capturing all your other TODO's.
- The method used to identify code “types” (such as classes, protocols, enums, etc.) is based on a simple heuristic—scanning for capitalized words and then locating their definitions—which may not capture every scenario accurately.
