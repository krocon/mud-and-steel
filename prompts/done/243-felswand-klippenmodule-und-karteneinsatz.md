# Prompt 243 – Realistische Felswände (Klippenmodule, Shader-Schliff & Einsatz auf Spielkarte)

> In einer frischen Session im Repo-Ordner ausführen. Benötigt Blender
> (`docs/VERSIONS.md`). Lies zuerst `CLAUDE.md`, `docs/ASSETS.md` (Umwelt-Objekte,
> Felsen aus Prompt 074, Budgets, `env_embed_m`, Material-Slots), `docs/SOURCES.md`,
> `docs/SCALE.md`, `docs/MAPS.md` (Objekte, `gen_sketch`, `[[mesa]]`, Karteneditor),
> `docs/ARCHITECTURE.md` („Map objects and decoration“) sowie
> `prompts/done/074-felsen.md`.
>
> Im Code:
> `tools/blender/kit/rocks.py`,
> `assets-src/blueprints/neutral/env_rock_massif.toml`,
> `assets-src/blueprints/neutral/env_rock_outcrop.toml`,
> `godot/assets/shaders/env.gdshader` (`rock_base()`, `rock_albedo()`, Slot 16 `Rock`),
> `godot/assets/shaders/relief.gdshaderinc`,
> `data/catalog/neutral.toml`,
> `data/objects.toml`,
> `data/vision.toml`,
> `crates/bridge/src/objects.rs`,
> `godot/scenes/world/map_objects.gd`,
> `godot/scenes/editor/editor_data.gd`,
> `crates/mapgen/src/gen_sketch/` (`schema.rs`, `mesa.rs`, `plan.rs`, `mod.rs`),
> `data/map_sketches/pit_hill.toml`.
>
> **Stil-Referenz (nur lokal ansehen, nichts daraus übernehmen):**
> `.temp/vorlage/felswand-referenz.jpg` (nicht im Repo, `.temp/` ist ignoriert;
> liegt die Datei nicht vor, reicht die ausführliche Beschreibung in Abschnitt 1).
> **Keine Bilder, Texturen oder Assets aus der Vorlage ins Repo.**
>
> **Hinweis zum Golden Hash:** Das neue Umwelt-Objekt `env_rock_cliff` in
> `data/catalog/neutral.toml`, `data/objects.toml` und `data/vision.toml` ist ein
> gewollter Regelzuwachs und ändert die Golden Hashes (`crates/sim/tests/determinism.rs`,
> `crates/sim/tests/economy.rs`). Gegenprobe ohne neue Einträge durchführen, Ursache
> dokumentieren und Golden Hashes sauber aktualisieren.

## 1. Problem und Zielbild

Unser Spiel hat seit Prompt 074 bereits lose Steine (`env_rock`), flache Platten
(`env_rock_slab`), Blockgruppen (`env_rock_outcrop`) und freistehende Felsrücken
(`env_rock_massif`) sowie die Fels-Schicht im Terrain-Shader an Steilhängen.
An Hangkanten und Geländestufen (z. B. `[[mesa]]`-Ränder, Talflanken, Bahneinschnitte)
wirkt das Gelände jedoch noch zu glatt (2-m-Höhenraster ohne echte Überhänge, tiefe
Schattenspalten und gestaffelte Felsrippen) oder die symmetrischen `env_rock_massif`-Blöcke
wirken eher wie einzeln abgestellte Quaderreihen als wie eine gewachsene, dramatische
**Felswand**.

**Was die Felswand in der Vorlage (`.temp/vorlage/felswand-referenz.jpg`) so gut aussehen lässt:**

