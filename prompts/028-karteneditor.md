# Prompt 028 – Karteneditor

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md`, `docs/MAPS.md` und `docs/DETERMINISM.md`.

## 1. Ziel

Karten im Spiel bauen statt nur per `cli gen-map`: Höhen formen, Geländearten und
Wasser malen, Objekte und Vegetation setzen, Startpositionen festlegen, prüfen und
im bestehenden Kartenformat (`data/maps/<name>/`: `map.toml`, `height.png`,
`terrain.png`, `objects.csv`) speichern. Eine gespeicherte Karte erscheint im
Einzelspieler-Menü und lässt sich spielen; die Simulation lädt sie über denselben
Weg wie jede andere Karte, deterministisch.

## 2. Grundsatzentscheidung: Editor-Modus im Spiel

Entschieden: **eigener Editor-Modus im Spiel** (Hauptmenü → „Map editor“), kein
Godot-Editor-Plugin.

- Der Editor nutzt die vorhandene Darstellung unverändert weiter: Terrain-Renderer mit
  Bereichs-Neuaufbau (`terrain.gd: heights_changed(rect)`), Objekt-Darstellung per
  MultiMesh (`map_objects.gd`), Wasser (`water.gd`), Atmosphäre/Biome, RTS-Kamera und
  Boden-Picking ohne Physik. Ein Plugin bräuchte `@tool`-Fassungen all dieser Skripte
  und eine GDExtension, die im Godot-Editor läuft; beides gibt es nicht.
- Das Kartenformat und alle Regeln (Objektregeln, Brücken, Wegfindung) liegen in Rust.
  Ein Editor-Modus im Spiel ruft sie direkt über die Bridge auf; Speichern und Prüfen
  laufen durch denselben Code, mit dem die Simulation lädt.
- Er funktioniert in jedem Build des Spiels, nicht nur mit geöffnetem Godot-Editor,
  und lässt sich wie das übrige Spiel headless per Selbsttest prüfen.
- Nachteil: eigene Werkzeug-UI statt Godot-Inspector. Für den Umfang (Pinsel,
  Listen, Schieberegler) ist das wenig Code.

## 3. Umfang

1. **Editier-Kern in Rust**, neues Crate `crates/editor` (kein Godot, keine Floats,
   gleiche Lints wie `sim`), damit dasselbe Skript dieselben Dateien und denselben
   Karten-Hash ergibt:
   - `EditMap`: Raster (Höhen in mm, Gelände-Ids), Objektliste
     (`kind, variant, x_mm, y_mm, heading_deg`), Startpositionen, Marker, Biom,
     Fließrichtung. Neue Karte in wählbarer Größe (Vielfaches von 4 m, 2-m-Raster),
     Laden aus einem Kartenordner, Speichern in einen Kartenordner.
   - Pinsel (Mittelpunkt, Radius, Stärke) mit glatter Ganzzahl-Abnahme:
     **Anheben/Absenken**, **Glätten** (3×3-Mittel), **Ebnen** (auf eine Zielhöhe),
     **Gelände malen** (jede Id aus `terrain.toml`), **Fluss** (Bett ausheben mit
     Querprofil, tiefes Wasser in der Mitte, flaches am Rand, Sand am Ufer; Pegel
     vom Strich-Anfang), **Objekte streuen** (Liste von Arten/Varianten, Dichte pro
     Hektar, Mindestabstand, nicht auf Wasser und Wegen; Zufall aus `sim::Rng` mit
     Editor-Seed), **einzelnes Objekt setzen** (Brücken rasten auf 90° und die
     Mittellinie des 4-m-Wegrasters ein), **Objekte entfernen**, **Startposition
     setzen**.
   - **Rückgängig/Wiederholen** pro Pinselstrich (`begin_stroke`/`end_stroke`):
     geänderte Raster-Rechtecke vorher/nachher, Objektliste und Metadaten.
   - **Speichern**: Höhen-Kodierung (`height_base_mm`, `height_step_mm`) aus dem
     Höhenbereich wie im Generator; wird die Stufe gröber als 1 mm, übernimmt der
     Editor die quantisierten Werte, damit Datei und Editor gleich sind. Gelände-PNG
     indiziert mit der Palette aus `terrain.toml`.
   - **Validierung** über genau den Ladeweg der Simulation (`Map::load` +
     `add_objects`): Fehler beim Laden (z. B. falsch ausgerichtete Brücke),
     Startpositionen für Spieler 0 und 1, Aufstellraum der Szenario-Einheiten
     befahrbar, und **erreichbare Wege** zwischen den Starts für die Panzer aller
     Fraktionen (Fehler) und Infanterie (Warnung) mit `PathGrid::find_path`. Ergebnis
     mit Karten-Hash.
2. **Simulation (klein, additiv):** Das Szenario stellt die Gruppen eines Spielers an
   der Startposition der Karte auf (verschoben um die Differenz zur ersten Gruppe des
   Spielers). Für `test_small` und `test_large` sind die Starts gleich den
   Szenario-Mittelpunkten, der Golden-Hash bleibt unverändert.
3. **Bridge:** Godot-Klasse `MapEditor` (eigene Datei) mit Editier-, Undo-,
   Speicher- und Prüffunktionen sowie derselben Lese-Schnittstelle, die
   `terrain.gd`, `map_objects.gd`, `water.gd` und `ground_pick.gd` von `SimHost`
   nutzen (`terrain_layers`, `object_groups`, `water_mesh`, `ray_pick`, …). Die
   Renderer nehmen dafür einen untypisierten Host statt `SimHost` (wenige Zeilen).
4. **Godot:** Szene `scenes/editor/map_editor.tscn` mit Werkzeugleiste (Werkzeug,
   Radius, Stärke, Dichte, Geländeart, Objekt-Vorlagen), Pinsel-Kreis auf dem Boden,
   Start-Markierungen, Dialogen für Neu (Name, Größe, Biom), Öffnen, Speichern,
   Prüfen; Strg+Z / Strg+Y. Terrain wird pro Pinsel-Tupfer im geänderten Rechteck neu
   aufgebaut, Objekte und Wasser am Strich-Ende. Hauptmenü bekommt den Knopf
   „Map editor“; gespeicherte Karten landen in `data/maps/<name>/` und stehen damit
   in der Kartenwahl des Einzelspieler-Menüs.
5. **CLI/justfile:** `cli map-check <dir>` (Validierung wie im Editor),
   `just map-check <name>`, `just editor`, `just editor-selftest`.

Nicht im Umfang: Marker bearbeiten (bleiben beim Laden/Speichern erhalten), Flüsse
mit Gefälle, Straßen-Werkzeug mit Kurven (Gelände „road“ wird gemalt), Mehrspieler-
Szenarien mit mehr als zwei Startpositionen.

## 4. Akzeptanzkriterien

1. `just check` grün, **Golden-Hash unverändert**.
2. Rust-Test (`crates/editor/tests`): baut per Skript eine Karte (neue Karte, alle
   Höhen-Werkzeuge, Straße, Fluss mit Furt, Wald per Pinsel, Haus, Brücke,
   Startpositionen), prüft Rückgängig/Wiederholen, speichert sie in einen
   Temp-Ordner, lädt sie auf **zwei Peers** in die Simulation und spielt das
   Demo-Szenario per Lockstep über Loopback: identische Hashes pro Tick, kein
   Desync. Ein zweiter Lauf desselben Skripts ergibt byte-gleiche Dateien.
3. Tests für Validierung (fehlende Startposition, unerreichbares Ziel, schiefe Brücke)
   und für die Aufstellung an den Startpositionen der Karte.
4. `just editor-selftest`: öffnet den Editor aus dem Hauptmenü, baut eine Karte mit
   den Werkzeugen, prüft Undo/Redo und das Neuzeichnen, prüft, speichert, wählt sie im
   Einzelspieler-Menü, startet das Match, Einheiten fahren; danach wird die Testkarte
   wieder gelöscht.
5. `just selftest` (inkl. Flow-Selbsttest), `-- --demo`, `just lan-selftest` und
   `just gallery-check` funktionieren weiter.
6. `docs/MAPS.md` (Editor, Aufstellung an Startpositionen) und
   `docs/ARCHITECTURE.md` (Abschnitt Karteneditor) aktualisiert.

## 5. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Plugin oder Spielmodus | Spielmodus | siehe Abschnitt 2 |
| Wo liegt die Editier-Logik | Rust-Crate `editor`, nicht `sim` | `sim` bleibt Spielregeln; der Editor ist ein Werkzeug wie `cli gen-map`, nutzt aber die Typen und den Ladeweg von `sim` |
| Floats im Editor | Nein, Ganzzahlen (mm, Promille) | gleiche Skripte → gleiche Dateien; testbar ohne Toleranzen |
| Datenhaltung für die Darstellung | `MapEditor` liefert dieselbe Lese-Schnittstelle wie `SimHost` | Renderer bleiben unverändert, kein Sim-Objekt ohne Spiel nötig |
| Undo-Granularität | ein Pinselstrich | so erwartet man es aus Malprogrammen |
| Kartenformat | unverändert | Editor schreibt, was der Generator schreibt; `generator`-Feld bleibt erhalten |
| Speicherort | `data/maps/<name>/` | dort sucht das Menü; PNGs landen per `.gitattributes` in Git LFS |

## 6. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, Commits pro Schritt. Parallel laufende Prompts
(018, 019) berühren dieselben Dateien: Änderungen an `crates/sim`,
`crates/bridge/src/lib.rs`, `main.gd`, Menüs, `justfile` und Doku klein und additiv
halten. Am Ende nach `prompts/done/` verschieben und `prompts/README.md`
aktualisieren. Abschlussbericht: was gebaut wurde, wie ich es teste, Messwerte, was
offen ist.
