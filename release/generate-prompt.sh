#!/bin/bash
# file-types.sh
#
# Define the allowed file types for searches.
# These variables can be used by grep and find commands.

## For grep, use multiple --include flags.
ALLOWED_GREP_INCLUDES=(--include="*.swift" --include="*.h" --include="*.m" --include="*.js")

## For find, use a grouped expression.
ALLOWED_FIND_EXPR=(-name "*.swift" -o -name "*.h" -o -name "*.m" -o -name "*.js")
# get-git-root.sh
#
# This function determines the Git repository root directory based on the current working directory.
# If the current directory is not within a Git repository, it outputs an error message and returns a non-zero status.
#
# Usage: get-git-root
#
# Outputs:
#   On success: prints the Git repository root directory.
#   On failure: prints an error message to stderr and returns a non-zero status.
get-git-root() {
    local git_root
    git_root=$(git rev-parse --show-toplevel 2>/dev/null) || {
        echo "Error: Not a git repository." >&2
        return 1
    }
    echo "$git_root"
}

# Allow running this file directly for a quick test.
    get-git-root
fi
# get-package-root.sh
#
# This function determines the root of a Swift package by looking for a Package.swift
# in the current directory or one of its ancestors.
#
# Usage: get-package-root <file_path>
#
# On success: prints the package root directory.
# On failure: prints nothing (caller may then use the Git root).
get-package-root() {
    local file_path="$1"
    local dir
    # Start in the directory of the given file.
    dir=$(dirname "$file_path")
    # Walk upward until reaching the filesystem root.
    while [ "$dir" != "/" ]; do
        if [ -f "$dir/Package.swift" ]; then
            echo "$dir"
            return 0
        fi
        dir=$(dirname "$dir")
    done
    return 1
}

# Allow direct execution for testing.
    get-package-root "$1"
fi
# filter-files-singular.sh
#
# This function returns a temporary file containing only the Swift file
# that holds the TODO instruction.
#
# Usage: filter_files_singular <todo_file>
filter_files_singular() {
    local todo_file="$1"
    local filtered_file
    filtered_file=$(mktemp)
    echo "$todo_file" > "$filtered_file"
    echo "$filtered_file"
}

# Allow direct execution for testing.
    filter_files_singular "$1"
fi
# find-prompt-instruction.sh
#
# This function looks for a file that contains a TODO instruction marked by:
#   - "// TODO: - "
#
# It supports multiple file types (Swift, Objective-C header, and Objective-C implementation)
# by using the allowed file type filters defined in file-types.sh.
#
# If no such file exists, it outputs an error message.
# If more than one file contains the instruction, it chooses the file which is most
# recently edited and logs a message listing the ignored TODO files.
#
# Usage: find-prompt-instruction <search_directory>
#
# Outputs:
#   On success: prints the file path (for further processing) of the chosen instruction.
#   On failure: prints an error message to stderr and returns a non-zero exit code.
#
# Note: If the global variable VERBOSE is set to "true" (for example via --verbose in generate-prompt.sh),
# this function will output additional debug logging to stderr.

# Source file-types.sh to include allowed file extensions.