1. **Asymmetrischer Geländesprung statt freistehender Mauer:**
   - Die Wand klebt als durchgehende, ~60–120 m lange und ~6–11 m hohe **Steilkante**
     direkt an einem bewaldeten Hang/Plateau (oben dichter, dunkler Nadelwald mit
     einzelnen Herbst-/Laubakzenten bis unmittelbar an die Abbruchkante; unten
     grasiger Hangfuß / Bahndamm / Talboden).
   - Die **Talseite (Front)** ist schroff, steil (`65°–85°`), stark zerklüftet und
     mit vorspringenden Felsrippen, kleinen Überhängen und tiefen, fast schwarzen
     Spalten durchzogen.
   - Die **Hangseite (Rückseite)** schmiegt sich als schräg abfallende Felsschulter
     tief in die obere Geländestufe, sodass nirgends Spalten oder schwebende Kanten
     entstehen.
2. **Große angelehnte Fußblöcke & Übergang ins Gelände:**
   - Direkt vor dem Wandfuß lehnen einzelne **massive, gekippte Felsplatten und
     Stützblöcke** (2–5 m breit, `25°–45°` schräg angelehnt), umgeben von kantigem
     Geröll, Erde, Grasbüscheln und einzelnen niedrigen Büschen.
   - An beiden Enden läuft die Wand nicht abrupt als Kasten aus, sondern treppt sich
     über niedrigere Schulterblöcke und Hanggeröll weich in die Wiese ab.
3. **Starker Licht-/Schatten-Kontrast & natürliche Bruchflächen:**
   - Große, klar geschnittene **ebene Bruchfacetten** (Granit-/Kalkstein-Klüftung)
     mit gebrochenen, leicht aufgehellten Kanten.
   - **Helle, warm sandig-graue Sonnenseiten** stehen in scharfem Kontrast zu
     **kühlen schiefergrauen Schattenflanken** und **tiefdunklen, nahezu schwarzen
     Klüften/Unterschneidungen** (kräftig gebackene Spalten-AO + harte Schlagschatten).
4. **Moos- und Graspolster auf waagerechten Absätzen:**
   - Auf flachen Felsbändern, oberen Blockköpfen und Nischen liegen deutliche,
     fleckige **oliv- bis moosgrüne Gras-/Moospolster** (im Wald-Biom; in der Wüste
     Sand-/Staubablagerungen, im Winter Schnee), die die Felswand organisch mit dem
     bewaldeten Oberhang und dem grünen Hangfuß verbinden.

## 2. Ziel und Abnahme

Fertig ist dieser Prompt, wenn:

1. ein neues, modulares Klippen-Set **`env_rock_cliff`** („Felswand“ / „Rock cliff wall“)
   im Blender-Baukasten gebaut, validiert, mit Thumbnails versehen und sauber in
   Katalog, Regeln, Sicht, Bridge/Dekoration und Karteneditor integriert ist,
2. der **`Rock`-Shader** in `godot/assets/shaders/env.gdshader` die knackige
   Facetten-/Kanten-Lesbarkeit, dunkle Spalten und Moospolster auf waagerechten
   Absätzen (biomabhängig, flimmerfrei, 100 % prozedural) liefert,
3. der Skizzen-Generator (`crates/mapgen/src/gen_sketch/`) Felswände entlang von
   Linien/Hangkanten platzieren kann (**opt-in**, alle nicht geänderten Skizzenkarten
   bleiben Bit-für-Bit unverändert),
4. diese Felswand in **mindestens einer spielbaren Karte** des Spiels sichtbar und
   taktisch sinnvoll eingesetzt wird (konkret in **`pit_hill`** an den markanten
   bewaldeten Hangkanten entlang von Bahnlinie und Tal; optional zusätzlich an
   weiteren passenden Stellen, ohne Tests/Messreihen zu brechen),
5. Vergleichsbilder aus dem Spiel neben `.temp/vorlage/felswand-referenz.jpg` die
   Wirkung aus Nah-, RTS- und Fernperspektive belegen und alle Prüfungen
   (`just check`, `just validate-assets`, `just glb-check`, `just l10n-check`,
   `just map-check pit_hill`, `just map-reach pit_hill`, `just stuck-audit`,
   `just selftest`, `just editor-selftest`) grün sind.

