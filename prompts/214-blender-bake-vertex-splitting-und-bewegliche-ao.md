# Prompt 214 – Blender-AO-Bake reparieren: kein Vertex-Splitting auf glatten Flächen und keine eingefrorenen Schatten beweglicher Teile

> In einer frischen Session im Repo-Ordner ausführen. Benötigt Blender (`docs/VERSIONS.md`).
> Lies zuerst `CLAUDE.md` und `docs/ASSETS.md` (Abschnitte „Material system“, „Environment objects“).
> Im Code:
> `tools/blender/kit/bake.py`,
> `tools/blender/kit/core.py`,
> `tools/blender/kit/env.py`,
> `tools/blender/kit/build.py`,
> `tools/blender/validate.py`,
> `tools/blender/validate_env.py`.
> Reine Asset-Pipeline – **keine Änderungen in `crates/sim`**, Golden Hash unverändert.

## 1. Problem

Die Analyse von `tools/blender/kit/bake.py` und allen 438 exportierten `.glb`-Dateien
hat drei konkrete Fehler im Vertex-Color-/AO-Bake aufgedeckt:

1. **Unnötiges Vertex-Splitting auf smooth-geshadeten Flächen (`bake.py:95-104`):**
   - `bake.py` speichert `Color` auf der `CORNER`-Domäne (`FLOAT_COLOR` → 16-Bit `USHORT VEC4`
     im glTF) und nutzt als Cache-Key pro Loop `key = (vi, round(n.x, 2), round(n.y, 2), round(n.z, 2))`,
     wobei `n = (nm @ poly.normal).normalized()` die **Flächennormale** ist!
   - Auf **smooth-geshadeten** Flächen (Gusstürme wie Sherman/T-34, Geschützrohre, Laufrollen,
     Flugzeugrümpfe) unterscheiden sich `poly.normal` benachbarter Flächen an jedem geteilten
     Vertex → jede Ecke bekommt einen leicht anderen AO-Wert → der glTF-Exporter muss
     jeden glatten Vertex mehrfach aufspalten!
   - Gemessen: `us_m4.glb` `Turret[0]` verdoppelt sich von **180 auf 353 Vertices**;
     `Wheel_R_01[0]` von **48 auf 96**; `de_pz4_h.glb` `Gun[1]` von **100 auf 269**.
2. **Statische AO-Verschattung zwischen beweglichen Baugruppen (`bake.py:68-81`):**
   - `bake(objects)` wirft **alle** Meshes eines Fahrzeugs (`Hull`, `Turret`, `Gun`,
     `Wheel_*`, `Sprocket_*`, `Idler_*`, `Propeller`, `Ramp`, `Bridge`, `Mount`) in
     **einen einzigen `BVHTree`**.
   - Folgen im Spiel:
     - Dreht sich ein Panzerturm um 90°/180°, bleibt der eingebrannte dunkle Schatten des
       Turms und Rohrs fest nach vorne (+Y) auf dem Wannendach stehen.
     - Klappt die Bug-Rampe (`Ramp`) eines Landungsboots herunter oder wird eine getragene
       Brücke (`Bridge`) abgelegt, bleibt deren dunkler Schatten am Rumpf kleben.
     - Bei Flugzeugen wirft der statische `Propeller` AO-Schatten auf die Nase (und umgekehrt).
     - Bei Flaks brennt die gedrehte `Mount` ihren Schatten in die Kreuzlafette.
3. **Veraltete Umwelt-Modelle mit Varianten-Selbstverschattung (`env.py:510`, `docs/ASSETS.md:621`):**
   - `env.py:510` merkt an, dass Umwelt-Modelle, die *vor* Prompt 059 gebaut wurden, noch
     ihre `Intact`-, `Damaged`- und `Ruin`-Varianten gegenseitig am Ursprung verschattet im
     AO-Bake tragen.
   - `docs/ASSETS.md:621` nennt zudem `env_castle_chapel` als noch nicht neu gebautes Altmodell
     mit dem korrigierten Kirchturm-Schallloch-Fix.

## 2. Ziel

