# Prompt 013 – Vegetation und Objekte platzieren

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md`, `docs/DETERMINISM.md`, `docs/MAPS.md` und den Abschnitt
> „Environment objects“ in `docs/ASSETS.md`. Voraussetzungen: 007 (Modelle),
> 010 (Karte), 011 (Terrain-Renderer), 012 (Wegfindung).

## 1. Ziel

Das Gelände so dicht bestücken wie in den Stil-Vorlagen: Wäldchen, Hecken an
Feldwegen, Dörfer mit Höfen, Mauern und Obstgärten, Einzelbäume, Felsen, Heuhaufen,
Telegrafenmasten, dazu Gras, Blumen und Büsche als Deko. Flüssig auf der 4-km-Karte
`test_large`. Spielrelevante Objekte gehören der Simulation und blockieren die
Wegfindung; Deko existiert nur in Godot.

## 2. Umfang

### 2.1 Karten-Objekte in der Simulation (`crates/sim`)

1. **Objektregeln** `data/objects.toml` (in `sim` einkompiliert wie `terrain.toml`):
   pro Objektart (Katalog-Id aus `data/catalog/neutral.toml`) die Blockade-Klasse
   (`none`, `small`, `large`, `all`, bezogen auf die Größenklassen der Wegfindung)
   und die Grundfläche (`circle` mit Radius, z. B. Stamm, oder `box`; Box-Maße
   standardmäßig Länge × Breite aus dem Katalog). Das sind Spielregeln, keine
   Katalogwerte.
2. **Objektliste pro Karte** `data/maps/<name>/objects.csv` (optional, Git LFS):
   eine Zeile pro Objekt `kind,variant,x_mm,y_mm,heading_deg`. Ganzzahlig, `heading_deg`
   0–359 (0: die Blender-Achse +Y des Modells zeigt nach Norden, also −y in der Sim;
   im Uhrzeigersinn von oben gesehen, 90 = Osten). Unbekannte Arten, Varianten-Namen ohne Modell oder
   Objekte außerhalb der Karte sind Ladefehler.
3. `Map::add_objects(text)` liest die Liste, löst Arten gegen die Regeln auf und nimmt
   Objekte samt aufgelöster Grundfläche in den **Karten-Hash** auf. `Map::load`
   bleibt unverändert; CLI, Bridge und die einkompilierte `test_small` laden die
   Datei mit, wenn sie existiert.
4. **Wegfindung:** `PathGrid::new` rastert die Grundflächen in zwei neue Zell-Flags
   (blockiert klein / blockiert groß). Eine Zelle ist blockiert, wenn die Grundfläche
   sie echt überlappt (Kreis: nächster Punkt, Box: Trennachsen-Test in `Fixed`).
   Drehung über eine feste Sinustabelle in Ganzzahlen (kein `f64` in `sim`).
   `set_path_obstacle` (Zerstörung, Prompt 020) bleibt davon getrennt.
5. Startregeln für Spawns: Szenario-Einheiten auf blockierten Zellen sind ein Fehler
   der Karte; der Generator hält Wiese, Startpunkte und Testziele frei.

### 2.2 Generator (`cli gen-map`)

Objekte mit eigenem Zufallsstrom (Höhen und Geländearten bleiben bit-identisch):

- **Wäldchen** aus einer Rausch-Maske (Laub-, Nadel- und Birkenbestände, Birken am
  Rand), Abstand ca. 6 m, Waldwege bleiben ≥ 12 m frei.
- **Hecken** als Segmentketten neben Feldwegen, mit Lücken (Feldeinfahrten).
- **Dörfer** um die Dorfplätze: Häuser und Scheunen zum Platz gedreht, eine Kirche
  im ersten Dorf, Höfe mit Mauern oder Zäunen, **Obstgärten** (junge Laubbäume im
  Raster) hinter den Höfen, Heuhaufen, Holzstapel, Kisten und Fässer.
- **Allee** und **Telegrafenmasten** an der Straße, Einzelbäume und Heuhaufen in
  Feldern, Felsen an Hängen.
- Frei bleiben: Startwiese, Flussbett, Furten, Gleise der Wege, der Gipfel des
  Wahrzeichen-Hügels und der Weg von der Wiese dorthin (Selbsttest, Demo).

### 2.3 Darstellung (Godot, nur Präsentation)

1. Bridge liefert die Objekte gruppiert nach (Art, Variante, Chunk) als fertige
   `MultiMesh`-Puffer (Bodenhöhe, Drehung, leichte Größenstreuung bei Pflanzen nur
   in der Darstellung).
2. `godot/scenes/world/map_objects.gd`: pro Chunk (128 m wie das Terrain) und Mesh
   eine `MultiMeshInstance3D`; Gebäude als Teile (`_Walls`, `_Roof`, `_Floor`) mit
   gleichen Transformationen; Bäume nah mit vollem Mesh, ab 180 m mit `_Far`-Mesh
   (Sichtweiten-Bereiche); Sichtweite pro Objektklasse; Schatten nur für Bäume,
   Gebäude, Mauern, Hecken und große Büsche.
3. **Deko** (Gras, Blumen, Büsche, kleine Felsen, Baumstümpfe): in Rust (Bridge)
   pro Chunk aus Geländeart, Rauschen und Nähe zu Bäumen gestreut, fester Seed,
   nicht in Gebäuden, Mauern und auf Wegen; nur Chunks nahe der Kamera, beim
   Verlassen freigegeben.
4. Biome über `EnvLook` (Instanz-Uniforms pro `MultiMeshInstance3D`, Standard
   `forest`); Wind nutzt die vorhandenen Shader-Parameter, Phase aus der Instanz-Position.
5. Debug-Menü (F3): Objekte und Deko ein-/ausblenden; Instanz-Zahlen im Overlay
   des Benchmarks.

## 3. Akzeptanzkriterien

1. `just check`, `just selftest`, `just gallery-check` grün. Golden-Hash bewusst
   geändert (Objekte auf `test_small`), Grund im Commit.
2. Rust-Tests: Objektliste laden (Fehlerfälle), Hash deckt Objekte ab, Rasterung von
   Kreis und gedrehter Box, Panzer umfahren Baumreihe und Mauer durch eine Lücke,
   Infanterie geht über die Mauer, Haus blockiert beide; zwei Generator-Läufe ergeben
   dieselbe Datei.
3. Selbsttest: Objekte stehen auf der Simulations-Bodenhöhe, Instanz-Zahl der
   Darstellung = Objekte der Karte, Deko erscheint nahe der Kamera.
4. Messwerte im Bericht und in der Doku: Objekt-Zahl, Ladezeit, Speicher,
   `just path-bench` vorher/nachher, Bildrate, Draw Calls, Dreiecke und Instanzen
   auf `test_large` nah (15 m), mittel (120 m), fern (600 m).
5. Bildvergleich per `--write-movie` im Scratchpad, im Bericht beschrieben.
6. `docs/MAPS.md` (Objektformat, Generator), `docs/ARCHITECTURE.md` (Objekte in
   Wegfindung und Darstellung), `docs/DETERMINISM.md` (Golden-Hash) aktualisiert.

## 4. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Wo liegen Objekte? | Platzierung in der Karte (Hash), Regeln in `data/objects.toml`, beim Laden aufgelöst | Kein neues Feld in `SimConfig`, also kein neues Replay-Format (Prompt 025 arbeitet parallel im `net`-Crate); eine Regeländerung ändert den Karten-Hash und alte Replays melden das sauber. |
| Format | CSV-Text, Ganzzahlen, Git LFS | Lesbar und in jedem Werkzeug bearbeitbar, ohne neue Abhängigkeit; zehntausende Zeilen parsen in Millisekunden; LFS wie die PNGs. |
| Blockade | Zell-Flags im 4-m-Raster der Wegfindung | Bewegung und Pfade nutzen dasselbe Raster; kein zweites Kollisionssystem. Bäume blockieren nur große Einheiten (Wald ist für Panzer zu, für Infanterie offen), Mauern und Hecken ebenso, Häuser alles, Zäune, Büsche und Heu nichts. |
| Deko | Nur Godot, in der Bridge gestreut | Kein Einfluss aufs Spiel; Rust streut zehntausende Instanzen pro Chunk schneller als GDScript. |
| Ferne Bäume | `_Far`-Meshes aus 007 über Sichtweiten | Belegt in 007; keine Impostor-Texturen. |
| Chunks | 128 m wie das Terrain | Wenige tausend Knoten auf 4 km, Culling pro Chunk. |

## 5. Arbeitsweise

Mit mir Deutsch, im Repo Englisch. Kein Push, keine Installation ohne mein Go. Kein
Test wird abgeschwächt. Parallel laufen 015 (Licht, Biome) und 025 (LAN): gemeinsame
Dateien (`main.gd`, `main.tscn`, `justfile`, Doku) nur klein und additiv ändern;
`-- --demo` und der Selbsttest müssen weiter laufen. Commits pro Schritt, am Ende
nach `prompts/done/` verschieben und `prompts/README.md` aktualisieren.
