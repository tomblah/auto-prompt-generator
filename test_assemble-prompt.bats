#!/usr/bin/env bats

setup() {
  # Create a temporary directory for test files.
  TMP_DIR=$(mktemp -d)

  # Override pbcopy so that it does nothing.
  pbcopy() { :; }
}

teardown() {
  rm -rf "$TMP_DIR"
}

# Load the assemble-prompt component. Adjust the path if needed.
load "${BATS_TEST_DIRNAME}/assemble-prompt.sh"

@test "assemble-prompt formats output correctly with fixed instruction" {
  # Create two temporary Swift files.
  file1="$TMP_DIR/File1.swift"
  file2="$TMP_DIR/File2.swift"

  # File1 contains a TODO marker in the old format.
  cat <<'EOF' > "$file1"
class MyClass {
    // TODO: - Do something important
}
EOF

  # File2 contains a simple struct definition.
  cat <<'EOF' > "$file2"
struct MyStruct {}
EOF

  # Create a temporary file listing found file paths (including a duplicate).
  found_files_file="$TMP_DIR/found_files.txt"
  echo "$file1" > "$found_files_file"
  echo "$file2" >> "$found_files_file"
  echo "$file1" >> "$found_files_file"  # duplicate entry to test deduplication.

  # Define a sample instruction content that should be ignored.
  instruction_content="This is the instruction content that will be ignored."

  # Run the assemble-prompt function.
  run assemble-prompt "$found_files_file" "$instruction_content"
  [ "$status" -eq 0 ]

  # Check that the output includes the headers for both files.
  [[ "$output" == *"The contents of File1.swift is as follows:"* ]]
  [[ "$output" == *"The contents of File2.swift is as follows:"* ]]
  
  # Verify that the TODO marker is replaced.
  [[ "$output" == *"// TODO: ChatGPT: Do something important"* ]]

  # Check that the content from each file is present.
  [[ "$output" == *"class MyClass {"* ]]
  [[ "$output" == *"struct MyStruct {}"* ]]
  
  # Confirm that the fixed instruction content is appended at the end.
  fixed_instruction="Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"
  [[ "$output" == *"$fixed_instruction"* ]]
}

@test "assemble-prompt processes files with substring markers correctly" {
  # Create a temporary Swift file that includes substring markers.
  file_with_markers="$TMP_DIR/MarkedFile.swift"
  cat <<'EOF' > "$file_with_markers"
import Foundation
// v
func secretFunction() {
    print("This is inside the markers.")
}
// ^
func publicFunction() {
    print("This is outside the markers.")
}
EOF

  # Create a temporary file listing the file path.
  found_files_file="$TMP_DIR/found_files_markers.txt"
  echo "$file_with_markers" > "$found_files_file"

  # Run the assemble-prompt function.
  run assemble-prompt "$found_files_file" "ignored instruction"
  [ "$status" -eq 0 ]

  # Check that the output includes a header for MarkedFile.swift.
  [[ "$output" == *"The contents of MarkedFile.swift is as follows:"* ]]

  # Check that content between the markers is included.
  [[ "$output" == *"func secretFunction() {"* ]]
  [[ "$output" == *"print(\"This is inside the markers.\")"* ]]
  # And that content outside the markers is NOT included.
  [[ "$output" != *"func publicFunction() {"* ]]
}

@test "assemble-prompt includes diff output when DIFF_WITH_BRANCH is set" {
  # Create a temporary Swift file.
  file1="$TMP_DIR/FileDiff.swift"
  echo "class Dummy {}" > "$file1"

  # Create a temporary file listing the file path.
  found_files_file="$TMP_DIR/found_files_diff.txt"
  echo "$file1" > "$found_files_file"

  # Set DIFF_WITH_BRANCH so that diff logic is activated.
  export DIFF_WITH_BRANCH="dummy-branch"

  # Override get_diff_with_branch to simulate a diff.
  get_diff_with_branch() {
    echo "Dummy diff output for $(basename "$1")"
  }

  # Run assemble-prompt.
  run assemble-prompt "$found_files_file" "ignored"
  [ "$status" -eq 0 ]
  # Check that the output contains the simulated diff output.
  [[ "$output" == *"Dummy diff output for FileDiff.swift"* ]]
  [[ "$output" == *"against branch dummy-branch"* ]]

  unset DIFF_WITH_BRANCH
}

@test "assemble-prompt includes exclusion suggestions when prompt exceeds threshold" {
  # Create three temporary Swift files.
  file_todo="$TMP_DIR/TodoFile.swift"
  file_other1="$TMP_DIR/Other1.swift"
  file_other2="$TMP_DIR/Other2.swift"

  # File with TODO.
  cat <<'EOF' > "$file_todo"
class TodoClass {
    // TODO: - Do something!
}
EOF

  # Other file 1.
  cat <<'EOF' > "$file_other1"
struct Other1 {}
EOF

  # Other file 2.
  cat <<'EOF' > "$file_other2"
struct Other2 {}
EOF

  # Create a temporary file listing found file paths.
  found_files_file="$TMP_DIR/found_files_exclusions.txt"
  echo "$file_todo" > "$found_files_file"
  echo "$file_other1" >> "$found_files_file"
  echo "$file_other2" >> "$found_files_file"

  # Force a low threshold to trigger exclusion suggestions.
  export PROMPT_LENGTH_THRESHOLD=1

  # Set the TODO_FILE environment variable to the TODO file.
  export TODO_FILE="$file_todo"

  # Run the assemble-prompt function.
  run assemble-prompt "$found_files_file" "ignored instruction"
  [ "$status" -eq 0 ]

  # Verify that the output contains the "Suggested exclusions:" block.
  [[ "$output" == *"Suggested exclusions:"* ]]

  # Check that the TODO file is NOT suggested.
  [[ "$output" != *"--exclude $(basename "$file_todo")"* ]]

  # And that the other files are suggested.
  [[ "$output" == *"--exclude $(basename "$file_other1")"* ]]
  [[ "$output" == *"--exclude $(basename "$file_other2")"* ]]
}

@test "assemble-prompt does not include exclusion suggestions when prompt is below threshold" {
  # Create two temporary Swift files.
  file_todo="$TMP_DIR/TodoFile.swift"
  file_other="$TMP_DIR/Other.swift"

  # File with TODO.
  cat <<'EOF' > "$file_todo"
class TodoClass {
    // TODO: - Do something!
}
EOF

  # Other file.
  cat <<'EOF' > "$file_other"
struct Other {}
EOF

  # Create a temporary file listing found file paths.
  found_files_file="$TMP_DIR/found_files_no_exclusions.txt"
  echo "$file_todo" > "$found_files_file"
  echo "$file_other" >> "$found_files_file"

  # Set a high threshold so that the prompt length is below it.
  export PROMPT_LENGTH_THRESHOLD=10000000

  # Set the TODO_FILE environment variable.
  export TODO_FILE="$file_todo"

  # Run the assemble-prompt function.
  run assemble-prompt "$found_files_file" "ignored instruction"
  [ "$status" -eq 0 ]

  # Verify that there is no "Suggested exclusions:" block.
  [[ "$output" != *"Suggested exclusions:"* ]]
}