find-prompt-instruction() {
    local search_dir="$1"
    if [ "${VERBOSE:-false}" = true ]; then
       echo "[VERBOSE] Starting search in directory: $search_dir" >&2
    fi

    # Pattern matching only "// TODO: - " (with trailing space)
    local grep_pattern='// TODO: - '

    # Read all matching file paths into an array.
    local files_array=()
    while IFS= read -r line; do
        files_array+=("$line")
    done < <(grep -rlE "$grep_pattern" --exclude-dir=Pods "${ALLOWED_GREP_INCLUDES[@]}" "$search_dir" 2>/dev/null)

    if [ "${VERBOSE:-false}" = true ]; then
       echo "[VERBOSE] Found ${#files_array[@]} file(s) matching TODO pattern." >&2
       for file in "${files_array[@]}"; do
            echo "[VERBOSE] Matched file: $file" >&2
       done
    fi

    local file_count="${#files_array[@]}"

    if [ "$file_count" -eq 0 ]; then
        echo "Error: No files found containing '// TODO: - '" >&2
        return 1
    fi

    if [ "$file_count" -eq 1 ]; then
        if [ "${VERBOSE:-false}" = true ]; then
           echo "[VERBOSE] Only one matching file found: ${files_array[0]}" >&2
        fi
        echo "${files_array[0]}"
        return 0
    fi

    # More than one file: determine the one with the most recent modification time.
    local chosen_file="${files_array[0]}"
    local chosen_mod_time
    chosen_mod_time=$(stat -f "%m" "${chosen_file}")
    if [ "${VERBOSE:-false}" = true ]; then
       echo "[VERBOSE] Initial chosen file: $chosen_file with modification time $chosen_mod_time" >&2
    fi

    for file in "${files_array[@]}"; do
        local mod_time
        mod_time=$(stat -f "%m" "$file")
        if [ "${VERBOSE:-false}" = true ]; then
           echo "[VERBOSE] Evaluating file: $file with modification time $mod_time" >&2
        fi
        if [ "$mod_time" -gt "$chosen_mod_time" ]; then
            chosen_file="$file"
            chosen_mod_time="$mod_time"
            if [ "${VERBOSE:-false}" = true ]; then
               echo "[VERBOSE] New chosen file: $chosen_file with modification time $chosen_mod_time" >&2
            fi
        fi
    done

    # Build a list of files that were not chosen.
    local ignored_files=()
    for file in "${files_array[@]}"; do
        if [ "$file" != "$chosen_file" ]; then
            ignored_files+=("$file")
        fi
    done

    if [ "${VERBOSE:-false}" = true ]; then
       echo "[VERBOSE] Ignoring the following files:" >&2
       for file in "${ignored_files[@]}"; do
           local base
           base=$(basename "$file")
           echo "[VERBOSE] Ignored file: $base" >&2
       done
    fi

    echo "--------------------------------------------------" >&2
    echo "Multiple TODO instructions found (${file_count} files), the following TODO files were IGNORED:" >&2
    for file in "${ignored_files[@]}"; do
        local base
        base=$(basename "$file")
        # Extract the first matching TODO line from the file.
        local todo_text
        todo_text=$(grep -m 1 -E "$grep_pattern" "$file" | sed 's/^[[:space:]]*//')
        echo "  - ${base}: ${todo_text}" >&2
        echo "--------------------------------------------------" >&2
    done

    echo "$chosen_file"
}

# Allow running this file directly for a quick manual test.
    find-prompt-instruction "$1"
fi
# extract-instruction-content.sh
#
# This function extracts the TODO instruction content from a given Swift file.
# It looks for a line that matches the marker "// TODO: - ".
#
# Usage: extract-instruction-content <swift_file>
#
# On success: prints the extracted instruction line (trimmed).
# On failure: prints an error message and returns a non-zero exit code.
extract-instruction-content() {
    local swift_file="$1"
    local instruction_line

    # Search for the matching TODO instruction.
    instruction_line=$(grep -E '// TODO: - ' "$swift_file" | head -n 1)
    
    if [ -z "$instruction_line" ]; then
        echo "Error: No valid TODO instruction found in $swift_file" >&2
        return 1
    fi

    # Trim leading whitespace and output the result.
    echo "$instruction_line" | sed 's/^[[:space:]]*//'
}

# Allow direct execution for a quick test.
    extract-instruction-content "$1"
fi
# assemble-prompt.sh
#
# This function assembles the final ChatGPT prompt by including:
#   - The contents of Swift (or other allowed) files where type definitions were found
#     (optionally filtered by substring markers), and
#   - A fixed instruction (ignoring the extracted TODO instruction).
#
# It takes two parameters:
#   1. <found_files_file>: A file (typically temporary) containing a list of file paths.
#   2. <instruction_content>: The TODO instruction content (now ignored).
#
# The function outputs the final assembled prompt to stdout and also copies it
# to the clipboard using pbcopy.
#
# If the environment variable DIFF_WITH_BRANCH is set (for example by running:
#   generate-prompt.sh --diff-with develop
# then for each file that differs from that branch, a diff report is appended after the file's content.

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source the helper that filters file content based on substring markers.

if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
fi

