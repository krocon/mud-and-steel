# Prompt 209 – Rust Quick-Wins in `sim`, `bridge` und `ai` (CPU, Latenzen und Bugs)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md` und `docs/PERFORMANCE.md`. Im Code:
> `crates/bridge/src/lib.rs`,
> `crates/bridge/src/ai_funcs.rs`,
> `crates/bridge/src/infantry_funcs.rs`,
> `crates/bridge/src/economy_funcs.rs`,
> `crates/bridge/src/hud_funcs.rs`,
> `crates/bridge/src/vision_funcs.rs`,
> `crates/net/src/session.rs`,
> `crates/ai/src/game.rs`,
> `crates/ai/src/lib.rs`,
> `crates/ai/src/view.rs`,
> `crates/sim/src/vision.rs`,
> `crates/sim/src/catalog.rs`,
> `crates/sim/src/combat.rs`,
> `crates/sim/src/terrain.rs`,
> `crates/sim/src/works.rs`,
> `crates/sim/src/economy.rs`,
> `crates/editor/src/validate.rs`.
> **Wichtig:** Reine Performance-Optimierungen und Präsentations-/Bridge-Bugfixes –
> **keine Regeländerung in `crates/sim`**, alle Golden Hashes (`determinism.rs`,
> `economy.rs`, `cli bench`) müssen **bit-identisch** bleiben!

## 1. Problem

Eine Tiefenanalyse der Rust-Crates hat mehrere hochwirksame, sauber abgegrenzte
Quick-Wins und konkrete Bugs gefunden:

1. **`SimHost` berechnet `state_hash()` jeden Tick und wirft 95–100 % weg:**
   `crates/bridge/src/lib.rs:190-191` ruft jeden Tick (20 Hz) `self.sim.state_hash()` auf
   und übergibt den Wert an `LockstepSession::record_hash()` (`crates/net/src/session.rs:437-459`),
   das bei `!tick.is_multiple_of(20)` oder `remote_count() == 0` (Einzelspieler!) sofort
   `Ok(())` zurückgibt. Im Einzelspieler sind 100 %, im Mehrspieler 95 % der Hash-Läufe verschwendet.
2. **`PlayerView::new` läuft jeden Tick für jede KI und baut 2.250 ungenutzte Soldaten-Snapshots:**
   - `crates/bridge/src/ai_funcs.rs:123-131` und `crates/ai/src/game.rs:126-132` bauen jeden
     Tick eine volle `PlayerView::new(&sim, player)`, obwohl `Ai::think()` (`ai/src/lib.rs:776-789`)
     auf `Hard` 9/10, auf `Normal` 19/20, auf `Easy` 79/80 und auf `Passive` 100 % der Ticks
     sofort ohne Arbeit verlässt.
   - `PlayerView::new` (`ai/src/view.rs:128-246`) ruft `sim.snapshot_for(player)` auf und füllt
     alle 1.125–2.250 `SoldierSnapshot`s, die `PlayerView` sofort ignoriert; außerdem ruft es
     `sim.find_site()` für jeden baubaren Gebäudetyp in jedem Tick auf.
3. **`VisionGrid::new` macht 64× redundante Arbeit + Per-Viewshed-Allokationen:**
   - `crates/sim/src/vision.rs:480-483` ruft `for i in 0..ground_mm.len() { grid.update_block(i); }`
     auf. Da `update_block(i)` alle 8×8 = 64 Zellen des Blocks durchläuft, wird jeder Block
     **64-mal** berechnet (~20,4 Mio. statt 319k Iterationen auf `coastal_heights`!). Ebenso
     dedupliziert `set_ground` (`vision.rs:656-658`) geänderte Blöcke nicht.
   - `viewshed_downhill` (`vision.rs:786-802`) allokiert pro Viewshed-Update zwei Heap-`Vec`s
     (`canopy: Vec<(i64, i64)>` mit maximal `<= 16` Einträgen und `shed.bits`).
4. **`load_map_toml_only` dekodiert trotzdem die kompletten PNGs:**
   `crates/bridge/src/lib.rs:1347-1356` ruft `Map::load(&toml, &read("height.png")?, &read("terrain.png")?)`
   auf (~70 ms / 25 MB RAM auf `coastal_heights`), nur um `starts` und `variants` aus
   `map.toml` für die Lobby zu lesen.
