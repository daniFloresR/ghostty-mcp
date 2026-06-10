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

# Pull image (blocking — guarantees image exists on first run)
echo "Pulling $IMAGE..."
docker pull "$IMAGE"

# Generate wrapper script for auto-updates
WRAPPER="$HOME/.local/bin/ghostty-mcp"
echo "Installing wrapper script at $WRAPPER..."
mkdir -p "$HOME/.local/bin"
cat > "$WRAPPER" << 'WRAPPER_EOF'
#!/usr/bin/env bash
set -euo pipefail

IMAGE="ghcr.io/danifloresr/ghostty-mcp:latest"

# Detect Ghostty config directory
if [ -n "${GHOSTTY_CONFIG_PATH:-}" ]; then
    CONFIG_DIR="$(dirname "$GHOSTTY_CONFIG_PATH")"
elif [ -f "$HOME/Library/Application Support/com.mitchellh.ghostty/config" ]; then
    CONFIG_DIR="$HOME/Library/Application Support/com.mitchellh.ghostty"
elif [ -n "${XDG_CONFIG_HOME:-}" ] && [ -d "$XDG_CONFIG_HOME/ghostty" ]; then
    CONFIG_DIR="$XDG_CONFIG_HOME/ghostty"
elif [ -d "$HOME/.config/ghostty" ]; then
    CONFIG_DIR="$HOME/.config/ghostty"
else
    CONFIG_DIR="$HOME/.config/ghostty"
fi

# Pull latest in background (updates image for next session)
docker pull "$IMAGE" &>/dev/null &

# Run with current cached image. --user makes files written to the bind
# mount owned by the invoking user instead of root.
exec docker run -i --rm --user "$(id -u):$(id -g)" -v "$CONFIG_DIR:/config/ghostty" "$IMAGE"
WRAPPER_EOF
chmod +x "$WRAPPER"

# Register with Claude Code (points to wrapper, not direct docker run).
# Remove any previous registration first so re-running the installer works.
echo "Registering MCP server..."
claude mcp remove ghostty-mcp --scope user >/dev/null 2>&1 || true
claude mcp add ghostty-mcp --scope user -- "$WRAPPER"

echo ""
echo "Done! Start a new Claude Code session and try:"
echo '  "search for transparency options in ghostty"'
