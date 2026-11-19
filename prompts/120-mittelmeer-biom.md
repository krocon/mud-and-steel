# Prompt 120 – Mittelmeer-Biom: Licht, Boden, Pinien, Oliven, Zypressen, helle Häuser, Trockenmauern

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/015-licht-atmosphaere-biome.md`, `prompts/done/013-vegetation-platzieren.md`,
> `prompts/done/059-wuestendorf.md` und `prompts/done/089-wueste.md` (jeweils Ziel,
> Entscheidungen, Ergebnis; 059 ist das Muster für eine neue Gebäudefamilie). In
> `docs/ARCHITECTURE.md` „Lighting, atmosphere and biomes (presentation only)“,
> „Map objects and decoration“, Zerstörung und Infanterie in Gebäuden; in
> `docs/MAPS.md` „Biome“, „Objects“ und „Maps from real geodata (prompt 081)“
> (Schritt 7 und 10: Gebäude und Grün im Importer); `docs/ASSETS.md`,
> `docs/SOURCES.md`, `docs/PERFORMANCE.md`. Dann `godot/assets/biomes/*.tres` und
> `biome_preset.gd`, `godot/assets/shaders/env_look.gd`,
> `godot/scenes/world/atmosphere.gd`, `map_objects.gd`, `tall_grass.gd`,
> `godot/scenes/terrain/terrain.gd`, `godot/scenes/audio/ambience.gd`, die
> Biom-Prüfung in `crates/sim/src/map.rs`, `data/objects.toml`,
> `data/compounds.toml`, `tools/blender/kit/desert.py`, `desert_props.py` und
> `env.py` sowie die Stellen, die die Biom-Namen auflisten:
> `rg -l '"desert"' crates godot tools data web`.
>
> **Dieser Prompt läuft in drei Stufen** (Abschnitt 7). Nach jeder Stufe den
> Zwischenstand zeigen (Bilder aus dem Spiel) und ohne Halt weiterarbeiten.

## 1. Ziel

Ein viertes Biom `mediterranean` für Karten in Italien, Südfrankreich, Griechenland
und an der Küste Nordafrikas: hartes Sommerlicht, trockener Boden, dunkle Pinien und
Zypressen, silbrige Olivenhaine, Macchia, helle Häuser mit Ziegeldächern, Mauern
aus Feldsteinen und Terrassen an den Hängen.

Es wird für die Landungskarte im Golf von Campo (Prompt 121) gebraucht und soll
auch für `harbor_city` gelten, die heute als `forest` zu nordisch aussieht.

Wie alle Biome ist das Licht reine Darstellung: Die Simulation prüft nur den Namen,
keine Regel liest ihn; Karte und Spielstand behalten ihren Hash. Neue **Objekte**
(Bäume, Häuser, Mauern) sind dagegen Regeldaten (Blockieren, Sicht, Deckung,
Zerstörung) wie in Prompt 059.

## 2. Licht, Himmel, Boden (Stufe 1)

- `godot/assets/biomes/mediterranean.tres` (`BiomePreset`): hoher Sonnenstand,
  warmes, helles Licht, harte Schatten, tiefblauer Himmel, wenig, aber
  heller Dunst, leicht warme Farbabstimmung. Werte durch Vergleich mit `forest` und
  `desert` festlegen und begründen.
- **Boden:** die vorhandenen Geländearten bekommen Farben und Texturen für dieses
  Biom (`terrain.gd` `apply_biome()`): Gras trocken und gelbgrün bis strohfarben,
  Sand hell, Feldwege rötlich-ocker (Terra rossa), Straßen heller Staub bzw.
  Pflaster, Felder als Stoppeln, Weinberge und Gemüse; hohes Gras als Macchia-Look
  (`tall_grass.gd`). Texturen nur CC0 oder selbst gemacht (`docs/ASSETS.md`).
- **Wasser:** klarer, türkis an Sandstränden, tiefblau im Tiefen (Wasserfarben des
  Bioms).
- **Tarnung:** `camo_biome` wie `forest` (die Fahrzeuge in Italien waren nicht in
  Wüstenfarben), außer die Session findet einen belegten Grund für etwas anderes.
- **Umgebungsgeräusch** (`ambience.gd`): Zikaden am Tag, Wind, Meer an der Küste,
  aus der freien Tonbibliothek (`docs/AUDIO_LIBRARY.md`, CC0); ohne passende Quelle
  das Geräusch von `forest` und ein offener Punkt.
- Biom-Name an allen Stellen ergänzen (Simulation: erlaubte Werte; Godot: Listen in
  `env_look.gd`, Menü, Debug-Menü, Editor, Selbsttests; CLI: `--biome`;
  Kartenskizzen). `docs/MAPS.md` und `docs/ARCHITECTURE.md` nachführen.

## 3. Pflanzen (Stufe 2)

Neue Modelle aus dem Baukasten (headless Blender, wie die vorhandenen Bäume), nach
den Budgets in `docs/ASSETS.md`, mit Wind wie die anderen Bäume:

- **Pinie** (Schirmkiefer): hoher, kahler Stamm, flache, breite Krone; drei
  Varianten (jung, alt, schief).
- **Zypresse:** schlank und säulenförmig; zwei Varianten (hoch, niedrig). Typisch
  in Reihen an Wegen und um Friedhöfe und Höfe.
- **Olivenbaum:** knorriger, kurzer Stamm, silbrig-graue lockere Krone; drei
  Varianten. In Hainen in Reihen gepflanzt.
- **Macchia-Busch** (Steineiche, Erdbeerbaum, Ginster): zwei bis drei
  Buschvarianten, dichter als `env_bush`.
- Optional, wenn Zeit bleibt: Feigenkaktus, Agave, Weinstöcke als Reihen.

Regeln wie bei den vorhandenen Bäumen (`env_tree_*`): blockieren je nach Stamm,
Sicht und Deckung über die Waldregeln, Zerstörung und Niederwalzen wie bei anderen
Bäumen (Prompt 106, falls erledigt). Die Pinie entspricht im Spiel etwa
`env_tree_conifer`, Olive und Macchia sind niedriger: Werte nach Quelle (Wuchshöhe,
Kronendurchmesser) und als Daten in `data/objects.toml`.

## 4. Gebäude und Mauern (Stufe 3)

Eine neue Gebäudefamilie nach dem Muster des Wüstendorfs (059), je in den Zuständen
Intact, Damaged und Ruin, mit Plätzen für Infanterie, Deckung und Zerstörung als
Daten:

- **Haus, klein:** ein bis zwei Geschosse, verputzt in hellen Farben (weiß, ocker,
  hellrosa, sandfarben), flach geneigtes Dach mit Mönch-und-Nonne-Ziegeln,
  Fensterläden.
- **Reihenhaus im Dorf:** zwei bis vier Geschosse, schmal, mit Laden im Erdgeschoss;
  Varianten für Ecke und Mitte, aneinanderreihbar.
- **Bauernhaus** (Casale) mit Außentreppe und Nebengebäude, dazu Scheune.
- **Kirche** mit Campanile (Glockenturm), **Turm** (Wachturm an der Küste) und ein
  größerer **öffentlicher Bau** (Rathaus, Kaserne).
- **Trockenmauer** aus Feldsteinen (Segment, niedrig, verfallen, Ecke, Ende, Tor) und
  **Terrassenstufe** (Stützmauer am Hang), die die Hangregeln nicht verletzt:
  Terrassen sind Objekte, das Gelände bleibt die Karte.
- Gehöfte und Dorfgruppen in `data/compounds.toml` und als Editor-Werkzeug
  („Village (mediterranean)“) wie beim Wüstendorf.

Die Häuser sind Vorbild Italien um 1940, ohne echte Gebäude nachzubauen. Fotos nur als
Referenz (`docs/SOURCES.md`).

## 5. Importer und Karten

- `tools/geo_import.py` wählt Bäume und Gebäude nach dem Biom der Karte: im
  Mittelmeer-Biom Pinien, Zypressen, Oliven und Macchia statt Eiche und Fichte; die
  neue Gebäudefamilie statt der mitteleuropäischen. Olivenhaine aus OSM
  (`landuse=orchard` mit `trees=olive_trees`, sonst Obstgärten in diesem Biom als
  Olivenhain) in Reihen; Weinberge (`landuse=vineyard`) als Feldart mit Reihen, falls
  das ohne neue Geländeart geht, sonst als offener Punkt.
- **`harbor_city` umstellen** auf `mediterranean`: neu erzeugen, `just gen-geo-check`,
  Stufenbilder vorher/nachher, `just map-reach` und KI-Turnier vorher/nachher
  (neue Objekte ändern Blockieren und Sicht, also auch Partien auf dieser Karte).
  Den Golden Hash betrifft das nur, wenn ein Szenario `harbor_city` nutzt; sonst
  bleibt er gleich.
- Eine kleine Testkarte mit allen neuen Objekten (wie `desert_village`) oder eine
  vorhandene Generator-Karte mit `--biome mediterranean`.

## 6. Prüfen

- `just thumbnails --only=…` für jedes neue Modell, `just units-web-sync`,
  `just catalog-doc`, falls der Katalog betroffen ist.
- Bilder aus dem Spiel: dieselbe Stelle in `forest`, `desert` und `mediterranean`;
  ein Olivenhain mit Terrassen; ein Dorf mit Kirche; `harbor_city` vorher/nachher.
- Messung (`just bench-view` bzw. `objects-bench`) auf `harbor_city` mit dem neuen
  Biom: jede Ansicht in hoch/mittel weiter über 60 FPS, Ergebnisse in
  `docs/PERFORMANCE.md`.
- `just check`, `just selftest`, `just editor-selftest`.

## 7. Stufen

1. **Licht, Boden, Wasser, Ton, Name überall** (Abschnitt 2); Bild aus dem Spiel.
2. **Pflanzen** (Abschnitt 3); Bilder, Messung.
3. **Gebäude, Mauern, Gehöfte, Importer, `harbor_city`** (Abschnitt 4 und 5); Bilder,
   Messung, KI-Turnier, Doku.

## 8. Nicht Teil dieses Prompts

- Ein eigenes Biom für den Sommer an der Pazifikküste (gelbes Gras) oder für Nordsee
  und Dünen; die Werte hier können aber als Ausgangspunkt dienen.
- Neue Geländearten (Weinberg, Terrasse als Bodenart).
- Hafenanlagen, Kräne, Fischerboote als Kulisse.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
