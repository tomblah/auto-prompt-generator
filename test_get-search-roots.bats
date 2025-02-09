#!/usr/bin/env bats

setup() {
  # Create a temporary directory to simulate a repository.
  TMPDIR=$(mktemp -d)
  
  # Create a Package.swift file in the repo root.
  touch "$TMPDIR/Package.swift"
  
  # Create a subdirectory that is also a Swift package.
  mkdir -p "$TMPDIR/SubPackage"
  touch "$TMPDIR/SubPackage/Package.swift"
  
  # Create another subdirectory that is not a package.
  mkdir -p "$TMPDIR/NonPackage"
  echo "just some text" > "$TMPDIR/NonPackage/somefile.txt"
}

teardown() {
  rm -rf "$TMPDIR"
}

@test "get-search-roots returns both the main repo and subpackage directories" {
  result="$(bash ./get-search-roots.sh "$TMPDIR")"
  
  # The output should contain the main repo (TMPDIR)
  [[ "$result" == *"$TMPDIR"* ]]
  
  # The output should contain the SubPackage directory.
  [[ "$result" == *"$TMPDIR/SubPackage"* ]]
  
  # The output should NOT include the NonPackage directory.
  if echo "$result" | grep -q "$TMPDIR/NonPackage"; then
    fail "NonPackage directory should not be included in the search roots."
  fi
}
