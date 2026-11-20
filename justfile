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

# Format, lint and test exactly like CI (the tests need cargo-nextest)
[group('rust')]
check:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo clippy -p bridge --no-default-features --all-targets -- -D warnings
    cargo clippy -p cli --features steam --all-targets -- -D warnings
    cargo nextest run --workspace
    python3 tools/vehicle_refs.py --check
    python3 -I tools/geo_import.py check assets-src/geo/harbor_city/site.json
    python3 -I tools/sounds_web.py --check

# Format all Rust code
[group('rust')]
fmt:
    cargo fmt

# Run all tests with cargo-nextest: all tests in parallel across the test binaries, about three times as fast as `cargo test --workspace`
[group('rust')]
test *args:
    cargo nextest run --workspace {{ args }}

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
build: && (_steam-lib "debug") _ensure-imported
    cargo build -p bridge

# Import the Godot project when a script declares a class_name Godot does not
# know yet (after a merge or a fresh clone), so scripts using it parse
[private]
_ensure-imported:
    bash tools/ensure-imported.sh

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

# Build the bridge and run the headless self-test: translations, menu, settings, three matches (one against the computer), result (exit code 0 = passed)
[group('godot')]
selftest: l10n-check build
    {{ GODOT }} --path godot --headless --fixed-fps 60 -- --self-test

# Check the translations (prompt 043): tables complete, no hard-coded interface text in the game scripts, every mission text translated
[group('godot')]
l10n-check:
    python3 tools/l10n_check.py
    cargo run -q -p cli -- mission-po --check

# Screenshots of every menu screen, settings tab and the match HUD in a language (prompt 043; needs a window), e.g. `just l10n-shots de build/l10n`
[group('godot')]
l10n-shots language="de" dir="build/l10n" *args: build
    {{ GODOT }} --path godot --always-on-top --resolution 1600x900 -- --l10n-shots={{ justfile_directory() }}/{{ dir }} --language={{ language }} --settings=user://settings-l10n-shots.cfg {{ args }}

# Screenshots of the unit values in the interface (prompt 066; needs a window): ratings on the picker tiles, tooltip, unit card per tab, unit reference; German and English at interface scale 1 and 1.25, and German at 1280x720, into build/unit-cards/
[group('godot')]
unit-card-shots dir="build/unit-cards" *args: build
    {{ GODOT }} --path godot --always-on-top --resolution 1600x900 -- --unit-card-shots={{ justfile_directory() }}/{{ dir }}/de-1.0 --language=de --ui-scale=1.0 --settings=user://settings-unit-card-shots.cfg {{ args }}
    {{ GODOT }} --path godot --always-on-top --resolution 1600x900 -- --unit-card-shots={{ justfile_directory() }}/{{ dir }}/de-1.25 --language=de --ui-scale=1.25 --settings=user://settings-unit-card-shots.cfg {{ args }}
    {{ GODOT }} --path godot --always-on-top --resolution 1600x900 -- --unit-card-shots={{ justfile_directory() }}/{{ dir }}/en-1.0 --language=en --ui-scale=1.0 --settings=user://settings-unit-card-shots.cfg {{ args }}
    {{ GODOT }} --path godot --always-on-top --resolution 1600x900 -- --unit-card-shots={{ justfile_directory() }}/{{ dir }}/en-1.25 --language=en --ui-scale=1.25 --settings=user://settings-unit-card-shots.cfg {{ args }}
    {{ GODOT }} --path godot --always-on-top --resolution 1280x720 -- --unit-card-shots={{ justfile_directory() }}/{{ dir }}/de-720p --language=de --ui-scale=1.0 --settings=user://settings-unit-card-shots.cfg {{ args }}

# Write the translation files of the mission texts (data/maps/*/missions/<id>.<lang>.po): new texts get an empty entry to translate
[group('godot')]
mission-po:
    cargo run -q -p cli -- mission-po

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

# Serve the unit browser (web/units/): every catalog entry with picture, 3D model, camouflage and values; opens http://localhost:8137/web/units/
[group('assets')]
units-web *args:
    python3 tools/units_web.py {{ args }}

# Copy every file the unit page loads (catalog, palettes, models, thumbnails) to web/units/ and write its prices; run before uploading web/units/ to the website
[group('assets')]
units-web-sync:
    python3 tools/units_web.py --sync

