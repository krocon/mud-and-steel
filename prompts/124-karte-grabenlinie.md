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
