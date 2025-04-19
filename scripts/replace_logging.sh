#!/usr/bin/env bash
#
# replace_logging.sh
#
# Recursively replaces all `println!` and `eprintln!` macros in Rust source files
# under the `crates/` directory with `log::info!` and `log::error!`, except for
# verbose eprintlns (prefixed with "[VERBOSE]").
#
# Usage:
#   chmod +x scripts/replace_logging.sh
#   ./scripts/replace_logging.sh
#
set -euo pipefail

echo "🔄 Replacing println!/eprintln!…"

find crates -name '*.rs' -print0 | \
  xargs -0 perl -pi -e '
    # Replace println!(...) → log::info!(...)
    s/\bprintln!\s*\(\s*(.*?)\s*\)/log::info!($1)/sg;

    # Replace eprintln!(...) → log::error!(...), but skip verbose eprintlns
    s/\beprintln!\s*\(\s*(?!"\[VERBOSE\])(.*?)\s*\)/log::error!($1)/sg;
  '

echo "✅ Done – now 'git diff' and review the replacements."
