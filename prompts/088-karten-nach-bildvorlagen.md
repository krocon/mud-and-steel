# Prompt 088 – Zehn Karten nach Bildvorlagen, mit Feldern

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `docs/MAPS.md` ganz (Format, Geländearten, Objekte, Brücken, alle Generatoren,
> besonders „Landscape maps (prompt 056)“, „Large city (prompt 079)“ und „Maps from
> real geodata (prompt 081)“, Karteneditor), in `docs/ARCHITECTURE.md` die Abschnitte
> zu Wegfindung, „Vision and fog of war“, „Map objects and decoration“ und „Tall
> grass (prompt 073)“, in `docs/ASSETS.md` „Environment objects (prompt 007)“ bis
> „Checks and previews“ und „Thumbnails“, dazu `docs/PERFORMANCE.md`, `docs/AI.md`
> und `docs/SOURCES.md`. Dann den Code: `crates/cli/src/gen_land.rs`, `gen_town.rs`,
> `gen_city.rs`, `tools/geo_import.py`, `data/terrain.toml`, `data/objects.toml`,
> die Gebäude in `data/catalog/neutral.toml`, `godot/scenes/terrain/` mit
> `terrain.gdshader` und dem hohen Gras.
>
> **Arbeite selbständig, ohne Rückfragen und ohne Halt zwischen den Stufen.** Was
> der Prompt offen lässt, entscheidest du selbst, begründest es kurz und trägst es
> am Ende in die Tabelle „Entscheidungen“ in dieser Datei ein (vorläufig). Anhalten
> nur, wenn etwas installiert, gelöscht oder schwer umkehrbar entschieden werden
> müsste (`CLAUDE.md`). Kein `git push`.
>
> **Nicht gleichzeitig mit Prompt 085 ausführen:** beide ändern Regeltabellen und
> damit die goldenen Hashes. Prompt 086 (Wind) fasst die Böen des hohen Grases an,
> auf denen das Getreide aufbaut: Stufe 1 erst danach oder davor, nicht zugleich.
> Andere laufende Sessions im selben Ordner vor schweren
> Builds und Messungen prüfen.

**Vorlagen** (nicht im Repo, `docs/.vorlagen/` ist ignoriert), zehn Bilder in
`docs/.vorlagen/_todo/maps/`: `img.png`, `img_1.png` bis `img_8.png` und
`map5_wesertal_5120_grid.png`. Die Bilder sind groß (bis 5120 Pixel, bis 19 MB):
zum Ansehen verkleinerte Kopien im Scratchpad anlegen, Ausschnitte in voller
Auflösung nur wo nötig. Fehlt ein Bild, diese Karte auslassen und es sagen.

## 1. Ziel

Aus **jedem Bild eine neue, spielbare Gefechtskarte**: zehn Karten. Das Bild gibt
den Aufbau vor (wo Fluss, Küste, Straßen, Wald, Felder, Ort und Höhen liegen), das
Spiel liefert die Bausteine. Was den Bildern an Bausteinen fehlt, baust du selbst:
vor allem **Felder** (Getreide, grüne Felder, gepflügte braune Erde), dazu die
Objekte aus Abschnitt 5.

Fertig ist der Prompt, wenn

- es ein Werkzeug gibt, das aus einer Kartenskizze im Repo eine Karte schreibt
  (`just gen-sketch <name>`), bei jedem Lauf Bit für Bit gleich,
- alle zehn Karten in `data/maps/` liegen, `just map-check` bestehen und im Menü
  wählbar sind,
- man jede Karte neben ihrer Vorlage wiedererkennt: Lauf des Wassers, Straßennetz,
  Lage und Dichte von Ort, Wald und Feldern,
- die Felder im Spiel wie Felder aussehen (nah und fern, ohne Flimmern) und im
  Editor malbar sind,
- auf jeder Karte ein Gefecht KI gegen KI zu Ende läuft, ohne dass Einheiten
  hängen bleiben, und Bildrate und Tickzeit in den Grenzen aus
  `docs/PERFORMANCE.md` liegen,
- `just check`, `just selftest` und `just editor-selftest` grün sind.

## 2. Was die Bilder sind und was nicht

Die Bilder sind **Vorbild für den Aufbau**, kein Material: keine Bildausschnitte,
keine Farben oder Texturen daraus, nichts davon ins Repo (auch keine verkleinerten
Kopien, Masken oder Vergleichsbilder; die liegen in `build/` oder im Scratchpad).
Der Aufbau wird grob nachgezeichnet, nicht Pixel für Pixel übernommen: Linien und
Flächen so vereinfacht, wie die Bausteine des Spiels es brauchen.

- **Keine Namen aus den Bildern.** Auch nicht „Wesertal“, „Bismarckturm“,
  „Königszinne“, „Altstadt“: keine echten Ortsnamen im Spiel, in Kartennamen,
  Markern oder Doku. Beschriftungen, Gitter (A1 bis P16), nummerierte Kreise, rote
  Rahmen und Wolken gehören nicht zur Landschaft.
- **Keine moderne Welt.** Autobahnkreuze, Kreisverkehre, Stadien, Golfplätze,
  Tanklager heutiger Art, Gewerbegebiete: durch das ersetzen, was es um 1940 gab
  (Landstraße, Kreuzung, Wiese, Lagerhäuser und Hallen), oder weglassen.
- **Gebäude sind Katalogmodelle.** Lage, Ausrichtung und Dichte folgen dem Bild,
  die Häuser kommen aus dem Bestand (wie bei `harbor_city`).
