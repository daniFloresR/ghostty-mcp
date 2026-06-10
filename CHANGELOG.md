# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

## [0.5.1] - 2026-06-10

### Fixed
- Docker distribution was broken since v0.1.0: the resolved config path pointed at the mounted directory instead of the `config` file inside it, so every config-touching tool failed with `Is a directory`. A CI smoke test now runs the built image end-to-end against a mounted config directory.
- Enum inference in the docs parser never fired: no option carried `valid_values`, so `write_config` silently accepted invalid values for fixed-set options like `cursor-style`. 38 options now validate as strict enums; special-value options (e.g. `cursor-color`, `working-directory`) keep their open types with the special values preserved as documentation.
- 14 options shipped with empty descriptions; grouped options (such as `font-family-bold`) now inherit their group's documentation, and orphan doc blocks no longer attach to the wrong option.
- CI broke on unchanged code when clippy 1.96 landed (`unnecessary_sort_by` under `-D warnings`); the toolchain is now pinned via `rust-toolchain.toml` and bumped deliberately.
- GitHub Release notes were corrupted by shell interpolation: backticked changelog spans executed as command substitution. Workflow scripts now receive untrusted text through `env`.
- The duplicate-tag guard in the release workflow could never fire (shallow tagless checkout); it now queries the remote.
- The scheduled docs update workflow was auto-disabled after 60 days of repo inactivity and had never succeeded; it now runs weekly with a keepalive, force-pushes its bot-owned branch, and dispatches CI on the PRs it opens.
- `serverInfo` now reports `ghostty-mcp` and the crate version instead of `rmcp/0.16.0`.
- `install.sh` runs the container with `--user` so config writes are not root-owned, and re-running the installer no longer fails at registration.

### Added
- MCP tool handler test suite plus a tool-surface contract test pinning the seven tool names.
- Data invariant tests and a CI gate verifying `data/ghostty-options.json` reproduces from the raw docs.
- Python test suite for `scripts/parse_docs.py`.
- Multi-arch Docker image: `linux/amd64` and `linux/arm64`.
- Declared MSRV (`rust-version = "1.88"`) and Dependabot for cargo and GitHub Actions.

### Changed
- Ghostty option data refreshed from 1.2.3 to 1.3.1 (180 to 200 options).
- tokio trimmed to the features actually used and `panic = "abort"` in release builds: the Docker image shrinks from 2.0MB to 1.5MB.

## [0.5.0] - 2026-02-28

### Added
- Repeatable option support for `write_config`: appends new entries for keybind, palette, and other repeatable options instead of overwriting.
- Per-value removal for `remove_config`: optional `value` parameter to comment out a specific entry from repeatable options.
- `read_config` now shows all values as a list for repeatable options.
- `get_all()` method on `ConfigFile` for retrieving all values of a key.
- `append_option()` in writer for grouped insertion after last occurrence of a key.
- `comment_option_value()` in writer for surgical single-value removal.

### Fixed
- `keybind`, `palette`, `custom-shader`, and `custom-shader-animation` now correctly marked as `repeatable: true` in generated option data.
- Parser heuristic supplemented with explicit override set for repeatable detection.

## [0.4.0] - 2026-02-27

### Added
- Dynamic MCP instructions generated at runtime with workflow patterns, search tips, category summary, and validation guidance.

## [0.3.0] - 2026-02-27

### Added
- Auto-update wrapper script for Docker installations (background pull on each startup).

## [0.2.2] - 2026-02-27

### Fixed
- Lowercase Docker image tags for GHCR compatibility.

## [0.2.1] - 2026-02-27

### Changed
- Merged auto-tag and Docker publish into a single release workflow.
- Added GitHub Release creation with changelog notes extracted automatically.
- Fixed GITHUB_TOKEN cascade limitation that prevented Docker publish on tag.

## [0.2.0] - 2026-02-27

### Added
- Auto-update workflow for Ghostty docs (daily cron + manual trigger).
- Auto-tag workflow that creates git tags when release PRs are merged.
- Portable ghostty binary discovery in parse-docs.sh (env var, macOS app, PATH).
- Version tracking with data/ghostty-version.txt.
- Project rules and /release skill in .claude/.
- CHANGELOG.md.

## [0.1.0] - 2025-06-01

### Added
- Initial release: MCP server for Ghostty terminal configuration.
- Tools: search_config, get_option, list_categories, read_config, write_config, remove_config, validate_config.
- Fuzzy search with synonym support via nucleo-matcher.
- 180 Ghostty options parsed and categorized.
- Docker image under 10MB.
