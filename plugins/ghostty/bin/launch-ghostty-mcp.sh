#!/usr/bin/env bash
# Launcher used by the Claude Code plugin: runs the published ghostty-mcp
# Docker image with the host Ghostty config directory mounted, mirroring
# scripts/install.sh. Kept dependency-free so the plugin works anywhere
# Docker does.
set -euo pipefail

IMAGE="ghcr.io/danifloresr/ghostty-mcp:latest"

if ! command -v docker >/dev/null 2>&1; then
    echo "ghostty plugin: Docker is required to run ghostty-mcp." >&2
    echo "Install it from https://docker.com, or install the server natively:" >&2
    echo "  cargo install --git https://github.com/daniFloresR/ghostty-mcp" >&2
    exit 1
fi

# Detect the Ghostty config directory (same chain the server itself uses)
if [ -n "${GHOSTTY_CONFIG_PATH:-}" ]; then
    CONFIG_DIR="$(dirname "$GHOSTTY_CONFIG_PATH")"
elif [ -f "$HOME/Library/Application Support/com.mitchellh.ghostty/config" ]; then
    CONFIG_DIR="$HOME/Library/Application Support/com.mitchellh.ghostty"
elif [ -n "${XDG_CONFIG_HOME:-}" ] && [ -d "$XDG_CONFIG_HOME/ghostty" ]; then
    CONFIG_DIR="$XDG_CONFIG_HOME/ghostty"
else
    CONFIG_DIR="$HOME/.config/ghostty"
fi

# Pre-create the directory: if Docker creates a missing bind source on
# Linux it ends up root-owned and writes fail under --user.
mkdir -p "$CONFIG_DIR"

# Refresh the image in the background; this session keeps the cached one.
docker pull "$IMAGE" >/dev/null 2>&1 &

exec docker run -i --rm --user "$(id -u):$(id -g)" -v "$CONFIG_DIR:/config/ghostty" "$IMAGE"