assemble-prompt() {
    local found_files_file="$1"
    local instruction_content="$2"  # This parameter is now ignored.
    
    # Sort and filter out duplicate file paths.
    local unique_found_files
    unique_found_files=$(sort "$found_files_file" | uniq)
    
    local clipboard_content=""
    
    # Process each file and format its content.
    while IFS= read -r file_path; do
        local file_basename file_content diff_output
        file_basename=$(basename "$file_path")
        
        if grep -qE '^[[:space:]]*//[[:space:]]*v' "$file_path"; then
            file_content=$(filter_substring_markers "$file_path")
        else
            file_content=$(cat "$file_path")
        fi
        
        clipboard_content="${clipboard_content}
The contents of ${file_basename} is as follows:

${file_content}

"
        # If DIFF_WITH_BRANCH is set, append a diff report (if there are changes).
        if [ -n "${DIFF_WITH_BRANCH:-}" ]; then
            diff_output=$(get_diff_with_branch "$file_path")
            if [ -n "$diff_output" ]; then
                clipboard_content="${clipboard_content}
--------------------------------------------------
The diff for ${file_basename} (against branch ${DIFF_WITH_BRANCH}) is as follows:

${diff_output}

"
            fi
        fi
        
        clipboard_content="${clipboard_content}
--------------------------------------------------
"
    done <<< "$unique_found_files"
    
    # Fixed instruction that will be appended.
    local fixed_instruction="Can you do the TODO:- in the above code? But ignoring all FIXMEs and other TODOs...i.e. only do the one and only one TODO that is marked by \"// TODO: - \", i.e. ignore things like \"// TODO: example\" because it doesn't have the hyphen"
    
    local final_clipboard_content="${clipboard_content}

${fixed_instruction}"
    
    # Copy the assembled prompt to the clipboard and print it.
    echo "$final_clipboard_content" | pbcopy
    echo "$final_clipboard_content"
}

# If executed directly, print usage instructions.
# extract-types.sh
#
# This function extracts potential type names (classes, structs, enums, etc.)
# from a given Swift file. It processes the file in several stages:
#
#   1. Preprocessing: Replace non-alphanumeric characters with whitespace.
#   2. Stage 0: Trim leading spaces.
#   3. Stage 1: Remove import lines.
#   4. Stage 2: Remove comment lines.
#   5. Stage 3: Extract capitalized words (and types within brackets),
#               then sort and remove duplicates.
#
# Usage: extract-types <swift_file>
#
# Output:
#   On success: prints the path to a temporary file containing a sorted,
#               unique list of potential type names.
#
#   All intermediate temporary files (except the final output) are cleaned up.
extract-types() {
    local swift_file="$1"
    
    # Create a temporary directory for all intermediate files.
    local tempdir
    tempdir=$(mktemp -d)

    # Define paths for intermediate files inside the temporary directory.
    local temp_preprocess="$tempdir/temp_preprocess"
    local temp_stage0="$tempdir/temp_stage0"
    local temp_stage1="$tempdir/temp_stage1"
    local temp_stage2="$tempdir/temp_stage2"
    local types_file="$tempdir/types_file"

    # Set a trap to ensure the temporary directory is removed if the function exits prematurely.
    trap 'rm -rf "$tempdir"' EXIT

    # Preprocessing: Replace all non-alphanumeric characters with whitespace.
    awk '{gsub(/[^a-zA-Z0-9]/, " "); print}' "$swift_file" > "$temp_preprocess"

    # Stage 0: Trim leading spaces from each line.
    awk '{$1=$1; print}' "$temp_preprocess" > "$temp_stage0"

    # Stage 1: Remove lines starting with "import".
    awk '!/^import /' "$temp_stage0" > "$temp_stage1"

    # Stage 2: Remove lines starting with comment markers.
    awk '!/^\/\//' "$temp_stage1" > "$temp_stage2"

    # Stage 3: Scan for potential type names:
    #         - Words that start with a capital letter.
    #         - Words within square brackets (e.g., [TypeName]).
    awk '
    {
        for(i = 1; i <= NF; i++) {
            if ($i ~ /^[A-Z][A-Za-z0-9]+$/) {
                print $i
            } else if ($i ~ /\[[A-Z][A-Za-z0-9]+\]/) {
                gsub(/\[|\]/, "", $i)
                print $i
            }
        }
    }' "$temp_stage2" | sort | uniq > "$types_file"

    # Copy the final types file to a new temporary file outside of tempdir.
    local final_types_file
    final_types_file=$(mktemp)
    cp "$types_file" "$final_types_file"

    # Clean up: Remove the temporary directory and its contents.
    rm -rf "$tempdir"
    trap - EXIT

    # Output the path to the final file containing the sorted, unique list of types.
    echo "$final_types_file"
}