# Download vehicle reference data and drawings from onwar.com into docs/.vorlagen/fahrzeuge (not in git); --nation GER|US|RU, --limit N, --force
[group('assets')]
vehicle-refs *args:
    node tools/onwar_download.mjs {{ args }}

# Write the vehicle database data/vehicles/*.json and docs/VEHICLE_REFS.md from docs/.vorlagen/fahrzeuge (needs `just vehicle-refs` first), then check it
[group('assets')]
vehicle-db:
    python3 tools/vehicle_refs.py --write --check

# Compare a vehicle model with its references (prompt 063): orthographic side/front/top views at a fixed scale with dimension lines, a table model / data.json / catalog and the drawings of the reference folder, into build/renders/compare/<id>.png and .md (never committed)
ref-compare id *args:
    {{ CALL }}"{{ BLENDER }}" -b --factory-startup --python-exit-code 1 --python tools/blender/ref_compare.py -- {{ id }} {{ args }}

# Check the vehicle database against its schema, data/vehicles/index.json and the catalog (part of `just check`; needs no reference folder)
[group('assets')]
vehicle-db-check *args:
    python3 tools/vehicle_refs.py --check {{ args }}

# Build the bridge and open the model gallery (all models, camouflage switch)
[group('godot')]
gallery: build
    {{ GODOT }} --path godot res://scenes/tools/model_gallery.tscn

# Headless check of the model gallery: loads every model in every camouflage
[group('godot')]
gallery-check: build
    {{ GODOT }} --path godot --headless res://scenes/tools/model_gallery.tscn -- --self-test

# Render the picker thumbnails of every buildable unit and building into godot/assets/thumbnails/ (prompt 058; needs a window), e.g. `just thumbnails --only=de_pz4_h`
[group('godot')]
thumbnails *args: build
    {{ GODOT }} --path godot res://scenes/tools/thumbnail_render.tscn {{ if args != "" { "-- " + args } else { "" } }}
    just import

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
gen-map name size_m="500" seed="1" start_distance_m="120" *args:
    cargo run -q --release -p cli -- gen-map --name {{ name }} --size-m {{ size_m }} --seed {{ seed }} --start-distance-m {{ start_distance_m }} {{ args }}

# Regenerate the committed test maps (changes the golden hash if test_small changes)
[group('maps')]
test-maps:
    just gen-map test_small 500 7 60
    just gen-map test_large 4000 11 120 --players 6
    just gen-map twin_meadow 1000 21 120 --players 4

# Generate the mountain map (prompt 060) into data/maps/mountain_pass/: basins, a pass, a serpentine, 4 starts
[group('maps')]
gen-mountains:
    just gen-map mountain_pass 2000 60 1200 --players 4 --relief mountains

# Generate the rock map (prompt 074) into data/maps/rock_hills/: dry mountains with rock ridges, outcrops and slab fields
[group('maps')]
gen-rock-hills:
    just gen-map rock_hills 1200 74 700 --biome desert --relief mountains --rocks

# Generate the small-town map (prompt 041) into data/maps/town_small/
[group('maps')]
gen-town:
    cargo run -q --release -p cli -- gen-town

# Generate the apartment block quarter (prompt 078) into data/maps/city_blocks/: eleven blocks at three streets, courtyards
[group('maps')]
gen-blocks:
    cargo run -q --release -p cli -- gen-blocks

# Generate the large city map (prompt 079) into data/maps/city_large/: block quarter, suburb, land, 4 starts (winter)
[group('maps')]
gen-city *args:
    cargo run -q --release -p cli -- gen-city {{ args }}

# Generate the desert village map (prompt 059) into data/maps/desert_village/
[group('maps')]
gen-village:
    cargo run -q --release -p cli -- gen-village

# Generate the landscape maps of prompt 056 (winter_village, farms, river_land, or all) into data/maps/; `ambush_meadow` (prompt 082, the video scene's ground) only when named
[group('maps')]
gen-land map="all":
    cargo run -q --release -p cli -- gen-land --map {{ map }}

