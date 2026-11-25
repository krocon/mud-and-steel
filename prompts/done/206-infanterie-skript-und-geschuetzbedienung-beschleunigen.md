# Prompt 206 – Infanterie- und Geschützbedienungs-Skripte beschleunigen (CPU-Budget)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md` (Abschnitt „Performance“) und `docs/PERFORMANCE.md` (insbesondere
> stehende Armeen bei 40/150 m auf `brick_quarter`, `coastal_heights`, `dune_coast` und
> Abschnitt „Gun crews“). Im Code:
> `godot/scenes/world/infantry_view.gd`,
> `godot/scenes/world/soldier_motion.gd`,
> `godot/scenes/world/gun_crew_view.gd`,
> `godot/scenes/world/combat_view.gd`,
> `godot/scenes/world/unit_pick.gd`,
> `godot/scenes/world/effects.gd`.
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hash bleibt unverändert.

## 1. Problem

Auf großen Karten (`brick_quarter`, `coastal_heights`, `dune_coast`, `city_large`) fallen
stehende Armeen bei 40 m und 150 m unter 60 FPS (46–58 FPS). Laut `docs/PERFORMANCE.md`
schluckt das Infanterie-Skript allein **3–5 ms pro Frame** (bei 1.125 stehenden Soldaten),
und 80 Geschützbedienungs-Figuren addieren **+1.449 Draw-Calls** bei 40 m.

Konkrete Ursachen im Code:
1. **`infantry_view.gd` hat keinen Stillstands-Fast-Path:**
   - Zeilen 279–280 schreiben `member.position = position` und `member.rotation.y = s["yaw"]`
     **jeden Frame** bedingungslos auf jeden sichtbaren Soldaten – auch wenn er komplett still
     steht. Das invalidiert pro Frame 1.125 24-Bone-`Skeleton3D`-Teilbäume!
   - Zeilen 68, 167, 194–280 nutzen ungetypte String-Dictionaries (`~25–30` Lookups/Soldat =
     ~30.000+ Hash-Lookups/Frame).
   - Zeilen 212 & 222: `_soldiers.get(key, {})` allokiert in GDScript **bei jedem Aufruf** ein
     neues leeres `{}` als Default-Argument (auch bei Cache-Treffer!).
   - Zeilen 195, 218, 232: `_by_unit` wird jeden Frame geleert und mit ~125 neuen Arrays
     neu aufgebaut, obwohl sich Trupp-Mitgliedschaften nur beim Spawnen/Sterben ändern.
   - Zeile 245: `crossing.is_swimming(unit_id)` wird pro Soldat (~1.125×/Frame) statt einmal
     pro Trupp abgefragt.
   - Zeilen 268, 393: `distance_to()` (`sqrt`) statt `distance_squared_to()`.
   - Zeilen 482, 489–516: `player.speed_scale` wird jeden Frame neu geschrieben und
     `player.advance()` evaluiert im Nahbereich (<70 m) auch stehende Idle-Animationen
     jeden einzelnen Frame mit voller Skelett-Rate.
   - `Gear`- und `Weapon`-Meshes bleiben bis `MID_M = 260 m` sichtbar, obwohl Ausrüstung
     schon bei ~140–150 m Sub-Pixel-Größe hat.
2. **`soldier_motion.gd` (Zeilen 273–287):**
   `standing_clip()` läuft jeden Frame für alle stehenden Soldaten, allokiert temporäre
   3-Element-Array-Literale in Deckung und prüft `player.has_animation(clip)` jeden Frame.
3. **`gun_crew_view.gd` (Zeilen 57–60, 178–221, 401, 431–441):**
   - Kein Frustum-Culling und kein Ausblenden von `Gear`/`Weapon` im Fern-Band (bleiben bis
     450 m aktiv).
   - Ruft jeden Frame `node.get_node_or_null("Model")` auf (obwohl in `crew["model"]` gecacht),
     allokiert pro Geschütz ein `state`-Dictionary und in `_work_clip()` ein leeres `{}` per `.get(clip, {})`.
4. **Unbegrenzte Caches für zerstörte Einheiten:**
   `_soldiers`, `_factions` (`infantry_view.gd`), `_turrets`, `_max_hp` (`combat_view.gd`),
   `_boxes` (`unit_pick.gd`) und `_infantry_ids`, `_sockets`, `_guns` (`effects.gd`) räumen
   entfernte Unit-IDs über lange Basisbau-Partien nie auf.

## 2. Ziel

1. **Typisierte Zustandsklassen statt String-Dictionaries:**
   - In `infantry_view.gd` eine innere `class SoldierState:` mit typisierten Feldern
     einführen (O(1)-Feldzugriff, null `{}`-Default-Allokationen).
   - In `gun_crew_view.gd` analog `class CrewState:` / `class FigureState:` einführen,
     gecachtes `model` und vorgecachte `gun_load`-Länge verwenden.
2. **Stillstands-Fast-Path & Animations-Drosselung:**
   - `member.position` und `member.rotation.y` nur schreiben, wenn sich Position (> `1e-4` m)
     oder Yaw (> `1e-4` rad) geändert haben.
   - `player.speed_scale` nur bei Änderung setzen; `has_animation()` nur bei Clip-Wechsel.
   - Stehende/kniende/liegende **Idle-Clips** auch innerhalb von 70 m gestaffelt nur jeden
     2. oder 3. Frame per `advance()` aktualisieren (Bewegung, Schießen, Nachladen, Fallen,
     Werfen bleiben auf voller Framerate!).
   - `_by_unit` inkrementell beim Erzeugen/Entfernen pflegen statt jeden Frame neu zu bauen;
     `is_swimming(unit_id)` pro Trupp cachen; `distance_squared_to()` nutzen.
   - `soldier_motion.gd`: Array-Literale in `const`-Tabellen auslagern.