1. **Smooth-Normalen im AO-Bake nutzen (`bake.py`):**
   - Für Polygone mit `poly.use_smooth == True` die geglättete Vertex-Normale
     (`me.vertices[vi].normal` bzw. `me.corner_normals[li].vector` nach `nm`-Transformation)
     sowohl als Ray-Hemisphären-Normale als auch als Cache-Key verwenden, und den
     Origin-Offset symmetrisch am Vertex (nicht Richtung `poly.center`, da das pro Polygon
     abweicht!) ansetzen – unter Beibehaltung der Rückseiten-Durchlässigkeit `_first_front_hit`.
   - Für harte/flache Flächen (`poly.use_smooth == False`, z. B. gewalzte Panzerplatten)
     bleibt die bisherige Flächennormalen-Logik erhalten, damit scharfe Plattenkanten weiter
     saubere AO-Kontraste ohne Bleed zeigen.
   - Prüfen, ob `BYTE_COLOR` statt `FLOAT_COLOR` für `Color` die glTF-Dateigröße messbar
     reduziert (8-Bit `UBYTE VEC4` = 4 B statt 8 B pro Vertex) ohne sichtbares Banding auf
     AO/Höhe/Wear (und ob `validate.py`/`validate_env.py` damit grün bleiben).
2. **Bewegliche Baugruppen im Fahrzeug-AO sauber trennen (`bake.py`):**
   - Statische Rumpfgruppe (`Hull` + feste Anbauteile, Ketten, Schürzen, Räder) gegen sich
     selbst + Bodenplane backen (ohne `Turret`, `Mount`, `Gun`, `Ramp`, `Bridge`, `Propeller`,
     `Bomb_*`, `Rocket_*`, `Gear_*` im Occluder-BVH!).
   - Drehbaugruppe (`Turret` + `Cupola` + `TurretSkirt` + `Turret`-MGs, oder `Mount` bei Flaks):
     gegen sich selbst + eine **rotationssymmetrische Boden-/Deck-Scheibe** auf Turmringhöhe
     backen, damit die Turmunterkante eine natürliche, rundum gleichmäßige Kontaktabdunklung
     behält, die beim Drehen immer stimmt!
   - `Gun`, `Ramp`, `Bridge`, `Propeller`, `Bomb_*`, `Rocket_*`: jeweils eigenständig
     (Selbst-AO) backen, sodass sie beim Bewegen/Ausblenden keine Geisterschatten hinterlassen.
   - Hinweis: `bake_env()` (`env.py:517`) ruft `bake()` bereits pro Variante auf und bleibt
     kompatibel.
3. **Modelle kontrolliert neu bauen:**
   - Alle betroffenen Fahrzeug- und veralteten Env-Modelle (inkl. `env_castle_chapel` und
     vor Prompt 059 gebaute Multi-Varianten-Env-Modelle) neu exportieren (`just units`,
     `just env ...`), `just import`, `just validate-assets`.
   - `source_hash` oder `KIT_VERSION` prüfen, damit klar ist, welche Modelle den neuen Bake haben.

## 3. Prüfen und zeigen

- **Messwerte vorher/nachher:**
  - Vertex-Zahlen auf glatten Meshes (`us_m4.glb` `Turret`, `Wheel_*`; `de_pz4_h.glb` `Gun`;
    `de_fw190_a8.glb` `Hull`) und Gesamtgröße der Fahrzeug-GLBs vergleichen.
- **Visuelle Prüfung:**
  - Panzer mit um **90° und 180° gedrehtem Turm** (`de_pz4_h`, `us_m4`, `su_t34_76`): kein
    eingefrorener Turm-/Rohrschatten mehr auf dem Motordeck, dennoch satter Kontaktschatten
    unter der Wanne und den Laufrollen!
  - Landungsboot (`us_lcvp`) mit geöffneter Rampe und Brückenleger ohne Geisterschatten.
  - `env_castle_chapel` und ein neu gebackenes vor-059-Haus im Vergleich (`Intact`/`Damaged`/`Ruin`).
- **Thumbnails & Checks:**
  - Betroffene Thumbnails neu rendern (`just thumbnails`), `just units-web-sync`,
    `just validate-assets`, `just gallery-check`, `just check`, `just selftest` grün.

## 4. Nicht Teil dieses Prompts

- Soldaten-Pipeline (Prompt 213).
- Geometrie-/Detail-Umbau an Panzern, Flugzeugen und Häusern (Prompt 215).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/214-blender-bake-vertex-splitting-und-bewegliche-ao.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.
