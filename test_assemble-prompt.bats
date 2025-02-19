#!/usr/bin/env bats

#
# NB: I'm a bit skeptical of some of the validity of these tests, should really be checked
#

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
  
  # Verify that the TODO marker is not being replaced by the legacy, and remains the same.
  [[ "$output" != *"// TODO: ChatGPT: Do something important"* ]]
  [[ "$output" == *"// TODO: - Do something important"* ]]
  
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

@test "assemble-prompt excludes file exceeding chop limit and includes file not exceeding chop limit" {
  # Create two files: one that will be too long and one that is short.
  file_exceed="$TMP_DIR/LongFile.swift"
  file_include="$TMP_DIR/ShortFile.swift"

  # Create a file with long content. (1000 X's will yield a block length > 1000.)
  printf 'X%.0s' {1..1000} > "$file_exceed"
  # Create a file with short content.
  echo "short content" > "$file_include"

  # Create a temporary file listing the found file paths.
  found_files_file="$TMP_DIR/found_files_chop.txt"
  echo "$file_exceed" > "$found_files_file"
  echo "$file_include" >> "$found_files_file"

  # Set a chop limit that is low enough so that the block for file_exceed will exceed it,
  # yet high enough to include file_include.
  export CHOP_LIMIT=1000
  
  # Run the assemble-prompt function.
  run assemble-prompt "$found_files_file" "ignored instruction"
  [ "$status" -eq 0 ]
  
  # Check that the output includes the excluded files block.
  [[ "$output" == *"The following files were excluded due to the chop limit of ${CHOP_LIMIT} characters:"* ]]
  # Verify that file_exceed appears in the excluded block.
  [[ "$output" == *"LongFile.swift"* ]]
  
  # Extract the "Files (final list):" section from the output.
  final_list=$(echo "$output" | sed -n '/Files (final list):/,$p')
  # Check that the final list includes file_include but not file_exceed.
  [[ "$final_list" == *"ShortFile.swift"* ]]
  [[ "$final_list" != *"LongFile.swift"* ]]
}

@test "assemble-prompt includes all files when none exceed chop limit" {
  # Create two small files.
  file1="$TMP_DIR/SmallFile1.swift"
  file2="$TMP_DIR/SmallFile2.swift"
  echo "content1" > "$file1"
  echo "content2" > "$file2"
  
  # Create a temporary file listing found file paths.
  found_files_file="$TMP_DIR/found_files_no_chop.txt"
  echo "$file1" > "$found_files_file"
  echo "$file2" >> "$found_files_file"

  # Set a very high chop limit so that neither file is excluded.
  export CHOP_LIMIT=100000
  
  # Run the assemble-prompt function.
  run assemble-prompt "$found_files_file" "ignored instruction"
  [ "$status" -eq 0 ]
  
  # Check that the output reports that no files were excluded.
  [[ "$output" == *"No files were excluded due to the chop limit of ${CHOP_LIMIT} characters."* ]]
  
  # And check that the final file list includes both files.
  [[ "$output" == *"Files (final list):"* ]]
  [[ "$output" == *"SmallFile1.swift"* ]]
  [[ "$output" == *"SmallFile2.swift"* ]]
}

@test "assemble-prompt always includes the TODO file regardless of chop limit" {
  # Create a TODO file and another file that is very long.
  todo_file="$TMP_DIR/TodoAlways.swift"
  other_file="$TMP_DIR/Other.swift"

  # Create the TODO file.
  cat <<'EOF' > "$todo_file"
class TodoAlways {
    // TODO: - Always include this file even if chop limit is low!
}
EOF

  # Create another file with very long content.
  printf 'X%.0s' {1..2000} > "$other_file"
  
  # Create a temporary file listing both file paths.
  found_files_file="$TMP_DIR/found_files_todo.txt"
  echo "$todo_file" > "$found_files_file"
  echo "$other_file" >> "$found_files_file"
  
  # Set a chop limit low enough that the long file would normally be excluded.
  export CHOP_LIMIT=500
  
  # Set the TODO_FILE environment variable.
  export TODO_FILE="$todo_file"
  
  # Run the assemble-prompt function.
  run assemble-prompt "$found_files_file" "ignored instruction"
  [ "$status" -eq 0 ]
  
  # Check that the output includes the header for the TODO file.
  [[ "$output" == *"The contents of $(basename "$todo_file") is as follows:"* ]]
  # Check that the TODO file content is present.
  [[ "$output" == *"Always include this file even if chop limit is low!"* ]]
  # And ensure that the other file is excluded from the final list.
  final_list=$(echo "$output" | sed -n '/Files (final list):/,$p')
  [[ "$final_list" != *"$(basename "$other_file")"* ]]
}

