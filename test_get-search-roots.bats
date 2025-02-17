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
  result="$(./rust/target/release/get_search_roots "$TMPDIR")"
  
  # The output should contain the main repo (TMPDIR)
  [[ "$result" == *"$TMPDIR"* ]]
  
  # The output should contain the SubPackage directory.
  [[ "$result" == *"$TMPDIR/SubPackage"* ]]
  
  # The output should NOT include the NonPackage directory.
  if echo "$result" | grep -q "$TMPDIR/NonPackage"; then
    fail "NonPackage directory should not be included in the search roots."
  fi
}

@test "get-search-roots returns only the package root when given a package root" {
  # Create a temporary directory for a package root.
  pkgRoot="$(mktemp -d)"
  touch "$pkgRoot/Package.swift"
  mkdir -p "$pkgRoot/SubPackage"
  touch "$pkgRoot/SubPackage/Package.swift"

  result="$(./rust/target/release/get_search_roots "$pkgRoot")"
  # Since the provided directory is already a package, we expect only that directory to be returned.
  [ "$result" = "$pkgRoot" ]
  
  rm -rf "$pkgRoot"
}

@test "get-search-roots excludes directories under .build" {
  # Create a .build directory with a Package.swift file in it.
  mkdir -p "$TMPDIR/.build/ThirdParty"
  touch "$TMPDIR/.build/ThirdParty/Package.swift"
  
  result="$(./rust/target/release/get_search_roots "$TMPDIR")"
  
  # The result should not include any paths with .build in them.
  if echo "$result" | grep -q "/.build/"; then
    fail ".build directories should not be included in the search roots."
  fi
}

@test "get-search-roots does not return the .build directory itself" {
  # Create a .build directory in TMPDIR.
  mkdir -p "$TMPDIR/.build"
  
  result="$(./rust/target/release/get_search_roots "$TMPDIR/.build")"
  [ -z "$result" ]
}