- Zeigt ein Bild erkennbar die Karte eines anderen Spiels, gilt `docs/SOURCES.md`
  erst recht: nur der grobe Aufbau, keine Einzelheiten, die es wiedererkennbar
  machen (genaue Hauspositionen, Zielpunkte, Bezeichnungen).

## 3. Vom Bild zur Karte: Skizze und Generator

| Frage | Festlegung | Grund |
|---|---|---|
| Was liegt im Repo? | Je Karte eine **Skizze** `data/map_sketches/<name>.toml` und die erzeugte Karte `data/maps/<name>/` | Die Skizze ist lesbar, änderbar und die einzige Quelle; die Karte entsteht daraus jederzeit neu. |
| Was steht in der Skizze? | Größe, Biom, Wetter, Startwert; Linien und Flächen in Metern: Flüsse (mit Breite), Küsten und Seen, Straßen und Wege (Klasse, Breite), Bahnlinien, Wälder (Art, Dichte), Felder (Art, Reihenrichtung), Siedlungsflächen (Dorf, Stadt, Blockviertel, Lager, Industrie), Einzelgebäude, Höhen (Lage, Radius, Höhe), Starts, Versorgungspunkte, Marker | Mehr trägt ein Bild nicht; alles Weitere (einzelne Häuser, Bäume, Zäune) setzt der Generator aus seinem Zufallsstrom. |
| Wie entsteht die Skizze? | Du siehst das Bild an und zeichnest nach. Ein Hilfswerkzeug (`tools/map_trace.py`) darf aus den Bildfarben Rohmasken für Wasser, Wald und Felder ziehen und zu Linien und Vielecken vereinfachen; du prüfst und korrigierst das Ergebnis. Die Masken bleiben außerhalb des Repos | Wald- und Uferränder von Hand abzutippen kostet Stunden und wird ungenau; die Farben der Bilder sind dafür klar genug. Was das Werkzeug nicht sicher trennt (Straßen, Häuser), zeichnest du. |
| Generator | Neuer `cli gen-sketch <name>` (`crates/cli/src/gen_sketch.rs`, `just gen-sketch [name]`, ohne Namen alle), nach dem Muster von `gen_land` und `gen_city` | Ein Generator für zehn Karten statt zehn Generatoren. |
| Wiederverwendung | Fluss, Ufer, Brücken mit Dämmen, Straßen auf Wegzellen-Mittellinien, Dorf- und Hofbausteine, Blockviertel, Wald, Hecken, Platzierer ohne Überlappung: aus `gen_map`, `gen_land`, `gen_town`, `gen_city` teilen | Nur herausziehen, was die bestehenden Karten Bit für Bit gleich lässt (prüfen). |
| Gleichheit | Nur Grundrechenarten und `sqrt` in Gleitkomma, kein `atan2` (Hinweis in „Landscape maps“); ein Test erzeugt jede Karte zweimal und vergleicht | Wie bei allen Generatoren. |

**Maßstab.** Kein Bild hat einen sicheren Maßstab. Die Größen in Abschnitt 6 sind
Anhaltswerte; prüfe sie am Bild (ein Haus ist 8 bis 20 m lang, eine Landstraße 6
bis 10 m breit, ein Gitterfeld der Bilder mit Gitter etwa 100 m) und lege die
Kartengröße so fest, dass Häuser und Straßen des Spiels in die Flächen des Bildes
passen. Breite und Tiefe sind ganze, gerade Meter; größer als 4 km wird keine
Karte.

**Was der Generator aus dem Bild machen muss, damit man darauf spielen kann:**

- Straßen mindestens 8 m, Hauptstraßen 12 bis 16 m breit, auf den Mittellinien der
  4-m-Wegzellen; Wege im Ort für Fahrzeuge offen (Erfahrung aus 079: 12 m in
  Vorstädten).
- **Flüsse und Brücken.** Die Flüsse der Bilder sind breiter, als die Brücken des
  Spiels lang sind. An jedem Übergang das Wasser verengen und Dämme bauen wie
  `gen_map`; reicht das dem Bild nicht, längere Brückenvarianten bauen (Abschnitt
  5). Zwischen den Teams führen über jedes Wasser **mindestens zwei Wege für
  mittlere Panzer** (Steinbrücke oder Furt), auch wenn das Bild nur einen zeigt;
  Holzbrücken tragen 16 t.
- **Höhen.** Die Bilder zeigen kaum Relief. Ebenes Land bekommt sanfte Hügel und
  Rauschen wie `gen_land`; gezeichnete Berge, Felsen und Steilufer werden Höhen
  mit Fels aus Prompt 074. Flüsse liegen im Tal, Orte und Starts eben.
- **Starts und Versorgungspunkte** setzt du: die Bilder sind nicht symmetrisch,
  also wird die Gerechtigkeit **gemessen** statt gespiegelt. `just map-reach
  <name>` (Wege von jedem Start zu jedem Versorgungspunkt, Fahrzeuge und
  Infanterie): die Summen beider Teams weichen höchstens etwa 15 % ab; sonst
  Starts und Punkte verschieben, nicht die Landschaft. Starts 60 m ringsum eben
  und frei, mit Platz für den Basis-Modus; Versorgungspunkte auf Orten, die man
  sieht (Brücke, Platz, Hof, Bahnhof, Hafen), Namen `supply_*`.
- Nummerierte Kreise in den Bildern sind ein Hinweis auf Starts (Abschnitt 6),
  kein Muss.

## 4. Felder