@test "assemble-prompt processes related files with higher priority than others" {
  # Create a TODO file and two additional files: one related and one unrelated.
  todo_file="$TMP_DIR/Todo.swift"
  related_file="$TMP_DIR/TodoHelper.swift"
  unrelated_file="$TMP_DIR/Other.swift"

  # Create the TODO file.
  cat <<'EOF' > "$todo_file"
class TodoClass {
    // TODO: - Perform critical operation
}
EOF

  # Create the related file (its basename "TodoHelper" shares "Todo" with the TODO file).
  cat <<'EOF' > "$related_file"
struct TodoHelper {
    // Some helper code.
}
EOF

  # Create an unrelated file with long content to force it to be chopped.
  printf 'X%.0s' {1..1500} > "$unrelated_file"
  
  # Create a temporary file listing all file paths.
  found_files_file="$TMP_DIR/found_files_related.txt"
  echo "$todo_file" > "$found_files_file"
  echo "$related_file" >> "$found_files_file"
  echo "$unrelated_file" >> "$found_files_file"
  
  # Set a chop limit that allows the TODO file and related file, but not the unrelated file.
  export CHOP_LIMIT=1000

  # Set the TODO_FILE environment variable.
  export TODO_FILE="$todo_file"

  # Run the assemble-prompt function.
  run assemble-prompt "$found_files_file" "ignored instruction"
  [ "$status" -eq 0 ]

  # Check that the output includes the header for the TODO file.
  [[ "$output" == *"The contents of $(basename "$todo_file") is as follows:"* ]]
  # Check that the output includes the header for the related file.
  [[ "$output" == *"The contents of $(basename "$related_file") is as follows:"* ]]
  # Extract the "Files (final list):" section.
  final_list=$(echo "$output" | sed -n '/Files (final list):/,$p')
  # Confirm that the final file list includes the TODO and related file, but not the unrelated file.
  [[ "$final_list" == *"$(basename "$todo_file")"* ]]
  [[ "$final_list" == *"$(basename "$related_file")"* ]]
  [[ "$final_list" != *"$(basename "$unrelated_file")"* ]]
}

# --- New tests for first-class file behavior ---

@test "assemble-prompt first-class file immune to chopping" {
  # Create a TODO file that references a specific type.
  todo_file="$TMP_DIR/TodoWithReference.swift"
  cat <<'EOF' > "$todo_file"
class TodoRef {
    // TODO: - Process data using SpecialType
}
EOF

  # Create a file corresponding to the referenced type (SpecialType.swift)
  special_file="$TMP_DIR/SpecialType.swift"
  # Generate long content (which would normally exceed the chop limit)
  printf 'A%.0s' {1..1000} > "$special_file"

  # Create a regular file that is not referenced.
  regular_file="$TMP_DIR/Regular.swift"
  printf 'B%.0s' {1..1000} > "$regular_file"

  # Create a temporary file listing these file paths.
  found_files_file="$TMP_DIR/found_files_firstclass.txt"
  echo "$todo_file" > "$found_files_file"
  echo "$special_file" >> "$found_files_file"
  echo "$regular_file" >> "$found_files_file"

  # Set a chop limit low enough to force chopping.
  export CHOP_LIMIT=500
  export TODO_FILE="$todo_file"

  # Run assemble-prompt with instruction that mentions "SpecialType".
  run assemble-prompt "$found_files_file" "Process data using SpecialType"
  [ "$status" -eq 0 ]

  # Check that the output includes the header for SpecialType.swift.
  [[ "$output" == *"The contents of SpecialType.swift is as follows:"* ]]
  # Extract the final file list.
  final_list=$(echo "$output" | sed -n '/Files (final list):/,$p')
  # Verify that SpecialType.swift is included and Regular.swift is excluded.
  [[ "$final_list" == *"SpecialType.swift"* ]]
  [[ "$final_list" != *"Regular.swift"* ]]
}

