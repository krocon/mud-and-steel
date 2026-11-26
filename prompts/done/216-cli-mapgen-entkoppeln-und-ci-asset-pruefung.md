# Prompt 216 – `crates/mapgen` aus `cli` auslagern, CI-Asset- & L10n-Prüfung ergänzen und Bridge-Hygiene

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `Cargo.toml`, `.github/workflows/ci.yml` und `docs/ARCHITECTURE.md`. Im Code:
> `crates/cli/src/`,
> `crates/bridge/Cargo.toml`,
> `crates/bridge/src/`,
> `tools/l10n_check.py`,
> `tools/blender/validate.py`,
> `tools/blender/validate_env.py`.
> Reine Struktur-, Wartbarkeits- und CI-Verbesserung – **keine Regeländerung in `crates/sim`**,
> alle Golden Hashes und erzeugten Karten bleiben **bit-identisch**!

## 1. Problem

Mit dem Wachstum des Projekts auf ~129k Zeilen Rust und 438 3D-Modelle haben sich
strukturelle Reibungspunkte und CI-Lücken angesammelt:

1. **`crates/cli` ist zu einem 23.071-LoC-Monolithen gewachsen:**
   - Rund **14.500 LoC (~70 %)** von `crates/cli` sind prozedurale Karten- und Textur-Generatoren
     (`gen_sketch.rs` allein **5.485 LoC** mit 77 `.unwrap()`/`.expect()`, `gen_land.rs` 2.151,
     `gen_city.rs` 1.593, `gen_map.rs` 1.410, `gen_map/objects.rs` 1.213, `gen_textures.rs` 922,
     `gen_town.rs` 612, `gen_blocks.rs` 492, `gen_village.rs` 364, `gen_borgo.rs` 301).
   - Jede kleine Änderung an Benchmark-, Replay- oder Audit-Kommandos kompiliert und linkt
     den gesamten Kartengenerator mit, und `gen_sketch.rs` ist kaum noch navigierbar.
2. **Lücken in `.github/workflows/ci.yml`:**
   - **`tools/l10n_check.py`** wird in CI derzeit nicht ausgeführt (Übersetzungsfehler
     fallen erst lokal auf).
   - **Keine 3D-Asset-Prüfung in CI:** `validate.py` / `validate_env.py` benötigen Blender 5.x
     und laufen daher nie in CI. Ein schneller, reiner Python-3-GLB-Prüfer (ohne Blender!)
     kann in wenigen Sekunden alle 438 `.glb`-Dateien in CI auf Material-Slots, `COLOR_0`,
     `TEXCOORD_0`, Einheitsskalierung, Dreieck-Budgets und Datei-/Animations-Bloat prüfen!
3. **`crates/bridge`-Wartbarkeit:**
   - `crates/bridge/Cargo.toml` hat keinen `[lints]`-Abschnitt (anders als `sim`, `ai`, `net`).
   - Über die 40+ `*_funcs.rs`-Dateien von `SimHost` sind kleine Hilfsfunktionen
     (`point`/`target_of`, `collect_events` mit `MAX_PENDING_EVENTS`, `to_i32`) mehrfach
     dupliziert, und `crates/bridge` hat bei 13.298 LoC nur 1 Integrationstest.
   - (Optional dokumentieren / messen): In `crates/sim/src/sim.rs:257-326` trägt `Unit`
     29 Inline-Felder (mehrere hundert Bytes pro Einheit); kurz prüfen, ob Auslagern kalter
     Felder (wie `squad` bereits `Option<Box<Squad>>` ist) die Cache-Lokalität in `bench`
     messbar verbessert – nur umsetzen, wenn es sich lohnt und einfach bleibt!

## 2. Ziel

1. **Neues Workspace-Crate `crates/mapgen` auslagern:**
   - Alle `gen_*`-Module aus `crates/cli/src/` nach `crates/mapgen/src/` verschieben und als
     Workspace-Bibliothek (`mapgen = { path = "crates/mapgen" }`) in `Cargo.toml` einbinden;
     `crates/cli` bleibt ein schlanker Runner über `sim`, `net`, `ai`, `editor`, `mapgen`.
   - `gen_sketch.rs` (5.485 LoC) in klar geschnittene Submodule zerlegen (z. B.
     `gen_sketch/mod.rs`, `schema.rs`, `relief.rs`, `water.rs`, `roads.rs`, `settlements.rs`,
     `vegetation.rs`), ohne das erzeugte Karten-Ergebnis um ein einziges Byte zu verändern!
   - `docs/ARCHITECTURE.md` und `CLAUDE.md` (Layout-Tabelle) um `crates/mapgen/` ergänzen.
