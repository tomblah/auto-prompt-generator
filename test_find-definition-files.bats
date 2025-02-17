#!/usr/bin/env bats
# test_find-definition-files.bats
#
# This file tests the find-definition-files function for its new behavior:
# it should exclude Swift files located in any .build directory and now also any in Pods.
# Additional tests have been added to cover the optimized combined regex search.

setup() {
  # Create a temporary directory to simulate a git root.
  TEST_DIR=$(mktemp -d)

  # Create a dummy Rust binary for get_search_roots in TEST_DIR.
  mkdir -p "$TEST_DIR/rust/target/release"
  cat << 'EOF' > "$TEST_DIR/rust/target/release/get_search_roots"
#!/bin/bash
# Dummy Rust binary: simply echo back the root passed to it.
echo "$1"
EOF
  chmod +x "$TEST_DIR/rust/target/release/get_search_roots"

  # (Assume the new Rust binary "find_definition_files" has been built and placed
  # in the appropriate directory. If not, you can build it and copy it there.)
  
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
    find-definition-files() {
      local types_file="$1"
      local root="$2"
      local script_dir="'"$TEST_DIR"'"
      
      local search_roots
      search_roots=$("$script_dir/rust/target/release/get_search_roots" "$root")
      
      echo "Debug: Search roots:" >&2
      for sr in $search_roots; do
         echo "  - $sr" >&2
      done

      local tempdir
      tempdir=$(mktemp -d)
      local temp_found="$tempdir/found_files.txt"
      touch "$temp_found"

      while IFS= read -r TYPE; do
        for sr in $search_roots; do
          echo "Debug: Searching for type '\''$TYPE'\'' in '\''$sr'\''" >&2
          "$script_dir/rust/target/release/find_definition_files" "$types_file" "$sr" >> "$temp_found" || true
        done
      done < "$types_file"

      local final_found
      final_found=$(mktemp)
      sort -u "$temp_found" > "$final_found"
      rm -rf "$tempdir"
      echo "$final_found"
    }

    result_file=$(find-definition-files "'"$TEST_DIR/types.txt"'" "'"$TEST_DIR"'")
    cat "$result_file"
  '
  [[ "$output" == *"/Sources/MyType.swift"* ]]
  [[ "$output" != *"/.build/"* ]]
}

@test "find-definition-files returns deduplicated file list using combined regex" {
  mkdir -p "$TEST_DIR/Combined"
  
  cat << 'EOF' > "$TEST_DIR/Combined/BothTypes.swift"
class TypeOne { }
struct TypeTwo { }
EOF

  cat << 'EOF' > "$TEST_DIR/Combined/OnlyTypeOne.swift"
enum TypeOne { }
EOF

  cat << 'EOF' > "$TEST_DIR/Combined/Other.swift"
protocol OtherType { }
EOF

  NEW_TYPES_FILE="$TEST_DIR/new_types.txt"
  echo "TypeOne" > "$NEW_TYPES_FILE"
  echo "TypeTwo" >> "$NEW_TYPES_FILE"

  run bash -c '
    find-definition-files() {
      local types_file="$1"
      local root="$2"
      local script_dir="'"$TEST_DIR"'"
      
      local search_roots
      search_roots=$("$script_dir/rust/target/release/get_search_roots" "$root")
      
      local tempdir
      tempdir=$(mktemp -d)
      local temp_found="$tempdir/found_files.txt"
      touch "$temp_found"
      
      while IFS= read -r TYPE; do
        for sr in $search_roots; do
          "$script_dir/rust/target/release/find_definition_files" "$types_file" "$sr" >> "$temp_found" || true
        done
      done < "$types_file"
      
      local final_found
      final_found=$(mktemp)
      sort -u "$temp_found" > "$final_found"
      rm -rf "$tempdir"
      echo "$final_found"
    }
    
    result_file=$(find-definition-files "'"$NEW_TYPES_FILE"'" "'"$TEST_DIR/Combined"'")
    cat "$result_file"
  '
  
  [[ "$output" == *"BothTypes.swift"* ]]
  [[ "$output" == *"OnlyTypeOne.swift"* ]]
  [[ "$output" != *"Other.swift"* ]]
  
  count_both=$(echo "$output" | grep -c "BothTypes.swift")
  [ "$count_both" -eq 1 ]
  count_only=$(echo "$output" | grep -c "OnlyTypeOne.swift")
  [ "$count_only" -eq 1 ]
}

