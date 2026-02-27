#!/usr/bin/env bash
set -euo pipefail

IMAGE="ghcr.io/danifloresr/ghostty-mcp:latest"

echo "ghostty-mcp installer"
echo "====================="

# Detect Ghostty config directory
if [ -n "${GHOSTTY_CONFIG_PATH:-}" ]; then
    CONFIG_DIR="$(dirname "$GHOSTTY_CONFIG_PATH")"
    CONFIG_FILE="$GHOSTTY_CONFIG_PATH"
elif [ -f "$HOME/Library/Application Support/com.mitchellh.ghostty/config" ]; then
    CONFIG_DIR="$HOME/Library/Application Support/com.mitchellh.ghostty"
    CONFIG_FILE="$CONFIG_DIR/config"
elif [ -n "${XDG_CONFIG_HOME:-}" ] && [ -d "$XDG_CONFIG_HOME/ghostty" ]; then
    CONFIG_DIR="$XDG_CONFIG_HOME/ghostty"
    CONFIG_FILE="$CONFIG_DIR/config"
elif [ -d "$HOME/.config/ghostty" ]; then
    CONFIG_DIR="$HOME/.config/ghostty"
    CONFIG_FILE="$CONFIG_DIR/config"
else
    CONFIG_DIR="$HOME/.config/ghostty"
    CONFIG_FILE="$CONFIG_DIR/config"
    echo "No Ghostty config found, will use: $CONFIG_FILE"
fi

echo "Config directory: $CONFIG_DIR"

# Check Docker
if ! command -v docker &>/dev/null; then
    echo "Error: Docker is required. Install from https://docker.com"
    exit 1
fi

# Check Claude Code
if ! command -v claude &>/dev/null; then
    echo "Error: Claude Code CLI is required. Install from https://claude.ai/claude-code"
    exit 1
fi

# Pull image
echo "Pulling $IMAGE..."
docker pull "$IMAGE"

# Register with Claude Code
echo "Registering MCP server..."
claude mcp add ghostty-mcp --scope user -- \
    docker run -i --rm -v "$CONFIG_DIR:/config/ghostty" "$IMAGE"

echo ""
echo "Done! Start a new Claude Code session and try:"
echo '  "search for transparency options in ghostty"'
