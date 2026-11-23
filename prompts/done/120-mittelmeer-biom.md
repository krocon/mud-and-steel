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

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Licht** `mediterranean.tres`: Sonne 62° (Wald 37°, Wüste 58°), Energie 2,5, volle Schatten, Belichtung 0,85 (wie Wüste, weil der Boden heller ist), tiefblauer Himmel mit klarem Horizont, Dunst 24 % ab 0,8 Kameraabständen (Wald 35 %, Wüste 42 %), Kontrast 1,08, Sättigung 1,06, warme Lichter. **[vorläufig]** | Hohe Sommersonne bei 40–44° N, klare Luft statt Wüstenstaub; Bilder in `build/biome-shots/`. |
| 2 | **Boden ohne neue Texturen:** Gras bleibt Wiese mit trockener Palette (`MeadowTufts.PALETTES`, auch für Halme und Streuschicht), Feldwege und Erde Terra rossa per Tönung, Felder über neue Preset-Farben `field_grain/leaf/soil/earth` (Stoppeln, staubige Reihen, rote Erde); hohes Gras als Macchia-Palette; keine Schneegrenze. **[vorläufig]** | Keine neuen Bilddateien, Lizenzfrage entfällt; die Feldfarben waren Konstanten im Shader. |
| 3 | **Tarnung `forest`.** | Fahrzeuge in Italien trugen keine Wüstenfarben; kein Gegenbeleg gefunden. |
| 4 | **Ton:** Zikaden = das schon vorhandene, im Repo synthetisierte Zikadenbett der Wüste (+3 dB), Wind des Waldes, Singvögel seltener, Meer über die Wasserschicht. Kein Download. **[vorläufig]** | Passende Quelle im Repo; Pegel ohne Abhören. |
| 5 | **Eigene Katalog-Ids** für Pflanzen (`env_tree_stone_pine`, `_cypress`, `_olive`, `env_bush_macchia`) statt Varianten vorhandener Bäume; Varianten-Namen eindeutig (`StonePineYoung` …), weil `crush_variants` nur Namen kennt. | Präfix `env_tree_` bringt Baumregeln, Wind, Fernmodell, Effekte mit; eigene Quellen und Importer-Wahl je Art. |
| 6 | **Werte Pflanzen:** Pinie 16 m / Krone 12 m, Zypresse 18 × 2 m, Olive 6 m / 7 m, Macchia 2,5 × 3 m; Stamm-Radien 200–400 mm; Feuer: Pinie/Zypresse wie Nadelbäume, Olive wie Laubbäume; niederwalzbar: junge Pinie, kurze Zypresse (40 t), junge Olive (18 t). **[vorläufig]** | Quellen in `neutral.toml` (Wikipedia), Werte unter den Maxima. |
| 7 | **Olivenlaub als neues Kartenmuster 10** im Laub-Shader (silbergrau, lockere Krone). | Laubfarbe ist sonst ein Biomwert für alle Bäume; ein Muster stört keine vorhandenen Modelle. |
| 8 | **Gebäude aus den vorhandenen Generatoren** (`house`, `church`) mit neuen Parametern `shutters`, `shop`, `outside_stair`, `campanile` und `[surface]` (Putzfarbe, Mönch-Nonne-Ziegel); neu nur `kit/med.py` (Feldsteinmauer, Terrassenmauer, Wachturm). Neun Haus-Ids, Hausgrößen geschätzt. **[vorläufig]** | Weniger neuer Code, Zerstörungsstufen und Stockwerkseinsturz kommen mit. |
| 9 | **Regeln Häuser:** Lebenspunkte 2 400–7 000, Besatzung 6–24, Exposition 5–15 %; Feldsteinmauer wie Trockenmauer (Infanterie klettert, ab 18 t durchschoben); **Terrassenmauer** blockiert Fahrzeuge, wird nicht zerstört, Gelände bleibt Karte. **[vorläufig]** | Spielregeln wie 059/078. |
| 10 | **Budgets:** Eckhaus 4 500, Rathaus 6 000 Dreiecke (Fensterläden an 60–90 Fenstern). **[vorläufig]** | Statt Läden wegzulassen. |
| 11 | **Gehöft-Familien** in `compounds.toml` (`[walls.mediterranean]`, `family`), Editor-Werkzeug „Village (mediterranean)“; Gehöfte 90°-gedreht wie 059. | Bestehendes Wüstendorf unverändert. |
| 12 | **Importer** tauscht Gebäudeklassen über `MED_BUILDINGS`, pflanzt Pinie/Olive/Zypresse/Macchia, Gartenmauern statt Hecken, Olivenhaine in Reihen auf jedem `landuse=orchard` (das Tag `trees` ist im Extrakt nicht erhalten), Weinberge als `field_green` – nur im Mittelmeer-Biom. **[vorläufig]** | Andere Karten bleiben Bit für Bit gleich (`gen-geo-check`). |
| 13 | **Testkarte** neuer Generator `cli gen-borgo` → `med_village` (600 m, zwei Starts). | Wie `desert_village`, alle neuen Objekte. |
| 14 | **Golden Hashes** (Demo, Wirtschaft) neu: nur die gehashten Regeltabellen wachsen; Demo endet mit identischen Einheiten. | Beabsichtigte Regeländerung. |