## 3. Teil A – Modulares Klippen-Set `env_rock_cliff` (Blender-Baukasten)

Erweitere `tools/blender/kit/rocks.py` um einen neuen Generator `@env_part("rock_cliff")`
und lege den Bauplan `assets-src/blueprints/neutral/env_rock_cliff.toml` an. Alle
Formen bleiben rein prozedural und deterministisch aus dem Varianten-`seed`.

### Achsen- und Einbettungskonvention (wichtig für Generator & Editor)
- `+Y` (Blender, entspricht Heading): läuft **entlang der Felswand**.
- `+X`: ist die **Talseite / steile Schauseite** (schroffe Klippenfront mit
  angelehnten Fußblöcken).
- `-X`: ist die **Hangseite / Rückenschulter** (läuft etwas höher/breiter nach
  hinten aus und taucht in die obere Geländestufe ein).
- **Tiefer versenkter Fuß (`env_embed_m = 3.0` m, mind. `2.5` m):** Da Felswände
  direkt auf steilen Hangstufen sitzen, muss der untere Mantel tief genug unter
  `z = 0` reichen, damit weder an der steilen Front noch an schrägen Hängen
  irgendwo Lücken unter dem Fels sichtbar werden.

### Dreischichtiger Aufbau je Klippenmodul
1. **Zerklüftete Hauptwand (4–7 gestaffelte Felsrippen/Pfeiler):**
   - Unterschiedlich weit nach `+X` vorspringende und zurückspringende, leicht
     gekippte (`5°–20°`) kantige Großblöcke mit klaren Bruchflächen (`cuts = 4–6`,
     bevelled Kanten mit `wear = True` in `COLOR.g`), die **echte, 0,3–0,9 m tiefe
     vertikale und schräge Spalten/Klüfte** zwischen sich lassen.
   - Keine gleichmäßige Quaderreihe: wechselnde Blockbreiten, gestufte Höhen und
     einzelne leicht überhängende oder schräg gespaltene Klippenköpfe.
2. **Stufige Oberkante & Rückenschulter:**
   - Abgetreppte Felsabsätze oben (für Moospolster und sauberen Anschluss an den
     oberen Wald) plus kleinere aufgesetzte Gratblöcke; nach `-X` eine breitere,
     sanfter abfallende Rückseite zum nahtlosen Eintauchen in den Hang.
3. **Vorgelagerte Stützblöcke & Fußgeröll (`+X`-Front):**
   - 2–5 kräftige, schräg an die Wand gelehnte oder vorgelagerte **Fußblöcke**
     (2–5 m breit, `20°–45°` gekippt bzw. versetzt, wie die auffälligen großen
     Schrägblöcke links/mittig in der Vorlage) sowie kantige Geröllsteine (`kind = 1`)
     entlang des Fußes.
4. **Spalten-AO im Modell:**
   - Beim AO-Bake (`bake.py`) darauf achten, dass die tiefen Fugen zwischen den
     Pfeilern und hinter den angelehnten Fußblöcken deutlich abdunkeln (`COLOR.r`),
     damit die Zerklüftung auch im Schatten und aus größerer RTS-Distanz klar
     lesbar bleibt.

### Sechs kombinierbare Varianten (`env_rock_cliff.toml`)
Die Enden der Module fallen leicht abgestuft ab bzw. verzahnen sich organisch,
sodass sich beliebig lange, leicht gebogene Felswände ohne harte Kanten oder
sichtbare Wiederholungen zusammenstecken lassen:

| Variante | Ungefähre Maße (`L × B × H`, m) | Rolle |
|---|---|---|
| `Straight` | `20 × 10 × 8,0` | Standard-Felswand mit starker Mittelrippe, tiefen Spalten und angelehnten Fußblöcken |
| `Long` | `26 × 10 × 7,5` | Lange, schräg geschichtete Wandstrecke mit gestuften Absätzen |
| `High` | `18 × 11 × 10,5` | Mächtiger, hoher Felsvorsprung / Klippenkopf als Blickfang |
| `Buttress` | `16 × 11 × 7,0` | Wandabschnitt mit besonders großen, schräg vorgelagerten Stützblöcken am Fuß |
| `Corner` | `16 × 13 × 8,0` | Konvexe Klippenecke / Felsnase für Vorsprünge und Hangkanten-Bögen |
| `End` | `14 × 9 × 6,0` | Abgetrepptes Wandende, das organisch in Fußblöcke und Hang ausläuft |