Bisher gibt es Felder nur als Wiese mit Heuhaufen. Die Vorlagen leben von ihnen:
gelbe Getreideschläge, grüne Felder mit Reihen, braune gepflügte Äcker, als
Flickenteppich zwischen Wegen, Hecken und Wald.

| Frage | Festlegung | Grund |
|---|---|---|
| Wie kommen Felder in die Karte? | Drei neue Geländearten in `data/terrain.toml`: `field_grain` (reifes Getreide), `field_green` (Rüben, Klee, junge Saat: niedrig, grün, in Reihen), `field_ploughed` (gepflügte Erde) | Wie `tall_grass` in Prompt 073: im Editor malbar, im Kartenformat ohne Änderung, für KI und Regeln sichtbar. |
| Regeln | Alle drei wie Gras (Tempo, Aufweichen bei Regen und Schnee, keine Deckung, keine Tarnung), vorläufig | Erst das Aussehen; Acker bremst Räder, Getreide tarnt Infanterie: eigene Regeln, offener Punkt. |
| Goldener Hash | Ändert sich wegen der Regeltabellen, wie bei 073 und 074. Nachweisen, dass es nur daran liegt (ein Lauf auf einer bestehenden Karte verhält sich gleich), und im Commit nennen | `CLAUDE.md`. |
| Reihenrichtung | Die Reihen laufen entlang der langen Kante des Schlags, nicht im Weltraster. Die Richtung ist Darstellung und steht nicht im Karten-Hash; wo sie liegt (Liste der Schläge in `map.toml`, eigene kleine Datei, Richtungstextur), entscheidest du nach dem Code. Ohne Angabe: eine feste Richtung je zusammenhängendem Schlag | Felder im Weltraster sehen nach Schachbrett aus; die Vorlagen zeigen jede Richtung. |

Aussehen, nach den Vorlagen und nach Fotos echter Felder (Quellen in
`docs/SOURCES.md`, nichts aus Spielen):

- **Getreide:** dichtes, warmes Gelb mit feinen Reihen und Fahrgassen, nah als
  Halme (die Technik des hohen Grases wiederverwenden: Büschel nah, Shader fern,
  Windwellen, flach unter Fahrspuren und Ketten), hüfthoch. Ein Teil der Schläge
  abgeerntet: Stoppeln, Garben oder Ballen aus dem Bestand.
- **Grünes Feld:** niedrige Reihen in sattem bis graugrünem Ton, dazwischen Erde.
- **Gepflügt:** dunkle bis rotbraune Erde mit Furchen, die im Streiflicht Schatten
  werfen (Normale im Shader, keine Geometrie); nach Regen dunkler.
- **Ränder:** jeder Schlag hat eine klare Kante, davor ein Streifen Gras, oft ein
  Feldweg, eine Hecke oder ein Zaun; Schläge 60 bis 250 m groß, mehrere
  nebeneinander in wechselnder Art und Richtung.
- **Biome:** im Winter liegen die Felder unter Schnee, Furchen und Reihen zeichnen
  sich schwach ab (Vorlage `img_5.png`); in der Wüste gibt es keine oder bewässerte
  kleine Gärten; für `forest` der Sommer wie oben.
- **Ohne Flimmern** bei Kamerafahrt und in der Ferne (`just ground-motion`, Maß wie
  bei Wiese und hohem Gras), über 60 FPS auf „hoch“ über einer Feldflur.
- Editor: die drei Arten im Geländepinsel mit Farbfeld; der Feld-Baustein des
  Generators (Schlag aus Vieleck, Rand, Weg, Hecke) steht auch `gen_land` offen,
  ohne dessen Karten zu ändern.

Eine kleine Testkarte oder ein Ausschnitt reicht für Bilder und Messung; eine
Bildreihe `just field-shots` (nah, mittel, fern, Winter, Regen, Fahrspur).

## 5. Weitere neue Bausteine: nur wenn nötig

Baue Neues, wenn ein Motiv in mindestens zwei Vorlagen vorkommt oder einer Karte
ihr Gesicht gibt. Sonst nimm einen Stellvertreter aus dem Bestand und notiere es.
Jedes neue Modell: Katalogeintrag mit Quellen, Blueprint im Baukasten
(`tools/blender/kit/`), Varianten `Intact` / `Damaged` / `Ruin` wo Gebäude,
Budgets und Prüfungen aus `docs/ASSETS.md`, Regeln in `data/objects.toml`,
Vorschaubild (`just thumbnails --only=<id>`), `just units-web-sync`,
`just catalog-doc`.

| Motiv | In | Vorschlag |
|---|---|---|
| Bahnlinie | `img_2`, `img_7`, Wesertal | Gleis als Darstellung auf einem Schotterbett (Gelände wie Feldweg, für alle passierbar), flacher Damm, Bahnübergänge; Bahnbrücke über den Fluss als Steinbrücke mit Gleis oder als Stellvertreter die Steinbrücke. Keine Züge. |
| Lange Brücke | fast alle | Längere Varianten von Holz- und Steinbrücke (mehr Joche), wenn das Verengen der Flüsse das Bild zu sehr verfälscht. |
| Baracken und Hüttenlager | `img_4`, `img_5` | Reihen und Ringe aus Blockhaus, Scheune, Schuppen; ein eigenes Modell `env_barrack` nur, wenn der Bestand nicht nach Lager aussieht. |
| Fabrik und Lager | `img`, `img_1`, `img_2`, Wesertal | Lagerhaus, Halle, Depot aus dem Bestand; ein Schornstein oder Wasserturm als neues Modell, wenn das Viertel sonst nicht als Industrie lesbar ist. |
| Burg, Schloss, Aussichtsturm | `img`, `img_1`, Wesertal | Stellvertreter: Kirche oder Rathaus auf der Höhe mit Mauer; ein einfacher steinerner Turm als neues Modell, wenn die Höhe sonst leer wirkt. Kein Schloss. |
| Kiesgrube, Steinbruch | `img_2`, Wesertal | Aus Gelände: Sand, Abbruchkanten, See aus flachem und tiefem Wasser, Felsen aus 074. Kein Modell. |
| Meer, Lagune, Seen | `img_6`, `img_8`, `img` | Tiefes Wasser bis zum Kartenrand wie bei `harbor_city`, flaches Wasser am Ufer, Sandstrand; Dämme mit Straße als Gelände, Brücken über die Durchlässe. |
| Mittelmeer- und Wüstenhäuser | `img_6` | Bestand der Wüstenhäuser (Prompt 059). |

