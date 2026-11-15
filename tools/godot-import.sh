#!/usr/bin/env bash
# Imports the Godot project headless (needed once after cloning and after
# adding assets). Works in bash on macOS, Linux and Git Bash on Windows.
#
# The extension list is seeded first: Godot 4.7.2 crashes at exit when a
# GDExtension that registers classes is discovered during the first scan of a
# fresh project (godotengine/godot#111645, fix pending in #123658).
set -euo pipefail

GODOT="${GODOT:-godot}"
PROJECT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../godot" && pwd)"

mkdir -p "$PROJECT_DIR/.godot"
if [[ ! -f "$PROJECT_DIR/.godot/extension_list.cfg" ]]; then
    echo "res://mud_and_steel.gdextension" > "$PROJECT_DIR/.godot/extension_list.cfg"
fi

"$GODOT" --headless --path "$PROJECT_DIR" --import