2. **Blender-freien GLB-Prüfer & `l10n_check.py` in CI aufnehmen:**
   - Neues leichtgewichtiges Skript `tools/glb_check.py` (nur Python-3-Standardbibliothek,
     parst den JSON-Header aller `.glb` in `godot/assets/models/`):
     - Prüft: gültiges glTF-2.0-GLB, nur erlaubte Material-Slots (`docs/ASSETS.md`; Ausnahme
       für `placeholder_tank.glb` sauber benennen), `COLOR_0` & `TEXCOORD_0` vorhanden,
       keine Non-Unit-Scales (`!= 1.0`), Dreieck-Budgets pro Kategorie/Katalogeintrag
       eingehalten, keine versehentlich eingebetteten Texturbilder (`images == []`).
     - Rezepte in `justfile` (`just glb-check`) und Schritt in `.github/workflows/ci.yml`
       (Linux-Job, sofern LFS-Objekte ausgecheckt sind – `lfs: true` ist dort bereits an!).
   - `python3 -I tools/l10n_check.py` ebenfalls in `.github/workflows/ci.yml` und `just check`
     prüfen/aufnehmen.
3. **`crates/bridge`-Hygiene:**
   - Sinnvolle `[lints.clippy]` in `crates/bridge/Cargo.toml` ergänzen (ohne `float_arithmetic`,
     da die Bridge bewusst nach `f32` konvertiert!).
   - Duplizierte Hilfsfunktionen (`point`, `target_of`, `collect_events`, etc.) in ein kleines
     gemeinsames Modul (`crates/bridge/src/helpers.rs`) zusammenführen.
   - Reine Logik-Funktionen in `crates/bridge` (z. B. Konvertierungen, Interpolation,
     Header-Parsing) mit zusätzlichen Unit-Tests absichern.

## 3. Prüfen und zeigen

- **Bit-Identität der Kartengeneratoren:**
  - Bestehende Karten-Generator-Tests (`just check`) sowie Stichprobe `just gen-sketch` /
    `just gen-city` gegen die committeten Karten unter `data/maps/` (0 Diff!).
- **Neue Checks lokal grün:**
  - `python3 -I tools/glb_check.py` (alle 438 GLBs grün, Laufzeit < 3 s!).
  - `python3 -I tools/l10n_check.py` grün.
  - `just check`, `cargo clippy -p bridge --no-default-features --all-targets -- -D warnings`,
    `cargo clippy -p cli --features steam --all-targets -- -D warnings`, `just selftest` grün.
  - Golden Hashes unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an Spielregeln, Shadern oder 3D-Modell-Dateien.
- Aufbrechen von `crates/sim/src/sim.rs` (nur wenn nebenbei risikofrei, kein Muss).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/216-cli-mapgen-entkoppeln-und-ci-asset-pruefung.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (2026-11-26)

- **Neues Workspace-Crate `crates/mapgen`:**
  - Alle 9 prozeduralen Generatoren (`gen_blocks`, `gen_borgo`, `gen_city`, `gen_land`, `gen_map` + `gen_map/{objects,relief}`, `gen_sketch`, `gen_textures`, `gen_town`, `gen_village`) inkl. `prepared_ground()` aus `crates/cli/src/` in `crates/mapgen/src/` ausgelagert (`mapgen = { path = "crates/mapgen" }`, `[profile.dev.package.mapgen] opt-level = 2`, `[lints.rust] unsafe_code = "forbid"`).
  - `crates/cli/src/` schrumpft von ~23 100 auf **8 131 LoC** (−64,8 %); `crates/mapgen/src/` trägt die **15 066 LoC** Generatoren.
  - `gen_sketch.rs` (5 534 LoC) verlust- und bit-identisch in 7 Submodule unter `crates/mapgen/src/gen_sketch/` aufgeteilt: `mod.rs` (779), `schema.rs` (631), `plan.rs` (820), `settlements.rs` (557), `works.rs` (676), `mesa.rs` (990) und `tests.rs` (1 143).
  - `CLAUDE.md` (Layout-Tabelle) und `docs/ARCHITECTURE.md` (Schichten-Tabelle) um `crates/mapgen/` ergänzt.
  - **Bit-Identität geprüft:** Alle 22 Skizzenkarten im `mapgen`-Testsuite sowie Stichproben `gen-sketch bocage_village`, `gen-sketch river_front` und `gen-sketch mesa_desert` gegen `data/maps/` mit **0 Byte Diff**; `gen-city` erzeugt identisches `map.toml`, `objects.csv`, `height.png` und identischen `IDAT`-Pixelstrom in `terrain.png` (einziger Unterschied ist der ältere 9-Farben-`PLTE`-Header aus Prompt 079 vs. heutige 12 Gelände-IDs).
