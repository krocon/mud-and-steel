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
    cargo clippy -p bridge --no-default-features --all-targets -- -D warnings
    cargo clippy -p cli --features steam --all-targets -- -D warnings
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

# Print the prices and production times of the economy (prompt 023, data/economy.toml)
[group('rust')]
economy-table:
    cargo run -q -p cli -- economy-table

# Validate the unit catalog (all files, or the given ones)
[group('rust')]
catalog-check *files:
    cargo run -q -p cli -- catalog-check {{ files }}

# Run the demo headless and write a replay file
[group('replay')]
record file ticks="1000" seed="42":
    cargo run -q -p cli -- run --ticks {{ ticks }} --seed {{ seed }} --record "{{ file }}"

# Play a replay headless and check its end hash (exit code 1 on mismatch)
[group('replay')]
replay file:
    cargo run -q -p cli -- replay "{{ file }}"

# Check a saved game (prompt 038): restore its snapshot and re-simulate its replay; both must agree
[group('replay')]
save-check file:
    cargo run -q -p cli -- save-check "{{ file }}"

# What loading costs (prompt 038): re-simulating a replay vs. writing and restoring a snapshot, e.g. `just save-bench --scenario data/scenarios/skirmish.toml`
[group('replay')]
save-bench *args:
    cargo run -q --release -p cli -- save-bench {{ args }}

# Build the GDExtension (debug)
[group('godot')]
build: && (_steam-lib "debug")
    cargo build -p bridge

# Copy the Steam API library (from the steamworks-sys build) next to the
# bridge, where it is loaded from (prompt 026, docs/ONLINE.md)
[private]
[unix]
_steam-lib profile:
    #!/usr/bin/env sh
    lib=$(ls -t $(find target/{{ profile }}/build -path '*steamworks-sys*' -name 'libsteam_api.*' 2>/dev/null) 2>/dev/null | head -n 1)
    if [ -n "$lib" ]; then cp "$lib" target/{{ profile }}/; fi

[private]
[windows]
_steam-lib profile:
    Get-ChildItem target/{{ profile }}/build -Recurse -Filter steam_api64.dll -ErrorAction SilentlyContinue | Where-Object FullName -like '*steamworks-sys*' | Sort-Object LastWriteTime -Descending | Select-Object -First 1 | Copy-Item -Destination target/{{ profile }}/

# Import the Godot project headless (once after cloning and after adding assets; needs bash, on Windows Git Bash)
[group('godot')]
import:
    bash tools/godot-import.sh

# Build the bridge and start the game (main menu; `just run --skip-menu` starts the test match directly)
[group('godot')]
run *args: build
    {{ GODOT }} --path godot {{ if args != "" { "-- " + args } else { "" } }}

# Build the bridge and open the Godot editor
[group('godot')]
editor: build
    {{ GODOT }} --path godot -e

# Build the bridge and run the headless self-test: menu, settings, three matches (one against the computer), result (exit code 0 = passed)
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

# Regenerate the 1.80 m reference figure and scale bars
[group('assets')]
model-scale-reference:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python tools/blender/scale_reference.py

# --- Prompt 004: part kit, blueprints, validation, gallery -------------------

# Directory for preview renders (turntables, line-up); not part of the repository
RENDER_DIR := env("RENDER_DIR", "build/renders")

# Build one unit from its blueprint: .blend, .glb and a turntable preview
[group('assets')]
unit faction id:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python tools/blender/build_unit.py -- {{ faction }} {{ id }} --render "{{ RENDER_DIR }}"

# Build every unit that has a blueprint in assets-src/blueprints/
[group('assets')]
units:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python tools/blender/build_all.py -- --render "{{ RENDER_DIR }}"

# Check all built models against the catalog and docs/ASSETS.md
[group('assets')]
validate-assets:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python tools/blender/validate.py
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/validate_soldier.py
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_squad.py -- --check
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/validate_env.py

# Render all built models side by side with the 1.80 m reference figure
[group('assets')]
lineup:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python tools/blender/lineup.py -- "{{ RENDER_DIR }}/lineup.png"

