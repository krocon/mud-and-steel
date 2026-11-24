# Prompt 124 – Karte nach Bildvorlage: Grabenlinie (Stellungskrieg im Schlamm)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md`, `021-gelaendeverformung.md`,
> `035-pioniere-minen-stellungen.md`, `052-stellungen-stufe2.md` und
> `085-baeume-unter-schwerem-feuer.md` (jeweils Ziel, Entscheidungen, Ergebnis). In
> `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz und „Objects“
> (`fort_*`, Depots, kahle Bäume), in `docs/ARCHITECTURE.md` „Field works“,
> „Region updates (craters, prompt 021)“, die Deckungsregeln (Krater als
> Deckungsstellen, gegrabene Gräben), indirektes Feuer und „Lighting, atmosphere and
> biomes“. Dann `crates/cli/src/gen_sketch.rs`, `data/map_sketches/river_fork.toml`
> (zerschossenes Dorf, zertrampelter Sand) als Muster, `data/works.toml`,
> `data/objects.toml`, `data/scale.toml` (`[range]`). Sind die Prompts 122 oder 123
> erledigt, auch deren Ergebnis (Look je Karte, Reichweitenrechnung, Symmetrie).
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/stellungskrieg/stellungskrieg.webp`.
> Ein mit KI gezeichnetes Bild, das die Oberfläche eines kommerziellen Spiels
> nachahmt. Verwendet wird **nur die Landschaft**: Anordnung von Gräben,
> Niemandsland und Hinterland, die Stimmung. Nichts von Oberfläche, Minimap,
> Symbolen oder dem Stil jenes Spiels übernehmen (`docs/SOURCES.md`: nichts aus
> anderen Spielen).
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel neben
> der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine frei erfundene Gefechtskarte `trench_line` (im Spiel „Grabenlinie“ /
„Trench line“) für einen **festgefahrenen Stellungskrieg**: zwei Grabensysteme
gegeneinander, dazwischen ein zerwühltes Niemandsland, dahinter Artillerie und
Nachschub. Braun, nass und zerschossen, passend zum Namen des Spiels.

Die Vorlage zeigt:

- **Grabensysteme** im Zickzack mit Querverbindungen (Laufgräben nach hinten),
  Unterständen, Bohlen auf dem Grabenboden, Sandsack- und Holzbrüstungen;
- **Niemandsland:** dicht an dicht Granattrichter, viele mit Wasser gefüllt,
  Stacheldraht vor den Gräben, ein zerschossenes Haus, kahle und abgebrochene
  Bäume, ein Bach mit sumpfigen Ufern;
- **Hinterland:** Geschütze in Sandsackringen, Zelte, Munitions- und
  Materiallager, Lkw, schlammige Wege, kleine Holzbrücken über den Bach, Reste von
  Wald an den Rändern, flache Hügel.

## 2. Spielgedanke

- **Punktsymmetrisch** um die Kartenmitte (wie `city_large`): beide Seiten gleich
  stark. Die Session zeichnet eine Hälfte nach der Vorlage und spiegelt sie.
- **Größe** etwa 2,4 km × 1,6 km, die Fronten laufen quer. Abstand der vordersten
  Gräben 250–400 m (Niemandsland), Feuerstellungen der Artillerie 600–900 m hinter
  der eigenen Front. Abstände nach Reichweite rechnen (indirektes Feuer 1/8,
  Sicht 1/4; Tabelle wie in Prompt 123): Mörser erreichen aus dem eigenen Graben die
  gegnerische Front, leichte Haubitzen das Niemandsland und die gegnerischen
  Gräben, aber nicht die gegnerischen Feuerstellungen.
- **Zwei bis drei Lücken** im Draht und Gassen im Trichterfeld, durch die Fahrzeuge
  kommen; sonst ist das Niemandsland für Fahrzeuge zäh (Schlamm, Krater, Draht),
  für Infanterie gefährlich, aber voller Deckung (Krater als Deckungsstellen).
- **Flanken:** Waldreste und ein Bachgrund an den Seiten als zweiter Weg.
- **Starts** hinter den Lagern, je zwei pro Team (2 gegen 2), außerhalb der Reichweite
  leichter Artillerie aus den gegnerischen Feuerstellungen. **Versorgungspunkte:**
  das zerschossene Haus in der Mitte (umkämpft), Brücken am Bach, Waldreste an den
  Flanken, je ein Punkt im eigenen Grabensystem.