- **Blender-freie GLB-Prüfung & L10n-Check in CI/`justfile`:**
  - Neues reines Python-3.10+-Skript `tools/glb_check.py` (nur Standardbibliothek, parst nur den JSON-Chunk ohne Binärpuffer): prüft alle **454 `.glb`-Dateien** in `godot/assets/models/` in **0,22–0,28 s** auf gültigen glTF-2.0-Header, `images == []`, Einheitsskalierung (`1.0 ± 1e-3`), erlaubte Material-Slots (`docs/ASSETS.md`, explizite Ausnahme `DEV_EXCEPTIONS["placeholder_tank.glb"]`), `COLOR_0` & `TEXCOORD_0`, Animations-Hygiene (`0` Clips + gültiges `anim_library` in Soldaten-GLBs, alle 61 Clips in `infantry/anims/*.glb`) sowie Dreieck-Budgets (Fahrzeuge gesamt, Env pro Variante & `*_Far`, Soldaten Körper+Ausrüstung ≤ 3 000 / Waffe ≤ 800).
  - Als `just glb-check` sowie zusammen mit `python3 -I tools/l10n_check.py` in `just check` und im Linux-Job von `.github/workflows/ci.yml` verdrahtet.
- **`crates/bridge`-Hygiene & Tests:**
  - `crates/bridge/Cargo.toml` um `[lints.rust]` (`unsafe_code = "deny"`, einzig isoliert auf `mod gdext_entry` für `#[gdextension] unsafe impl ExtensionLibrary`) und `[lints.clippy]` (`dbg_macro`/`todo`/`unimplemented = "deny"`, `lossy_float_literal`/`inefficient_to_string`/`manual_let_else`/`unnested_or_patterns`/`flat_map_option = "warn"`, bewusst ohne `float_arithmetic`) ergänzt.
  - Neues Modul `crates/bridge/src/helpers.rs` bündelt `fixed_to_f32`, `meters_to_millimeters`, `ground_point`, `ground_to_godot`, `vec3_to_godot`, `flat_to_godot`, `to_unit_ids`, `push_capped` und `collect_tick_events` über 12 `*_funcs.rs`-Dateien (`air`, `artillery`, `combat`, `crater`, `crossing`, `destruction`, `economy`, `feedback`, `hud`, `move`, `orders`, `positions`, `strike`, `works`).
  - **9 neue Unit-Tests** in `crates/bridge` (`helpers::tests` × 5 + `lib::tests` × 4 für `clamp_stride`, `grid_indices`, `height_f32`, `ray_pick`) → 28 Bridge-Tests insgesamt.
  - **Notiz zu `Unit` in `crates/sim/src/sim.rs:257-326`:** Bei maximal ~500 Einheiten (`bench_large`: 504) liegen alle `Unit`-Structs zusammen bereits bequem im L2-Cache; ein Auslagern über `Option<Box<T>>` würde den Postcard-Snapshot verändern oder pro Snapshot hunderte Heap-Allokationen einführen, daher bewusst unverändert gelassen.
- **Verifikation:** `just check` (inkl. `cargo fmt --check`, alle 3 `clippy -D warnings`, `cargo nextest run --workspace` inkl. Golden Hashes, `vehicle_refs.py`, 8× `geo_import.py check`, `sounds_web.py`, `l10n_check.py` [0 Probleme], `glb_check.py` [454 GLBs in 0,28 s]) und `just selftest` (`flow`, `riders`, `gun crews`, `boats`) komplett grün.
