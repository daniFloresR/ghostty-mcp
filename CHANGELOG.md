# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/).

## [Unreleased]

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
