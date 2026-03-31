#!/usr/bin/env bash
# Setup script for the lazyclaude demo.
# Creates the encoded project directory so memory and sessions resolve correctly.
#
# Usage:
#   cd lazyclaude
#   ./demo/setup.sh
#   lazyclaude --claude-dir demo/claude-home --project-dir demo/project

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CLAUDE_HOME="$SCRIPT_DIR/claude-home"
PROJECT_DIR="$SCRIPT_DIR/project"

# Encode project path the same way lazyclaude does:
# replace all non-alphanumeric characters (except -) with -
ENCODED=$(echo "$PROJECT_DIR" | sed 's/[^a-zA-Z0-9-]/-/g')

PROJ_CONFIG="$CLAUDE_HOME/projects/$ENCODED"

# Copy template data into the encoded project directory
if [ -d "$CLAUDE_HOME/projects/demo-project" ] && [ "$CLAUDE_HOME/projects/demo-project" != "$PROJ_CONFIG" ]; then
    mkdir -p "$PROJ_CONFIG"
    cp -r "$CLAUDE_HOME/projects/demo-project/"* "$PROJ_CONFIG/" 2>/dev/null || true
    echo "Created project config at: $PROJ_CONFIG"
elif [ ! -d "$PROJ_CONFIG" ]; then
    mkdir -p "$PROJ_CONFIG/memory"
    cp -r "$CLAUDE_HOME/projects/demo-project/"* "$PROJ_CONFIG/" 2>/dev/null || true
    echo "Created project config at: $PROJ_CONFIG"
else
    echo "Project config already exists at: $PROJ_CONFIG"
fi

echo ""
echo "Run lazyclaude with:"
echo "  lazyclaude --claude-dir '$CLAUDE_HOME' --project-dir '$PROJECT_DIR'"
echo ""
echo "Or from the repo root:"
echo "  cargo run -- --claude-dir demo/claude-home --project-dir demo/project"