## 6. Die zehn Karten

Namen sind festgelegt (keine echten Orte). Größe, Biom und Spielerzahl sind
Anhaltswerte, die du am Bild prüfst. Wo nichts anderes steht: Biom `forest`,
Wetter klar, zwei Teams, ein Spiel zu zweit nutzt die Spieler 0 und 1.

| Bild | Karte | Größe | Was das Bild zeigt und was die Karte daraus macht |
|---|---|---|---|
| `img_4.png` | `river_fields` | 1,6 × 1,6 km | Sommer. Ein breiter Fluss von West nach Ost mit einer Insel, eine helle Hauptstraße Nord–Süd über eine Steinbrücke, drei Holzbrücken und ein Steg, Feldwege. Nördlich Baracken in Reihen, ein Hof mit vier Schlägen (Getreide, grün, gepflügt), ein befestigter Platz; südlich zwei Hüttenlager im Ring, ein Hofplatz mit Rondell, viele Schläge. Wald im Norden, Osten und Süden. Vier Starts: Team 0 im Süden, Team 1 im Norden. **Die erste Karte**: an ihr entstehen Generator und Felder. |
| `img_5.png` | `river_fields_winter` | 1,6 × 1,6 km | Dieselbe Landschaft im Winter: gleiche Skizze, Biom `winter`, Wetter Schnee, Felder unter Schnee, Nadelwald, geräumte Straßen. Der Fluss bleibt offen. Aus derselben Skizze mit anderem Biom erzeugen, nicht neu zeichnen. |
| `img_7.png` | `river_fork` | 1,6 × 1,6 km | Sommer. Zwei Flüsse fließen im Westen zusammen (einer von Norden, einer von Osten). Im Winkel dazwischen und südlich davon ein zerschossenes Dorf mit vielen verzweigten Sandwegen und einzelnen Häusern, Mauerresten; sechs Brücken; eine Bahnlinie quer durch den Süden mit Bahnbrücke; ringsum Wald mit gewundenen Wegen. Der rote Rahmen im Bild ist der Gefechtsraum: Versorgungspunkte innen, Starts außen. Viele Gebäude als `Damaged` und `Ruin`. |
| `img_8.png` | `island_chain` | 1,6 × 1,6 km | Sommer, Küste. Eine Kette von Inseln und Halbinseln im Meer, verbunden durch **eine** Straße über vier Brücken von Nordost nach Südwest; Höfe, wenige Schläge, Wald, kleine Teiche. Kreise 1 und 2 im Bild: die beiden Starts an den Enden. Die Kette bleibt der Charakter der Karte; dazu mindestens eine Furt als zweiter Weg (Abschnitt 3). Fernes Land am Rand ohne Zugang darf unerreichbar bleiben und trägt nichts Spielwichtiges. |
| `img_6.png` | `desert_lagoon` | 1,6 × 1,6 km | Wüste am Meer. Küste im Nordosten, eine verzweigte Lagune in der Mitte mit Dämmen und drei Brücken, geschwungene Pisten, verstreute Wüstenhäuser und ein kleiner Ort im Osten, Palmen- und Buschhaine, dichter Bewuchs im Nordwesten und Süden. Kreise 1 bis 6 im Bild: sechs Starts, 1 bis 3 im Osten (Team 0), 4 bis 6 im Norden (Team 1). Biom `desert`. |
| `img_3.png` | `winter_town` | 1,6 × 1,6 km | Winter. Eine Kleinstadt mit mehreren hundert Häusern beiderseits eines Flusses von Nord nach Süd, drei Brücken, enges gewachsenes Straßennetz mit Platz und großen Gebäuden (Kirche, Rathaus, Hallen), ein Ortsteil im Nordwesten, Nadelwald rundum, Landstraßen zu allen Rändern. Team 0 im Westen, Team 1 im Osten. Dichte prüfen: Fahrzeuge kommen durch jede Hauptstraße. |
| `img.png` | `river_town` | etwa 3,0 × 2,0 km | Sommer. Eine Kleinstadt in einer Flussschleife, der Fluss windet sich von Norden nach Südosten, zwei Seen (einer mit Insel), sieben Brücken, Ortsteile am anderen Ufer, Felder am Rand, viel Wald, zwei Felshöhen, eine Höhe mit Burg. Schnellstraße und Kreisel werden Landstraßen, Sportplatz und Golfplatz Wiese, Gewerbe wird Lagerviertel. Vier Starts. |
| `img_1.png` | `valley_fields` | etwa 3,0 × 2,0 km | Sommer. Im Westen eine Stadt mit Hallen und Bahnhof, ein Fluss in großen Bögen quer durch die Karte, in der Mitte und im Osten Dörfer zwischen vielen Getreide- und Grünschlägen, im Osten Felsberge mit Stromschnellen. Die Feldkarte: hier müssen die Felder aus Abschnitt 4 tragen. Vier Starts. |
| `map5_wesertal_5120_grid.png` | `river_valley` | etwa 3,6 × 2,0 km (Ausschnitt um den Fluss; das Bild zeigt rund 5 × 2,8 km, Maßstabsbalken unten links) | Sommer. Ein Fluss von Norden nach Süden durch ein Tal zwischen bewaldeten Bergen (drei Gipfel, zwei Türme), am Westufer eine Stadt mit Altstadt an der Brücke, Vorstadt und Lagerhallen, im Nordwesten eine Kiesgrube mit See, Felder in vielen Farben, eine Fernstraße und eine Bahnlinie von West nach Ost, ein kleiner Hafen. Erkennbar ein echter Ort: Aufbau ja, Namen nein, und so vereinfacht, dass es eine Landschaft dieser Art ist und kein Stadtplan. Vier Starts, Team 0 im Westen, Team 1 im Osten. |
| `img_2.png` | `river_city` | etwa 3,6 × 2,4 km | Sommer. Eine Großstadt aus Blockvierteln mit strahlenförmigen Straßen, ein Fluss in einer großen Schleife um das Zentrum mit acht Brücken, ein Park mit Teichen im Nordosten, Bahnlinie mit Bahnhof im Süden, Steinbruch und Hallen im Westen, Felder am Rand. Bausteine von `city_large` (Blöcke, Stadthäuser, Vorstadt); Autobahnkreuz wird Landstraße. Kräftig vereinfachen: lieber lichter und flüssig als dicht und unter 60 FPS. Die schwerste Karte, zuletzt. |

