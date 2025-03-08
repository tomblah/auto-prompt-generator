# Auto Prompt Generator

Bash script that turns your **TODO**'s into AI-friendly **prompts** by smartly including surrounding **context**.

## Getting Started

1. Add a TODO to your Swift project, e.g.: `// TODO: - something` (you must include the hyphen!)

2. `generate_prompt.sh`

3. Paste the generated prompt into your favorite AI.

## CLI Options

The legacy `generate-prompt.sh` script supports several command-line options to control how the prompt is generated. Below is a description of each available option:

- **`--singular`**  
  Only include the Swift file that contains the `// TODO: -` instruction. When used, the script processes solely that file and ignores any additional context files.

- **`--force-global`**  
  Use the entire Git repository as the context for prompt generation, even if the TODO file is within a package. This option overrides package boundaries and forces a global context.

- **`--include-references`**  
  *(Experimental)* Additionally include files that reference the enclosing type of the TODO instruction. This option augments the prompt with extra context by finding files that mention the type (e.g. class, struct, or enum) associated with the TODO. *(Note: Currently supported only for Swift files.)*

- **`--diff-with <branch>`**  
  For each included file that has changes compared to the specified Git branch (e.g. `main` or `develop`), include a diff report in the prompt. This helps highlight what modifications have been made relative to that branch.

- **`--exclude <filename>`**  
  Exclude files from being included in the prompt if their basename exactly matches the provided filename. This option can be specified multiple times to exclude multiple files.

- **`--verbose`**  
  Enable verbose logging to output detailed debugging information about the prompt generation process, such as which files are discovered and how they are processed.

- **`--chop <character_limit>`**  
  Limit the length of the prompt content by truncating it at the specified character limit.

Remember: You must write your question using the exact format `// TODO: - ...` (including the hyphen) so that the script correctly identifies the instruction file.

## How It Works

The legacy `generate_prompt.sh` script automates the creation of an AI-friendly prompt by:

1. **Locating the Instruction:**  
   It scans your project for the Swift file containing your unique TODO instruction (formatted as `// TODO: - ...`).

2. **Extracting Context:**  
   The script extracts the TODO instruction text and gathers related context by searching for type definitions (classes, structs, enums, etc.) in your repository. Depending on the options used, it can also:
   - Include additional files that reference the enclosing type.
   - Append a diff report comparing changes against a specified Git branch.

3. **Assembling the Prompt:**  
   All the collected information is combined into a single prompt, which is then automatically copied to your clipboard for use with your favorite AI tool.


## Caveats

- This script is a work in progress and some features remain experimental.
- It currently supports **Swift** projects only.
- Your TODO instruction must follow the exact format `// TODO: - ...` (including the hyphen) for the script to detect it correctly.
- The method for identifying code “types” (classes, protocols, enums, etc.) is based on heuristics (scanning for capitalized words and their definitions), which may not cover every scenario perfectly.