## Ergebnis

- **Stufe 1:** Biom `mediterranean` in Simulation (`Biome::Mediterranean`), CLI, Menü, Kartenauswahl, Debug-Menü, Editor, Selbsttests, Kartenskizzen; Preset, Wiesenpalette, Feldfarben, Macchia-Gras, Streuschicht, Wasser türkis, Zikaden; `just biome-shots` (gleiche Stellen in Wald/Wüste/Mittelmeer).
- **Stufe 2:** Pinie (3), Zypresse (2), Olive (3), Macchia (3) – 255–721 Dreiecke, mit Wind und Fernmodell; Regeln, Editor-Pinsel, Effekte, Vorschaubilder.
- **Stufe 3:** neun Gebäude je Intact/Damaged/Ruin (796–4 984 Dreiecke), Feldsteinmauer (6 Varianten), Terrassenmauer (3); Regeln; acht Gehöfte, Dorf-Pinsel; Importer; `harbor_city` neu (1 498 Gebäude, 11 650 Bäume, 313 Palmen, 3 534 Macchia, 1 349 Mauerstücke); Testkarte `med_village`.
- **Messung** `objects-bench harbor_city --blocks-bench`: hoch 73,9 / 78,4 / 83,6 FPS (vorher Wald 76,2 / 79,0 / 86,6), mittel 103,9 / 111,1 / 120 FPS.
- **map-reach harbor_city:** 56,0 → 56,9 % offen, größter Teil 81,7 → 82,0 %, alle Starts/Punkte verbunden. **KI-Turnier** hard/normal, 12 Spiele, 30 000 Ticks: vorher 33 %/8 %/7 Zeitüberschreitungen, nachher 33 %/58 %/1.
- **Bilder:** `build/biome-shots/{stage1,final}/`, `build/med-shots/{stage3,final}/`, `build/harbor-shots/{before,after}/`, `build/renders/med/`, `build/med120/` (Erreichbarkeit, Messungen, Turnier).

**Offen:** Modellgalerie zeigt das Biom nicht (Tasten 1–3); Feigenkaktus, Agave, Weinstöcke als Objekte; Terrassen aus dem Importer an Hängen; Glockenturm-Lücke der alten `env_church` (eine Turmwand reicht bis zur Halle, schon vorher, nicht neu gebaut); Pinienkrone wirkt aus der Nähe eher laubig; Ton nicht abgehört; harbor_city hat keine Weinberge in den Daten; Turnier-Unterschied bei 12 Spielen nicht belastbar.