# Allow running this file directly for quick manual testing.
    extract-types "$1"
fi
# find-definition-files.sh
#
# This function searches for files that contain definitions for any of the types
# listed in a given types file. It now builds a combined regex for all types to reduce
# the number of find/grep executions.
#
# Usage: find-definition-files <types_file> <root>
#
# Output:
#   On success: prints the path to a temporary file containing a list of files
#   where definitions were found.

# Source file-types.sh to import the allowed file expressions.

find-definition-files() {
    local types_file="$1"
    local root="$2"

    local script_dir
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

    # Get the search roots (optimized by the new get-search-roots.sh).
    local search_roots
    search_roots=$("$script_dir/get-search-roots.sh" "$root")
    
    if [ "${VERBOSE:-false}" = true ]; then
         echo "[VERBOSE] Search roots: $search_roots" >&2
    fi

    local temp_found
    temp_found=$(mktemp)

    # Build a combined regex: join all type names with "|"
    # (Assumes that type names are simple and need no extra escaping.)
    local types_regex
    types_regex=$(paste -sd '|' "$types_file")
    
    if [ "${VERBOSE:-false}" = true ]; then
         echo "[VERBOSE] Combined regex: $types_regex" >&2
    fi

    # For each search root, perform one find command using the combined regex.
    for sr in $search_roots; do
         if [ "${VERBOSE:-false}" = true ]; then
              echo "[VERBOSE] Running find command in directory: $sr" >&2
         fi
         find "$sr" -type f \( "${ALLOWED_FIND_EXPR[@]}" \) \
             -not -path "*/.build/*" -not -path "*/Pods/*" \
             -exec grep -lE "\\b(class|struct|enum|protocol|typealias)\\s+($types_regex)\\b" {} \; >> "$temp_found" || true
         if [ "${VERBOSE:-false}" = true ]; then
              echo "[VERBOSE] Completed search in directory: $sr" >&2
         fi
    done

    local found_count
    found_count=$(wc -l < "$temp_found")
    if [ "${VERBOSE:-false}" = true ]; then
         echo "[VERBOSE] Total files found (before deduplication): $found_count" >&2
    fi

    # Deduplicate the found files.
    local final_found
    final_found=$(mktemp)
    sort -u "$temp_found" > "$final_found"
    rm -f "$temp_found"

    if [ "${VERBOSE:-false}" = true ]; then
         local final_count
         final_count=$(wc -l < "$final_found")
         echo "[VERBOSE] Total unique files found: $final_count" >&2
    fi

    echo "$final_found"
}

# Allow direct execution for testing.
    find-definition-files "$1" "$2"
fi
# filter-files.sh
#
# This function filters a list of Swift file paths when slim mode is enabled.
# It always includes the TODO file and excludes files whose names match
# certain keywords (e.g. ViewController, Manager, Presenter, Router, Interactor,
# Configurator, DataSource, Delegate, or View).
#
# Usage: filter-files_for_slim_mode <todo_file> <found_files_file>
#   <todo_file> is the file containing the TODO.
#   <found_files_file> is a file listing paths to candidate files.
#
# It outputs the path to a temporary file containing the filtered list.
filter-files_for_slim_mode() {
    local todo_file="$1"
    local found_files_file="$2"

    if [ "${VERBOSE:-false}" = true ]; then
         echo "[VERBOSE] Starting filtering in filter-files_for_slim_mode" >&2
         echo "[VERBOSE] TODO file: $todo_file" >&2
         echo "[VERBOSE] Candidate file list file: $found_files_file" >&2
    fi

    local filtered_files
    filtered_files=$(mktemp)

    # Always include the file containing the TODO.
    echo "$todo_file" >> "$filtered_files"
    if [ "${VERBOSE:-false}" = true ]; then
         echo "[VERBOSE] Added TODO file to filtered list: $todo_file" >&2
    fi

    # Process each file in the found files list.
    while IFS= read -r file; do
        # Skip if it's the TODO file.
        if [ "$file" = "$todo_file" ]; then
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[VERBOSE] Skipping candidate as it matches the TODO file: $file" >&2
            fi
            continue
        fi
        local base
        base=$(basename "$file")
        if [ "${VERBOSE:-false}" = true ]; then
            echo "[VERBOSE] Processing file: $file (basename: $base)" >&2
        fi
        # Exclude files that are likely not models.
        if [[ "$base" =~ (ViewController|Manager|Presenter|Router|Interactor|Configurator|DataSource|Delegate|View) ]]; then
            if [ "${VERBOSE:-false}" = true ]; then
                echo "[VERBOSE] Excluding file based on pattern match: $base" >&2
            fi
            continue
        fi
        echo "$file" >> "$filtered_files"
        if [ "${VERBOSE:-false}" = true ]; then
            echo "[VERBOSE] Including file: $file" >&2
        fi
    done < "$found_files_file"

    if [ "${VERBOSE:-false}" = true ]; then
         local count
         count=$(wc -l < "$filtered_files")
         echo "[VERBOSE] Total files in filtered list: $count" >&2
    fi

    echo "$filtered_files"
}