# Smoke test of every kit part (builds demo vehicles without saving)
[group('assets')]
kit-test:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python tools/blender/kit_test.py

# Build the bridge and open the model gallery (all models, camouflage switch)
[group('godot')]
gallery: build
    {{ GODOT }} --path godot res://scenes/tools/model_gallery.tscn

# Headless check of the model gallery: loads every model in every camouflage
[group('godot')]
gallery-check: build
    {{ GODOT }} --path godot --headless res://scenes/tools/model_gallery.tscn -- --self-test

# --- Infantry (prompt 005) -----------------------------------------------------------

# Build one soldier: .blend + godot/assets/models/<faction>/infantry/<role>.glb
[group('infantry')]
soldier faction role biome="forest":
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_soldier.py -- {{ faction }} {{ role }} {{ biome }}

# Build every soldier of every faction (see tools/blender/soldier/loadouts.py)
[group('infantry')]
soldiers:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_soldier.py -- --all

# Validate all soldiers (height 1.80 m +- 2 %, bone names, sockets, budgets, clips)
[group('infantry')]
validate-soldiers *files:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/validate_soldier.py -- {{ files }}

# Render the soldier line-up with tank and reference figure into a scratch directory
[group('infantry')]
soldier-lineup out="build/lineup":
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/soldier_lineup.py -- {{ out }}

# Open the infantry preview (all soldiers, biomes, animations)
[group('infantry')]
infantry-preview: build
    {{ GODOT }} --path godot res://scenes/tools/infantry_preview.tscn

# Headless check of all soldier models in Godot (clips, skeleton, sockets, biomes)
[group('infantry')]
infantry-selftest: build
    {{ GODOT }} --path godot --headless res://scenes/tools/infantry_preview.tscn -- --self-test

# --- Squads (prompt 006) ------------------------------------------------------------

# Build one infantry squad scene from assets-src/blueprints/<faction>/squads/<id>.toml
[group('infantry')]
squad faction id:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_squad.py -- {{ faction }} {{ id }}

# Build every squad scene (all factions)
[group('infantry')]
squads:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_squad.py -- --all

# Check all squads against the catalog (squad size, roles, built soldiers, up to date)
[group('infantry')]
validate-squads:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_squad.py -- --check

# --- Maps (prompt 010) ---------------------------------------------------------------

# Generate a test map from a seed into data/maps/<name>/ (see docs/MAPS.md)
[group('maps')]
gen-map name size_m="500" seed="1":
    cargo run -q --release -p cli -- gen-map --name {{ name }} --size-m {{ size_m }} --seed {{ seed }}

# Regenerate both committed test maps (changes the golden hash if test_small changes)
[group('maps')]
test-maps:
    just gen-map test_small 500 7
    just gen-map test_large 4000 11

# Load a map and report load time, memory, height range and terrain shares
[group('maps')]
map-info name="test_large":
    cargo run -q --release -p cli -- map-info data/maps/{{ name }}

# Check a map like the map editor: loading, start positions, paths between the starts
[group('maps')]
map-check name="test_small":
    cargo run -q --release -p cli -- map-check data/maps/{{ name }}

# Build the bridge and open the map editor (prompt 028); args e.g. --edit=test_small
[group('maps')]
map-editor *args: build
    {{ GODOT }} --path godot res://scenes/editor/map_editor.tscn {{ if args != "" { "-- " + args } else { "" } }}

# Headless self-test of the map editor: build a map, save, play it from the menu, delete it
[group('maps')]
editor-selftest: build
    {{ GODOT }} --path godot --headless --fixed-fps 60 -- --editor-self-test

# Build the bridge and start the game on a map from data/maps/
[group('maps')]
run-map name="test_large": build
    {{ GODOT }} --path godot -- --map={{ name }}

# --- Environment objects (prompt 007) ----------------------------------------------

# Build environment objects from assets-src/blueprints/neutral/ (all, or the given ids)
[group('assets')]
env *ids:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_env.py -- {{ ids }}