# Generate a map from its picture sketch data/map_sketches/<name>.toml into data/maps/<name>/ (prompt 088; without a name every sketch map; docs/MAPS.md)
[group('maps')]
gen-sketch *name:
    cargo run -q --release -p cli -- gen-sketch {{ name }}

# Generate a map from real geodata (prompt 081) into data/maps/<site>/ from assets-src/geo/<site>/ (OpenStreetMap + elevation model; docs/MAPS.md); `--report DIR` also writes pictures of the stages
[group('maps')]
gen-geo site="harbor_city" *args:
    python3 -I tools/geo_import.py generate assets-src/geo/{{ site }}/site.json {{ args }}

# Check that a geodata map regenerates identically (twice, and as committed); part of `just check`
[group('maps')]
gen-geo-check site="harbor_city":
    python3 -I tools/geo_import.py check assets-src/geo/{{ site }}/site.json

# Fire benchmark (prompt 041): ticks without and with a burning wood, terrace and dumps on the town map; on city_large (prompt 079) a burning block cell
[group('maps')]
fire-bench units="40" ticks="6000" map="town_small":
    cargo run -q --release -p cli -- fire-bench --units {{ units }} --ticks {{ ticks }} --map data/maps/{{ map }}

# Load a map and report load time, memory, height range and terrain shares
[group('maps')]
map-info name="test_large":
    cargo run -q --release -p cli -- map-info data/maps/{{ name }}

# Check a map like the map editor: loading, start positions, paths between the starts
[group('maps')]
map-check name="test_small":
    cargo run -q --release -p cli -- map-check data/maps/{{ name }}

# Reachability of a map (prompt 081): land open to vehicles, to infantry only, to nobody; connected parts; where starts and markers lie; e.g. `just map-reach harbor_city build/reach.png`
[group('maps')]
map-reach name="test_small" png="":
    cargo run -q --release -p cli -- map-reach data/maps/{{ name }} {{ if png != "" { "--png " + png } else { "" } }}

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

# Benchmark a salvo into the densest wood of a map (prompt 085): ticks timed, trees felled and stripped; e.g. `just salvo-bench city_large --caliber 1500 --rounds 18`
[group('performance')]
salvo-bench map="test_large" *args:
    cargo run -q --release -p cli -- salvo-bench --map data/maps/{{ map }} {{ args }}

# Trees in the wind (prompt 086; needs a window): two stills one second apart of a wood's edge, mid and near zoom, and their difference into build/wind_shots/<prefix>_*; extra args go to the shots (e.g. --off=wind is not supported there: compare prefixes instead)
[group('godot')]
wind-shots prefix="after": build
    rm -rf build/trailer/stills
    bash tools/trailer-shots.sh --stills --only=survey_wind_mid,survey_wind_near
    python3 -I tools/wind_shots.py {{ prefix }}

# Fields in pictures (prompt 088; needs a window): river_fields near, middle, far, in rain and in winter, and both sketch maps from above, as stills into build/trailer/stills/
[group('godot')]
field-shots: build
    bash tools/trailer-shots.sh --stills --only=survey_fields_near,survey_fields_mid,survey_fields_far,survey_fields_rain,survey_fields_winter,survey_river_fields,survey_river_fields_winter

# Water colors in pictures (prompt 091; needs a window): a bank on river_city, desert_lagoon, island_chain and winter_town by day, river_city by night and in rain, as stills into build/water-color/<prefix>/
[group('godot')]
water-shots prefix="after": build
    bash tools/trailer-shots.sh --stills --only=survey_water_river_city,survey_water_desert_lagoon,survey_water_island_chain,survey_water_winter_town,survey_water_night,survey_water_rain
    mkdir -p build/water-color/{{ prefix }} && mv build/trailer/stills/survey_water_*.png build/water-color/{{ prefix }}/

# Cloud shadows in pictures (prompt 087; needs a window): meadow, column, village, overcast, evening and a series every 2 s into build/cloud_shots/<prefix>_*; `just cloud-shots before --off` without them
[group('godot')]
cloud-shots prefix="after" *args: build
    bash tools/cloud-shots.sh {{ prefix }} {{ args }}

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

# Faction balance (prompt 045, docs/BALANCE.md): AI tournaments over faction pairings, maps and modes, e.g. `just balance --maps test_small,skirmish --modes base --games 10`
[group('ai')]
balance *args:
    cargo run -q --release -p cli -- balance {{ args }}