### Budgets, LOD & Validierung
- Jede Variante erhält ein **`<Variant>_Far`**-Mesh (gleiche Großform und
  `env_embed_m`, ohne Bevels und Kleingeröll, `≤ 450 Tris`).
- Detail-Budget für `env_rock_cliff`: **`≤ 4 000 Tris`** je Variante (in
  `docs/ASSETS.md`, `tools/blender/validate_env.py`, `tools/glb_check.py` und
  `godot/scenes/tools/building_preview.gd` konsistent eintragen; möglichst darunter
  bleiben).

## 4. Teil B – Oberflächen- und Shader-Schliff (`env.gdshader`, Slot `Rock`)

Verbessere `rock_albedo()` (Slot 16 `Rock`) in `godot/assets/shaders/env.gdshader`
rein prozedural, in Weltkoordinaten und **streng flimmerfrei** (`relief.gdshaderinc`,
`aa_line`, `roof_fine`):

1. **Knackiger Sonnen-/Schatten-Kontrast & Kanten-Highlights:**
   - Warmes, helles Kalk-/Granit-Graubeige auf nach oben/zur Sonne geneigten
     Bruchflächen gegenüber kühleren, dunkleren Schiefer-Flanken.
   - Die im Mesh gebackene Kantenabnutzung (`vc.g`) subtil als hellere, abgewitterte
     Bruchkante einmischen – dadurch „knallen“ die kantigen Felsrippen wie in der
     Vorlage schon aus 60–120 m RTS-Entfernung!
2. **Moos- und Graspolster auf waagerechten Absätzen:**
   - Auf deutlich nach oben zeigenden Flächen (`smoothstep(0.66, 0.90, up)`) im
     Wald-/gemäßigten/Herbst-Biom (`1.0 - env_dry`) fleckige, satt-natürliche
     **Moos- und Graspolster** (`fbm3`-Inseln mit klarerem, leicht rauem
     Übergang, nicht als gleichmäßiger Grünschleier) zeichnen – abgestimmt auf
     die Wiesen-/Waldfarbe des Spiels.
   - In der Wüste (`env_dust`) stattdessen heller, trockener Staub/Ocker auf den
     Absätzen; im Winter (`env_snow`) geschlossene Schneehauben auf den Absätzen
     über die bestehende Schneelogik.
3. **Schichtung, Diagonalrisse & Relief-Normale:**
   - Neben horizontalen Schichtbändern eine zweite, leicht geneigte **Klüftungs-/Rissrichtung**
     und schärferes Mikrorelief (`relief` → `bump_h`) auf den steilen Flanken,
     das mit der Pixelgröße sauber ausblendet (`roof_fine`).
   - Dunkler, leicht erdig-feuchter Wandfuß (`smoothstep(0.0, 1.3, h)`) für
     nahtlosen Übergang in Gras, Erde und Geröll.
4. **Rückwärtsprüfung:** Bestehende Felsen (`env_rock`, `env_rock_slab`,
   `env_rock_outcrop`, `env_rock_massif`) nutzen denselben Slot und profitieren
   mit – im Spiel prüfen, dass sie auf `rock_hills` (Wüste), Wald- und
   Winterkarten farblich weiterhin exakt zur Fels-Schicht des Terrain-Shaders passen.

## 5. Teil C – Katalog, Regeln, Einbettung & Karteneditor

