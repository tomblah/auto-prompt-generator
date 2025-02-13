#!/usr/bin/env bats

setup() {
  TMP_DIR=$(mktemp -d)
  cd "$TMP_DIR"
  git init -q .
  # Configure Git user info.
  git config user.email "test@example.com"
  git config user.name "Test User"
  echo "initial content" > test.txt
  git add test.txt
  git commit -q -m "Initial commit"
  # Ensure branch is named main.
  git branch -M main

  # Copy diff-with-branch.sh from the test directory to TMP_DIR.
  cp "${BATS_TEST_DIRNAME}/diff-with-branch.sh" .
  chmod +x diff-with-branch.sh
}

teardown() {
  cd /
  rm -rf "$TMP_DIR"
}

@test "get_diff_with_branch returns empty for unmodified file" {
  export DIFF_WITH_BRANCH="main"
  run bash -c "source ./diff-with-branch.sh && get_diff_with_branch test.txt"
  [ "$status" -eq 0 ]
  [ -z "$output" ]
}

@test "get_diff_with_branch returns non-empty diff for modified file" {
  export DIFF_WITH_BRANCH="main"
  # Modify the file.
  echo "new content" >> test.txt
  run bash -c "source ./diff-with-branch.sh && get_diff_with_branch test.txt"
  [ "$status" -eq 0 ]
  [ -n "$output" ]
  [[ "$output" == *"+new content"* ]]
}
