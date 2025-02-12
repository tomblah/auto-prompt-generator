#!/bin/bash
# file-types.sh
#
# Define the allowed file types for searches.
# These variables can be used by grep and find commands.

## For grep, use multiple --include flags.
ALLOWED_GREP_INCLUDES=(--include="*.swift" --include="*.h" --include="*.m" --include="*.js")

## For find, use a grouped expression.
ALLOWED_FIND_EXPR=(-name "*.swift" -o -name "*.h" -o -name "*.m" -o -name "*.js")