# Check environment objects only (also part of validate-assets)
[group('assets')]
validate-env *ids:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/validate_env.py -- {{ ids }}

# Render environment objects with the 1.80 m figure and a tank into a scratch folder
[group('assets')]
env-lineup out="build/renders/env" *ids:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/env_lineup.py -- "{{ out }}" {{ ids }}

# --- Tier-2 kit (prompt 008) ---------------------------------------------------------

# Render the kit-test demo units (casemate, armored car, AA gun, mortar, aircraft, forts) into a folder
[group('assets')]
kit-test-render out="build/renders/kit":
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/kit_test.py -- --render "{{ out }}"

# List the soldier roles of a faction (weapon, built?) and the infantry entries without squad file
[group('infantry')]
roles *factions:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_squad.py -- --roles {{ factions }}

# Build every soldier role added in prompt 008 for every faction that has it
[group('infantry')]
soldiers-tier2:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/build_soldier.py -- --tier2

# --- Pathfinding (prompt 012) ---------------------------------------------------------

# Benchmark: units with simultaneous path requests on a map, time per tick (group = units per order)
[group('pathfinding')]
path-bench map="test_large" units="200" group="1":
    cargo run -q --release -p cli -- path-bench --map data/maps/{{ map }} --units {{ units }} --group {{ group }}

# Benchmark (prompt 021): craters among moving units of two players, time per tick
[group('pathfinding')]
crater-bench map="test_large" craters="100" per_tick="5" range_m="200":
    cargo run -q --release -p cli -- crater-bench --map data/maps/{{ map }} --craters {{ craters }} --per-tick {{ per_tick }} --range-m {{ range_m }}

# Benchmark (prompt 018): moving units of two players with vision, time per tick
[group('pathfinding')]
vision-bench map="test_large" units="500" ticks="600":
    cargo run -q --release -p cli -- vision-bench --map data/maps/{{ map }} --units {{ units }} --ticks {{ ticks }}

# Overall benchmark (prompt 016): two Hard AIs play data/scenarios/bench_large.toml (504 units on test_large); time per tick of simulation, phases, AI and snapshot; e.g. `just bench --runs 3`
[group('performance')]
bench *args:
    cargo run -q --release -p cli -- bench {{ args }}

# Frame rate with the bench scenario (prompt 016): standing and fighting armies from three camera distances (opens a window; extra args e.g. --quality=low)
[group('performance')]
bench-view *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --scenario=bench_large.toml --ai=hard --perf-bench {{ args }}

# Benchmark infantry in the simulation: 504 soldiers stand, march and fight (prompt 022)
[group('infantry')]
infantry-bench ticks="1200":
    cargo run -q --release -p cli -- infantry-bench --ticks {{ ticks }}

# Benchmark drawing 504 soldiers in Godot: frame time per camera distance (prompt 022)
[group('infantry')]
infantry-view-bench: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --scenario=infantry_bench.toml --infantry-bench

# Computer players against each other over many seeds (prompt 024), e.g. `just ai-tournament --a hard --b normal --factions su,su`; default: every difficulty against every lower one
[group('ai')]
ai-tournament *args="--matrix":
    cargo run -q --release -p cli -- ai-tournament {{ args }}

# Build the bridge and start the game with the path debug view on (F4 costs, F5 paths)
[group('pathfinding')]
run-paths map="test_small": build
    {{ GODOT }} --path godot -- --map={{ map }} --path-debug

# --- Terrain rendering (prompt 011) --------------------------------------------------

# Regenerate the procedural ground textures in godot/assets/textures/terrain/ (then: just import)
[group('terrain')]
terrain-textures size="1024" preview="":
    cargo run -q --release -p cli -- gen-terrain-textures --size {{ size }} {{ if preview != "" { "--preview " + preview } else { "" } }}

# Measure frame rate and draw calls of the terrain at 1080p (opens a window; extra args e.g. --quality=low --biome=winter)
[group('terrain')]
terrain-bench map="test_large" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --terrain-bench --map={{ map }} {{ args }}

