# Project Rules

## Language
- The official language of this project is English. All code, comments, commit messages, PR descriptions, and communication must be in English, regardless of the user's language.

## Style
- No emojis anywhere — code, comments, commits, PRs, communication.

## Commits
- Never add AI attribution or "Co-Authored-By" lines to commits.
- Use conventional commits: `feat:`, `fix:`, `ci:`, `chore:`, `release:`, etc.

## Project Overview
- MCP server that gives Claude full access to Ghostty terminal configuration: search, read, write, and validate ~180 options.
- Built with Rust (rmcp, tokio, serde, nucleo-matcher). Data generated with Python.

## Data Pipeline
- `ghostty +show-config --default --docs` → `data/ghostty-docs-raw.txt` → `scripts/parse_docs.py` → `data/ghostty-options.json` → embedded in Rust binary at compile time.
- The JSON is generated — never edit it by hand. Change the parser instead.

## Project Layout
- `src/` — Rust MCP server (server, search, tools/, config/, data/)
- `data/` — Generated JSON + raw docs + version tracker
- `scripts/` — parse_docs.py (parser), parse-docs.sh (orchestrator), install.sh

## Constraints
- Docker image must stay under 10MB.
- `cargo clippy -- -D warnings` must pass with zero warnings.
- CI runs fmt, clippy, tests, and docker size check on every push/PR.
- The Rust toolchain is pinned in `rust-toolchain.toml` (keep the Dockerfile base image tag in sync). Bump it deliberately in a dedicated PR; never float on latest stable.

## Testing
- `cargo test` for unit/integration tests.
- The project follows strict TDD: write failing tests first, then the minimal code to pass them.
- CI workflow: `.github/workflows/ci.yml`.

## Release Flow
- Use `/release patch|minor|major` to create a release PR.
- Merging a `release/v*` PR to main triggers `release.yml`, which creates the git tag, the GitHub Release, and builds + pushes the Docker image to GHCR in a single job.
- Never push tags manually -- let the automation handle it.
