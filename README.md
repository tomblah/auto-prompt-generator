# Auto Prompt Generator

Turns your **TODO**'s into AI-friendly **prompts** by smartly including surrounding **context**.

## Installation

TBD

## How It Works

Copies an AI-friendly prompt to your **clipboard** by:

1. Finding your most recent `// TODO: - `,
2. Identifying all associated classes, structs, enums, etc.,
3. Including these definitions in your prompt for context,
4. Copying the complete prompt to your clipboard.

This ensures your prompt contains **exactly** the information needed for your favourite AI to provide a **relevant answer**.

## Getting Started

TBD

## Example

TBD

## Caveats

- Very much a work in progress
- Currently supports **Swift** only.
- You must write your question in the form // TODO: - (including the hyphen). This prevents the script from accidentally picking up all your TODOs.
- The script uses a simple method to identify “types” (classes, protocols, enums, etc.) by scanning for capitalized words, then searching for their definitions.