# One unit against another over distances and angles (prompt 045), e.g. `just duel --a de_pz4_h --b su_t34_76_1943`
[group('ai')]
duel *args:
    cargo run -q --release -p cli -- duel {{ args }}

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

# Record two camera rides and measure whether the ground texture sticks to the ground (prompt 071; opens a window; args: [dir] [--png] [--near] [--off=<parts>], prompt 068)
[group('terrain')]
ground-motion *args: build
    bash tools/ground-motion.sh {{ args }}

# Flicker from the zoom alone (prompt 090; needs a window, ffmpeg, numpy): the camera stands while the shadow distance runs through a 40-300 m zoom; with the steps and (`--off=shadowsteps`) gliding as before
[group('terrain')]
shadow-flicker *args: build
    bash tools/shadow-flicker.sh {{ args }}

# --- Map objects (prompt 013) ---------------------------------------------------------

# Measure frame rate, draw calls and instances over the densest wood and a village (opens a window); `just objects-bench rock_hills --rocks-bench`: over a rock ridge field (prompt 074)
[group('terrain')]
objects-bench map="test_large" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --objects-bench --map={{ map }} {{ args }}

# Record fixed views of woods, village, meadow and hedgerow as PNG frames into a folder; `just objects-shots <dir> rock_hills --rocks`: the rock selection in three biomes (prompt 074)
[group('terrain')]
objects-shots dir map="test_large" *args: build
    {{ GODOT }} --path godot --always-on-top --fixed-fps 30 --write-movie "{{ dir }}/f.png" res://scenes/main.tscn -- --objects-shots --map={{ map }} {{ args }}

# Save the rock views of prompt 074 as PNGs into a folder: slabs, outcrops and ridge pieces at 15-350 m, a ridge on a steep slope, views like the reference pictures, in three biomes (opens a window); e.g. --daytime=evening, --biomes=desert
[group('terrain')]
rock-shots dir="build/rock-shots" map="rock_hills" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --objects-bench --rocks --out="$(cd "{{ invocation_directory() }}" && mkdir -p "{{ dir }}" && cd "{{ dir }}" && pwd)" --map={{ map }} {{ args }}

# Save the roof views of prompt 075 as PNGs into a folder: the village at 15-350 m and like the reference picture, all roof coverings side by side, twenty equal houses, damaged and charred houses; forest by day, low sun and night, desert, winter (opens a window); e.g. --biomes=forest --daytimes=day --only=village-35
[group('terrain')]
roof-shots dir="build/roof-shots" map="test_large" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --objects-bench --roofs --out="$(cd "{{ invocation_directory() }}" && mkdir -p "{{ dir }}" && cd "{{ dir }}" && pwd)" --map={{ map }} {{ args }}

# Save the roof snow views of prompt 080 as PNGs into a folder: the winter village at 15-350 m, two houses like the reference picture, every roof kind side by side, twenty equal houses, damaged and charred houses; by day, low sun, night and in falling snow, and by day with snow amounts 0.3-1.0 (opens a window); e.g. --amounts=0.6 --only=village-compare-near
[group('terrain')]
snow-shots dir="build/snow-shots" map="winter_village" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --objects-bench --roofs --snow --out="$(cd "{{ invocation_directory() }}" && mkdir -p "{{ dir }}" && cd "{{ dir }}" && pwd)" --map={{ map }} {{ args }}

# Save the apartment block views of prompt 078 as PNGs into a folder: the quarter at 15-350 m and like the reference pictures, every block in all damage levels, a burnt block; desert by day, low sun and night, forest, winter (opens a window); e.g. --biomes=desert --only=compare-near
[group('terrain')]
block-shots dir="build/block-shots" map="city_blocks" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --objects-bench --blocks --out="$(cd "{{ invocation_directory() }}" && mkdir -p "{{ dir }}" && cd "{{ dir }}" && pwd)" --map={{ map }} {{ args }}

# Play the apartment block fight of prompt 078 on city_blocks and save pictures into a folder: a squad occupies a block, two tanks shell it to a ruin, the other units cross the quarter (opens a window)
[group('terrain')]
block-fight dir="build/block-fight" map="city_blocks" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --objects-bench --blocks-fight --out="$(cd "{{ invocation_directory() }}" && mkdir -p "{{ dir }}" && cd "{{ dir }}" && pwd)" --map={{ map }} {{ args }}

