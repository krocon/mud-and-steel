# Prompt 210 – `PathGrid`-Kosten deduplizieren, `SimSnapshot` entschlacken und Per-Tick-Allokationen abbauen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md` (Abschnitt „Snapshots and the rules hash“),
> `docs/ARCHITECTURE.md` („Pathfinding“) und `docs/PERFORMANCE.md`. Im Code:
> `crates/sim/src/path.rs`,
> `crates/sim/src/sim.rs`,
> `crates/sim/src/snapshot.rs`,
> `crates/sim/src/sim/spatial.rs`,
> `crates/sim/src/sim/combat.rs`,
> `crates/sim/src/sim/vision.rs`,
> `crates/net/src/snapshot.rs`.
> **Determinismus:** Das Simulationsverhalten und alle Golden Hashes (`determinism.rs`,
> `economy.rs`, `cli bench`) müssen **bit-identisch** bleiben! (Falls sich das
> Snapshot-/Savegame-Binärlayout durch `#[serde(skip)]` ändert, `FORMAT_VERSION` in
> `crates/net/src/snapshot.rs` sauber anheben und in `docs/DETERMINISM.md` vermerken.)

## 1. Problem

Auf großen Karten (4 km `test_large`, 6 km `coastal_heights`) mit vollem Einheitenkatalog
verbrauchen `PathGrid`, `ObjectIndex` und `SimSnapshot` unnötig viel RAM, Ladezeit und CPU:

1. **`PathGrid` dupliziert `cost: Vec<u16>` für jeden `UnitType` (~255 MB RAM auf `coastal_heights`):**
   - `crates/sim/src/path.rs:181-208, 511-555` dedupliziert zwar `ComponentGrid` nach
     `(class, tier, crush)`, allokiert aber eine eigene `cost: Vec<u16>`-Tabelle
     (`width * height`) für **jeden** Eintrag in `SimConfig::unit_types` (~100–200 Typen im
     Basisbau-Modus) – inklusive **unbeweglicher Gebäude** (`speed == 0`) und Dutzender
     identischer Bewegungsprofile!
   - Auf `coastal_heights` (1.500 × 850 = 1,275 Mio. Zellen = 2,55 MB/Grid) sind das
     **~255 MB RAM** allein für doppelte Kosten-Grids.
   - Jede Krater-/Baum-/Wrack-/Wetter-Änderung (`recost_rows:859`, `rebuild_cells:1420`)
     rechnet alle 100+ Tabellen durch – inkl. Gebäuden.
   - Da `PathGrid` in `SimSnapshot` serialisiert wird, landen alle ~100 doppelten Grids im Spielstand.
2. **Abgeleitete & statische Daten blähen `SimSnapshot` massiv auf:**
   - `sim.rs:578`: `unit_grid: spatial::UnitGrid` (laut Kommentar in `spatial.rs:6-8`
     *„neither hashed nor part of snapshots“*) fehlt `#[serde(skip)]` und wird mitserialisiert.
   - `sim.rs:549`: `events: Vec<CombatEvent>` (Per-Tick-Ausgabe) fehlt ebenfalls `#[serde(skip)]`.
   - `sim.rs:567`: `objects: combat::ObjectIndex` ist nach `ObjectIndex::new(&map)` komplett
     unveränderlich, wird aber trotzdem in Snapshots gespeichert **und** als `Vec<Vec<u32>>`
     gehalten – **63.001 Heap-`Vec`s** auf `test_large` bzw. **80.464** auf `coastal_heights`
     (und ebenso viele Einzel-Allokationen beim Laden!).
   - `path.rs:2310-2317`: `Pathfinder::workspace.nodes` (1,275 Mio. × 12 B = **15,3 MB**
     A*-Scratch-Speicher) wird auch dann serialisiert, wenn `active.is_none()`!
3. **Vermeidbare Per-Tick-Heap-Allokationen in `crates/sim`:**
   - `sim.rs:1293-1383` (`move_units`): 7–8 frische `Vec`s jeden Tick (`directions`,
     `waypoints`, `near`, `pushes`, `caps`, `blocked`, `stalled`, `driven`).
   - `sim/spatial.rs:61` (`UnitGrid::rebuild`): `self.start.clone()` (16 KB) 2× pro Tick.
   - `sim.rs:998-1004` (`step_observed`): `exempt: Vec<PlayerId>` jeden Tick (bei `MAX_PLAYERS == 64`
     reicht ein `u64`-Bitmask).
   - `sim/combat.rs:431` (`ObjectIndex::hit`): allokiert/sortiert `candidates: Vec<u32>` pro Projektil-Segment.
   - `sim/vision.rs:327-335`: allokiert `windows: Vec<usize>` jeden Tick für jeden besetzten Trupp.

