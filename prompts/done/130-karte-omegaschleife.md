# Prompt 130 – Skizzenkarte: Omegaschleife (eigener Entwurf)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` (Ziel, Entscheidungen, Ergebnis),
> in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz (besonders
> `[[river]]`, `[[crossing]]` mit `stone`, `[[lake]]`, `[[wood]]`, `[[scatter]]`,
> `[[field]]`, `[[village]]`, `[[farm]]`, `[[building]]`, `[[rail]]`) und
> „Bridges“, die Karten `river_fork` und `river_town` als Muster für Flüsse mit
> vielen Brücken (`data/map_sketches/river_town.toml`), `crates/cli/src/gen_sketch.rs`,
> `data/scale.toml` (`[range]`), `docs/AI.md`. Ist Prompt 115 (lange Brücken)
> erledigt, auch dessen Ergebnis.
>
> **Vorlage** (nicht im Repo, nur lesen):
> `docs/.vorlagen/omegaschleife/omegaschleife_entwurf.svg`, ein eigener Entwurf aus
> der Unterhaltung, vom Nutzer nach zwei Runden so angenommen. Maßstab: das
> Kartenquadrat (x 60–620, y 20–580) ist 2 km, 1 px = 3,57 m, Mitte (340, 300) =
> Kartenmitte; x nach Osten, y nach Süden. Die Osthälfte ist das Spiegelbild der
> Westhälfte.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 Bilder aus dem Spiel neben
> dem Entwurf zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine ländliche Gefechtskarte `omega_bend` (im Spiel „Omegaschleife“ / „Omega
bend“), 2 km × 2 km, **spiegelsymmetrisch** an der Nord-Süd-Achse (West gegen Ost),
als Skizze für `just gen-sketch`. Herkunft: Der Nutzer brachte die Minimap einer
Stadtkarte aus einem kommerziellen Spiel als Idee; die wird **nicht** nachgebaut.
Der Entwurf ist eigen und wurde mit dem Nutzer geändert: **wenige Häuser, viel
Wiese und Wald, alles geschwungen**, kein Wahrzeichen in der Mitte.

## 2. Aufbau nach dem Entwurf

Richtwerte in Metern (x Osten, y Süden, Mitte 0/0); die Osthälfte ist das
Spiegelbild (x, y) → (−x, y). Die Session übernimmt die Werte und glättet sie nach
den Stufenbildern.

- **Fluss mit Omega-Schleife:** Mittellinie vom Westrand (−1000, −179) fast gerade
  nach Osten, bei (−196, −339) in eine fast runde Schleife nach Norden (Mittelpunkt
  (0, −594), Radius etwa 320 m, oberster Punkt etwa 30 m vom Nordrand), zurück bei
  (196, −339) und gespiegelt zum Ostrand (1000, −179). Breite etwa 85 m, Uferstreifen
  beiderseits.
- **Halbinsel in der Schleife:** über einen **Hals** von knapp 400 m (zwischen den
  Flussfüßen) mit dem Süden verbunden. Darauf ein kleines Dorf mit Kirche um
  (0, −593) (vier bis fünf Häuser, Kirche bei (0, −714)), ein Wäldchen bei
  (−100, −779) und gespiegelt, Wiese. Ein Weg vom Hals (0, −307) nach Norden durch
  das Dorf, ein Querweg zu den beiden Schleifenbrücken.
- **Vier Steinbrücken** (`[[crossing]]` `stone`):
  - Nord-Süd über den Fluss bei x −711 (Fluss bei y etwa −179) und gespiegelt bei
    x 711;
  - West-Ost über die Schleifenseiten bei y −593 (Fluss bei x etwa −321) und
    gespiegelt.
  Brücken sind achsparallel. Der Fluss ist breiter als die längste Steinbrücke
  (21 m). Ist Prompt 115 erledigt: mehrbogige Steinbrücken über die volle Breite.
  Sonst verengt der Generator das Wasser an den Brücken wie bisher, und das ist so
  in Ordnung (als Entscheidung festhalten).
- **Nordviertel** (außerhalb der Schleife, nördlich des Flusses), nur über die
  Brücken erreichbar: ein **Gehöft** bei (−804, −768) mit Feld daneben
  (−893 … −571, −950 … −793), ein Wald bei (−964 … −786, −536 … −321), ein
  Wäldchen bei (−371, −893), ein einzelnes Haus bei (−625, −357) an der Brücke; ein
  Weg von der Brücke zur Schleifenbrücke und einer zum Westrand.