## 7. Reihenfolge

Ohne Halt, in dieser Reihenfolge; nach jeder Karte ein Commit und ein kurzer
Zwischenstand mit Vergleichsbild an den Nutzer, dann weiter.

1. **Stufe 1: Werkzeug und Felder.** Skizzenformat, `gen-sketch`, die drei
   Feldarten mit Darstellung und Editor, `river_fields` und
   `river_fields_winter`. Erst wenn das Vergleichsbild von `river_fields`
   stimmt, weiter.
2. **Stufe 2: die 1,6-km-Karten.** `river_fork`, `island_chain`,
   `desert_lagoon`, `winter_town`.
3. **Stufe 3: die großen.** `river_town`, `valley_fields`, `river_valley`,
   `river_city`.

Was der Generator für eine spätere Karte lernt, darf frühere Karten nur ändern,
wenn du sie neu erzeugst, prüfst und es im Commit sagst. Führe am Ende dieser
Datei eine Tabelle „Stand je Karte“ (Skizze, erzeugt, geprüft, gemessen, offen),
damit eine zweite Session weitermachen kann, falls diese nicht alle zehn schafft.
Eine Karte, die nicht spielbar wird, bleibt mit Grund als offen stehen und kommt
nicht ins Menü.

## 8. Prüfen, je Karte

1. **Vergleichsbild:** die ganze Karte senkrecht von oben neben der Vorlage, im
   gleichen Ausschnitt (`build/sketch-compare/<name>.png`). Nach jeder
   Änderungsrunde neu. Dazu `SHOT_SET=land just terrain-shots <dir> <name>` und
   zwei, drei Ansichten aus Spielhöhe.
2. `just map-check <name>`, `just map-reach <name>` (Erreichbarkeit, getrennte
   Teile, Wege von den Starts, Ausgleich nach Abschnitt 3); die Karte im Editor
   öffnen und speichern (gleicher Hash).
3. **Bildrate** (`just bench-view` und eine Ansicht über der dichtesten Stelle,
   jede Grafikstufe: über 60 FPS), **Tickzeit** mit einem großen Gefecht,
   Ladezeit, Dateigrößen, Objektzahl; als Tabelle, verglichen mit `test_large`
   und `city_large`.
4. **KI:** `just ai-tournament` auf der Karte mit mehreren Startwerten; jedes
   Spiel endet, keine Einheit steht minutenlang fest, kein Team gewinnt
   durchgehend. Bei Brückenkarten: die KI findet die Übergänge.
5. **Menü:** Name, Größe, Biom und Wetter erscheinen; deutsche und englische
   Anzeigenamen wie bei `harbor_city`.

Am Ende einmal alles: `just gen-sketch` schreibt alle zehn Bit für Bit gleich neu
(Test), `just check`, `just selftest`, `just editor-selftest`,
`just units-web-sync`.

## 9. Abschluss

- `docs/MAPS.md`: die neuen Geländearten in der Tabelle, Abschnitt „Maps from
  picture sketches (prompt 088)“ (Skizzenformat, Generator, je Karte Aufbau,
  Maße, Starts, Versorgungspunkte, Marker, offene Punkte).
  `docs/ARCHITECTURE.md`: Felder in der Darstellung. `docs/ASSETS.md` und
  `docs/SOURCES.md`: neue Modelle und Quellen. `docs/PERFORMANCE.md`: die
  Messwerte. `docs/AI.md`, falls die KI etwas lernen musste. `CLAUDE.md` und
  `justfile`: `just gen-sketch`, `just field-shots`. Auf Englisch, ohne die
  Vorlagen.