5. **Interpolations-Bug in `SimHost::previous_positions` (1-Tick-Teleport):**
   `crates/bridge/src/lib.rs:1153-1169` prüft im Fast-Path nur
   `self.previous.units.len() == self.current.units.len()`. Verschwindet eine Einheit im Nebel
   und taucht im selben Tick eine andere auf, stimmen die Längen, aber nicht die IDs →
   Interpolation von der falschen Einheit! Außerdem macht `soldier_previous_positions`
   (`crates/bridge/src/infantry_funcs.rs:71-80`) bis zu 2.250 Binary-Searches pro Frame.
6. **Weitere Bridge- & Regel-Lade-Flaschenhälse:**
   - `SimHost::building_at` (`infantry_funcs.rs:164-195`) scannt bei Hover/Klick linear alle
     bis zu **98.000 `MapObject`s** statt nur Gebäude bzw. eine räumliche Suche zu nutzen.
   - `economy_funcs.rs:11-23, 55-59, 197-200` klont `EconomyRules` und alle ~200 `UnitType.name`-Strings
     in Schleifen (selbst `unit_type_name(idx)` klont erst alle 200 Namen!).
   - `hud_funcs.rs:151-156` scannt alle 2.250 Soldaten linear für einen Trupp statt `partition_point`.
   - `vision_funcs.rs:187-194` allokiert bei jeder Sichtänderung zwei volle Grid-`Vec`s (~320k Elemente).
   - `Catalog::builtin()` und fast alle `*Rules::builtin()` in `crates/sim` parsen die eingebetteten
     TOMLs bei jedem Aufruf neu (nur `EconomyRules` nutzt `OnceLock`, und selbst das klont);
     `crates/editor/src/validate.rs:42-51` ruft `Catalog::builtin()` in einer Schleife pro Fraktion/Variante auf.

## 2. Ziel

1. **Lazy `state_hash()`:** `LockstepSession::record_hash` auf `hash: impl FnOnce() -> u64`
   (oder `session.needs_hash(tick) -> bool`) umstellen und in `crates/bridge/src/lib.rs:191` nutzen.
2. **KI-Think & `PlayerView` gaten:**
   - `Ai::wants_think(&self, tick: u64, decided: bool) -> bool` einführen und `PlayerView::new`
     in `bridge/src/ai_funcs.rs` und `ai/src/game.rs` dahinter legen.
   - `Sim::snapshot_for_ai_into` (oder Flag `include_soldiers: bool`) ohne Soldaten-Snapshots
     und mit wiederverwendetem `Snapshot`-Puffer verwenden; `find_site()` nur ausführen, wenn
     die KI überhaupt bauen will/kann (Determinismus der KI-Befehle und KI-Tests wahren!).
3. **`VisionGrid` beschleunigen:**
   - `VisionGrid::new`: `block_min_mm` in einem einzigen linearen Durchlauf berechnen (64× schneller).
   - `set_ground`: geänderte Block-Indizes deduplizieren.
   - `viewshed_downhill`: `canopy` als Stack-Array `[(i64, i64); 17]` + `len` statt `Vec`,
     `Viewshed::bits`-Puffer beim Aktualisieren wiederverwenden; `deck_low_mm` aus der
     inneren Strahl-Schleife ziehen.
4. **Echtes `load_map_toml_only`:**
   Leichtgewichtigen Header-Parser (`sim::map::MapHeader::parse(&toml)` o. ä.) nutzen, ohne
   `height.png`/`terrain.png` zu lesen oder zu dekodieren.
5. **Two-Pointer-Merge für `previous_positions` & `soldier_previous_positions`:**
   Da beide Slices nach `id` (bzw. `(unit, index)`) sortiert sind, in O(n) ohne
   ID-Verwechslung und ohne Binary-Searches zusammenführen. Regressionstest für
   gleichzeitiges Erscheinen/Verschwinden einer Einheit schreiben!
6. **Bridge-Flaschenhälse & `OnceLock`:**
   - `building_at`: über die vorindizierten Gebäude (`sim.infantry` / Gebäudeliste) statt
     aller 98.000 Map-Objekte suchen.
   - `economy_funcs.rs`: `&[UnitType]` direkt aus `game.config.unit_types` borgen, `&'static EconomyRules` nutzen.
   - `hud_funcs.rs`: `partition_point` für Trupp-Soldaten; `vision_funcs.rs`: Scratch-Puffer
     auf `FogFade` wiederverwenden.
   - Alle `sim::*::builtin()`-Lader (`Catalog`, `CombatRules`, `TerrainRules`, `EconomyRules`,
     `WorksRules`, `VisionRules`, `InfantryRules`, usw.) über `OnceLock` cachen (z. B.
     `builtin_ref() -> &'static ...` bzw. direkt `&'static` wo passend).