# Allow direct execution for testing.
    filter-files_for_slim_mode "$1" "$2"
fi
# exclude-files.sh
#
# This function removes file paths from a list (in a temporary file) if their
# basenames match any of the exclusion patterns provided.
#
# Usage: filter_excluded_files <found_files_file> <exclusion1> [<exclusion2> ...]
#
# It outputs the path to a new temporary file containing the filtered list.
filter_excluded_files() {
    local found_files_file="$1"
    shift
    local exclusions=("$@")
    local filtered_file
    filtered_file=$(mktemp)

    # Process each file in the found files list.
    while IFS= read -r file; do
        local base
        base=$(basename "$file" | xargs)
        local exclude=false
        for pattern in "${exclusions[@]}"; do
            if [[ "$base" == "$pattern" ]]; then
                exclude=true
                break
            fi
        done
        if [ "$exclude" = false ]; then
            echo "$file" >> "$filtered_file"
        fi
    done < "$found_files_file"

    echo "$filtered_file"
}
# diff-with-branch.sh
#
# This function generates a diff for a given file between the current branch and
# the branch specified in the environment variable DIFF_WITH_BRANCH.
# It assumes that the file is tracked by Git. If the file is unmodified relative to
# the given branch, an empty string is returned.
#
# Usage: get_diff_with_branch <file_path>
get_diff_with_branch() {
    local file="$1"
    local branch="${DIFF_WITH_BRANCH:-main}"
    
    # Check if the file is tracked in Git.
    if ! git ls-files --error-unmatch "$file" >/dev/null 2>&1; then
        return 0
    fi

    # Get the diff between the file in the current branch and the given branch.
    git diff "$branch" -- "$file"
}

# Allow direct testing.
    get_diff_with_branch "$1"
fi
# extract-enclosing-type.sh
#
# This helper defines a function to extract the enclosing type
# (class, struct, or enum) from a given Swift file. It scans until
# it reaches the TODO instruction and returns the last encountered type.
#
# If no enclosing type is found (i.e. if the TODO is outside any type),
# it falls back to using the file’s basename (without the .swift extension)
# as the type name.
#
# Usage (when sourcing):
#   extract_enclosing_type <swift_file>
#
# When executed directly, it performs a quick test.