## 3. Skizze und neue Bausteine (Stufe 1)

`data/map_sketches/trench_line.toml`, erzeugt mit `just gen-sketch trench_line`.
Der Skizzen-Generator bekommt dafür neue Tabellen (in `docs/MAPS.md` beschreiben):

- **`[[trench]]`**: ein Linienzug wird zu einem Zickzack-Graben (Zackenlänge und
  -tiefe als Werte) aus den vorhandenen Grabenstücken (`fort_trench_section`) bzw.
  gegrabenen Gräben wie in Prompt 035/052, mit Querverbindungen nach hinten, Ecken
  und Enden. Die Gräben geben dieselbe Deckung wie im Spiel gegrabene. Die Session
  wählt, ob die Gräben als Objekte oder als vorab gegrabenes Gelände gespeichert
  werden, und begründet es.
- **`[[crater_field]]`**: Polygon mit Dichte (Trichter je Hektar), Größenverteilung
  nach Kaliber und Anteil mit Wasser. Die Trichter werden ins Höhenfeld gegraben wie
  Laufzeit-Krater (Prompt 021), damit sie dieselbe Deckung geben; eigener
  Zufallsstrom, deterministisch.
- **Wasser in Trichtern:** neu. Vorschlag: der Boden im Trichter wird `mud`, die
  Darstellung legt eine braune Wasserfläche in die Mulde (Pfütze, reine
  Darstellung, ohne eigenen Wasserspiegel in der Simulation). Gleiches für nasse
  Senken am Bach.
- **`[[wire]]`**: Stacheldraht entlang einer Linie aus `fort_barbed_wire`, mit
  Lücken an angegebenen Stellen.
- **Zerschossener Wald:** kahle und abgebrochene Bäume (Prompt 085) in Flächen, mit
  Stümpfen (`env_tree_stump`).
- **Lager:** Depots (`env_dump_ammo`, `env_dump_fuel`), Zelte (neues einfaches
  Objekt `env_tent`, zwei Varianten, mit Vorschaubild; oder als offener Punkt),
  abgestellte Lkw nur als Kulisse.
- **Geschützstellungen:** Sandsackringe (`fort_sandbag_position`) auf ebenem Boden,
  für gezogene Geschütze erreichbar.
- **Unterstände und Bohlenwege:** Unterstand als kleines Objekt am Graben
  (Infanterie kann hinein wie in ein Gebäude), Bohlenweg als Wegstück über
  Schlamm. Beides neu; wenn die Stufe zu groß wird, Unterstände als `fort_bunker_mg`
  und Bohlenwege als offener Punkt.
- **Bach** als `[[river]]`, schmal und sumpfig, mit Holzbrücken (`wood`) und Furten;
  **Wege** als `track`, schlammig (`[[paint]]` mit `mud` in Flecken entlang der Wege).

## 4. Prüfen (Stufe 2)

- `just map-check trench_line`, `just map-reach trench_line` (Starts, Feuerstellungen,
  Versorgungspunkte für Fahrzeuge mit gezogenem Geschütz erreichbar; Gassen durch das
  Niemandsland vorhanden), Symmetrietest, Test wie bei den anderen Skizzenkarten
  (zweimal erzeugt, gleiche Dateien).
- Sichtfeld-Bilder von den Feuerstellungen und aus den vordersten Gräben;
  Reichweitentabelle in `docs/MAPS.md`.
- **Pfadfindung:** Das Trichterfeld darf die Wegsuche nicht unzumutbar teuer machen;
  `just bench` bzw. `path-bench` auf der Karte, Ergebnis in `docs/PERFORMANCE.md`.
- **KI-Turnier**, Ergebnis in `docs/AI.md`: Greift die KI über das Niemandsland an,
  nutzt sie die Gassen, endet es im Patt? Probleme als offene Punkte, die KI nicht
  umbauen.
- Bildrate in jeder Ansicht (viele Krater, Draht, Grabenstücke).

## 5. Schlamm-Look (Stufe 3)

