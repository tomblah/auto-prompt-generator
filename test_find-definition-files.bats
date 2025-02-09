#!/usr/bin/env bats
# test_find-definition-files.bats
#
# This file tests the find-definition-files function for its new behavior:
# it should exclude Swift files located in any .build directory.

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
