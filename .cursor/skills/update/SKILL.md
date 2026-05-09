---
name: update
description: Update Rust dependencies for this project, validate the full clean build, commit Cargo.lock, merge back to main with a no-fast-forward merge, and push. Use when the user asks to run the project update workflow or dependency update process.
---

# Update

## Workflow

Use this skill for the repository dependency update workflow. This skill is allowed to merge and push automatically, but only after the full validation gate passes.

1. Confirm the working tree is clean before starting:
   ```bash
   git status --short
   ```
   If there are unrelated changes, stop and report them. Do not stash, revert, or overwrite user work.

2. Record the current branch, then refresh `main`:
   ```bash
   original_branch="$(git branch --show-current)"
   git fetch origin
   git checkout main
   git pull --ff-only origin main
   ```

3. Update Rust dependencies on `main`:
   ```bash
   cargo update
   ```

4. If `Cargo.lock` did not change, return to the original branch and stop:
   ```bash
   if git diff --quiet -- Cargo.lock; then
     git checkout "$original_branch"
     echo "No Cargo.lock updates available."
   fi
   ```
   If this condition is true, stop the workflow here. Do not create an update branch, commit, merge, or push when there are no dependency updates.

5. Create a timestamped update branch from `main`, carrying the changed lock file:
   ```bash
   git checkout -b "update/cargo-lock-$(date +%Y%m%d-%H%M)"
   ```

6. Run the full clean validation gate:
   ```bash
   make all
   ```
   In this repo, `make all` runs `clean`, `fix-headers`, `build`, `test`, and `coverage`.

7. Inspect the resulting changes:
   ```bash
   git status --short
   git diff -- Cargo.lock
   ```
   Continue only if the intended change is `Cargo.lock`. If other files changed, stop and report them unless they are clearly required by the update and the user approves including them.

8. Commit only the lock file:
   ```bash
   git add Cargo.lock
   git commit -m "$(cat <<'EOF'
chore: update Cargo.lock

EOF
)"
   ```

9. Refresh `main` again, merge with a merge commit, and push:
   ```bash
   update_branch="$(git branch --show-current)"
   git checkout main
   git pull --ff-only origin main
   git merge --no-ff "$update_branch"
   git push origin main
   ```

## Safety Rules

- Never force push.
- Never skip hooks or bypass validation.
- Never merge or push if `cargo update`, `make all`, or the commit fails.
- Never commit unrelated files or generated artifacts other than `Cargo.lock` unless the user explicitly approves.
- If `cargo update` does not change `Cargo.lock`, return to the original branch and stop without creating an update branch.
- If any command fails, stop and report the failed command, relevant output, and the next recommended step.