Reine Darstellung, nur für diese Karte: ein Look „Schlamm“ mit gedämpften,
erdigen Farben, nassem Glanz auf Schlamm und Pfützen, grauem, diesigem Himmel,
etwas Nebel über dem Niemandsland, vereinzeltem Rauch von Bränden. Ist der
Mechanismus „Look je Karte“ aus Prompt 122 schon da, hier ein zweiter Look; sonst
diesen Mechanismus hier einführen (Feld in `map.toml`, nicht im Hash; siehe 122,
Abschnitt 4) und in 122 darauf verweisen. Vergleich mit `forest` neben der
Vorlage; der Nutzer urteilt.

Danach Menüname und Übersetzung, Doku in `docs/MAPS.md` (Tabelle der Skizzenkarten,
neue Tabellen der Skizze) und `docs/ARCHITECTURE.md` (Pfützen, Look),
`just check`, `just selftest`, `just editor-selftest` (falls der Editor neue
Objekte kennt). Der Golden Hash ändert sich nicht durch eine neue Karte; ändern
neue Objekte Regeltabellen, die als Ganzes gehasht werden, ist das beabsichtigt
und gehört mit Grund in den Commit.

## 6. Nicht Teil dieses Prompts

- Vorgegebene Startarmeen in den Gräben (Mission oder Szenario).
- Gräben im Karteneditor als Linienwerkzeug (sinnvoller Folgeauftrag).
- Wetter mit Regen, der Schlamm vermehrt; Gas; Flammenwerfer-Effekte am Graben.
- KI-Verbesserungen für Grabenkampf und Durchbruch.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 23.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Gräben als Objekte oder Gelände | **Weder noch: vorbereitete Feldarbeiten** in `map.toml` (`[[field_work]]`: `trench`, `comm_trench`, `wire`, `mg_nest`, `dugout` als Linien bzw. Punkte), von der Simulation vor dem ersten Tick als fertige neutrale Arbeiten gegraben (wie Stützpunkte, `crates/sim/src/sim/prepared.rs`). Im Kartenhash (nur Karten mit solchen). **[vorläufig]** | Nur so gelten genau die Regeln gegrabener Gräben (Deckung jedes gegrabenen Samples, Feuerplätze, Brustwehr, Treffer an der Brustwehr) und dasselbe Aussehen (`trench_builder.gd`); vorab gegrabenes Gelände hätte keine Deckung (die Deckung hängt am Grab-Raster bzw. am Krater-Delta), `fort_trench_section`-Objekte geben keine. |
| 2 | Grabtiefe der Kartengräben | `map_dig_pct = 75` (`[positions]` in `data/works.toml`, nicht eigens gehasht: wirkt über den Boden, den die Krater-Prüfsumme deckt). **[vorläufig]** | Der volle 0,5-m-Schnitt mit Brustwehr liegt genau an der 60-%-Grenze einer Wegzelle; auf geneigtem Boden waren die Grabenlinien für alle gesperrt. Deckung unverändert. Im Spiel gegrabene Gräben bleiben bei 100 % (dort besteht das Problem auf Hängen weiter, offen). |
| 3 | Trichter | `[[crater]]` in `map.toml` (Kaliber, Wasser), vor Tick 0 mit `plan_crater` wie Bodentreffer gegraben, zählen nicht als Granattrichter, keine Ereignisse; Skizze `[[crater_field]]` mit eigenem Zufallsstrom. Kaliber 105–280 mm (110 je ha im Niemandsland), weil Trichter unter 2 m Radius auf dem 2-m-Raster kaum sichtbar sind. **[vorläufig]** | Dieselbe Deckung und derselbe Schlamm wie Laufzeit-Krater. Trichtermitten sperren wie jeder Granattrichter die Zelle (Regel aus 021). |
| 4 | Wasser in Trichtern | Wie vorgeschlagen: Boden `mud` (macht der Krater), Wasser reine Darstellung (`water`-Flag, `[[puddle]]` für nasse Senken, `[[marsh]]` in der Skizze). Dazu eine **Trichterhaut** je Trichter (dunkle nasse Mulde, heller Auswurfrand), über das Gelände drapiert (`world/puddles.gd`). **[vorläufig]** | Ohne Haut liest sich ein 2-m-Trichter von oben nicht als Granatloch. |
| 5 | Draht | Als vorbereitete Arbeit `wire` statt `fort_barbed_wire`-Objekte (Vorschlag des Prompts). **[vorläufig]** | Gleiche Regeln wie gelegter Draht: sperrt Infanterie, wird von Fahrzeugen plattgewalzt, Pioniere räumen ihn. |
| 6 | Symmetrie | `mirror = "point"`: Nordhälfte um (1, 1) gedreht; Höhen in 24 m um die Mittellinie mit ihrem Bild gemischt (sonst Stufe an der Naht); Objekt genau im Mittelpunkt (Haus) bleibt einmal. **[vorläufig]** | Prompt verlangt Punktsymmetrie; Test `trench_line_is_point_symmetric` (teilt sich den Rumpf mit dem Spiegeltest von 123). |
| 7 | Größe | 2,4 × 2,0 km statt 1,6 km Tiefe. **[vorläufig]** | Reichweitenregel: Feuerstellungen 640 m hinter der Front (leichte Haubitzen 1 099–1 541 m erreichen Gräben, nicht die Gegenstellungen ≥ 1 572 m) und Starts dahinter passen nicht in 1,6 km. |
| 8 | Bach | Zwei Bäche an den Flanken (Nord nach Süd), im Niemandsland ganz flach (`[[river]] shallow`), dahinter mit Rinne, Holzbrücken und Furten; Bachgrund und zerschossener Wald als zweiter Weg. **[vorläufig]** | Ein Bach durch die Kartenmitte müsste bei Punktsymmetrie durch den Mittelpunkt (das Haus). |
| 9 | Gassen | Zwei Gassen (x = −540/542 m, 16 m) ohne Trichter, Draht offen, fester Grasboden; dazu je Seite zwei Infanterielücken im Draht. **[vorläufig]** | Eine dritte Gasse durch die Mitte ginge wegen der Punktsymmetrie durch das Haus. |
| 10 | Unterstände, Bohlenwege | Unterstände = Unterstand-Arbeit aus 052 (bessere Deckung), nicht betretbar wie ein Gebäude; Bohlenwege offen. **[vorläufig]** | Stufe sonst zu groß (Prompt erlaubt es). |
| 11 | Zelte | Neues Objekt `env_tent` (`Pyramidal`, `Wall`) mit Katalogeintrag, Modell, Vorschaubild; Maße Schätzwerte (unsicher markiert). Lkw als Kulisse fehlen (kein neutrales Lkw-Objekt). **[vorläufig]** | Prompt. |
| 12 | Wetter und Look | Kartenwetter `clear` (Regel), Look `mud` (`forest_mud.tres`) mit grauem Himmel, Dunst, braunem Gras und Wasser, neues Preset-Feld `wet_ground` (immer nasser Boden); Rauch an `smoke_<n>`, Nebelschleier an `mist_<n>`. **[vorläufig]** | `overcast` hätte die Sicht verkürzt und die Reichweiten-/Sichttabelle verändert. Nutzer urteilt über den Look. |
| 13 | Leistung | `works_view.gd`: Gelenksuche über ein Halbmeterraster, Neuaufbau nur bei geänderter Signatur (`SimHost.field_works_signature`); Simulation: Hinderniszellen nur aus Arbeiten im Rechteck, Draht der Karte in einem Durchgang, Kastenprobe beim Plattwalzen. **[vorläufig]** | Vorher 90-ms-Ruckler alle 0,25 s, 1 s `Sim::new`, 5 ms je Tick. Ergebnisse gleich (gleiche Anfragezahl, Golden Hash unverändert). |