## 3. Prüfen und zeigen

- **Determinismus-Pflichtcheck:**
  - `just check` (alle Golden Hashes unverändert!)
  - `just sim 1000 42` liefert denselben Hash.
  - `just bench --runs 3`: End-Hash unverändert, Vision-/AI-/Snapshot-Zeiten vorher/nachher
    dokumentieren (`docs/PERFORMANCE.md`).
  - `just bench-view --perf-quick`: `SimHost.process_usec` vorher/nachher vergleichen.
  - Neuer Unit-Test für den `previous_positions`-ID-Wechsel-Fall und `load_map_toml_only`.
- `just selftest` grün.

## 4. Nicht Teil dieses Prompts

- `PathGrid`-Deduplizierung & Snapshot-Format-Umbau (Prompt 210).
- Änderung der A*-Budget-Regeln (Prompt 211).
- Netzwerk-Protokoll / Host-Relay-Prüfung (Prompt 212).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/209-sim-bridge-und-ki-quickwins.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (2026-11-26)

- **Lazy `state_hash()`:** `LockstepSession::needs_hash(tick)` und `record_hash_with(tick, || ..., transport)` in `crates/net/src/session.rs` ergänzt und in `crates/bridge/src/lib.rs` sowie `crates/cli/src/lan.rs` genutzt.
- **KI-Think & `PlayerView`:** `Ai::wants_think(tick, decided)` eingeführt; `bridge/src/ai_funcs.rs` und `ai/src/game.rs` überspringen `PlayerView`-Aufbau auf Nicht-Think-Ticks und nutzen `PlayerView::new_into` + `Sim::snapshot_for_ai_into` (ohne Soldaten-/Projektil-/Event-Snapshots, mit wiederverwendetem `Snapshot`-Puffer; doppeltes `find_site` bei `front == base` vermieden).
- **`VisionGrid`:** `VisionGrid::new` berechnet `block_min_mm` in einem linearen O(n)-Durchlauf (64× weniger Arbeit), `set_ground` dedupliziert geänderte Blöcke, `viewshed_downhill_into` nutzt Stack-`canopy`-Puffer (`<= 16`), wiederverwendetes `Viewshed::bits` in `Sim::update_vision` und aus der inneren Schleife gezogenes `has_decks`.
- **Echtes `load_map_toml_only`:** `sim::MapHeader::parse(&toml)` parst `starts` und `variants` direkt aus `map.toml`, ohne `height.png`/`terrain.png` zu lesen oder zu dekodieren; Unit-Test `load_map_toml_only_reads_without_pngs`.
- **Two-Pointer-Merge & Teleport-Fix:** `previous_positions` (`bridge/src/lib.rs`) und `soldier_previous_positions` (`bridge/src/infantry_funcs.rs`) nutzen O(n+m)-Two-Pointer-Merges statt fehlerhaftem Längenvergleich bzw. Binary-Searches; Regressionstests `previous_positions_matches_by_id_when_lengths_equal` und `previous_soldier_positions_matches_by_unit_and_index`.
- **Bridge-Hygiene & `OnceLock`:** `Sim::building_at` nutzt den 16-m-Bucket-Index (`self.infantry.index`) statt 98k-Scan; `economy_funcs.rs` borgt `&[UnitType]` und `EconomyRules::builtin_ref()`; `hud_funcs.rs` nutzt `partition_point` und `binary_search_by_key`; `vision_funcs.rs` (`FogFade`) wiederverwendet `states: Vec<CellState>`; alle `sim::*::builtin()`-Lader nutzen `OnceLock` + `builtin_ref()` (auch in `crates/editor/src/validate.rs` und `crates/ai/src/air.rs`).
- **Messung & Prüfung:** `just sim 1000 42` (`0xb2f5c679c3028f78`) und `just bench --runs 3` (`0xec3793783e796d0d`) bit-identisch; `AI think` mean 0,154→0,066 ms (−57 %, median 0,094→0,000 ms), `vision` 0,595→0,543 ms (max 4,064→3,646 ms); `bench-view` `sim` 40/150/600 m 1,94/1,03/0,68→1,73/0,80/0,46 ms (−11 % bis −32 %), FPS 73,7/74,3/73,6→75,0/75,6/74,7. `just check` und `just selftest` grün.
