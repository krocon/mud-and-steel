# Prompt 222 – Umland um die Karte (kein leerer Hintergrund am Kartenrand)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Abschnitte „Camera rig“ → „Perspectives“ / „Map edge“,
> „Lighting, atmosphere and biomes“, Gelände/Chunks/LOD) und `docs/PERFORMANCE.md`
> („Camera perspectives“, „Far camera distances“). Im Code:
> `godot/scenes/terrain/terrain.gd` (`_build_edge`, `edge_depth`, `_map_bounds`, Chunks/LOD),
> `godot/assets/materials/terrain_edge.gdshader`,
> `godot/scenes/world/atmosphere.gd` (Dunst, Himmel: `ground_horizon`, `ground_bottom`),
> `godot/assets/biomes/biome_preset.gd` und die `.tres` dazu,
> `godot/scenes/camera/rts_camera.gd` (`PERSPECTIVES`, `ground_reach`, `bounds`),
> `godot/scenes/world/fog_of_war.gd`, `godot/scenes/hud/minimap.gd`,
> `godot/scenes/tools/trailer_shots.gd` (`survey_perspective_edge`).
>
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hash unverändert.
> Das Umland ist nie Spielfläche: keine Einheit, kein Pfad, kein Befehl, keine Sicht dort.

## 1. Problem

Die Karte ist ein Dioramen-Block: an den vier Rändern ein senkrechter Erdschnitt
(`_build_edge`, Prompt 011), dahinter nichts – man sieht die untere Hälfte des Himmels
(`ground_horizon` / `ground_bottom`), eine graublaue Fläche.

- Klassische Perspektive (30°, 45–65°): Am Rand ist das ein Streifen am oberen Bildrand
  (der Boden reicht nur 1,4 Kameraentfernungen tief).
- Weitwinkel-Perspektive (Prompt 217: 52° aus 33°): Der Boden reicht 4 Kameraentfernungen
  tief. Steht der Blickpunkt nah am Rand und schaut nach außen, ist die **obere
  Bildhälfte leer** – siehe `just perspective-shots` → `survey_perspective_edge`
  (Karte `farms`, Blickpunkt 10 m vor dem Nordrand). Auf einer 1-km-Karte passiert das
  ständig, z. B. für den Spieler, dessen Basis am Rand liegt.

Prompt 217 hat das offen gelassen („Open: ground around the map“).

## 2. Ziel

Der Blick über den Kartenrand zeigt Land, das im Dunst ausläuft – kein Loch, keine
harte Kante am Horizont. Die Spielfläche bleibt klar erkennbar.

1. **Umland-Gelände:** Rund um die Karte ein Streifen Boden, der an den Kartenrand
   anschließt (Höhe am Rand = Höhe der Karte dort, keine Stufe, kein Spalt) und nach
   außen sanft in eine ruhige Fläche übergeht. Erst prüfen, was am einfachsten trägt,
   dann eines wählen und begründen:
   - a) ein Ring grober Gelände-Chunks aus fortgesetzten/gespiegelten Randhöhen,
     nach außen auf eine Grundhöhe geglättet;
   - b) eine große flache Schürze auf Höhe der Randpunkte mit dem Bodenshader in
     einfachster Form (nur Makrofarbe des Bioms, keine Wiese, keine Streu);
   - c) nur Himmel/Dunst anpassen (untere Himmelshälfte in Bodenfarbe + Dunst) – als
     Rückfall für Qualität Niedrig oder wenn a/b zu teuer sind.
   Breite: mindestens so weit, wie der Weitwinkel-Blick am fernsten Zoom reicht
   (`ground_reach()` × Entfernung; rund 550 m), bei Karten am Wasser passend
   (Küstenkarten: Meer läuft weiter, Land nicht).
2. **Bewuchs sparsam:** Das Umland darf Bäume/Wald als billige Fernform tragen (die
   vorhandene Fernstufe der Kartenobjekte, keine neuen Modelle), passend zum Biom und
   ungefähr zur Dichte am Kartenrand. Keine Häuser, keine Straßen, keine Streu, keine
   Wiesenbüschel. Deterministisch aus dem Kartennamen/-seed, damit Bilder gleich bleiben.
3. **Spielfläche lesbar:** Der Rand der Spielfläche bleibt sichtbar, aber dezent –
   z. B. Umland leicht entsättigt/abgedunkelt wie unerkundetes Gebiet oder eine feine
   Linie am Boden. Kein Erdschnitt mehr dort, wo Umland anschließt (der Schnitt bleibt
   am äußeren Rand des Umlands oder entfällt, wenn der Dunst ihn ohnehin verdeckt).
   Nebel des Krieges: Umland zählt als nie sichtbar, ohne dass die Maske außerhalb der
   Karte Müll liest.
4. **Dunst und Schatten:** Das Umland liegt im selben Tiefendunst wie die Karte und
   läuft zum Horizont in die Dunstfarbe aus. Schatten von Umland-Bäumen nur, wenn sie
   nichts messbar kosten; sonst keine.
5. **Nichts Spielrelevantes:** Kamera-`bounds`, Bodenpick, Befehle, Minimap (Karte und
   Sichttrapez), Zielvorschau und Kartenmarken bleiben auf der Spielfläche. Ein Klick
   ins Umland gibt keinen Befehl und kein Ziel außerhalb der Karte.
6. **Karteneditor:** Zeigt das Umland ebenfalls (oder bewusst nicht – entscheiden und
   begründen); Pinsel wirken nie außerhalb der Karte.

## 3. Prüfen und zeigen

- **Vorher/Nachher-Standbilder** am Kartenrand, klassisch und Weitwinkel:
  `survey_perspective_edge` plus derselbe Blick klassisch, außerdem eine Ecke der Karte
  (zwei Ränder im Bild), eine Küstenkarte (z. B. `dune_coast`) und eine Karte mit Höhen
  am Rand (z. B. `mountain_pass` oder `coastal_heights`). Jedes Bild nach dem Rendern
  mit Namen zeigen.
- **Kein Spalt, kein Flimmern:** Naht zwischen Karte und Umland aus der Nähe (15–30 m,
  flachste Neigung) und beim Zoomen (LOD-Wechsel) ansehen.
- **Selbsttest:** Bodenpick und Rechtsklick im Umland geben kein Ziel außerhalb der
  Karte; Minimap-Sichttrapez und Kamera-`bounds` unverändert; Umland vorhanden und am
  Rand höhengleich (Stichproben an allen vier Rändern).
- **Performance:** `just bench-view --perf-quick --perf-dof=off` klassisch und mit
  `--camera-perspective=wide`, zusätzlich je eine Ansicht am Kartenrand (dort ist das
  Umland im Bild), vorher/nachher in `docs/PERFORMANCE.md`. Budget: höchstens 1 ms pro
  Bild am Rand auf Qualität Hoch; auf Niedrig Variante c oder aus.
- `just check`, `just selftest`, `just editor-selftest` grün; Golden Hash unverändert.
- `docs/ARCHITECTURE.md` („Map edge“ unter „Perspectives“ und der Gelände-Abschnitt)
  nachziehen.

## 4. Nicht Teil dieses Prompts

- Spielbares Gelände außerhalb der Karte, größere Karten, Änderungen an Kartendaten
  oder Generatoren (`data/maps/`, `gen-*`).
- Neue Modelle oder Texturen; ferne Kulissen (Berge, Städte am Horizont).
- Änderungen in `crates/sim`.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/222-umland-um-die-karte.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf
„erledigt“ setzen.
