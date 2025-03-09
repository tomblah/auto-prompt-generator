# Auto Prompt Generator

Transforms your TODO comments into AI-friendly prompts with relevant contextual code snippets.

*(NB: Swift projects only at the moment)*

## Getting Started

1. `make`

2. Add a TODO comment in your code:  
   `// TODO: - your task here` *(include the hyphen)*
   
3. Run `generate_prompt` in your project

4. Paste the generated prompt into your favorite AI.

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

- This tool is still under active development.
- It currently supports **Swift** projects, with only partial support for JavaScript.
- TODO comments must be written in the exact format `// TODO: - ...` (including the hyphen) to prevent inadvertently capturing all your other TODO's.
- The method used to identify code “types” (such as classes, protocols, enums, etc.) is based on a simple heuristic—scanning for capitalized words and then locating their definitions—which may not capture every scenario accurately.