# --- Effects (prompt 019) ------------------------------------------------------------

# Show every effect in fixed views and save PNG screenshots into a folder (also desert, winter)
[group('godot')]
effects-shots dir: build
    {{ GODOT }} --path godot --always-on-top --fixed-fps 30 res://scenes/main.tscn -- "--effects-shots={{ dir }}"

# Record base building (preview, construction site, production bar, bases) as PNG files into a folder (prompt 023)
[group('godot')]
base-shots dir: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --mode=base --ai=normal "--base-shots={{ dir }}"

# Own-unit markers in pictures (prompt 164; needs a window): off, faint markers, cursor over a squad, show-all key; e.g. `just unit-marker-shots build/unit-visibility --prefix=night --daytime=night`
[group('godot')]
unit-marker-shots dir="build/unit-visibility" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- "--unit-marker-shots={{ justfile_directory() }}/{{ dir }}" {{ args }}

# Target preview and range circles in pictures (prompt 099; needs a window): line, wedge, column, circles; e.g. `just order-preview-shots build/order-preview --prefix=snow --biome=winter`
[group('godot')]
order-preview-shots dir="build/order-preview" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- "--order-preview-shots={{ justfile_directory() }}/{{ dir }}" {{ args }}

# Alarms out of view in a picture (prompt 101; needs a window): edge arrows for fire and a loss off the screen
[group('godot')]
alert-shots dir="build/alerts" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- "--alert-shots={{ justfile_directory() }}/{{ dir }}" {{ args }}

# Ground tooltip in pictures (prompt 100; needs a window): road, woods, house, water, meadow, field, mud of the map; e.g. `just ground-tooltip-shots build/ground-tooltip --map=farms`
[group('godot')]
ground-tooltip-shots dir="build/ground-tooltip" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- "--ground-tooltip-shots={{ justfile_directory() }}/{{ dir }}" {{ args }}

# Overview, HUD off and photo mode in pictures (prompt 103; needs a window); e.g. `just view-shots build/view --map=river_town`
[group('godot')]
view-shots dir="build/view" *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- "--view-shots={{ justfile_directory() }}/{{ dir }}" {{ args }}

# Record a trench strongpoint (prompt 052) as PNG files into a folder: overview, firing bay, dugouts, ring and wire (needs a window)
[group('godot')]
position-shots dir="build/positions": build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- "--position-shots={{ dir }}"

# Record big blasts, a tank's ammunition explosion and a heavy rocket strike by day and night as PNG files (prompt 054, needs a window)
[group('godot')]
blast-shots dir="build/explosions": build
    {{ GODOT }} --path godot --always-on-top --fixed-fps 30 res://scenes/main.tscn -- "--blast-shots={{ justfile_directory() }}/{{ dir }}"

# Every explosion kind as a picture series, stacked under the reference video's series if .temp/rts-explosionen.mp4 is there (prompt 072, needs a window, ffmpeg and numpy)
[group('godot')]
explosion-series dir="build/explosion-ref/now": build
    bash tools/explosion-series.sh {{ dir }}

# A rocket salvo as a picture series after the reference (winter, dusk), then side view, target, day, night, biomes and factions (prompt 076, needs a window); `--only=<run>,<run>` records some
[group('godot')]
salvo-shots dir="build/salvo" *args: build
    {{ GODOT }} --path godot --always-on-top --fixed-fps 30 res://scenes/main.tscn -- --salvo-series "--blast-shots={{ justfile_directory() }}/{{ dir }}" {{ args }}

# Every gun class firing as a picture series around the muzzle (calibers, howitzer, distances, daytimes, biomes), beside the reference picture if .temp/blitz-aus-rohr.png is there (prompt 077, needs a window, ffmpeg and numpy); `--only=<run>,<run>` records some
[group('godot')]
muzzle-shots dir="build/muzzle/now" *args: build
    bash tools/muzzle-series.sh {{ dir }} {{ args }}