extract_enclosing_type() {
    local swift_file="$1"
    local extracted_type

    extracted_type=$(awk '
       BEGIN { regex="(class|struct|enum)[[:space:]]+" }
       /\/\/ TODO: -/ { exit }
       {
           pos = match($0, regex)
           if (pos > 0) {
               # Get the substring immediately after the matched keyword.
               type_line = substr($0, RSTART+RLENGTH)
               # Split the remainder by any non-alphanumeric/underscore character.
               split(type_line, arr, /[^A-Za-z0-9_]/)
               if (arr[1] != "") { type = arr[1] }
           }
       }
       END { if (type != "") print type }
    ' "$swift_file")

    # Fallback: if no type was found, use the file's basename (without .swift)
    if [ -z "$extracted_type" ]; then
         extracted_type=$(basename "$swift_file" .swift)
    fi

    echo "$extracted_type"
}

# Allow direct execution for testing.
    extract_enclosing_type "$1"
fi
# find-referencing-files.sh
#
# This helper defines a function to search for files (Swift, Objective-C header,
# and Objective-C implementation) that reference a given type name. It returns a
# temporary file containing a list of matching files.
#
# Usage (when sourcing):
#   find_referencing_files <type_name> <search_root>
#
# When executed directly, it performs a quick test.

# Source file-types.sh to import allowed file type includes.

find_referencing_files() {
    local type_name="$1"
    local search_root="$2"

    local temp_file
    temp_file=$(mktemp)

    # Search for occurrences of the type name as a whole word in files using the allowed
    # file types, excluding common build directories.
    grep -rlE "\\b$type_name\\b" "${ALLOWED_GREP_INCLUDES[@]}" "$search_root" \
         --exclude-dir=Pods --exclude-dir=.build > "$temp_file" 2>/dev/null

    echo "$temp_file"
}

# Allow direct execution for testing.
    find_referencing_files "$1" "$2"
fi
# filter-substring-markers.sh
#
# This function checks if a given file contains “substring markers.”
# The markers are defined as follows:
#   - An opening marker: a line that, when trimmed, exactly matches:
#         // v
#   - A closing marker: a line that, when trimmed, exactly matches:
#         // ^
#
# If these markers are found in the file, only the text between them is output.
# Any omitted regions (before the first block, between blocks, and after the last block)
# are replaced with a single placeholder (with an extra blank line above and below):
#
#         (blank line)
#         // ...
#         (blank line)
#
# If no markers are found, the entire file is output unchanged.
#
# Usage:
#   filter_substring_markers <file_path>
filter_substring_markers() {
    local file="$1"
    # If no opening marker exists (strictly matching), output the file unchanged.
    if ! grep -qE '^[[:space:]]*//[[:space:]]*v[[:space:]]*$' "$file"; then
        cat "$file"
        return 0
    fi

    awk '
    BEGIN {
        inBlock = 0;
        lastWasPlaceholder = 0;
    }
    # Function to print a placeholder (with extra blank lines above and below)
    # only if the previous printed line was not already a placeholder.
    function printPlaceholder() {
        if (lastWasPlaceholder == 0) {
            print "";
            print "// ...";
            print "";
            lastWasPlaceholder = 1;
        }
    }
    {
        # Check for the opening marker: when trimmed, the line must be exactly "// v"
        if ($0 ~ /^[[:space:]]*\/\/[[:space:]]*v[[:space:]]*$/) {
            printPlaceholder();
            inBlock = 1;
            next;
        }
        # Check for the closing marker: when trimmed, the line must be exactly "// ^"
        if ($0 ~ /^[[:space:]]*\/\/[[:space:]]*\^[[:space:]]*$/) {
            inBlock = 0;
            printPlaceholder();
            next;
        }
        # If inside a marked block, print the line.
        if (inBlock) {
            print $0;
            lastWasPlaceholder = 0;
        }
    }
    ' "$file"
}

# Allow direct execution for testing.
    filter_substring_markers "$1"
fi
# get-search-roots.sh
#
# This function returns a list of directories that are potential Swift package roots.
# If the provided root is already a package (contains Package.swift), then only that
# directory is returned.
#
# Usage: get-search-roots <git_root_or_package_root>
#
# Output: a list of directories (one per line).
get-search-roots() {
    local root="$1"
    
    # If the root itself is a Swift package (contains Package.swift), return it.
    if [ -f "$root/Package.swift" ]; then
        echo "$root"
        return 0
    fi

    # Otherwise, include the root if it is not a .build directory.
    if [ "$(basename "$root")" != ".build" ]; then
        echo "$root"
    fi

    # Find any subdirectories that contain Package.swift, but exclude those inside .build folders.
    find "$root" -type f -name "Package.swift" -not -path "*/.build/*" -exec dirname {} \; | sort -u
}

# Allow direct execution for testing.
    get-search-roots "$1"
fi
set -euo pipefail

##########################################
# generate-prompt.sh
#
# This script finds the unique Swift file that contains a
# TODO instruction (specifically “// TODO: - ”),
# processes it along with related type definitions in the repository,
# and then assembles a ChatGPT prompt that is copied to the clipboard.
#
# Usage:
#   generate-prompt.sh [--slim] [--singular] [--force-global] [--include-references] [--diff-with <branch>] [--exclude <filename>] [--verbose] [--exclude <another_filename>] ...
#
# Options:
#   --slim         Only include the file that contains the TODO instruction
#                  and “model” files. In slim mode, files whose names contain
#                  keywords such as “ViewController”, “Manager”, “Presenter”,
#                  “Configurator”, “Router”, “DataSource”, “Delegate”, or “View”
#                  are excluded.
#   --singular     Only include the Swift file that contains the TODO instruction.
#   --force-global Use the entire Git repository for context inclusion, even if the TODO file is in a package.
#   --include-references
#                  Additionally include files that reference the enclosing type.
#   --diff-with <branch>
#                  For each included file that differs from the specified branch,
#                  include a diff report. (e.g. --diff-with main or --diff-with develop)
#   --exclude      Exclude any file whose basename matches the provided filename.
#   --verbose      Enable verbose console logging for debugging purposes.
#
# Note:
#   You must write your question in the form // TODO: - (including the hyphen).
#
#   - find-prompt-instruction.sh       : Locates the unique Swift file with the TODO.
#   - extract-instruction-content.sh   : Extracts the TODO instruction content from the file.
#   - extract-types.sh                 : Extracts potential type names from a Swift file.
#   - find-definition-files.sh         : Finds Swift files containing definitions for the types.
#   - filter-files.sh                  : Filters the found files in slim mode.
#   - exclude-files.sh                 : Filters out files matching user-specified exclusions.
#   - assemble-prompt.sh               : Assembles the final prompt and copies it to the clipboard.
#   - get-git-root.sh                  : Determines the Git repository root.
#   - get-package-root.sh              : Determines the package root (if any) for a given file.
#   - filter-files-singular.sh         : Returns only the file that contains the TODO.
#
# New for reference inclusion:
#   - extract-enclosing-type.sh        : Extracts the enclosing type from the TODO file.
#   - find-referencing-files.sh        : Finds files that reference the enclosing type.
#
# New for diff inclusion:
#   --diff-with <branch>              For each included file that differs from the
#                                     specified branch, include a diff report.
##########################################

# Process optional parameters.
SLIM=false
SINGULAR=false
VERBOSE=false
FORCE_GLOBAL=false
INCLUDE_REFERENCES=false
# DIFF_WITH_BRANCH will be set by the --diff-with option.
EXCLUDES=()
while [[ $# -gt 0 ]]; do
    case "$1" in
        --slim)
            SLIM=true
            shift
            ;;
        --singular)
            SINGULAR=true
            shift
            ;;
        --force-global)
            FORCE_GLOBAL=true
            shift
            ;;
        --include-references)
            INCLUDE_REFERENCES=true
            shift
            ;;
        --diff-with)
            if [ -n "${2:-}" ]; then
                export DIFF_WITH_BRANCH="$2"
                shift 2
            else
                echo "Usage: $0 [--diff-with <branch>]" >&2
                exit 1
            fi
            ;;
        --exclude)
            if [ -n "${2:-}" ]; then
                EXCLUDES+=("$2")
                shift 2
            else
                echo "Usage: $0 [--exclude <filename>]" >&2
                exit 1
            fi
            ;;
        --verbose)
            VERBOSE=true
            shift
            ;;
        *)
            echo "Usage: $0 [--slim] [--singular] [--force-global] [--include-references] [--diff-with <branch>] [--exclude <filename>] [--verbose]" >&2
            exit 1
            ;;
    esac
