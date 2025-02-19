#!/usr/bin/env bats

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

@test "find-referencing-files returns files that reference the given type" {
  # Create one file that references the type
  file1="$TMP_DIR/Ref.swift"
  cat <<EOF > "$file1"
import Foundation
let instance = MyType()
EOF

  # Create another file that does not reference the type
  file2="$TMP_DIR/NoRef.swift"
  cat <<EOF > "$file2"
import Foundation
print("Hello World")
EOF

  run bash -c "source ./find-referencing-files.sh; find_referencing_files \"MyType\" \"$TMP_DIR\""
  [ "$status" -eq 0 ]
  referencing_file_list=$(cat "$output")
  [[ "$referencing_file_list" == *"Ref.swift"* ]]
  [[ "$referencing_file_list" != *"NoRef.swift"* ]]
}

@test "find-referencing-files excludes files in Pods and .build directories" {
  # Create a file in a Pods directory that references the type
  mkdir -p "$TMP_DIR/Pods"
  pods_file="$TMP_DIR/Pods/PodsRef.swift"
  cat <<EOF > "$pods_file"
import Foundation
let instance = MyType()
EOF

  # Create a file in a .build directory that references the type
  mkdir -p "$TMP_DIR/.build"
  build_file="$TMP_DIR/.build/BuildRef.swift"
  cat <<EOF > "$build_file"
import Foundation
let instance = MyType()
EOF

  # Create a normal file that references the type
  normal_file="$TMP_DIR/NormalRef.swift"
  cat <<EOF > "$normal_file"
import Foundation
let instance = MyType()
EOF

  run bash -c "source ./find-referencing-files.sh; find_referencing_files \"MyType\" \"$TMP_DIR\""
  [ "$status" -eq 0 ]
  referencing_file_list=$(cat "$output")
  [[ "$referencing_file_list" == *"NormalRef.swift"* ]]
  [[ "$referencing_file_list" != *"PodsRef.swift"* ]]
  [[ "$referencing_file_list" != *"BuildRef.swift"* ]]
}

@test "find-referencing-files returns an empty result if no file references the type" {
  file="$TMP_DIR/NoRef.swift"
  cat <<EOF > "$file"
import Foundation
print("This file does not reference the target type.")
EOF

  run bash -c "source ./find-referencing-files.sh; find_referencing_files \"NonExistentType\" \"$TMP_DIR\""
  [ "$status" -eq 0 ]
  referencing_file_list=$(cat "$output")
  [ -z "$referencing_file_list" ]
}

@test "find-referencing-files includes Objective-C header and implementation files" {
  # Create an Objective-C header file referencing the type.
  objc_header="$TMP_DIR/Ref.h"
  cat <<EOF > "$objc_header"
#import <Foundation/Foundation.h>
@interface MyType : NSObject
@end
EOF

  # Create an Objective-C implementation file referencing the type.
  objc_impl="$TMP_DIR/Ref.m"
  cat <<EOF > "$objc_impl"
#import "Ref.h"
@implementation MyType
@end
EOF

  run bash -c "source ./find-referencing-files.sh; find_referencing_files \"MyType\" \"$TMP_DIR\""
  [ "$status" -eq 0 ]
  referencing_file_list=$(cat "$output")
  [[ "$referencing_file_list" == *"Ref.h"* ]]
  [[ "$referencing_file_list" == *"Ref.m"* ]]

  # Clean up the Objective-C files.
  rm "$objc_header" "$objc_impl"
}
