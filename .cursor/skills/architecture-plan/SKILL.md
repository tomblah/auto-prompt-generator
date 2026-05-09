---
name: architecture-plan
description: Find narrow architectural improvement opportunities and create a test-first architecture plan. Use when the user asks to improve architecture, reduce tight coupling, correct patterns, improve clunky APIs, or plan a small architectural refactor with regression safety.
---

# Architecture Plan

## Purpose

Use this skill to create a plan for a small, reviewable architecture-improvement branch. The skill is for planning only: do not implement the architecture change unless the user separately accepts the plan and asks to execute it.

The plan must include a full regression-safety workflow before and after the architecture change. It must commit characterization or test-strengthening work separately from the implementation, and it must end with committing the architecture branch. Do not include merging into `main` or pushing unless the user explicitly asks for that later.

## Workflow

1. Confirm the repository state.
   - Check the current branch and working tree.
   - If there are unrelated changes, account for them in the plan and do not propose overwriting them.
   - Plan architecture work from an up-to-date `main`: fetch `origin`, check out `main`, and pull with `--ff-only` before creating the architecture branch.
   - Create the architecture branch from the updated `main`, not from an arbitrary feature branch.
   - If the user asked to create a branch immediately, include refreshing `main` and creating the branch as the first execution steps.

2. Look for narrow architecture candidates.
   - Prefer small, high-confidence issues: tight coupling, wrong dependency direction, leaky abstractions, clunky APIs, duplicated orchestration, unclear ownership boundaries, inconsistent patterns, or modules that know too much about each other.
   - Avoid broad rewrites, framework swaps, speculative layering, or style-only reorganizations.
   - Use code search, dependency/call-site mapping, focused file reads, and readonly exploration subagents in parallel for broad exploration.
   - Rank candidates by risk/reward and choose one narrow architectural slice unless the user asks for a larger design effort.

3. Map the architectural boundary before choosing the fix.
   - Identify producers, consumers, direct callers, integration tests, CLI/user-facing behavior, persisted data, public APIs, and cross-crate/module contracts.
   - Classify the change as behavior-preserving, contract-preserving with internal API changes, or contract-changing.
   - If the change is contract-changing, name the old contract, the proposed new contract, the migration impact, and why compatibility is or is not required.
   - Prefer existing project patterns, dependencies, and test style over inventing a new abstraction.

4. Create a plan that always includes these phases:
   - Add characterization tests for current behavior when coverage is weak or behavior is subtle.
   - Add or confirm focused unit and integration coverage around the architectural boundary.
   - Run the focused tests before the architecture change where practical.
   - Run the project validation gate and confirm it is green before changing production architecture.
   - Commit characterization and test-strengthening changes separately before starting the architecture change.
   - Make the smallest scoped architecture change that fixes the chosen smell.
   - Update or add integration tests only when the boundary contract is intentionally changed or currently untested.
   - Run focused tests and the entire test suite; fix any issues.
   - Commit the architecture implementation separately, but do not merge or push.

5. Include validation details in the plan.
   - Name exact commands to run, such as `cargo test -p <package> -- --test-threads=1`, targeted integration tests, `make test`, or `make all`.
   - If `make all` includes coverage in the repo, call that out and plan a quick coverage sanity check.
   - If tests are known to require serial execution, writable temp directories, git, network, or non-sandboxed permissions, document that in the plan.

6. Include commit guidance.
   - Commit only intended files.
   - Use at least two commits when characterization or pre-work tests are added: one commit for regression-safety tests, then one commit for the architecture implementation and any implementation-specific coverage.
   - Follow the project commit-message rule and use Conventional Commits-style subjects.
   - Prefer `test:` for characterization/test-strengthening commits and `refactor:`, `cleanup:`, or `feat:` for the architecture implementation depending on whether contracts change.
   - Exclude broad formatting, generated reports, coverage artifacts, or validation side effects unless they are the actual goal.
   - Stop after the final architecture commit and report the branch, commit hashes, tests run, and any residual risk.

## Architecture Risk Assessment

Every plan must include this assessment before the implementation steps:

- Boundary: Which module, crate, API, CLI path, or workflow changes?
- Consumers: Which callers, tests, commands, or users depend on the current shape?
- Contract classification: behavior-preserving, contract-preserving with internal API changes, or contract-changing.
- Compatibility: Whether the old shape must continue to work, and why.
- Test strategy: Which characterization, unit, and integration tests prove the change is safe.
- Rollback risk: What would be hard to undo if the design is wrong.

## Characterization Tests

Characterization tests are ordinary tests that capture what the code does today before changing the design. They do not mean the current behavior is ideal; they protect against accidental behavior changes while refactoring architecture.

Use characterization tests when behavior is under-tested, spread across modules, or surprising. Prefer integration tests when the behavior crosses a public crate, CLI, process, storage, or user-facing boundary. Prefer unit tests when the behavior is local and the architectural change is internal.

## Integration Test Guidance

- If the architecture change is behavior-preserving, integration tests should usually remain stable and prove the external behavior did not move.
- If the change modifies a public or cross-module contract, integration tests may need to change, but the plan must explicitly name the old contract, new contract, migration path, and why the change is worth it.
- If coverage is weak around a risky boundary, add characterization coverage before changing production code, then add implementation-specific coverage after the design change.

## Plan Template

Use this structure for the final plan:

```markdown
# Architecture <Area>

## Chosen Architecture Smell

<Describe the tight coupling, incorrect pattern, clunky API, leaky abstraction, duplicated orchestration, or unclear ownership. Include file paths and why this is the right small slice.>

## Architecture Risk Assessment

- Boundary: <module/crate/API/CLI/workflow>
- Consumers: <callers/tests/commands/users>
- Contract classification: <behavior-preserving | contract-preserving with internal API changes | contract-changing>
- Compatibility: <what must remain compatible, or why compatibility is not needed>
- Test strategy: <characterization/unit/integration coverage>
- Rollback risk: <main residual design risk>

## Proposed Branch

`<branch-name>`

Branch setup:

1. `git fetch origin`
2. `git checkout main`
3. `git pull --ff-only origin main`
4. `git checkout -b <branch-name>`

## Regression Safety First

1. Add or confirm characterization, unit, and integration coverage around the boundary before changing architecture.
2. Run the focused tests.
3. Run the project validation gate and sanity-check coverage if available.
4. Commit characterization and test-strengthening changes before changing production architecture using a Conventional Commits-style subject, usually `test:`.

## Architecture Steps

1. Make the smallest scoped architecture change.
2. Update call sites and integration tests only for intentional contract changes.
3. Add extra tests for new failure paths, migration behavior, or coverage gaps.
4. Run focused tests and the full suite.
5. Clean up unrelated validation side effects.
6. Commit the architecture implementation separately from the pre-work tests using a Conventional Commits-style subject, usually `refactor:`, `cleanup:`, or `feat:`.

## Out Of Scope

- Do not merge into `main`.
- Do not push.
- Do not include broad rewrites, speculative abstractions, unrelated formatting, or generated artifacts.
```

## Safety Rules

- In plan mode, do not edit non-markdown files or run mutating commands unless the user has explicitly switched to execution.
- Never include a merge or push step in the plan unless the user explicitly asks for it.
- Never skip tests to make an architecture change easier.
- Never commit unrelated user changes.
- Do not propose contract-breaking changes without an explicit migration and compatibility decision.