- In dieser Datei: „Entscheidungen“, „Stand je Karte“, „Ergebnis“ mit den Pfaden
  der Vergleichsbilder.
- `prompts/README.md`: Status mit einer Zeile Ergebnis; sind alle zehn Karten
  fertig, die Datei mit `git mv` nach `prompts/done/`, sonst bleibt sie mit dem
  Stand liegen.
- Commits auf Englisch in sinnvollen Schritten (Felder mit dem Hinweis auf den
  goldenen Hash, Generator, je Karte einer, neue Modelle, Doku). Kein `git push`.
- Was nicht erreicht wurde, steht im Abschlussbericht mit Grund, nicht als
  erledigt.

## 10. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Regeln für Felder:** Acker bremst Räder, Getreide tarnt Infanterie, Feldbrand,
  Ernte je Jahreszeit.
- **Fahrende Züge, Schiffe und Boote**, Bahnhof als besetzbares Gebäude.
- **Frei geformte Gebäude** aus Grundrissen, Burgen und Schlösser, Wahrzeichen.
- **Kartenvorschau im Menü** (keine Karte hat eine).
- **Missionen oder Videoeinstellungen** auf den neuen Karten.
- **Automatisches Erkennen** von Straßen und Häusern aus Bildern: das
  Hilfswerkzeug zieht nur Flächen, gezeichnet wird von Hand.

## Entscheidungen (vorläufig, Nachtlauf)

| Frage | Entscheidung | Grund |
|---|---|---|
| Skizzenformat | TOML je Karte in `data/map_sketches/`, Tabellen `start`, `supply`, `marker`, `river`, `island`, `lake`, `crossing`, `road`, `ring_road`, `square`, `wood`, `scatter`, `field`, `row`, `ring`, `farm`, `building`, `hill`; `base` für Abwandlungen (Winter) | Lesbar, eine Quelle; die Winterkarte erbt alles und ändert nur Biom, Wetter, Startwert. |
| Nachzeichnen | Von Hand an einer verkleinerten Kopie (1400 px), Pixel in Meter umgerechnet; kein `tools/map_trace.py` | Bei `river_fields` sind die Flächen groß und klar; ein Maskenwerkzeug lohnt erst bei den großen Karten. |
| Maßstab `river_fields` | 1,6 × 1,6 km (1 px der Vorlage = 0,4 m); Baracken ~30 m, Fluss ~70 m breit | Häuser und Wege passen in die Flächen des Bildes. |
| Brücken | Wasser an jeder Brücke verengt (Stein 21 m, Holz 7 m), gerade Zufahrt 55 m, Achse auf Wegzellen-Mittellinie | Brücken des Spiels sind höchstens 30 m lang und nur achsparallel; längere Varianten wären neue Modelle. |
| Zwei Panzerwege über den Fluss | Steinbrücke in der Mitte und Furt im Westen (Holzbrücken tragen 16 t) | Abschnitt 3; das Bild zeigt nur eine Steinbrücke. |
| Steg der Vorlage | entfällt | Kein Steg-Modell; die drei Brücken und die Furt genügen. |
| Felder als Gelände | Ids 9–11, Regeln wie Gras, Golden Hashes ändern sich | Abschnitt 4. |
| Reihenrichtung | Berechnet die Bridge je zusammenhängendem Schlag (Hauptachse der Flächenmomente), nicht in der Karte gespeichert | Keine Formatänderung, im Editor gemalte Felder bekommen sie automatisch. |
| Aussehen der Felder | Nur Shader (gefilterte Reihen, Fahrgassen, Furchen mit Normale), keine Halme | Halme nach der Technik des hohen Grases sind eine eigene Stufe (offen). |
| Starts im Wald | Zufahrtswege in der Skizze ergänzt | Sonst war ein Start für Fahrzeuge abgeschnitten (`map-reach`). |
| Nachzeichnen ab Stufe 2 | Vorlage auf 1000 „Spurpixel“ verkleinert, mit Gitter alle 25 px (ImageMagick, Skript im Scratchpad), Skizze in Pixeln geschrieben und mit einem Skript in Meter umgerechnet | Schneller und genauer als Schätzen am ganzen Bild; nichts davon im Repo. |
| Bahnlinie | `[[rail]]`: Schotterbett als Feldweg-Gelände (7 m), flacher Damm, Telegrafenmasten daneben; Bahnbrücke = Steinbrücke auf der Linie; keine Schienen | Ein Gleismodell wäre ein neuer Baustein mit Quellen, Baukasten und Vorschaubild; das Schotterbett mit Masten liest sich von oben als Bahn. Schienen bleiben offen. |
| Ruinendorf | `[[village]]` (lose Häuser entlang des nächsten Wegs, Anteil beschädigt/Ruine, Lehmmauerreste) und `[[paint]]` mit Rauschflecken für den zertretenen Sand | Die Vorlage zeigt kein Straßenraster, sondern verstreute Häuser auf Sandflecken. |
| Schräge Übergänge | Fluss in der Skizze an jedem Übergang 30 m quer zur Brückenachse gelegt | Brücken des Spiels sind achsparallel; schräg gemessen war das verengte Wasser zu breit für die Holzbrücke. |
| Meer und Inseln | `sea = true` mit `[[land]]`-Vielecken, Küste auf eine Höhe gezogen (flacher Meeresspiegel), `[[causeway]]` als Damm mit Brückenlücke quer über die ganze Breite, `[[shoal]]` als Untiefe | Die Wasserfläche des Spiels folgt den Ufern; mit einer Küstenhöhe bleibt das Meer eben. Dämme machen aus den 40–60 m breiten Lücken der Vorlage Übergänge für 30-m-Brücken. |
| Furten `island_chain` | Zwei lange Untiefen als zweiter Panzerweg: vom Nordostkap zur Waldinsel und zwischen Westwald und Südwestland; die Kette selbst nur über Brücken | Abschnitt 6 verlangt den Charakter der Kette; die Furten umgehen je zwei Brücken. |
| Dammbreite | 30 m statt 14 m | Bei 14 m endeten Brückendecks neben Wasser; Panzer blieben dort stehen. |
| Lagune | `[[bay]]`: Wasserflächen im Land, die zum Meer gehören (gleicher Spiegel), dürfen sich überlappen; Dämme mit Brücken wie bei den Inseln | Die Lagune hängt über eine Meerenge am Meer; als Seen hätte sie einen eigenen Spiegel. |
| Wüstenhaine | Waldart `grove`: Palmen mit Büschen | Palmen allein lesen sich nicht als Hain. Von ganz oben (3 km) blendet die Darstellung Palmen und Büsche aus; im Vergleichsbild fehlen sie deshalb, aus Spielhöhe sind sie da. |
| Straßenbebauung | `[[town]]`: Häuser beiderseits jeder Straße im Ortsvieleck, Längsseite zur Straße, dazu ein `[[village]]` über dasselbe Vieleck für die Hinterhöfe | Die Vorlage zeigt gewachsene Bebauung an Straßen, keine Blockraster wie `city_large`. |
| Ausgleich `winter_town` | Spieler 0 im Nordwesten, ein Versorgungspunkt an der Ostkreuzung statt am Westufer | Mit dem Weststart als Spieler 0 gewann der Westen mit beiden Fraktionsreihenfolgen; `map-reach` lag bei 25 %. |
| Maßstab `river_town` | 3 × 2 km, 1 Spurpixel = 3 m (Vorlage auf 1000 × 667 verkleinert) | Häuser der Vorlage 15–20 m; die Brücken der Vorlage sind ohnehin Sinnbilder. |
| Burg | Kirche auf einer 14-m-Höhe mit Steinmauerring (`[[ring]]` aus `env_wall_stone`) | Abschnitt 5: Stellvertreter statt Schloss. |
| Turnierlänge 3-km-Karten | `--max-ticks 12000` (10 min) | Bei 6000 Ticks erreichten die Armeen einander auf 3 km oft nicht. |
| Feldflur `valley_fields` | `[[patchwork]]`: Schläge im gedrehten Raster mit 6-m-Rainen, Art zufällig, 15 % Wiese; dazu die einzeln gezeichneten Schläge | Die Vorlage ist ein dichter Flickenteppich; jeden Schlag einzeln abzutippen lohnt nicht. |
| Ausschnitt `river_valley` | 720 × 400 Pixel der auf 1000 Pixel verkleinerten Vorlage (5 m je Pixel) um den Fluss; Türme als Steinhaus im Mauerring; Fluss 80 m statt bis 150 m | Größenvorgabe 3,6 × 2 km; ein Turmmodell lohnt erst, wenn die Höhen leer wirken. |
| Blockviertel `river_city` | Je Viertel ein gedrehtes Straßenraster (Linien im Skizzenskript am Viertelrand abgeschnitten, als normale `[[road]]` in der Skizze), dichte Straßenbebauung mit `env_block_town` und `gap_m = 2`; kein Teilen von `gen_city` | `gen_city` baut achsparallele Raster für eine eigene Kartenform; die Vorlage braucht gedrehte Viertel. |
| Ausgleich `river_fork` | Spieler 0 im Osten, Spieler 1 im Norden: das 1v1 trifft sich im Dorf über den Ostfluss | Mit dem Südweststart als Spieler 0 lief das 1v1 nur über die Westbrücken. Die Siegverteilung der KI folgt auf allen Karten (auch `test_small`) der Fraktion: mit `de,su` gewinnt Spieler 0, mit `su,de` Spieler 1; gemessen wird deshalb beides. |