1. **Katalog (`data/catalog/neutral.toml`) & `docs/SOURCES.md`:**
   - Eintrag `env_rock_cliff` (`name_de = "Felswand"`, `name_en = "Rock cliff wall"`,
     `category = "env_rock"`, `tier = 1`, Katalogbox `20000 × 10000 × 8000` mm,
     Quellenvermerk analog `env_rock_massif` mit `uncertain` für typische
     natürliche Felsstufen/Steilwände).
2. **Spielregeln (`data/objects.toml`, `data/vision.toml`):**
   - `env_rock_cliff` mit `blocks = "all"` und exakten Varianten-Maßen
     (`length_mm`, `width_mm`, `height_mm` + 0,4 m Sicherheitsrand wie bei
     `env_rock_outcrop`/`env_rock_massif`, sodass Panzer sauber am Fels halten,
     nicht hineinschneiden und vor keiner unsichtbaren Wand stehen).
   - `"env_rock_cliff"` in `data/vision.toml` unter `solid` aufnehmen (verdeckt
     Sichtlinie bis zur Variantenhöhe).
   - Keine Zerstörungsklasse (wie alle Felsen).
   - Simulationstests in `crates/sim/tests/rocks.rs` (Umfahrung einer
     Felswand-Strecke durch Panzer & Infanterie) und `crates/sim/tests/vision.rs`
     (Sichtsperre hinter `env_rock_cliff`).
3. **Einbettung & Präsentation (`crates/bridge/src/objects.rs`, `godot/scenes/world/map_objects.gd`):**
   - `"env_rock_cliff"` in `ROCK_KINDS` aufnehmen → automatischer 3-m-Saum aus
     Geröllsteinen, hohen Grasbüscheln und Büschen am Fuß.
   - Höhensetzung am Hang prüfen: `env_rock_cliff` steht auf steilen Hangkanten;
     sicherstellen, dass die vertikalen Referenzen (Mittelpunkt/Fußhöhe + `env_embed_m`)
     keine schwebende Talseite und kein zu tief versunkenes Wandprofil erzeugen
     (falls sinnvoll, in `crates/bridge/src/objects.rs` für `env_rock_cliff` eine
     saubere Hang-Ankerung wie z. B. Mittelhöhe minus kleinem Versatz prüfen und
     am echten Steilhang testen).
   - In `godot/scenes/world/map_objects.gd`: `{"prefix": "env_rock_cliff", "range": 0.0, "shadow": true}`
     (Landmarke ohne Ausblenden, mit hartem Schlagschatten).
4. **Karteneditor (`godot/scenes/editor/editor_data.gd`, Lokalisierung `godot/locale/editor.csv`):**
   - `env_rock_cliff` in die Objektpalette aufnehmen und eine Pinsel-Vorlage
     **„Felswand“ (`"Cliff wall"`)** ergänzen (setzt `env_rock_cliff`-Stücke mit
     Fußgeröll, Büschen und Nadelbäumen).

## 6. Teil D – Felswand-Generator (`gen_sketch`) & Einsatz auf mindestens einer Spielkarte

### 1. Skizzen-Baustein `[[cliff]]` (und/oder `[[mesa]]`-Option) in `crates/mapgen/src/gen_sketch/`
- **Rückwärtskompatibilität Pflicht:** Alle bestehenden Skizzenkarten, die das neue
  Feld nicht nutzen, bleiben **Bit-für-Bit unverändert** (per Gegenprobe prüfen!).
