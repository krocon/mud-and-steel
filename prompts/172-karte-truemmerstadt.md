# Prompt 172 – Karte nach Bildvorlage: Trümmerstadt (Flussgabel, drei Brücken, Häuserkampf)

> In einer frischen Session im Repo-Ordner ausführen. Keine harte
> Voraussetzung: Häuser mit `Intact` / `Damaged` / `Ruin` gibt es seit 078.
> **Empfohlen nach** 134 (Großstadthäuser), 155 (Fabrik, Schornstein,
> Kaimauer), 139 (Kleinstadtbauten) und 122 (Look je Karte); was davon fehlt,
> ersetzt die Session durch den nächsten vorhandenen Baustein und schreibt
> einen offenen Punkt. Keine neuen Modelle in diesem Prompt.
>
> Lies zuerst `CLAUDE.md`, dann `prompts/done/088-karten-nach-bildvorlagen.md`
> ganz und von `prompts/done/078-*` Ziel und Ergebnis, dazu, falls erledigt,
> `156-*` (Fabrikstadt) und `170-*` (Inselstadt). In `docs/MAPS.md` „Maps
> from picture sketches (prompt 088)“ ganz, die Einträge `river_city` und
> `city_large` (Symmetrie-Abschnitt: Punktsymmetrie), „Bridges“, „Objects“;
> in `docs/ARCHITECTURE.md` Wegfindung (Größenklassen: Panzer brauchen 8 m,
> besser 10–12 m), Sicht, „Destruction“, Gebäude besetzen, „Field works“;
> `data/scale.toml` (`[range]`), `data/objects.toml`, `data/terrain.toml`,
> `docs/TEAMS.md`, `docs/SOURCES.md`, `crates/cli/src/gen_sketch.rs` und
> `data/map_sketches/river_city.toml`.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/truemmerstadt/img.webp`: ein mit KI (ChatGPT) erzeugtes,
> frei erfundenes Bild, 1312 × 1199 px, **fast Draufsicht** (leicht schräg
> von Süden). Die Häuser sind im Bild zu klein gezeichnet: Die Session
> skaliert nach echten Hausmaßen (Wohnhaus 8–12 × 10–14 m, Straßen 10–14 m,
> Gassen 6–8 m) und zeigt den Plan in Stufe 1 neben dem Bild. Rauch und Brände
> sind Stimmung. Es gibt **keine SVG und keine Beschreibung**: Maße und Regeln
> kommen aus diesem Prompt.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Skizzenkarte `rubble_town` (im Spiel „Trümmerstadt“ / „Rubble town“),
Team Nord gegen Team Süd: eine **mittelgroße Stadt, schon weitgehend in
Trümmern**, die ein Fluss in zwei Hälften teilt. Fast jedes zweite Haus ist
beschädigt oder Ruine, in den Höfen stehen kahle Bäume, Schutt liegt in den
Straßen. Gekämpft wird **von Haus zu Haus** und um die **drei Brücken** in der
Stadtmitte. Am Stadtrand beider Seiten liegen Gräben mit Bunkern.

Die Vorlage zeigt (Bildkoordinaten in Pixeln, x nach rechts, y nach unten;
die Meter ergeben sich erst aus der Skalierung):

- **Fluss** etwa 40–60 m breit (im Bild 60–90 px), mit Ufermauern, kleinen
  Stegen und Booten: Ein **Westarm** läuft am linken Rand von Nord nach Süd
  (x ≈ 150–250), knickt bei y ≈ 600 und der **Hauptarm** läuft nach Osten zum
  rechten Rand (bei y ≈ 450); der Westarm läuft unten links weiter nach Süden;
- **Brücken:** drei breite **Straßenbrücken** über den Hauptarm bei
  (375, 660), (730, 650) und (1150, 490); kleinere Brücken und Stege über den
  Westarm bei (200, 70), (240, 245), (165, 580) und (60, 850);
- **Nordstadt** (zwischen Westarm und Hauptarm, oben): unregelmäßige Blöcke
  aus Wohnhäusern mit Gärten, eine **Kirche** mit Turm bei (500, 120) und ein
  **runder Platz mit Denkmal** davor (435, 200), ein großes **Herrenhaus** bei
  (835, 100), ein **Ringweg** um die Stadt, am Nordostrand Gräben;
- **Südstadt** (unter dem Hauptarm): ein **Fabrikviertel** mit Schornstein
  und langen Hallen im Südwesten um (250, 850), Wohnblöcke in der Mitte, eine
  **Parkanlage mit Brunnen** im Südosten um (985, 850), Lagerhallen am Ufer;
- **Westufer** (links vom Westarm): Wiese, Nadelwäldchen, Zickzack-Gräben;
- **Stadtrand Süd** (y ≈ 1000–1150): durchgehende Grabenlinie mit
  Holzverschalung, zwei **Rundbunker** bei (300, 1050) und (1020, 1025),
  Panzersperren, Schützenlöcher, dahinter Nadelwald;
- **Straßen** von Brücke zu Brücke und durch die Stadt, alle unbefestigt
  oder zerfahren, mit Fahrzeugwracks und Kisten.

## 2. Größe, Symmetrie, Spieler

- **Größe:** Die Stadt im Bild wird nach Hausmaßen auf etwa **1200 × 1100 m**
  skaliert. Die Karte legt Umland an, damit die Starts außer Reichweite liegen:
  Vorschlag **2400 × 2400 m**, Stadt in der Mitte, rundum Felder, Wälder und
  Vorstadt im Stil des Bildes. Biom `forest`, **später Winter / Vorfrühling**
  (kahle Bäume, braune Wiesen, ohne Schnee), Wetter bedeckt.
- **Punktsymmetrisch** um die Mitte des Hauptarms, (x, y) → (−x, −y), wie
  `city_large` und 156: Die Session zeichnet die **Nordhälfte** nach dem Bild
  (Nordstadt mit Kirche, Platz, Herrenhaus, Westarm nach Norden, Gräben am
  Nordrand) und dreht sie für die Südhälfte. Folgen, die so gewollt sind:
  - der Hauptarm läuft als flache S-Kurve West–Ost durch die Mitte; die
    mittlere der drei Straßenbrücken liegt auf dem Drehpunkt, die beiden
    anderen tauschen durch die Drehung die Plätze;
  - der Westarm nach Norden bekommt sein Gegenstück als **Ostarm nach Süden**
    (die Karte hat damit eine Z-Form aus Wasser), der Westarm nach Süden aus
    dem Bild entfällt;
  - Kirche, Denkmalplatz und Herrenhaus haben ihr Gegenstück im Süden; das
    **Fabrikviertel** und der **Park mit Brunnen** aus dem Bild werden in die
    Nordhälfte übernommen (Fabrik im Nordosten am Ostufer, Park im
    Nordwesten), damit beide Hälften beides haben. Hausvarianten dürfen
    abweichen, Grundrisse, Wege und Deckung nicht.
- **Spieler:** 1 gegen 1 und 2 gegen 2, Team Nord gegen Team Süd. Vorschlag:
  Spieler 0 (Nord) im Nordwesten hinter dem Westarm, Spieler 2 (Nord) im
  Nordosten, Spieler 1 (Süd) im Südosten hinter dem Ostarm, Spieler 3 (Süd)
  im Südwesten; bei 1 gegen 1 nur 0 und 1 (diagonal). Jeder Start: 60 m flach
  und frei, hinter der eigenen Grabenlinie, mindestens zwei Fahrzeugausfahrten.
- **Versorgungspunkte:** 9–11, paarweise gedreht, nach dem Ort benannt.
  Vorschlag: mittlere Brücke (Drehpunkt, neutral), je 1 an den beiden anderen
  Brückenköpfen, Kirchplatz, Herrenhaus, Fabrik, Park, je 1 hinter jedem
  Start.

## 3. Spielregeln für den Aufbau

- **Brücken:** drei Straßenbrücken über den Hauptarm (`env_bridge_stone`,
  mindestens 12 m Fahrbahn), jede einzeln sprengbar; jede Seite behält nach
  dem Sprengen einer beliebigen Brücke noch einen Fahrzeugweg über den Fluss.
  Die kleinen Brücken über den Seitenarm (Holz oder Stein) tragen leichte
  Fahrzeuge oder nur Infanterie: die Session entscheidet je Brücke und
  begründet. **Keine Furt** im Hauptarm; im Seitenarm höchstens eine flache
  Furt für Infanterie, wenn die Brückenprobe sonst scheitert.
- **Zerstörung:** etwa **40 %** der Häuser beginnen als `Damaged`, **25 %**
  als `Ruin`, gleich verteilt auf beide Hälften (durch die Drehung von
  selbst). Schutt nur an den Straßenrändern: Hauptstraßen behalten **12 m**
  freie Fahrbahn, Gassen 6–8 m für Infanterie und leichte Fahrzeuge.
- **Häuserkampf:** Häuser besetzbar, je Block mindestens zwei Eingänge,
  Ruinen als Deckung ohne Dach. Der **Kirchturm** ist der beste
  Aussichtspunkt; er darf keinen Start einsehen. Größte freie Fläche ohne
  Deckung in der Stadt höchstens 200 m, im Umland höchstens 500 m.
- **Stadtrand:** Grabenlinie, Draht, Sperren und Rundbunker
  (`fort_bunker_mg`, `fort_hedgehog`, `[[trench]]`, `[[wire]]`) wie im Bild,
  mit einer Fahrzeuggasse von mindestens 12 m an jeder Ausfallstraße.
- **Reichweiten:** Abstände rechnen und als Tabelle festhalten: Mörser vom
  eigenen Stadtrand erreichen den Fluss, leichte Artillerie vom Start
  erreicht die Brücken, aber nicht den gegnerischen Stadtrand.
- Gebäudezahl deutlich unter 3000 (Stadt etwa 250–400 Häuser).

## 4. Stufen

### Stufe 1: Plan und Skizze

Zuerst den skalierten Plan (Wasser, Brücken, Straßen, Blöcke, Plätze, Gräben,
Starts) als Bild neben der Vorlage zeigen, mit den gewählten Maßen und der
Drehung. Dann `data/map_sketches/rubble_town.toml`, erzeugt mit
`just gen-sketch rubble_town`. Bilder aus dem Spiel: Übersicht, jede
Straßenbrücke, Kirche mit Platz, Fabrik, Park, Grabenlinie mit Bunker, neben
dem Ausschnitt der Vorlage.

### Stufe 2: Prüfen

- `just map-check rubble_town`, `just map-reach rubble_town`: jeder Start und
  jeder Versorgungspunkt für Fahrzeuge erreichbar, auch mit jeder einzelnen
  gesprengten Brücke (Brückenprobe wie 131/152).
- Symmetrietest (Drehung), Test wie bei den anderen Skizzenkarten (zweimal
  erzeugt, gleiche Dateien).
- Engste Fahrzeuggasse auf jeder Brücke und Hauptstraße messen (≥ 12 m),
  auch mit dem Schutt der Anfangszerstörung.
- Sichtfeld-Bilder aus dem Kirchturm, vom Herrenhaus, von der mittleren
  Brücke; Reichweitentabelle.
- Wegsuche und Bildrate in der dichten, zerstörten Stadt wie bei
  `river_city` (`docs/PERFORMANCE.md`).
- **KI-Turnier** 1 gegen 1 und 2 gegen 2, auch mit vertauschten Seiten,
  Ergebnis in `docs/AI.md`: Ziel höchstens 55 % für eine Seite. Probleme der
  KI im Häuserkampf als offene Punkte; die KI nicht umbauen.

### Stufe 3: Look und Abschluss

Später-Winter-Look (kahle Bäume, braune Wiesen, nasse Straßen), Schutt- und
Brandspuren, Wasser mit Strömung. Menüname und Übersetzung, Eintrag in der
Tabelle der Skizzenkarten in `docs/MAPS.md` (mit Zuordnung Vorlage →
Baustein, Skalierung, Reichweiten- und Sichttabelle), `docs/SOURCES.md`
(Vorlage frei erfunden, kein reales Vorbild), `just selftest`,
`just lan-selftest`, `just check`. Der Golden Hash ändert sich durch eine neue
Karte nicht.

## 5. Nicht Teil dieses Prompts

- Neue Modelle (Herrenhaus, Denkmal, Brunnen, Bootsstege, falls nicht
  vorhanden): vorhandene Objekte als Ersatz, offene Punkte.
- Rauch und Brände als dauerhafte Kartenelemente.
- Eine Mission mit vorgegebenen Armeen.
- KI-Verbesserungen für Häuserkampf.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| Frage | Entscheidung | Grund |
|---|---|---|
| Vorlage | nur das Bild (fast Draufsicht, nach Hausmaßen skaliert); Maße und Regeln aus diesem Prompt | keine SVG vorhanden |
| Symmetrie | punktsymmetrisch um die mittlere Brücke; Westarm nach Süden entfällt, Ostarm nach Süden kommt dazu | beide Hälften sind im Bild ähnlich groß und haben je eine Grabenlinie. Vom Prompt vorgeschlagen, nicht vom Nutzer entschieden |
| Spieler | 1 gegen 1 und 2 gegen 2, Nord gegen Süd | wie 168–170; vom Prompt vorgeschlagen |
| Größe | Stadt etwa 1,2 × 1,1 km, Karte 2,4 × 2,4 km | Häuser im Bild zu klein; Starts außer Reichweite |
| Jahreszeit | später Winter / Vorfrühling ohne Schnee | kahle Bäume, braune Wiesen im Bild |

## Ergebnis

(von der Session auszufüllen)