@test "assemble-prompt multiple first-class files immune to chopping" {
  # Create a TODO file that mentions two types: TypeA and TypeB.
  todo_file="$TMP_DIR/TodoMultiple.swift"
  cat <<'EOF' > "$todo_file"
class TodoMultiple {
    // TODO: - Execute process using TypeA and TypeB for enhanced functionality
}
EOF

  # Create TypeA and TypeB files with long content.
  type_a="$TMP_DIR/TypeA.swift"
  type_b="$TMP_DIR/TypeB.swift"
  printf 'C%.0s' {1..1000} > "$type_a"
  printf 'D%.0s' {1..1000} > "$type_b"

  # Create a regular file that is not mentioned.
  regular_file="$TMP_DIR/NotReferenced.swift"
  printf 'E%.0s' {1..1000} > "$regular_file"

  # Create a temporary file listing all file paths.
  found_files_file="$TMP_DIR/found_files_multiple.txt"
  echo "$todo_file" > "$found_files_file"
  echo "$type_a" >> "$found_files_file"
  echo "$type_b" >> "$found_files_file"
  echo "$regular_file" >> "$found_files_file"

  # Set a chop limit low enough to force chopping of non-first-class files.
  export CHOP_LIMIT=500
  export TODO_FILE="$todo_file"

  # Run assemble-prompt with instruction that mentions both "TypeA" and "TypeB".
  run assemble-prompt "$found_files_file" "Execute process using TypeA and TypeB for enhanced functionality"
  [ "$status" -eq 0 ]

  # Extract the final file list.
  final_list=$(echo "$output" | sed -n '/Files (final list):/,$p')
  # Verify that both TypeA.swift and TypeB.swift are included.
  [[ "$final_list" == *"TypeA.swift"* ]]
  [[ "$final_list" == *"TypeB.swift"* ]]
  # And that the non-referenced file is excluded.
  [[ "$final_list" != *"NotReferenced.swift"* ]]
}

@test "assemble-prompt does not treat non-primary TODO file as first class" {
  # Create a TODO file with a normal TODO comment (without the dash)
  todo_file="$TMP_DIR/NormalTodo.swift"
  cat <<'EOF' > "$todo_file"
class NormalTodo {
    // TODO: this really should use BrandNewClass
}
EOF

  # Create BrandNewClass.swift with long content (which would normally exceed the chop limit)
  brandnew_file="$TMP_DIR/BrandNewClass.swift"
  printf 'Z%.0s' {1..1000} > "$brandnew_file"

  # Create a temporary file listing both file paths.
  found_files_file="$TMP_DIR/found_files_normal.txt"
  echo "$todo_file" > "$found_files_file"
  echo "$brandnew_file" >> "$found_files_file"

  # Set a chop limit low enough that BrandNewClass.swift would normally be chopped.
  export CHOP_LIMIT=500
  export TODO_FILE="$todo_file"

  # Run assemble-prompt with instruction content that does NOT include the primary marker (no " - ").
  run assemble-prompt "$found_files_file" "this really should use BrandNewClass"
  [ "$status" -eq 0 ]

  # Extract the final file list from the output.
  final_list=$(echo "$output" | sed -n '/Files (final list):/,$p')
  # Expect that BrandNewClass.swift is NOT included in the final list (i.e. it was subject to chopping).
  [[ "$final_list" != *"BrandNewClass.swift"* ]]
}
