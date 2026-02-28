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
- CI runs clippy, tests, and docker size check on every push/PR.

## Testing
- `cargo test` for unit/integration tests.
- CI workflow: `.github/workflows/ci.yml`.

## Release Flow
- Use `/release patch|minor|major` to create a release PR.
- Merging a `release/v*` PR to main auto-creates the git tag (`auto-tag.yml`).
- The tag push triggers Docker build + push to GHCR (`release.yml`).
- Never push tags manually -- let the automation handle it.