## Stand je Karte

| Karte | Skizze | erzeugt | geprüft | gemessen | offen |
|---|---|---|---|---|---|
| `river_fields` | ja | ja | `map-check`, `map-reach` (Ausgleich Fahrzeuge 12 %, Infanterie 3 %), Editor-Selftest | objects-bench: Wald 30 FPS (wie `test_large`), Mitte 66, Übersicht 82; KI Schlacht 16 Spiele: Süd 7, Nord 5, 4 Zeit | Getreidehalme nah, Steg, Fluss an Brücken stark verengt |
| `river_fields_winter` | ja (`base`) | ja | `map-check`, `map-reach` (12 % / 5 %) | KI Basis wie alle Karten Zeit | wie oben |
| `river_fork` | ja | ja | `map-check`, `map-reach` (Ausgleich Fahrzeuge 10 %, Infanterie 7 %) | objects-bench: Wald 25 FPS, Mitte 31, Übersicht 59, Dorf 112; KI Schlacht je 16 Spiele: `de,su` 13:1 (2 Zeit), `su,de` 0:15 (1 Zeit), also fraktions-, nicht kartenbedingt | Schienen (nur Schotterbett), Fluss an Brücken stark verengt |
| `island_chain` | ja | ja | `map-check`, `map-reach` (4 % / 5 %) | objects-bench: Wald 25 FPS, Mitte 31, Übersicht 68, Dorf 92; KI Schlacht je 16: `de,su` 4:4 (8 Zeit), `su,de` 9:3 (4 Zeit) | Patt an den Brücken (viele Zeitüberschreitungen), einzelne Panzer bleiben auf Brückendecks stehen (siehe offene Punkte) |
| `desert_lagoon` | ja | ja | `map-check`, `map-reach` (Teamsummen < 1 % / < 1 %) | objects-bench: nah 108 FPS, Mitte 75, Übersicht 136, Dorf 89; KI Schlacht je 16: `de,su` 10:4 (2 Zeit), `su,de` 5:10 (1 Zeit), 3v3 8 Spiele 5:2 (1 Zeit); keine hängenden Einheiten | Haine von ganz oben unsichtbar (Sichtweite der Objekte) |
| `winter_town` | ja | ja | `map-check`, `map-reach` (7 % / 13 %) | 770 Gebäude; objects-bench: Wald 24 FPS, Mitte 28, Übersicht 59, Ort 106; KI Schlacht je 16: `de,su` 4:4 (8 Zeit), `su,de` 1:6 (9 Zeit) | viele Zeitüberschreitungen im Ortskampf; Brückendeck-Hänger wie oben |
| `river_town` | ja | ja | `map-check`, `map-reach` (5,5 % / 5,8 %) | 879 Gebäude, 44 000 Bäume; objects-bench: Wald 23 FPS, Mitte 23, Übersicht 55, Ort 98; KI Schlacht je 16 (12 000 Ticks): `de,su` 5:8 (3 Zeit), `su,de` 10:3 (3 Zeit) | Wald und Mitte unter 60 FPS (Schatten, wie auf allen Waldkarten); kein Maskenwerkzeug nötig |
| `valley_fields` | ja | ja | `map-check`, `map-reach` (1,6 % / 12,6 %) | 931 Gebäude; objects-bench: Wald 28 FPS, Mitte 32, Übersicht 84, Ort 96; KI Schlacht je 16 (12 000 Ticks): `de,su` 13:2 (1 Zeit), `su,de` 2:8 (6 Zeit); keine hängenden Einheiten | Stromschnellen nur als Felsen am Ufer; Bahnhof nur als Schotterbett mit Hallen |
| `river_valley` | ja | ja | `map-check`, `map-reach` (10 % / 7 %) | 699 Gebäude, 57 000 Bäume; objects-bench: Wald 28 FPS, Mitte 25, Übersicht 41, Ort 104; KI Schlacht je 16 (12 000 Ticks): `de,su` 2:3 (11 Zeit), `su,de` 5:10 (1 Zeit) | Übersicht unter 60 FPS; viele Zeitüberschreitungen (KI hält die Stadt); Türme nur als Stellvertreter |
| `river_city` | ja | ja | `map-check`, `map-reach` (9,7 % / 1 %) | 3042 Gebäude; objects-bench: Wald 25 FPS, Mitte 32, Übersicht 61, Stadt 105; KI Schlacht je 16 (12 000 Ticks): `de,su` 8:2 (6 Zeit), `su,de` 4:11 (1 Zeit) | Brückendeck-Hänger wie oben |