### Ergebnis

- **Karte** `data/maps/trench_line/` aus `data/map_sketches/trench_line.toml`
  (2,4 × 2,0 km, punktsymmetrisch, 12 239 Objekte, 5 692 Trichter, 2 290
  Feuergraben-, 550 Laufgraben-, 2 530 Drahtstücke, 26 MG-Nester, 78
  Unterstände, 152 Pfützen), im Menü „Grabenlinie“ / „Trench line“.
- **Simulation**: `sim::prepared` (Format, Kartenhash), `Sim::prepare_ground`,
  `map_dig_pct`, `Look::Mud`; Editor behält die Tabellen. Tests
  `prepared_ground_is_read_and_hashed`, `prepared_ground_is_dug_before_the_first_tick`,
  `prepared_ground_is_deterministic`. **Golden Hash unverändert**.
- **Generator**: `mirror = "point"`, `[[trench]]`, `[[wire]]`, `[[lane]]`,
  `[[crater_field]]`, `[[marsh]]`, `[[camp]]`, `[[river]] shallow`, Wald
  `shattered`; alle zwölf anderen Skizzenkarten Bit für Bit gleich (geprüft).
- **Prüfungen**: `map-check` ok; `map-reach`: alle Starts, Punkte,
  Stellungen, Gassen im Hauptteil, Weglängen beider Seiten gleich;
  Symmetrietest; Reichweitentest `trench_line_ranges_and_sight`
  (Tabelle in `docs/MAPS.md`); `stuck-audit` 304/304 angekommen;
  `path-bench` 1,0 ms je Tick, Planung 4,9 ms; Bildrate hoch: Graben 83,
  Mitte 79, Übersicht 83, dichter Wald 27 FPS, mittel überall ≥ 63 FPS;
  KI-Turnier in `docs/AI.md` (Angreifer tut sich schwer, Basismodus Patt,
  Gassen werden genutzt).
