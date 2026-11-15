# Project commands. Run `just` to list them.
#
# Override tool paths with environment variables, e.g.
#   GODOT=/path/to/godot just run
#   BLENDER=/path/to/blender just model-tank

set windows-shell := ["powershell.exe", "-NoLogo", "-NoProfile", "-Command"]

export GODOT := env("GODOT", "godot")
BLENDER := env("BLENDER", if os() == "macos" { "/Applications/Blender.app/Contents/MacOS/Blender" } else { "blender" })

# PowerShell needs the call operator to run a quoted executable path.
CALL := if os() == "windows" { "& " } else { "" }

# List all commands
[private]
default:
    @just --list --unsorted

# Format, lint and test exactly like CI
[group('rust')]
check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo test --workspace

# Format all Rust code
[group('rust')]
fmt:
    cargo fmt

# Run all tests
[group('rust')]
test:
    cargo test --workspace

# Run the headless simulation and print the state hash
[group('rust')]
sim ticks="1000" seed="42":
    cargo run -p cli -- run --ticks {{ ticks }} --seed {{ seed }}

# Regenerate docs/UNIT_CATALOG.md from data/catalog/*.toml
[group('rust')]
catalog-doc:
    cargo run -q -p cli -- catalog-doc

# Validate the unit catalog (all files, or the given ones)
[group('rust')]
catalog-check *files:
    cargo run -q -p cli -- catalog-check {{ files }}

# Build the GDExtension (debug)
[group('godot')]
build:
    cargo build -p bridge

# Import the Godot project headless (once after cloning and after adding assets; needs bash, on Windows Git Bash)
[group('godot')]
import:
    bash tools/godot-import.sh

# Build the bridge and start the game
[group('godot')]
run: build
    {{ GODOT }} --path godot

# Build the bridge and open the Godot editor
[group('godot')]
editor: build
    {{ GODOT }} --path godot -e

# Build the bridge and run the headless self-test (exit code 0 = passed)
[group('godot')]
selftest: build
    {{ GODOT }} --path godot --headless --fixed-fps 60 -- --self-test

# Export a .blend file to godot/assets/models/<name>.glb
[group('assets')]
export-model blend:
    {{ CALL }}"{{ BLENDER }}" -b "{{ blend }}" --python tools/export_blender.py

# Regenerate the placeholder tank from primitives and export it
[group('assets')]
model-tank:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python tools/create_placeholder_tank.py
    just export-model assets-src/blender/placeholder_tank.blend

