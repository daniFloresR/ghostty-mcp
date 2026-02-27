#!/bin/bash
# Parse Ghostty docs output into structured JSON
# Usage: ./scripts/parse-docs.sh [output_file]
#
# Requires: ghostty CLI available, or raw docs at data/ghostty-docs-raw.txt

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
OUTPUT="${1:-$PROJECT_DIR/data/ghostty-options.json}"
RAW_DOCS="$PROJECT_DIR/data/ghostty-docs-raw.txt"

# Generate raw docs if not present
if [ ! -f "$RAW_DOCS" ]; then
    echo "Generating raw docs from ghostty CLI..."
    GHOSTTY="/Applications/Ghostty.app/Contents/MacOS/ghostty"
    if [ ! -x "$GHOSTTY" ]; then
        echo "Error: ghostty not found at $GHOSTTY"
        exit 1
    fi
    "$GHOSTTY" +show-config --default --docs > "$RAW_DOCS" 2>/dev/null
fi

echo "Parsing docs from $RAW_DOCS..."
echo "Output: $OUTPUT"

python3 "$SCRIPT_DIR/parse_docs.py" "$RAW_DOCS" "$OUTPUT"
