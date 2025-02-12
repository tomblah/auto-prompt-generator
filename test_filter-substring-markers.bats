#!/usr/bin/env bats
# test_filter-substring-markers.bats
#
# These tests verify the functionality of the filter_substring_markers function,
# which “filters” a file’s content when it contains strictly matched substring markers.
#
# The rules are:
#   - An opening marker is a line that, when trimmed, exactly matches either:
#         // v
#     or
#         //v
#   - A closing marker is a line that, when trimmed, exactly matches either:
#         // ^
#     or
#         //^
#
# If these markers are found in the file, only the text between each valid
# marker pair is output. Any omitted region (i.e. the content before the first block,
# between blocks, or after the last block) is replaced with a single placeholder block:
#
#         (blank line)
#         // ...
#         (blank line)
#
# If no markers are found, the entire file is output unchanged.
#
# Usage:
#   filter_substring_markers <file_path>

setup() {
  TMP_DIR=$(mktemp -d)
}

teardown() {
  rm -rf "$TMP_DIR"
}

# Helper: trim leading and trailing whitespace from each line.
trim_lines() {
  sed 's/^[ \t]*//; s/[ \t]*$//'
}

# Load the filter_substring_markers function.
# (Adjust the path if necessary so that it loads your filter-substring-markers.sh file.)
load "${BATS_TEST_DIRNAME}/filter-substring-markers.sh"

@test "filter_substring_markers outputs entire file if no markers are present" {
  file="$TMP_DIR/no_markers.txt"
  cat <<EOF > "$file"
Line one
Line two
Line three
EOF
  run filter_substring_markers "$file"
  [ "$status" -eq 0 ]
  expected="Line one
Line two
Line three"
  [ "$(echo "$output" | trim_lines)" = "$(echo "$expected" | trim_lines)" ]
}

@test "filter_substring_markers extracts code between markers with placeholders" {
  file="$TMP_DIR/with_markers.txt"
  cat <<'EOF' > "$file"
Outside block A
   // v
Inside block A line 1
Inside block A line 2
   // ^
Outside block B
   // v
Inside block B
   // ^
Outside block C
EOF
  run filter_substring_markers "$file"
  [ "$status" -eq 0 ]
  # Because valid markers (lines that, when trimmed, equal either "// v" or "//v", and similarly for closing markers)
  # are present, only the content inside those markers is included,
  # with the content outside replaced by a placeholder block.
  expected_output=$(cat <<'EOL'
  
// ...

Inside block A line 1
Inside block A line 2
  
// ...

Inside block B
  
// ...
  
EOL
)
  [ "$(echo "$output" | trim_lines)" = "$(echo "$expected_output" | trim_lines)" ]
}

@test "filter_substring_markers does not match lines that are similar but not exact" {
  file="$TMP_DIR/not_strict.txt"
  cat <<'EOF' > "$file"
Line one
// v extra text
Line two
// ^ extra
Line three
EOF
  run filter_substring_markers "$file"
  [ "$status" -eq 0 ]
  # Since there are no valid markers (the marker lines do not match exactly),
  # the entire file is output unchanged.
  expected="Line one
// v extra text
Line two
// ^ extra
Line three"
  [ "$(echo "$output" | trim_lines)" = "$(echo "$expected" | trim_lines)" ]
}

@test "filter_substring_markers deduplicates consecutive placeholders" {
  file="$TMP_DIR/consecutive_markers.txt"
  cat <<'EOF' > "$file"
   // v
Inside block
   // ^
   // ^
EOF
  run filter_substring_markers "$file"
  [ "$status" -eq 0 ]
  # There is one valid marker pair; the second closing marker should not produce an extra placeholder.
  expected_output=$(cat <<'EOL'
  
// ...

Inside block
  
// ...
  
EOL
)
  [ "$(echo "$output" | trim_lines)" = "$(echo "$expected_output" | trim_lines)" ]
}

@test "filter_substring_markers relaxed marker matching with optional spaces" {
  file="$TMP_DIR/strict_match.txt"
  cat <<'EOF' > "$file"
   //v
Should not match because trimmed it equals "//v"
   // v
Should match marker line
   // ^
Should match marker line
   //^
Should not match because trimmed it equals "//^"
EOF
  run filter_substring_markers "$file"
  [ "$status" -eq 0 ]
  # With the relaxed matching, both "//v" and "// v" trigger an opening marker,
  # and both "//^" and "// ^" trigger a closing marker.
  # Expected behavior (based on our AWK script):
  #   - Line 1 ("//v") is recognized as an opening marker → prints placeholder.
  #   - Line 2 is printed (inside block).
  #   - Line 3 ("// v") is recognized as an opening marker → prints placeholder.
  #   - Line 4 is printed (inside block).
  #   - Line 5 ("// ^") is recognized as a closing marker → prints placeholder.
  #   - Line 6 is not printed (outside any block).
  #   - Line 7 ("//^") is recognized as a closing marker; since a placeholder was just printed, no duplicate is added.
  #   - Line 8 is not printed.
  expected_output=$(cat <<'EOL'
  
// ...

Should not match because trimmed it equals "//v"
  
// ...

Should match marker line
  
// ...
  
EOL
)
  [ "$(echo "$output" | trim_lines)" = "$(echo "$expected_output" | trim_lines)" ]
}