- Neuer optionaler Skizzen-Block **`[[cliff]]`** in `schema.rs` / `mesa.rs` / `mod.rs`:
  - `points_m`: Polylinie entlang der Klippenkante (kann gerade oder leicht
    geschwungen verlaufen).
  - `facing_m` (Richtungspunkt oder Normalenrichtung zur Talseite, damit `+X`
    jedes Klippenmoduls garantiert zur Talseite zeigt; alternativ definierte
    Laufrichtung, klar in `docs/MAPS.md` dokumentiert).
  - `height_m` (optional, Default `0.0` wenn die Wand auf einer bestehenden
    `[[mesa]]`- oder `[[hill]]`-Kante sitzt; bei `> 0` formt der Generator eine
    saubere Geländestufe / Böschung hinter der Wand mit schmalem Fuß-Saum davor).
  - `boulders` (optional, Default `true`): streut ergänzend einzelne
    `env_rock_outcrop`-Blöcke, `env_rock`-Geröll und niedrige `env_bush`-Büsche
    entlang der Talseite am Wandfuß.
  - `forest_top` (optional, Default `true` im Wald-Biom): setzt entlang der oberen
    Klippenkante dichte Nadelbäume (`env_tree_conifer`) mit einzelnen
    Laub-/Herbstakzenten und Büschen bis dicht an die Felskrone – genau wie der
    bewaldete Oberhang in der Vorlage.
  - **Nahtlose Stückelung:** Verlegt überlappende `env_rock_cliff`-Module entlang
    der Polylinie (`End` am Anfang/Ende mit nach außen gerichteter Abtreppung,
    dazwischen deterministisch wechselnd `Straight`, `Long`, `High`, `Buttress`
    und `Corner` an stärkeren Knicken), hält Straßen, Feldwege, Bahnlinien,
    Brücken, Rampen, Start- und Nachschubpunkte automatisch frei.
  - **Symmetrie (`mirror`):** Respektiert `mirror = "point"` / `"north"` / `"west"`
    korrekt, inklusive richtiger Ausrichtung der Schauseite auf der gespiegelten
    Kartenhälfte!

### 2. Einsatz in mindestens einer spielbaren Karte (Pflicht!)
- **Primärkarte: `pit_hill` (`data/map_sketches/pit_hill.toml` → `just gen-sketch pit_hill`):**
  - `pit_hill` spielt im Wald-Biom (`biome = "forest"`), hat gestufte Höhen mit
    Felsbändern, bewaldete Randgebirge **und** Bahnlinien entlang des Bergfußes –
    also genau das Landschaftsbild der Vorlage!
  - Bestücke die markanten Felsbänder bzw. die bewaldeten Hangkanten entlang der
    Bahnstrecke und der zentralen Höhenstufen (symmetrisch für beide Seiten) mit
    `[[cliff]]`-Felswänden, gekrönt von dichtem Nadelwald oben und einem
    natürlichen Geröll-/Gras-/Buschsaum unten am Bahndamm/Talboden.
  - Achte streng darauf, dass **alle Rampen, Wege, Bahnlinien, Startpositionen,
    Nachschubpunkte und Sichtlinien** sauber frei bleiben (`just map-check pit_hill`,
    `just map-reach pit_hill`, `just stuck-audit`).
- *(Optional, nur falls schnell und sauber ohne Bruch bestehender `map_sight`-Tests:
  eine weitere Karte wie `river_valley` oder `valley_fields` an einem einzelnen
  markanten Felshang ergänzen. Priorität hat jedoch eine wirklich herausragende,
  polierte Umsetzung auf `pit_hill`!)*

## 7. Prüfen und zeigen

1. **Modell- und Biom-Ansicht (`build/cliff_wall/`):**
   - `just env-lineup build/cliff_wall env_rock_cliff` (alle 6 Varianten neben der
     1,80-m-Figur und dem Panzer IV auf Maßstab und Silhouette prüfen).
   - Ansicht einer zusammengesetzten Felswand in allen drei Biomen (`forest`,
     `desert`, `winter`) und Gegenprobe auf `rock_hills`, ob bestehende Felsen
     weiterhin stimmig wirken.