@test "find-definition-files excludes files in Pods directory" {
  mkdir -p "$TEST_DIR/Pods"
  cat << 'EOF' > "$TEST_DIR/Pods/PodsType.swift"
class MyType { }
EOF
  mkdir -p "$TEST_DIR/Sources"
  cat << 'EOF' > "$TEST_DIR/Sources/MyType.swift"
class MyType { }
EOF

  TYPES_FILE="$TEST_DIR/types.txt"
  echo "MyType" > "$TYPES_FILE"

  run bash -c '
    find-definition-files() {
      local types_file="$1"
      local root="$2"
      local script_dir="'"$TEST_DIR"'"
      
      local search_roots
      search_roots=$("$script_dir/rust/target/release/get_search_roots" "$root")
      
      local tempdir
      tempdir=$(mktemp -d)
      local temp_found="$tempdir/found_files.txt"
      touch "$temp_found"
      
      while IFS= read -r TYPE; do
        for sr in $search_roots; do
          "$script_dir/rust/target/release/find_definition_files" "$types_file" "$sr" >> "$temp_found" || true
        done
      done < "$types_file"
      
      local final_found
      final_found=$(mktemp)
      sort -u "$temp_found" > "$final_found"
      rm -rf "$tempdir"
      echo "$final_found"
    }
    
    result_file=$(find-definition-files "'"$TYPES_FILE"'" "'"$TEST_DIR"'")
    cat "$result_file"
  '
  
  [[ "$output" == *"/Sources/MyType.swift"* ]]
  [[ "$output" != *"/Pods/"* ]]
}

@test "find-definition-files returns empty when only files in Pods directory exist" {
  rm -rf "$TEST_DIR/Sources"
  rm -rf "$TEST_DIR/.build"
  
  mkdir -p "$TEST_DIR/Pods/SubModule"
  cat << 'EOF' > "$TEST_DIR/Pods/SubModule/MyType.swift"
class MyType { }
EOF

  TYPES_FILE="$TEST_DIR/types.txt"
  echo "MyType" > "$TYPES_FILE"

  run bash -c '
    find-definition-files() {
      local types_file="$1"
      local root="$2"
      local script_dir="'"$TEST_DIR"'"
      
      local search_roots
      search_roots=$("$script_dir/rust/target/release/get_search_roots" "$root")
      
      local tempdir
      tempdir=$(mktemp -d)
      local temp_found="$tempdir/found_files.txt"
      touch "$temp_found"
      
      while IFS= read -r TYPE; do
        for sr in $search_roots; do
          "$script_dir/rust/target/release/find_definition_files" "$types_file" "$sr" >> "$temp_found" || true
        done
      done < "$types_file"
      
      local final_found
      final_found=$(mktemp)
      sort -u "$temp_found" > "$final_found"
      rm -rf "$tempdir"
      echo "$final_found"
    }
    
    result_file=$(find-definition-files "'"$TYPES_FILE"'" "'"$TEST_DIR"'")
    cat "$result_file"
  '
  
  [ -z "$output" ]
}

@test "find-definition-files includes Objective-C header and implementation files" {
  mkdir -p "$TEST_DIR/ObjC"
  echo "class MyType { }" > "$TEST_DIR/ObjC/MyType.h"
  echo "class MyType { }" > "$TEST_DIR/ObjC/MyType.m"
  echo "MyType" > "$TEST_DIR/types.txt"
  
  run bash -c '
    find-definition-files() {
      local types_file="$1"
      local root="$2"
      local script_dir="'"$TEST_DIR"'"
      
      local search_roots
      search_roots=$("$script_dir/rust/target/release/get_search_roots" "$root")
      
      local tempdir
      tempdir=$(mktemp -d)
      local temp_found="$tempdir/found_files.txt"
      touch "$temp_found"
      
      while IFS= read -r TYPE; do
        for sr in $search_roots; do
          "$script_dir/rust/target/release/find_definition_files" "$types_file" "$sr" >> "$temp_found" || true
        done
      done < "$types_file"
      
      local final_found
      final_found=$(mktemp)
      sort -u "$temp_found" > "$final_found"
      rm -rf "$tempdir"
      echo "$final_found"
    }
    
    result_file=$(find-definition-files "'"$TEST_DIR/types.txt"'" "'"$TEST_DIR"'")
    cat "$result_file"
  '
  result="$output"
  [[ "$result" == *"MyType.h"* ]]
  [[ "$result" == *"MyType.m"* ]]
}