## Offene Punkte aus Stufe 2

- **Stehenbleibende Panzer an Übergängen** (alle Brückenkarten, auch `river_fields`): im Schlacht-Turnier
  bleiben einzelne Panzer minutenlang auf einem Brückendeck oder am Rand einer Furt stehen (`target`
  leer; die Simulation stoppt eine Einheit nach `STALL_TICKS` ohne Fortschritt am letzten Wegpunkt, der
  Angriffsbefehl der Gruppe gibt ihr keinen neuen Weg). Auf `island_chain` fuhren zwei Panzer
  nebeneinander auf das 5 m breite Deck. Gehört in einen eigenen Prompt (KI: angehaltene Gruppenmitglieder
  neu losschicken, oder Simulation: neu planen statt anhalten); die Karten umgehen es nicht.

## Ergebnis (Stufe 1)

- Generator `cli gen-sketch` / `just gen-sketch [name]` (`crates/cli/src/gen_sketch.rs`), Skizzen in
  `data/map_sketches/`, Test: jede Skizzenkarte zweimal erzeugt und Bit für Bit verglichen.
- Drei Feldarten (`field_grain`, `field_green`, `field_ploughed`, Ids 9–11) mit den Regeln von Gras;
  Darstellung im Gelände-Shader (`terrain_fields.gdshaderinc`: Reihen, Fahrgassen, Furchen, gefiltert
  gegen Flimmern), Reihenrichtung je Schlag aus der Bridge, Editor-Pinsel; Golden Hashes geändert
  (nur Regeltabellen, Gegenprobe mit den alten Dateien ergibt die alten Werte).
- `river_fields` und `river_fields_winter` (1,6 km, vier Starts), im Menü als „Flussfelder“.
- Vergleichsbilder: `build/sketch-compare/river_fields.png`, `river_fields_winter.png`,
  `fields_views.jpg`; Feldbilder `just field-shots`.
- `harbor_city/terrain.png` neu geschrieben: nur die Palette ist um die drei Feldfarben länger
  (Pixel und Karten-Hash gleich), sonst schlug die Geo-Prüfung fehl.
- Nicht erreicht (Stufen 2 und 3): die übrigen acht Karten (siehe Tabelle), Getreidehalme nah
  (Technik des hohen Grases), Flimmermessung mit `ground-motion` für Felder, Bahnlinie, lange
  Brücken, Straßenbebauung für Städte, `tools/map_trace.py`. Eine zweite Session kann mit
  `river_fork` weitermachen.
