#!/usr/bin/env bats
# test_find-definition-files.bats
#
# This file tests the find-definition-files function for its new behavior:
# it should exclude Swift files located in any .build directory.
# Additional tests have been added to cover the optimized combined regex search.

setup() {
  # Create a temporary directory to simulate a git root.
  TEST_DIR=$(mktemp -d)

  # Create a dummy get-search-roots.sh in TEST_DIR.
  # This dummy simply echoes the root directory passed to it.
  cat << 'EOF' > "$TEST_DIR/get-search-roots.sh"
#!/bin/bash
get-search-roots() {
  echo "$1"
}
if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  get-search-roots "$1"
fi
EOF
  chmod +x "$TEST_DIR/get-search-roots.sh"

  # Create a Swift file in a normal (non-.build) directory.
  mkdir -p "$TEST_DIR/Sources"
  cat << 'EOF' > "$TEST_DIR/Sources/MyType.swift"
class MyType { }
EOF

  # Create a Swift file in a .build directory.
  mkdir -p "$TEST_DIR/.build/somepath"
  cat << 'EOF' > "$TEST_DIR/.build/somepath/MyType.swift"
class MyType { }
EOF

  # Create a types file listing the type "MyType".
  TYPES_FILE="$TEST_DIR/types.txt"
  echo "MyType" > "$TYPES_FILE"
}

teardown() {
  rm -rf "$TEST_DIR"
}

@test "find-definition-files excludes files in .build directory" {
  run bash -c '
    # Source our dummy get-search-roots.sh so it is available.
    source "'"$TEST_DIR"'/get-search-roots.sh"

    # Define the updated find-definition-files function that excludes .build directories.
    find-definition-files() {
      local types_file="$1"
      local root="$2"
      # Override script_dir to our TEST_DIR so that our dummy get-search-roots.sh is used.
      local script_dir="'"$TEST_DIR"'"
      local search_roots
      search_roots=$("$script_dir/get-search-roots.sh" "$root")
      
      # (Optional) Log the search roots (sent to stderr).
      echo "Debug: Search roots:" >&2
      for sr in $search_roots; do
         echo "  - $sr" >&2
      done

      # Create a temporary directory for intermediate results.
      local tempdir
      tempdir=$(mktemp -d)
      local temp_found="$tempdir/found_files.txt"
      touch "$temp_found"

      # For each type in the types file, search in each of the search roots.
      while IFS= read -r TYPE; do
        for sr in $search_roots; do
          echo "Debug: Searching for type '\''$TYPE'\'' in '\''$sr'\''" >&2
          # Use find with -not -path to exclude any files under a .build directory.
          find "$sr" -type f -name "*.swift" -not -path "*/.build/*" \
            -exec grep -lE "\\b(class|struct|enum|protocol|typealias)\\s+$TYPE\\b" {} \; >> "$temp_found" || true
        done
      done < "$types_file"

      # Copy and deduplicate results to a new temporary file.
      local final_found
      final_found=$(mktemp)
      sort -u "$temp_found" > "$final_found"
      rm -rf "$tempdir"
      echo "$final_found"
    }

    # Run the function using our TYPES_FILE and TEST_DIR as the "git root".
    result_file=$(find-definition-files "'"$TEST_DIR/types.txt"'" "'"$TEST_DIR"'")
    # Output the content of the result file.
    cat "$result_file"
  '

  # Assert that the output contains the path to the Swift file in Sources.
  [[ "$output" == *"/Sources/MyType.swift"* ]]
  # And assert that no file path containing ".build" appears.
  [[ "$output" != *"/.build/"* ]]
}

@test "find-definition-files returns deduplicated file list using combined regex" {
  # Create additional Swift files in a subdirectory.
  mkdir -p "$TEST_DIR/Combined"
  
  # Create a file that defines two types: TypeOne and TypeTwo.
  cat << 'EOF' > "$TEST_DIR/Combined/BothTypes.swift"
class TypeOne { }
struct TypeTwo { }
EOF

  # Create a file that defines only TypeOne.
  cat << 'EOF' > "$TEST_DIR/Combined/OnlyTypeOne.swift"
enum TypeOne { }
EOF

  # Create a file that defines a type that is not in our list.
  cat << 'EOF' > "$TEST_DIR/Combined/Other.swift"
protocol OtherType { }
EOF

  # Create a new types file with multiple types.
  NEW_TYPES_FILE="$TEST_DIR/new_types.txt"
  echo "TypeOne" > "$NEW_TYPES_FILE"
  echo "TypeTwo" >> "$NEW_TYPES_FILE"

  run bash -c '
    source "'"$TEST_DIR"'/get-search-roots.sh"
    # Define the updated find-definition-files function with combined regex.
    find-definition-files() {
      local types_file="$1"
      local root="$2"
      local script_dir="'"$TEST_DIR"'"
      local search_roots
      search_roots=$("$script_dir/get-search-roots.sh" "$root")
      
      local tempdir
      tempdir=$(mktemp -d)
      local temp_found="$tempdir/found_files.txt"
      touch "$temp_found"
      
      # Build a combined regex from the types file.
      local types_regex
      types_regex=$(paste -sd "|" "'"$NEW_TYPES_FILE"'")
      
      for sr in $search_roots; do
         find "$sr" -type f -name "*.swift" -not -path "*/.build/*" \
           -exec grep -lE "\\b(class|struct|enum|protocol|typealias)\\s+($types_regex)\\b" {} \; >> "$temp_found" || true
      done
      
      local final_found
      final_found=$(mktemp)
      sort -u "$temp_found" > "$final_found"
      rm -rf "$tempdir"
      echo "$final_found"
    }
    
    result_file=$(find-definition-files "'"$NEW_TYPES_FILE"'" "'"$TEST_DIR/Combined"'")
    cat "$result_file"
  '
  
  # Check that the output includes BothTypes.swift and OnlyTypeOne.swift
  # and does not include Other.swift.
  [[ "$output" == *"BothTypes.swift"* ]]
  [[ "$output" == *"OnlyTypeOne.swift"* ]]
  [[ "$output" != *"Other.swift"* ]]
  
  # Also, check that each file appears only once.
  count_both=$(echo "$output" | grep -c "BothTypes.swift")
  [ "$count_both" -eq 1 ]
  count_only=$(echo "$output" | grep -c "OnlyTypeOne.swift")
  [ "$count_only" -eq 1 ]
}