3. **Detail-LOD & Frustum-Culling:**
   - Prüfen (mit Nah-/Spielhöhen-Bildern), ob `Gear` (und optional `Weapon`) ab ~140–160 m
     ausgeblendet werden kann (statt 260 m), ohne sichtbares Ploppen.
   - In `gun_crew_view.gd` Frustum-Culling und dasselbe Detail-LOD-Band wie in
     `infantry_view.gd` ergänzen.
4. **Cache-Bereinigung:**
   - Wenn eine Einheit aus dem Spiel/Szenenbaum entfernt wird (oder periodisch gegen
     `main._unit_nodes`), verwaiste IDs aus den genannten Dictionaries entfernen.

## 3. Prüfen und zeigen

- **Benchmarks vorher/nachher direkt hintereinander:**
  - `just bench-view` (insbesondere `infantry`-Skriptzeit in ms und stehend 40/150 m FPS;
    Ziel: Infanterie-Skriptzeit bei stehenden Armeen von ~3–5 ms auf **< 1,0 ms** senken).
  - `just bench-battery` (vorher/nachher Skriptzeit & Draw-Calls der Geschützbedienung).
  - Werte in `docs/PERFORMANCE.md` dokumentieren.
- **Visuelle & funktionale Prüfung:**
  - `just selftest` (inkl. `--gun-crew-test`), `just check` grün; keine eingefrorenen
    Lauf-/Schießanimationen und kein sichtbares Ploppen an der neuen `Gear`-Distanz.

## 4. Nicht Teil dieses Prompts

- Zusammenführen der 11 Soldaten-Surfaces über `UV2` (Prompt 207).
- Auslagern der Soldaten-Animationen in geteilte Bibliotheken / Smooth-Shading (Prompt 213).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/206-infanterie-skript-und-geschuetzbedienung-beschleunigen.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis

- **Typisierte Zustandsklassen & Bridge-Entlastung:**
  - `godot/scenes/world/infantry_view.gd`: typisierte `SoldierState`-Klasse statt ungetypter String-Dictionaries (inkl. Anpassung in `soldier_motion.gd`, `care_view.gd`, `unit_pick.gd`, `unit_highlight.gd` und Selftests); `_by_unit` inkrementell gepflegt; `sim_host`-Snapshot-Arrays & zusammenhängende Squad-Spans pro `current_tick()` gecacht (spart 19 FFI-Kopien auf 2 von 3 Frames), O(1)-Squad-Skipping (`_squad_ready`/`_squad_accum`), `is_swimming` pro Trupp, `distance_squared_to()`, Impostor-Stillstands-Fast-Path.
  - `godot/scenes/world/soldier_motion.gd`: Deckungs-Clip-Tabellen als `const`-Arrays ausgelagert, `has_animation()` nur bei Clip-Wechsel, `get_up_back`-Länge gecacht.
  - `godot/scenes/world/gun_crew_view.gd`: typisierte `CrewState`/`FigureState`-Klassen, gecachtes `model` und vorgecachte `gun_load`-Länge, Frustum-Culling (`CULL_MARGIN_M = 25 m`) und 150-m-Detail-LOD (`Gear`/`Weapon`).
- **Stillstands-Fast-Path, Idle-Drosselung & Detail-LOD (vorläufig):**
  - `member.position`/`member.rotation.y` nur bei Änderung (> `1e-4`), `player.speed_scale` nur bei Änderung.
  - Ruhige Idle-Clips laufen gestaffelt alle `NEAR_IDLE_STEP = 3` (nah) bzw. `MID_IDLE_STEP = 6` Frames (Mitte); Bewegung, Zielen, Schießen, Nachladen, Hechten, Fallen, Verwundetenhilfe bleiben auf voller Rate.
  - `DETAIL_M = 150 m` (vorläufig) blendet `Gear`/`Weapon`-Meshes an Soldaten & Geschützbedienungen aus (visuell bei `standing_150.png` und `battery_on_150.png` geprüft).
- **Cache-Bereinigung:** Periodische Bereinigung verwaister Unit-IDs (`prune_caches()`) in `infantry_view.gd`, `gun_crew_view.gd`, `combat_view.gd` (`_turrets`, `_max_hp`), `unit_pick.gd` (`_boxes`) und `effects.gd` (`_infantry_ids`, `_sockets`, `_guns`).
- **Benchmarks (`docs/PERFORMANCE.md`, M1 Max, 1280 × 720, `high`):**
  - `bench-view --perf-quick` stehend 40 / 150 / 600 m: `infantry` **1,78 / 2,64 / 2,51 ms → 0,29 / 0,90 / 1,13 ms**, FPS **67,3 / 63,3 / 69,4 → 71,0 / 71,8 / 72,3 FPS**.
  - `bench-battery` 40 / 150 m: `gun crews` **0,55 / 0,45 ms → 0,26 / 0,25 ms**, Draw-Calls bei 150 m **1435 → 1226**.
- Verifiziert mit `just selftest` (inkl. `--gun-crew-test`) und `just check` (Simulation & Golden Hashes unverändert).
