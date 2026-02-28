# ghostty-mcp

MCP server for [Ghostty](https://ghostty.org) terminal configuration. Search, read, write, and validate ~180 configuration options with fuzzy matching.

## Features

- **search_config** — Fuzzy search options by concept (e.g. "transparent", "font size")
- **get_option** — Full documentation for any option
- **list_categories** — Browse 22 categories with counts
- **read_config** — Read current config or specific options
- **write_config** — Set options with type validation
- **remove_config** — Comment out options (preserves for reference)
- **validate_config** — Check for errors, unknown options, type mismatches

## Install

### Option 1: Docker (recommended)

```bash
# One-line install
curl -fsSL https://raw.githubusercontent.com/daniFloresR/ghostty-mcp/main/scripts/install.sh | bash
```

This installs a wrapper script at `~/.local/bin/ghostty-mcp` that auto-updates the Docker image in the background on each Claude Code startup.

### Option 2: Build from source (no Docker)

```bash
cargo install --git https://github.com/daniFloresR/ghostty-mcp
claude mcp add ghostty-mcp --scope user -- ghostty-mcp
```

When running natively, the server auto-detects your config path:
1. `GHOSTTY_CONFIG_PATH` env var (if set)
2. `XDG_CONFIG_HOME/ghostty/config`
3. `~/Library/Application Support/com.mitchellh.ghostty/config` (macOS)
4. `~/.config/ghostty/config` (default)

## Updating

- **Docker**: Updates happen automatically. Each Claude Code startup pulls the latest image in the background. The update takes effect on the next session.
- **Source**: Run `cargo install --force --git https://github.com/daniFloresR/ghostty-mcp` to update manually.

## Usage with Claude Code

Once installed, start a new Claude Code session and ask:

```
> search for transparency options in ghostty
> set background opacity to 0.9
> show me all font-related options
> validate my ghostty config
```

## Development

```bash
git clone https://github.com/daniFloresR/ghostty-mcp
cd ghostty-mcp

# Run tests
cargo test

# Run clippy
cargo clippy --all-targets -- -D warnings

# Build Docker image
docker build -t ghostty-mcp:latest .

# Test locally (reads your actual Ghostty config)
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"0.1.0"}}}' | cargo run
```

## License

MIT
