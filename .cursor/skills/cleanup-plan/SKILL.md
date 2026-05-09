---
name: cleanup-plan
description: Find low-hanging code smells and create a test-first cleanup plan. Use when the user asks to clean up the project, look for smells, formalize a small cleanup PR, or plan cleanup work with tests before refactoring.
---

# Cleanup Plan

## Purpose

Use this skill to create a plan for a small, reviewable cleanup branch. The skill is for planning only: do not implement the cleanup unless the user separately accepts the plan and asks to execute it.

The plan must include a full regression-safety workflow before and after the cleanup. It must commit test-strengthening work separately from the cleanup implementation, and it must end with committing the cleanup branch. Do not include merging into `main` or pushing unless the user explicitly asks for that later.

## Workflow

1. Confirm the repository state.
   - Check the current branch and working tree.
   - If there are unrelated changes, account for them in the plan and do not propose overwriting them.
   - Plan cleanup work from an up-to-date `main`: fetch `origin`, check out `main`, and pull with `--ff-only` before creating the cleanup branch.
   - Create the cleanup branch from the updated `main`, not from an arbitrary feature branch.
   - If the user asked to create a branch immediately, include refreshing `main` and creating the branch as the first execution steps.

2. Look for low-hanging smell candidates.
   - Prefer small, high-confidence issues: duplicated constants, direct process exits in `Result`-returning code, silent fallbacks, production `unwrap`/`expect`, duplicated test helpers, dead code, or inconsistent error handling.
   - Use code search and focused file reads. For broad exploration, use readonly exploration subagents in parallel.
   - Rank candidates by risk/reward and choose one narrow cleanup unless the user asks for a larger sweep.

3. Think through the fix.
   - Identify the smallest behavioral surface to change.
   - Preserve intended CLI/user-facing behavior unless the smell is itself user-facing.
   - Prefer existing project patterns, dependencies, and test style.

4. Create a plan that always includes these phases:
   - Add pre-work integration and/or unit tests if the current regression coverage is weak.
   - Run those new or existing focused tests before the refactor where practical.
   - Run the project validation gate and confirm it is green before changing production behavior.
   - Commit the pre-work test-strengthening changes separately before starting the production cleanup.
   - Fix the smell with the smallest scoped code change.
   - Add extra unit tests for new failure paths or coverage gaps created by the refactor.
   - Run the focused tests and the entire test suite; fix any issues.
   - Commit the cleanup implementation separately, but do not merge or push.

5. Include validation details in the plan.
   - Name the exact commands to run, such as `cargo test -p <package> -- --test-threads=1` or `make all`.
   - If `make all` includes coverage in the repo, call that out and plan a quick coverage sanity check.
   - If tests are known to require serial execution or non-sandboxed permissions, document that in the plan.

6. Include commit guidance.
   - Commit only intended files.
   - Use at least two commits when pre-work tests are added: one commit for regression-safety tests, then one commit for the cleanup implementation and any implementation-specific coverage.
   - Follow the project commit-message rule and use Conventional Commits-style subjects.
   - Prefer `test:` for the pre-work test commit and `cleanup:` or `refactor:` for the cleanup implementation commit.
   - Exclude broad formatting, generated reports, coverage artifacts, or validation side effects unless they are the actual goal.
   - Stop after the final cleanup commit and report the branch, commit hashes, tests run, and any residual risk.

## Plan Template

Use this structure for the final plan:

```markdown
# Cleanup <Area>

## Chosen Smell

<Describe the smell, why it matters, and why it is low risk/high reward. Include file paths.>

## Proposed Branch

`<branch-name>`

Branch setup:

1. `git fetch origin`
2. `git checkout main`
3. `git pull --ff-only origin main`
4. `git checkout -b <branch-name>`

## Regression Safety First

1. Add or confirm focused unit/integration coverage before refactoring.
2. Run the focused tests.
3. Run the project validation gate and sanity-check coverage if available.
4. Commit the test-strengthening changes before changing production code using a Conventional Commits-style subject, usually `test:`.

## Cleanup Steps

1. Make the smallest scoped code change.
2. Add extra unit tests for new failure paths or coverage gaps.
3. Run focused tests and the full suite.
4. Clean up unrelated validation side effects.
5. Commit the cleanup implementation separately from the pre-work tests using a Conventional Commits-style subject, usually `cleanup:` or `refactor:`.

## Out Of Scope

- Do not merge into `main`.
- Do not push.
- Do not include unrelated formatting or generated artifacts.
```

## Safety Rules

- In plan mode, do not edit non-markdown files or run mutating commands unless the user has explicitly switched to execution.
- Never include a merge or push step in the plan unless the user explicitly asks for it.
- Never skip tests to make a cleanup easier.
- Never commit unrelated user changes.
