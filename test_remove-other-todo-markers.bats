#!/usr/bin/env bats

# Load the helper function from remove-other-todo-markers.sh.
# Adjust the relative path if necessary.
setup() {
  source "$(dirname "$BATS_TEST_FILENAME")/remove-other-todo-markers.sh"
}

@test "returns unchanged content when no special TODO markers are present" {
  input="This is a test.
No special markers here."
  run remove_other_todo_markers "$input"
  [ "$status" -eq 0 ]
  [ "$output" = "$input" ]
}

@test "removes a single special TODO marker line" {
  input="Line before.
    // TODO: - Remove this marker.
Line after."
  expected="Line before.
Line after."
  run remove_other_todo_markers "$input"
  [ "$status" -eq 0 ]
  [ "$output" = "$expected" ]
}

@test "removes multiple special TODO marker lines" {
  input="Start.
// TODO: - First marker.
Middle.
// TODO: - Second marker.
End."
  expected="Start.
Middle.
End."
  run remove_other_todo_markers "$input"
  [ "$status" -eq 0 ]
  [ "$output" = "$expected" ]
}

@test "handles extra whitespace correctly" {
  input="Line1.
      //    TODO: -   Should be removed.
Line2."
  expected="Line1.
Line2."
  run remove_other_todo_markers "$input"
  [ "$status" -eq 0 ]
  [ "$output" = "$expected" ]
}

@test "does not remove lines that are similar but not matching exactly" {
  input="// TODO: example - should not be removed
// TODO:-should not be removed either
//NOT TODO: - should not be removed"
  expected="// TODO: example - should not be removed
// TODO:-should not be removed either
//NOT TODO: - should not be removed"
  run remove_other_todo_markers "$input"
  [ "$status" -eq 0 ]
  [ "$output" = "$expected" ]
}