done

# Export VERBOSE so that helper scripts can use it.
export VERBOSE

# Save the directory where you invoked the script.
CURRENT_DIR="$(pwd)"

# Determine the directory where this script resides.
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Source external components from SCRIPT_DIR.

# If not in singular mode already, load the additional helpers.
if [ "$SINGULAR" = false ]; then
fi

if [ "${INCLUDE_REFERENCES:-false}" = true ]; then
fi

echo "--------------------------------------------------"

# Change back to the directory where the command was invoked.
cd "$CURRENT_DIR"

# Determine the Git repository root.
GIT_ROOT=$(get-git-root) || exit 1
echo "Git root: $GIT_ROOT"

# Move to the repository root.
cd "$GIT_ROOT"

# Use the external component to locate the file with the TODO instruction.
FILE_PATH=$(find-prompt-instruction "$GIT_ROOT") || exit 1
echo "Found exactly one instruction in $FILE_PATH"

# --- Enforce singular mode for JavaScript files (beta support) ---
if [[ "$FILE_PATH" == *.js ]]; then
    if [ "$SINGULAR" = false ]; then
        echo "WARNING: JavaScript support is currently in beta. Singular mode will be enforced, so only the file containing the TODO instruction will be used for context." >&2
        SINGULAR=true
    fi