# Record fixed terrain views (near, middle, overview, edge, LOD, crater) as PNG frames into a folder (extra args e.g. --biome=desert)
[group('terrain')]
terrain-shots dir map="test_large" *args: build
    {{ GODOT }} --path godot --always-on-top --fixed-fps 30 --write-movie "{{ dir }}/f.png" res://scenes/main.tscn -- --terrain-shots --map={{ map }} {{ args }}

# --- Map objects (prompt 013) ---------------------------------------------------------

# Measure frame rate, draw calls and instances over the densest wood and a village (opens a window)
[group('terrain')]
objects-bench map="test_large": build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --objects-bench --map={{ map }}

# Record fixed views of woods, village, meadow and hedgerow as PNG frames into a folder
[group('terrain')]
objects-shots dir map="test_large": build
    {{ GODOT }} --path godot --always-on-top --fixed-fps 30 --write-movie "{{ dir }}/f.png" res://scenes/main.tscn -- --objects-shots --map={{ map }}

# --- Effects (prompt 019) ------------------------------------------------------------

# Show every effect in fixed views and save PNG screenshots into a folder (also desert, winter)
[group('godot')]
effects-shots dir: build
    {{ GODOT }} --path godot --always-on-top --fixed-fps 30 res://scenes/main.tscn -- "--effects-shots={{ dir }}"

# Record base building (preview, construction site, production bar, bases) as PNG files into a folder (prompt 023)
[group('godot')]
base-shots dir: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --mode=base --ai=normal "--base-shots={{ dir }}"

# Measure the frame rate with 50 explosions at once in every graphics quality (opens a window)
[group('godot')]
effects-bench: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --effects-bench

# Measure the frame rate while 100 simulation craters fall within 2 s (prompt 021; 1920 x 1080, V-Sync off)
[group('effects')]
crater-view-bench map="test_small": build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --effects-craters --map={{ map }}

# --- LAN multiplayer (prompt 025) ----------------------------------------------------

# Build the bridge and open the LAN lobby (host, search, join, start); args e.g. --map=test_large
[group('lan')]
lan *args: build
    {{ GODOT }} --path godot res://scenes/lan/lan_lobby.tscn -- {{ args }}

# Two headless Godot instances play a LAN match over localhost (needs bash, on Windows Git Bash)
[group('lan')]
lan-selftest: build
    bash tools/lan-selftest.sh

# Build the bridge and open the online lobby over Steam (Steam must run and be logged in; docs/ONLINE.md)
[group('online')]
online *args: build
    {{ GODOT }} --path godot res://scenes/online/steam_lobby.tscn -- {{ args }}

# Automatic online match over Steam with the game: `host` on one account, `join` (first listed lobby, or `join --steam-join=<code>`) on another
[group('online')]
online-selftest role="host" *args: build
    {{ GODOT }} --path godot res://scenes/online/steam_lobby.tscn -- --steam-self-test --steam-{{ role }} {{ args }}

# Host a headless online game of the demo over Steam (CLI peer; prints the lobby code)
[group('online')]
steam-host ticks="600":
    cargo run -q -p cli --features steam -- steam-host --ticks {{ ticks }}

# Join a headless online game over Steam: by lobby code, or the first one listed
[group('online')]
steam-join code="" ticks="600":
    cargo run -q -p cli --features steam -- steam-join --ticks {{ ticks }} {{ if code != "" { "--code " + code } else { "" } }}

# Host a headless LAN game of the demo (CLI peer, e.g. against `just lan-join` on another machine)
[group('lan')]
lan-host ticks="600":
    cargo run -q -p cli -- lan-host --ticks {{ ticks }}

# Join a headless LAN game: the first one found, or the given address (ip or ip:port)
[group('lan')]
lan-join address="" ticks="600":
    cargo run -q -p cli -- lan-join --ticks {{ ticks }} {{ if address != "" { "--address " + address } else { "" } }}