- **Bilder**: `build/trench_line/final/compare_{top,oblique,nml,front}.png`
  (Vorlage, Schlamm-Look, Wald), Einzelbilder dort; `build/trench_line/reach.png`,
  `sight.png`; Zwischenstände der Stufen in `build/trailer/stills/survey_trench_line_*`.
- **Offen**: Look-Urteil des Nutzers; Lkw-Kulisse (Modell fehlt);
  Bohlenwege; Unterstände nicht betretbar; Gräben im Editor zeichnen;
  im Spiel gegrabene Gräben sperren auf Hängen weiterhin die Zellen;
  Zelte wirken sehr hell; Trichterhäute schneiden an steilen Stellen ins
  Gelände; dichter Wald < 60 FPS auf „hoch“; KI nutzt Gräben und
  Feuerstellungen nicht (eigener Prompt).

### Zweiter Aufbau (24.11.2026, auf Wunsch des Nutzers)

Der erste Aufbau (zwei gerade Grabenlinien quer über die Karte, Bäche an den
Flanken) traf die Vorlage nicht. Neu in `data/map_sketches/trench_line.toml`:

- **Bach diagonal** durch die Kartenmitte (Nordwest nach Südost), eine **Straße im
  rechten Winkel** dazu mit Furt in der Mitte; Team 0 Nordost, Team 1 Südwest.
- **Kreisförmiger Ringweg** (260 m Radius) um das Trichterfeld, zwei Holzbrücken
  über den Bach, je ein zerschossenes Haus pro Ufer im Ring.
- **Gezackte Gräben**: Frontlinie mit 120-m-Zacken, zweite Linie gegenläufig
  (geschlossene Zellen), Stützlinie, Laufgräben, Draht folgt den Zacken. Neuer
  Skizzenbaustein `[[trench]] kind = "line"` (von Hand gezeichneter Verlauf mit
  `nests_m`, `dugouts_m`).
- **Zwei leichte Anhöhen** (7 m) westlich und östlich des Rings mit je zwei
  Geschützstellungen, Lager und Versorgungspunkt.
- Entscheidungen 6–9 der Tabelle oben sind damit überholt (Bach durch die Mitte,
  Straße als Gasse, Fronten 620 m auseinander statt 290 m). Die Geschützstellungen
  erreichen sich jetzt gegenseitig mit leichten Haubitzen; die Starts liegen weiter
  außerhalb aller Geschütze auf der gegnerischen Anhöhe. Reichweitentabelle und Test
  `trench_line_ranges_and_sight` neu gefasst (`docs/MAPS.md`).
- Geprüft: `map-check`, `map-reach`, Symmetrietest, Reichweitentest, `stuck-audit`
  (271 von 272 angekommen), Bilder `build/trench_line/v2/`.
- **Offen**: KI-Turnier, Bildrate und `path-bench` für den neuen Aufbau nicht neu
  gemessen; Sandsackstellungen wirken neben der Vorlage klein; die Zellen der
  Gräben sind sehr regelmäßig.
