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
