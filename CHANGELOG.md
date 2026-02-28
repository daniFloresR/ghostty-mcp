# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

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