- **Süden (weite Wiese):**
  - **Weiler** am südlichen Brückenkopf um (−711, 18) (fünf Häuser um einen
    kleinen Platz) und gespiegelt;
  - **Weiler** am Start bei (−750, 429) (drei Häuser) und gespiegelt;
  - **Waldstreifen** entlang der Mitte bei x −536 … −393, y 107 … 536 (Deckung beim
    Vorrücken) und gespiegelt; **Wäldchen** bei (−250, 36), (−143, 579),
    (−929, 357) und ein größerer Wald im Südwesten (−571 … −357, 786 … 964);
    gespiegelt;
  - **Felder** bei (−964 … −793, −14 … 179) und (−536 … −321, 579 … 750);
    gespiegelt; eine etwas dunklere Wiesenfläche westlich der Mitte;
  - **Teich** in der Mitte bei (0, 214), etwa 230 m × 180 m;
  - **kleiner Bahnhof** bei (0, 857) mit Vorplatz, ein Gleis zum Südrand; ein
    einzelnes Haus bei (−179, 857) und gespiegelt.
- **Wege, alle geschwungen** (Feldwege `track`, Hauptwege `road`): vom Start über
  den Weiler zur Südbrücke; vom Südbrückenkopf im Bogen zum Hals; vom Hals in
  Schlangenlinie am Teich vorbei zum Bahnhof; vom Weiler im Bogen zum Teich; vom
  zweiten Start im Bogen zum Bahnhof; ein Randweg vom Westrand zum Weiler. Kein
  gerader Boulevard.

## 3. Spiel

- **Starts** (60 m eben und frei): Team West (−857, 607) und (−821, 914), Team Ost
  (857, 607) und (821, 914). 1 gegen 1 nutzt die beiden ersten.
- **Versorgungspunkte:** Teich-Nord (0, 36) umkämpft; Halbinseldorf (0, −464);
  Schleifenbrücken West (−500, −607) / Ost (500, −607); Südbrückenköpfe (−711, 0) /
  (711, 0); Bahnhof (0, 729); Weiler (−750, 536) / (750, 536).
- **Prüfen:** `just map-check omega_bend`, `just map-reach omega_bend` (alle Starts
  und Punkte für Fahrzeuge erreichbar; die Halbinsel über den Hals und über die
  Schleifenbrücken; die Nordviertel nur über Brücken), Symmetrietest (gespiegelte
  Höhen, Gelände, Objekte gleich), Sichtfeld-Bilder vom Halbinseldorf, vom Teich
  und aus den Waldstreifen, Reichweitentabelle wie in Prompt 123 (indirekt 1/8).
- **KI-Turnier**, Ergebnis in `docs/AI.md`: Nutzt die KI die Waldstreifen, kämpft
  sie um den Hals und die Brücken? Probleme als offene Punkte; die KI nicht umbauen.

## 4. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/omega_bend.toml` nach Abschnitt 2 und 3,
erzeugt mit `just gen-sketch omega_bend`, Biom `forest`, Wetter klar, sanftes
Gelände (leichte Wellen, Flussufer etwas tiefer). Kann der Generator die fast
runde Schleife nicht als `[[river]]` aus Punkten legen (zu enge Kurve), die
Punktdichte erhöhen oder die Schleife als See-Polygon mit `[[bay]]`-artiger
Verbindung bauen; die einfachste Lösung wählen und festhalten. Bestehende
Skizzenkarten müssen Bit für Bit gleich bleiben.

