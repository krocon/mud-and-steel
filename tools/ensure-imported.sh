#!/usr/bin/env bash
# Runs the headless Godot import when a script declares a `class_name` that is
# missing from Godot's global class cache (e.g. after a merge or a fresh clone).
# Without it, scripts that use the new class fail to parse at startup.
set -euo pipefail
cd "$(dirname "$0")/.."
cache=godot/.godot/global_script_class_cache.cfg
missing=0
if [[ ! -f $cache ]]; then
    missing=1
else
    while IFS= read -r name; do
        if ! grep -q "&\"$name\"" "$cache"; then
            echo "class_name $name is not imported yet"
            missing=1
            break
        fi
    done < <(grep -rhoE '^class_name [A-Za-z_][A-Za-z0-9_]*' godot --include='*.gd' --exclude-dir=.godot | awk '{print $2}' | sort -u)
fi
if [[ $missing == 1 ]]; then
    echo "Running the Godot import"
    bash tools/godot-import.sh
fi