## 2. Ziel

1. **`PathGrid`-Kostentabellen deduplizieren:**
   - `cost_grids: Vec<CostGrid>` (o. ä.) einführen, indiziert über `Profile::cost_grid: Option<usize>`
     (`None` für unbewegliche Typen `!t.mobile()`), mit Schlüssel
     `(class, offroad, road, tier, crush, draft_mm, ground_pct)` (exakt alle Parameter, die
     `cell_cost` beeinflussen – sorgfältig prüfen!).
   - `recost_rows`, `rebuild_cells` und `PathGrid::with_crushing` arbeiten nur noch auf den
     deduplizierten ~10–15 `CostGrid`s statt 100–200 Profilen.
   - Sicherheits-Test: Ein Test vergleicht für alle Katalog-Typen, dass `cost(profile, cell)`
     für jede Zelle exakt dem bisherigen Wert entspricht.
2. **`ObjectIndex` auf CSR-Layout umstellen & `SimSnapshot` entschlacken:**
   - `combat::ObjectIndex` von `buckets: Vec<Vec<u32>>` auf flaches CSR (`start: Vec<u32>`,
     `items: Vec<u32>`, analog zu `UnitGrid`) umstellen; in `ObjectIndex::hit` einen
     wiederverwendeten/Stack-Puffer nutzen (auf Determinismus der Trefferreihenfolge achten!).
   - Auf `Sim`: `#[serde(skip)]` für `unit_grid`, `events` und `objects`; in `Sim::restore()`
     (`crates/sim/src/snapshot.rs:99-107`) `objects = ObjectIndex::new(&map)` (und `unit_grid`
     mit Default/Kapazität) neu aufbauen.
   - In `Pathfinder`: `workspace.nodes` nicht serialisieren, wenn `active.is_none()`
     (z. B. eigene `Serialize`/`Deserialize`-Hilfsstruktur, die bei `active.is_none()` ein
     leeres `Vec` schreibt und bei Bedarf beim nächsten `Search::begin` lazy allokiert).
   - Alle bestehenden Snapshot- & Save-Roundtrip-Tests (`crates/net/tests/snapshot.rs`,
     `crates/ai/tests/save.rs`) müssen weiterhin bit-identische Fortsetzung beweisen!
3. **Per-Tick-Scratch-Puffer wiederverwenden:**
   - `#[serde(skip)]` Scratch-Struct für `move_units` auf `Sim` und `next: Vec<u32>` auf `UnitGrid`.
   - `exempt: Vec<PlayerId>` durch `u64`-Bitmask ersetzen.
   - `sim/vision.rs:327-335`: Fenster besetzter Gebäude inline iterieren ohne `Vec`-Allokation.

## 3. Prüfen und zeigen

- **Determinismus & Korrektheit:**
  - `just check` grün, **alle Golden Hashes unverändert**!
  - `crates/net/tests/snapshot.rs` und `crates/ai/tests/save.rs` grün (inkl. Snapshot mitten
    während einer laufenden A*-Suche *und* ohne aktive A*-Suche!).
- **Messungen vorher/nachher:**
  - `PathGrid`-RAM-Verbrauch auf `test_large` und `coastal_heights` mit vollem Economy-Katalog
    (Ziel: ~85–90 % Reduktion).
  - `just save-bench` (Snapshot-Größe unkomprimiert/komprimiert sowie Speicher-/Ladezeit).
  - `just bench --runs 3` und `just crater-bench` vorher/nachher (End-Hash unverändert!).
  - Messwerte in `docs/PERFORMANCE.md` dokumentieren.

## 4. Nicht Teil dieses Prompts

- Änderung der A*-Heuristik oder Budget-Regeln (Prompt 211).
- Änderungen an der Godot-Präsentation oder 3D-Modellen.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/210-pathgrid-deduplizieren-und-snapshot-entschlacken.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.