2. **Vorher/Nachher- & Referenz-Vergleich auf `pit_hill` (`build/cliff_wall/`):**
   - Mindestens drei Ingame-Aufnahmen direkt an der neuen Felswand auf `pit_hill`
     (insbesondere der Abschnitt oberhalb der Bahnlinie):
     - Nahansicht (~35–45 m, Fokus auf Bruchfacetten, Kanten, Spalten-AO,
       Moospolster, angelehnte Fußblöcke und spaltfreie Hang-Einbettung),
     - typische RTS-Spielansicht (~70–95 m, Pitch ~33°–45°, vergleichbar mit
       `.temp/vorlage/felswand-referenz.jpg`: Bahn/Tal vorne, Felswand mittig,
       dichter Nadelwald oben),
     - Fernansicht (~200 m, Prüfung des `_Far`-Übergangs und der Fernwirkung ohne
       Flimmern oder Kachel-Wiederholung).
   - Bilder direkt neben `.temp/vorlage/felswand-referenz.jpg` auswerten und
     Schwachstellen (z. B. zu flache Spalten, zu geringe Moos-Lesbarkeit,
     Wiederholungsmuster, Lücken am Hangfuß) in mindestens einer Nachbesserungsrunde
     beheben.
3. **Leistung & Technik:**
   - `just objects-bench` (vorher/nachher notieren) sowie Blick über die fertige
     Felswand auf `pit_hill` (Dreiecke, Draw-Calls, Bildzeit in `docs/PERFORMANCE.md`
     festhalten).
   - `just validate-assets`, `just glb-check`, `just l10n-check`, `just catalog-doc`.
   - `just thumbnails --only=env_rock_cliff` und `just units-web-sync`.
   - Gegenprobe: alle anderen Skizzenkarten außer `pit_hill` (und ggf. bewusst
     geänderten Karten) bleiben Bit-für-Bit gleich.
   - `just map-check pit_hill`, `just map-reach pit_hill`, `just stuck-audit`.
   - `just check`, `just selftest`, `just editor-selftest` alle grün; Golden Hashes
     nach Gegenprobe sauber aktualisiert.

## 8. Nicht Teil dieses Prompts

- Besteigbare oder sprengbare Felswände / Steinschlag-Physik.
- Neuer Panzerzug oder Eisenbahn-Fahrzeuge (die Bahnlinie auf `pit_hill` nutzt
  die vorhandenen Gleise; der Zug in der Vorlage dient nur als Maßstabs- und
  Kompositions-Orientierung).
