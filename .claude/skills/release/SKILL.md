---
name: release
description: >
  Create a versioned release of ghostty-mcp. Bumps the semver version in Cargo.toml,
  updates the changelog, runs CI checks locally, and opens a PR on a release branch.
  Once merged, the tag and Docker publish happen automatically.
  Usage: /release patch (bug fixes), /release minor (new features), /release major (breaking changes).
  Defaults to patch if no argument is given.
argument-hint: patch | minor | major
allowed-tools: Bash, Read, Edit, Grep, Glob
---

# Release

Create a versioned release of ghostty-mcp.

## Usage

```
/release          -- patch bump (0.1.0 -> 0.1.1)
/release patch    -- same as above
/release minor    -- minor bump (0.1.0 -> 0.2.0)
/release major    -- major bump (0.1.0 -> 1.0.0)
```

## Argument

The argument is the semver bump type: `patch`, `minor`, or `major`. Default to `patch` if not provided.

## Steps

1. **Read current version** from `Cargo.toml` (`version = "X.Y.Z"`).
2. **Calculate new version** based on the bump type argument.
3. **Update `Cargo.toml`** with the new version.
4. **Update `server.json`**: set the top-level `version` and the version tag
   in `packages[0].identifier` to the new version (CI fails on drift).
5. **Update `Cargo.lock`** by running `cargo check`.
6. **Update `CHANGELOG.md`** at the project root:
   - Add a new section under `## [Unreleased]` with the new version and today's date.
   - Move any items from `[Unreleased]` into the new version section.
   - If `[Unreleased]` is empty, ask the user what changed before proceeding.
7. **Create branch** `release/vX.Y.Z` from current HEAD.
8. **Run `cargo fmt --check`** -- stop if it fails.
9. **Run `cargo clippy --all-targets --locked -- -D warnings`** -- stop if it fails.
10. **Run `cargo test --locked`** -- stop if it fails.
11. **Commit** all changes with message: `release: vX.Y.Z`
12. **Push** the branch to origin.
13. **Create PR** targeting `main` using `gh pr create`:
    - Title: `Release vX.Y.Z`
    - Body: include the changelog entry for this version.

When the PR is merged, the `release.yml` workflow creates the git tag, the
GitHub Release, and builds + pushes the multi-arch Docker image to GHCR.
