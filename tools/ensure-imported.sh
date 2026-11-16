#!/usr/bin/env bash
# Runs the headless Godot import when a script declares a `class_name` that is
# missing from Godot's global class cache, or an asset file has no .import file
# yet (e.g. after a merge or a fresh clone). Without it, scripts that use the
# new class fail to parse and new sounds or models do not load.
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
# Asset files Godot imports (audio, images, models) whose imported data in
# godot/.godot/imported is missing or older than the file (new or rebuilt
# files after a merge: the .import file is committed, the imported data is not).
if [[ $missing == 0 ]]; then
    while IFS= read -r file; do
        if [[ ! -f "$file.import" ]]; then
            echo "$file is not imported yet"
            missing=1
            break
        fi
        dest=$(grep -m1 '^path' "$file.import" | sed -E 's/^path[^=]*="res:\/\/(.*)"$/\1/')
        if [[ -n $dest && ! -f "godot/$dest" ]]; then
            echo "$file is not imported yet"
            missing=1
            break
        fi
        # Newer than its imported data: compare with the source checksum
        # Godot recorded (a checkout touches files without changing them);
        # an unchanged file only gets its imported data touched.
        if [[ -n $dest && "$file" -nt "godot/$dest" ]]; then
            recorded=$(sed -n 's/^source_md5="\(.*\)"$/\1/p' "godot/${dest%.*}.md5" 2>/dev/null)
            actual=$( (md5 -q "$file" 2>/dev/null || md5sum "$file") | cut -d' ' -f1)
            if [[ "$recorded" != "$actual" ]]; then
                echo "$file changed since its import"
                missing=1
                break
            fi
            touch "godot/$dest"
        fi
    done < <(find godot -path godot/.godot -prune -o -type f \( -name '*.wav' -o -name '*.ogg' -o -name '*.mp3' -o -name '*.png' -o -name '*.glb' -o -name '*.jpg' -o -name '*.webp' \) -print)
fi
if [[ $missing == 1 ]]; then
    echo "Running the Godot import"
    bash tools/godot-import.sh
fi