- Umbau von Karten, an denen KI-Turniere, Benchmarks oder feste Sicht-Tests hängen
  (sofern nicht ausdrücklich geprüft).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/243-felswand-klippenmodule-und-karteneinsatz.md prompts/done/`
verschieben, Ergebnisabschnitt (Varianten, Maße, Dreieckszahlen, geänderte Karten,
Golden-Hash-Begründung, Messwerte, offene Punkte) ergänzen, `docs/ASSETS.md`,
`docs/MAPS.md`, `docs/SOURCES.md` und `docs/ARCHITECTURE.md` nachziehen und den
Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (Nachtlauf 27.11.2026)

- **Teil A (`env_rock_cliff`):** Neuer Generator `@env_part("rock_cliff")` in `tools/blender/kit/rocks.py` und Bauplan `assets-src/blueprints/neutral/env_rock_cliff.toml` mit 6 kombinierbaren Varianten (`Straight` 20×10×8 m, `Long` 26×10×7,5 m, `High` 18×11×10,5 m, `Buttress` 16×11×7 m, `Corner` 16×13×8 m, `End` 14×9×6 m), Achsenkonvention `+Y` entlang der Wand, `+X` schroffe Talseite mit gestaffelten Felsrippen, Spalten-AO, Überhängen, angelehnten Fußblöcken und Geröll, `-X` abgetreppte Hangschulter, `env_embed_m = 3.0 m`, sowie `<Variant>_Far`-LODs. Validierung: `1710/4000 tris` max. (`validate_env.py`, `glb_check.py` 465 GLBs ok). Budgets (4000 Detail / 450 Far) in `kit/env.py`, `tools/glb_check.py`, `building_preview.gd` und `docs/ASSETS.md` nachgezogen.
- **Teil B (`env.gdshader` Slot `Rock`):** `rock_albedo()` prozedural verfeinert (flimmerfrei in Weltkoordinaten): stärkerer warm/kühl-Facettenkontrast, aufgehellte Kantenabnutzung über `vc.g`, Diagonal-Klüftung + Relief-Normale, fleckige Moos-/Graspolster auf waagerechten Absätzen im Wald-/Herbst-Biom bzw. Staub in der Wüste, erdig-feuchter Wandfuß (`0..1.3 m`).
- **Teil C (Katalog, Regeln, Bridge, Editor):** `env_rock_cliff` in `data/catalog/neutral.toml` (Wikipedia-Quellen `Cliff`, `Escarpment`, `Joint (geology)`), `data/objects.toml` (`blocks = "all"`, +0,4 m Sicherheitsrand je Seite), `data/vision.toml` (`solid`), `crates/bridge/src/objects.rs` (`ROCK_KINDS` + Hang-Ankerung `min(center, low + 2.4 m)` für spaltfreien Sitz ohne tiefes Einsinken), `godot/scenes/world/map_objects.gd` (unbegrenzte Sichtweite + Schatten), Editor-Pinselvorlage `Cliff wall` / „Felswand“ in `editor_data.gd` & `editor.csv`. Thumbnail (`just thumbnails --only=env_rock_cliff`), `just units-web-sync` und `just catalog-doc` ausgeführt.
- **Teil D (`[[cliff]]` in `gen_sketch` & `pit_hill`):** Opt-in-Block `[[cliff]]` (`points_m`, `facing_m`, optional `height_m`, `boulders`, `forest_top`) in `schema.rs`, `plan.rs`, `works.rs`, `settlements.rs` (inkl. asymmetrischer Spiegelung unter `point`/`north`/`west`) und `mod.rs`. Nicht geänderte Skizzenkarten bleiben Bit-für-Bit unverändert (geprüft an `river_fields`). Auf `pit_hill` (`data/map_sketches/pit_hill.toml`) 130 `env_rock_cliff`-Module (65 je Hälfte, punktsymmetrisch) entlang Ost-/West-Bahnlinie (mit `height_m = 7.5` und Nadelwald-Krone), Nord-/Süd-Berghang und zentralem Felsband eingesetzt.
- **Golden Hashes (bewusste Regeländerung):** `crates/sim/tests/determinism.rs` → `0xbcd0_ed12_d711_b075` (vorher `0x462d_25c7_d644_3026`, Demo-Gefecht unverändert Tick 625) und `crates/sim/tests/economy.rs` → `0xdfe5_7153_fbe1_5491` (vorher `0x37de_a01e_1753_cb4e`) durch den neuen Eintrag `env_rock_cliff` in den gehashten Regeln `data/objects.toml` und `data/vision.toml`.
- **Prüfungen:** `rocks.rs` (5/5), `vision.rs` (17/17), `mapgen` (`pit_hill_is_point_symmetric`, `pit_hill_top_only_over_ramps`), `just map-check pit_hill` (0 Fehler/Warnungen, Hash `0x0d6c2274618113ea`), `just map-reach pit_hill` (alle Starts/Versorgungen/Rampen in `main`, 89,9 % fahrbar), `just stuck-audit` (`pit_hill`: 208/208, 0,0 % stuck), `just validate-assets`, `just glb-check`, `just l10n-check` (1481 Keys, 0 Probleme), `just check`, `just selftest`, `just editor-selftest` alle grün.
- **Vorläufige Entscheidungen (Nachtlauf):** `facing_m` als Richtungsvektor analog `[[trench]]`/`[[works]]`; Kopfende (`-Y`) einer Klippenlinie startet mit `Buttress` + vorgelagertem `env_rock_outcrop:Leaning`, damit die steile `+X`-Schauseite stets talwärts zeigt; Hang-Ankerung `min(center, low + 2.4 m)` bei `embed = 3.0 m`; Ingame-Aufnahmeserie (`build/cliff_wall/`) und `objects-bench` im Nachtlauf ausgespart (visuelle Abnahme im Spiel empfohlen).