fi

# --- Check for --include-references ---
if [ "$INCLUDE_REFERENCES" = true ]; then
    if [[ "$FILE_PATH" != *.swift ]]; then
        echo "Error: The --include-references option is currently only supported for Swift files. The TODO instruction was found in a non-Swift file: $(basename "$FILE_PATH")" >&2
        exit 1
    fi
fi

# --- Determine Package Scope ---
PACKAGE_ROOT=$(get-package-root "$FILE_PATH" || true)
if [ "${FORCE_GLOBAL}" = true ]; then
    echo "Force global enabled: ignoring package boundaries and using Git root for context."
    SEARCH_ROOT="$GIT_ROOT"
elif [ -n "$PACKAGE_ROOT" ]; then
    echo "Found package root: $PACKAGE_ROOT"
    SEARCH_ROOT="$PACKAGE_ROOT"
else
    SEARCH_ROOT="$GIT_ROOT"
fi
# --- End Package Scope ---

# Extract the instruction content from the file.
INSTRUCTION_CONTENT=$(extract-instruction-content "$FILE_PATH")

if [ "$SINGULAR" = true ]; then
    echo "Singular mode enabled: only including the TODO file"
    FOUND_FILES=$(filter_files_singular "$FILE_PATH")
else
    # Extract potential type names from the Swift file.
    TYPES_FILE=$(extract-types "$FILE_PATH")
    
    # Find Swift files containing definitions for the types.
    FOUND_FILES=$(find-definition-files "$TYPES_FILE" "$SEARCH_ROOT")
    
    # Ensure the chosen TODO file is included in the found files.
    echo "$FILE_PATH" >> "$FOUND_FILES"
    
    # If slim mode is enabled, filter the FOUND_FILES list.
    if [ "$SLIM" = true ]; then
         echo "Slim mode enabled: filtering files to include only the TODO file and model files..."
         FOUND_FILES=$(filter-files_for_slim_mode "$FILE_PATH" "$FOUND_FILES")
    fi
    
    # If any exclusions were specified, filter them out.
    if [ "${#EXCLUDES[@]}" -gt 0 ]; then
         echo "Excluding files matching: ${EXCLUDES[*]}"
         FOUND_FILES=$(filter_excluded_files "$FOUND_FILES" "${EXCLUDES[@]}")
    fi
fi

# --- Include referencing files if requested ---
if [ "${INCLUDE_REFERENCES:-false}" = true ]; then
    echo "Including files that reference the enclosing type..."
    # Extract the enclosing type from the TODO file using the helper function.
    enclosing_type=$(extract_enclosing_type "$FILE_PATH")
    if [ -n "$enclosing_type" ]; then
        echo "Found enclosing type '$enclosing_type'. Searching for files that reference '$enclosing_type' in: $SEARCH_ROOT"
        referencing_files=$(find_referencing_files "$enclosing_type" "$SEARCH_ROOT")
        # Append the referencing files to the FOUND_FILES list.
        cat "$referencing_files" >> "$FOUND_FILES"
        rm -f "$referencing_files"
    else
        echo "No enclosing type found in $FILE_PATH, skipping reference search."
    fi
fi
# --- End reference inclusion ---

# Register a trap to clean up temporary files.
cleanup_temp_files() {
    [[ -n "${TYPES_FILE:-}" ]] && rm -f "$TYPES_FILE"
    [[ -n "${FOUND_FILES:-}" ]] && rm -f "$FOUND_FILES"
}
trap cleanup_temp_files EXIT

echo "--------------------------------------------------"
if [ "$SINGULAR" = false ]; then
    echo "Types found:"
    cat "$TYPES_FILE"
    echo "--------------------------------------------------"
fi

echo "Files (final list):"
sort "$FOUND_FILES" | uniq | while read -r file_path; do
    basename "$file_path"
done

# Assemble the final clipboard content and copy it to the clipboard.
FINAL_CLIPBOARD_CONTENT=$(assemble-prompt "$FOUND_FILES" "$INSTRUCTION_CONTENT")

echo "--------------------------------------------------"
echo
echo "Success:"
echo
echo "$INSTRUCTION_CONTENT"
if [ "$INCLUDE_REFERENCES" = true ]; then
    echo
    echo "Warning: The --include-references option is experimental and may produce unexpected results."
fi
echo
echo "--------------------------------------------------"
