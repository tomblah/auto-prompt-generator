#!/usr/bin/env bash
set -euo pipefail

echo "🔄 Finalizing log level adjustments…"

# 1) Fix crates/find_prompt_instruction/src/lib.rs exactly:
perl -pi -e '
  # matching-file(s)
  s{^\s*eprintln!\("\[VERBOSE\] (\d+) matching file\(s\) found\."\s*,\s*(.+)\)}
    {log::debug!("[VERBOSE] {} matching file(s) found.", $2)};

  # ignoring header
  s{^\s*eprintln!\("\[VERBOSE\] Ignoring the following files:"\)}
    {log::debug!("[VERBOSE] Ignoring the following files:")};

  # only-one-file case
  s{^\s*eprintln!\("\[VERBOSE\] Only one matching file found: (.+)"\)}
    {log::debug!("[VERBOSE] Only one matching file found: {}", "$1")};

  # chosen-file
  s{^\s*eprintln!\("\[VERBOSE\] Chosen file: (.+)"\)}
    {log::debug!("[VERBOSE] Chosen file: {}", "$1")};

  # list items
  s{^\s*eprintln!\("  - (.+)"\)}
    {log::debug!("  - {}", "$1")};

  # divider lines
  s{^\s*eprintln!\("[-–—]{3,}"\)}
    {log::debug!("--------------------------------------------------")};

  # fallback for any other eprintln!
  s{^\s*eprintln!\((.*)\)}{log::debug!($1)};
' crates/find_prompt_instruction/src/lib.rs

# 2) Global fixes for all other crates:
find crates -name '*.rs' -not -path '*/find_prompt_instruction/*' -print0 | \
  xargs -0 perl -pi -e '
    # a) eprintln! → debug
    s{\beprintln!\((.*)\)}{log::debug!($1)}sg;

    # b) Warning errors → warn
    s{\blog::error!\(\s*("Warning:[^"]*"(?:\s*,\s*[^)]*)?)\)}{log::warn!($1)}xg;

    # c) [VERBOSE] errors → debug
    s{\blog::error!\(\s*("\[VERBOSE\][^"]*"(?:\s*,\s*[^)]*)?)\)}{log::debug!($1)}xg;

    # d) inner verbose markers → debug
    s{\blog::error!\(\s*("  - [^"]*")\)}{log::debug!($1)}g;
    s{\blog::error!\(\s*("[-–—]{3,}"?)\)}{log::debug!($1)}g;
  '

echo "✅ Done — now run:"
echo "    git grep -E 'eprintln!|log::error!.*(Warning|\\[VERBOSE\\])'"
echo "to confirm no more leftovers."