# Play a real heavy rocket strike on the base skirmish and record it as PNG files; fails if it does not show (prompt 054, needs a window)
[group('godot')]
strike-shots dir="build/explosions": build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --mode=base --strike-live "--blast-shots={{ justfile_directory() }}/{{ dir }}"

# Record the trailer shots of scenes/tools/trailer_shots.gd as build/trailer/<shot>.avi (1920 x 1080, 60 fps, with sound; prompt 070); `--only=<shot>,<shot>` records some, `--stills` saves still pictures to build/trailer/stills/ instead (needs a window, bash and ffmpeg)
[group('godot')]
trailer-shots *args: build
    bash tools/trailer-shots.sh {{ args }}

# Cut the recorded shots `<prefix>_*` into build/trailer/<prefix>_final.mp4 in the order of the shot table: hard cuts, sound faded over 3 frames at the cuts, -14 LUFS, H.264 and AAC (prompt 082; needs ffmpeg), e.g. `just trailer-cut ambush`
[group('godot')]
trailer-cut prefix *out:
    bash tools/trailer-cut.sh {{ prefix }} {{ out }}

# Scout a trailer scene headless (prompt 082): who fires, hits and is destroyed when; with seeds one line per seed, e.g. `just trailer-scout ambush_end 100 1 2 3`
[group('godot')]
trailer-scout shot seconds *seeds: build
    bash tools/trailer-scout.sh {{ shot }} {{ seconds }} {{ seeds }}

# Measure the frame rate with 50 explosions at once, two rocket salvos and 40 cannon shots a second in every graphics quality (opens a window); `--salvos` only the salvos, `--muzzles` only the cannon shots
[group('godot')]
effects-bench *args: build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --effects-bench {{ args }}

# Measure the frame rate while 100 simulation craters fall within 2 s (prompt 021; 1920 x 1080, V-Sync off)
[group('effects')]
crater-view-bench map="test_small": build
    {{ GODOT }} --path godot --always-on-top res://scenes/main.tscn -- --effects-craters --map={{ map }}

# --- Sound (prompt 029) --------------------------------------------------------------

# Synthesize the sounds into godot/assets/audio/ (all classes, or the given ones; needs python3 with numpy; then: just import)
[group('sound')]
audio *classes:
    python3 tools/audio/synth.py {{ classes }}

# Compose the music and ambience into godot/assets/audio/music/ and ambience/ (all, or names starting with the given ones; needs numpy and lame; then: just import)
[group('sound')]
music *names:
    python3 tools/audio/music.py {{ names }}

# Generate sound effects with ElevenLabs from tools/audio/sfx_prompts.toml into docs/.sfx/generated/ (dry run with the cost; `--yes` calls the API, needs ELEVENLABS_API_KEY; docs/AUDIO_LIBRARY.md)
[group('sound')]
sfx-generate *args:
    python3 tools/audio/elevenlabs_sfx.py {{ args }}

# Play every sound class once, then click to replay (`just sound-gallery --sound-gallery-once` quits after the round)
[group('sound')]
sound-gallery *args: build
    {{ GODOT }} --path godot -- --sound-gallery {{ args }}

# Serve the sound page (web/sounds/): every audio file playable, with when the game plays it; opens http://localhost:8138/web/sounds/ (`--port N`, `--no-open`; `--check`: every sound folder has a remark in web/sounds/usage.toml)
[group('sound')]
sounds-web *args:
    python3 -I tools/sounds_web.py {{ args }}

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

# --- Release (prompt 044, docs/RELEASE.md) ----------------------------------------------

# Build the GDExtension in the release profile and stage it with steam_api and the third-party licenses in target/export/
[group('release')]
release-lib:
    bash tools/release/build-lib.sh

# Export the game: macos, windows or linux into build/export/ and an archive in build/release/ (needs Godot's export templates); `pack` only the .pck (no templates)
[group('release')]
export system="pack":
    bash tools/release/export.sh {{ system }}

# Headless menu self-test of the exported game; `pack` runs the exported .pck with the editor binary and the debug bridge
[group('release')]
export-selftest system="pack": build
    bash tools/release/smoke-test.sh {{ system }}

# Write the SteamPipe scripts into build/steam/ (dry run); `just steam-upload --upload` uploads with steamcmd (manual only)
[group('release')]
steam-upload *args:
    bash tools/steam/upload.sh {{ args }}