**Stufe 2 – Prüfen und Abschluss:** Abschnitt 3, Messung von Bildrate und Wegsuche
(`docs/PERFORMANCE.md`), Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md`, Test wie bei den anderen Skizzenkarten (zweimal
erzeugt, gleiche Dateien). `just check`, `just selftest`. Der Golden Hash ändert
sich nicht (neue Karte); wenn doch, Ursache finden.

## 5. Nicht Teil dieses Prompts

- Ein Wahrzeichen in der Mitte (der Turm aus dem ersten Entwurf wurde verworfen).
- Winter- oder Herbstvariante (über `base` in der Skizze leicht nachzuholen).
- Boote auf dem Fluss (Prompt 119).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 24.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Spiegelung West gegen Ost | Neue Skizzen-Option `mirror = "west"`: Achse ist die Wegzellen-Mittellinie x = 1 m, die Westhälfte wird erzeugt und gespiegelt (Bild (2 − x, y), Kurs 360° − h); Objekte genau auf der Achse mit Kurs 0°/180° (Kirche, Bahnhofshalle) bleiben einmal stehen. Testhilfe `check_symmetry` mit `Sym::West`. **[vorläufig]** | Der Generator kannte nur Nord/Süd (`north`) und Punkt (`point`); Kirche und Bahnhof stehen im Entwurf auf der Achse. Opt-in: alle anderen Skizzenkarten bitgleich (geprüft). |
| 2 | Omega-Schleife | Als ein `[[river]]` aus Punkten (Kreis R 320 m um (0, −594), 24 Punkte je Hälfte), kein See/`[[bay]]`. Der Knick am Schleifenfuß des Entwurfs ist durch einen Bogen mit 112 m Radius ersetzt, der die Gerade (y −179) und den Schleifenkreis tangiert; Hals 392 m zwischen den Mittellinien (307 m Land). Die Geraden sind gerade statt leicht gebogen. **[vorläufig]** | Einfachste Lösung; der Generator misst den Abstand zur Polylinie, enge Kurven sind kein Problem. Der Knick hätte innen eine Spitze ergeben. |
| 3 | Brücken | Vier `stone_long` (je fünf hohe Bögen, 100 m, Deck 6 m über dem Ufer) über die volle Flussbreite 85 m, nicht verengt: Südbrücken x −711/713 (N–S), Schleifenbrücken y −595 (W–O; −593 auf die Wegzellen-Linie gerückt). **[vorläufig]** | Prompt 115/122 erledigt: mehrbogige Steinbrücken über die volle Breite. |
| 4 | Geschwungene Wege auf der Achse | Eine Schlangenlinie über die Achse kann nicht spiegelsymmetrisch sein: Hals → Teich und Teich → Bahnhof sind je zwei gespiegelte Bögen (Linsen); der Weg der Halbinsel vom Hals durchs Dorf zur Kirche liegt gerade auf der Achse. **[vorläufig]** | Symmetrie; die Linsen führen beide Seiten gleich lang an Teich und Hals. |
| 5 | Punkte | Alle Starts, Versorgungspunkte und Marker auf Wegzellenmitten (x, y = 1 + 4k): Starts (−855, 605), (−819, 913) und Bilder (857, 605), (821, 913); Punkte auf der Achse bei x = 1. Marker `height_*` (Waldstreifenrand, Nordviertel), `battery_*` (hinter dem Waldstreifen, am Start), `bridge_*` (Brückenmitten). **[vorläufig]** | Lehre aus den symmetrischen Karten; Marker für Sicht- und Reichweitentabelle. |
| 6 | Bahnhof | Lagerhalle als Bahnhofshalle auf der Achse über dem Gleisende, zwei Stadthäuser als Flügel, Vorplatz als `[[square]]`, ein `[[rail]]`-Gleis auf der Achse zum Südrand; kein eigenes Modell. **[vorläufig]** | Kein Bahnhofsmodell im Katalog (siehe Fehlende Modelle). |
| 7 | Häuser | Halbinseldorf: Kirche auf der Achse und vier Häuser; Brückenkopf-Weiler je fünf Gebäude um einen kleinen Platz (eine Scheune); Start-Weiler je drei; Einzelhaus an der Nordbrücke und am Bahnhof; Gehöft im Nordviertel (`[[farm]]`). Alles Einzelgebäude (`[[building]]`) statt Zufallsdörfer. **[vorläufig]** | „Wenige Häuser“; feste Lage wie im Entwurf. |
| 8 | Wälder | Alle Wälder mit 9 m Baumabstand statt 6,5 m; Streuung einzelner Bäume über der Südwest-Wiese. **[vorläufig]** | Bei 6,5 m (383 Bäume in einem Chunk) nur 21,5 FPS nah auf „hoch“; mit 9 m 33,8 (mittel 70). |
| 9 | Gelände, Wiese, Felder | Breite Anhebung 5 m (Land > 0 m), flache Wellen 2–4 m abseits des Flusses, 3 m Anhöhe unter dem Halbinseldorf; Flussufer 3 m tiefer (Standard). Dunklere Wiese als `tall_grass` in Flecken; alle drei Felder Getreide (Entwurfsfarbe). Teich tief, 230 × 180 m. **[vorläufig]** | „Sanftes Gelände“; Hügel am Fluss würden das Wasser heben. Grüne Felder wirkten aus der Höhe wie dunkle Flecken (Stufe-1-Bild). |

### Ergebnis

- **Karte** `data/maps/omega_bend/` aus `data/map_sketches/omega_bend.toml`
  (2,0 × 2,0 km, Biom `forest` ohne Look, klar, spiegelsymmetrisch West gegen
  Ost um x = 1 m), im Menü „Omegaschleife“ / „Omega bend“: Fluss (85 m) mit
  fast runder Schleife (R 320 m) und Hals (307 m Land), Halbinseldorf mit
  Kirche, vier lange Steinbrücken (je fünf Bögen, 100 m), Nordviertel mit
  Gehöft, Feld und Wäldern nur über Brücken, im Süden Weiler, Waldstreifen in
  dunklerer Wiese, Wäldchen, Felder, Teich, kleiner Bahnhof; alle Wege
  geschwungen.
- **Generator**: `mirror = "west"` (Spiegelung West gegen Ost, Objekte auf der
  Achse mit Kurs 0°/180° bleiben einmal; in `docs/MAPS.md`); Testhilfe
  `check_symmetry` mit `Sym`; alle anderen Skizzenkarten bitgleich (geprüft).
  `just omega-bend-shots` (Entwurf über Quick Look gerendert, weil der
  ImageMagick-SVG-Renderer Linien weglässt).
- **Prüfungen**: `map-check` ok; `map-reach`: alle Starts, Punkte, Marker im
  Hauptteil (96,3 % Fahrzeuge), Weglängen beider Seiten gleich; Tests
  `omega_bend_is_mirror_symmetric`, `omega_bend_north_only_over_bridges`
  (ohne Brücken kein Weg ins Nordviertel, Halbinsel über den Hals; ohne
  Südbrücken Halbinsel vom Nordviertel über die Schleifenbrücken), Karte im
  Test „zweimal erzeugt“; `stuck-audit` 365 von 368 angekommen (zwei Panzer IV
  neben der Nordrampe der Westbrücke, wie auf `archipelago`); Golden Hash
  unverändert.
- **Sicht und Reichweite**: Teich-Nord 18,3 ha, Halbinseldorf 15,8 ha,
  Waldstreifenrand 13,6–14,6 ha (Sichtweite 250 m); Reichweitentabelle in
  `docs/MAPS.md` (vom Start erreicht nichts den Gegnerstart; Teich und Dorf
  in Reichweite schwerer Mörser und mittlerer Artillerie beider Seiten).
- **KI-Turnier** (gleiche KI und Fraktion beidseitig): 56 % West über 293
  entschiedene Spiele (normal 51 %, schwer 60 %; hart su 71 %, de 62 % West);
  de normal halb Zeitüberschreitungen. Im Gefechtsmodus kämpft die KI auf der
  Südwiese, in und an den Waldstreifen und am Teich, nicht am Hals, an den
  Brücken oder auf der Halbinsel; im Aufbaumodus geht sie überall hin.
- **Messungen**: path-bench Planung 4,7 ms; Bildrate hoch: Waldstreifen nah
  33,8 / Mitte 48,7 FPS, sonst ≥ 75; mittel ≥ 70.
- **Bilder**: `build/omega_bend/stage1/` (Stufe 1), `build/omega_bend/checks/`
  (Erreichbarkeit, Sicht), `build/omega_bend/final/`
  (`compare_{top,oblique,loop,pond}.png`, `details.png`, `sight_three.png`,
  `reach.png`).
- **Offen**: KI kämpft im Gefechtsmodus nicht um Hals und Brücken; harte KI
  West-lastig; Bildrate über den Waldstreifen auf „hoch“ unter 60 (Schatten);
  Panzer bleiben neben Brückenrampen hängen (Böschung); im path-bench wartet
  ein Trupp hinter einem auf der Rampe geparkten Trupp (einspurige Brücke);
  dunkle Flecken im Teich (wie in der Bucht von `pit_hill`); eigene Modelle
  für Bahnhof und Kapelle/Dorfkirche fehlen.
