# Prompt 138 – Karte nach Bildvorlage: Uhrturm (Großstadt am Flussknie, 2 gegen 2)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 134 (Großstadthäuser, `[[city_block]]`) und 137 (Uhrturm, Ratsbau, Stadtpalast,
> Bahnhofshalle, Kraftwerk, Riesenrad, Backsteinblöcke, Fallregel) sind erledigt
> (`prompts/done/134-*`, `prompts/done/137-*`). Empfohlen nach 115 (lange
> Brücken). Lies zuerst `CLAUDE.md`, dann diese Prompts und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[river]]`, `[[crossing]]`,
> `[[road]]`, `[[ring_road]]`, `[[square]]`, `[[wood]]`, `[[rail]]`,
> `[[city_block]]` und die Zusammenstellungen aus 137), „Bridges“ und die
> Punktsymmetrie von `city_large`; `river_city` als Muster
> (`data/map_sketches/river_city.toml`); in `docs/ARCHITECTURE.md` Sicht,
> Wegfindung, „Destruction“; `data/scale.toml` (`[range]`),
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`. Ist Prompt
> 136 (Karte Eisenturm) erledigt, auch dessen Ergebnis: gleiche Bauweise, dort
> gefundene Lösungen (Boulevards, Kaimauern, Blockdichte, Bildrate) übernehmen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/uhrturm/uhrturm.webp`. Genau ansehen. Ein mit KI erzeugtes
> Stadtbild, das an eine nordeuropäische Hauptstadt erinnert, aber nicht
> lagegetreu ist. Der Maßstabsbalken ist unstimmig und wird ignoriert. Die Karte
> ist eine **frei erfundene Stadt** mit neutralem Namen; keine Ortsnamen in
> Karte, Markern, Menüs.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Stadtkarte `clock_tower` (im Spiel „Uhrturm“ / „Clock tower“) für **2 gegen
2** (Team West gegen Team Ost, auch 1 gegen 1): eine Backsteinstadt an einem
Fluss, der in zwei rechten Knien durch die Karte läuft, mit fünf Brücken,
Ringstraßen, runden Parks und Wahrzeichen auf beiden Ufern. Alle Bauten sind
zerstörbar; Uhrturm und Schornstein stürzen um und sperren Straßen.

Die Vorlage zeigt:

- einen **Fluss in Z-Form**: von Westen am Südrand entlang, dann im Knie nach
  Norden durch die Mitte, dann im zweiten Knie nach Osten am Nordrand entlang;
  Kaimauern, Uferstraßen, Anleger, vertäute Kähne;
- **fünf Brücken**: eine in der Mitte, je eine an den beiden Knien, je eine auf
  den Außenstrecken;
- am Westufer einen **Stadtpalast mit Vorplatz** in einem großen runden Park und
  am Kai einen **gotischen Bau mit Uhrturm**;
- am Ostufer ein **Riesenrad** am Kai, eine **Bahnhofshalle** mit Tonnendach und
  weiter nördlich ein **Kraftwerk mit Schornstein** am Wasser;
- **Backsteinblöcke** mit Innenhöfen und gebogene Häuserzeilen an Ringstraßen,
  kleine Kreisel, **runde Parks** mit Rondell (sieben davon mit Stern als Ziel);
  mehrere **Blöcke in Trümmern**;
- vier **Lager in Parkecken**.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe und Symmetrie

- **3,6 km × 2,4 km** (Seitenverhältnis der Vorlage), Biom `forest`, Wetter klar
  (bedeckt, falls es ein passendes Wetter gibt; sonst klar).
- **Keine Drehung nötig:** Der Fluss läuft schon achsparallel, alle Brücken
  liegen Nord-Süd oder West-Ost. Die schrägen Brücken an den Knien der Vorlage
  werden auf die geraden Strecken neben den Knien gelegt.
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y), für Fluss,
  Brücken, Straßennetz, Parks, Blöcke, Starts und Ziele. **Ausnahme sind die
  Wahrzeichen:** Der Westen hat Palast, Ratsbau und Uhrturm, der Osten
  Riesenrad, Bahnhof und Kraftwerk mit Schornstein. Sie stehen an gespiegelten
  Plätzen mit ähnlicher Ausdehnung (Palastpark gegen Bahnhofsgelände, Uhrturm am
  Kai gegen Riesenrad am Kai; der Schornstein bekommt als Gegenstück einen
  Kirchturm im Westen). Ob das ausgeglichen ist, zeigt das Turnier mit
  vertauschten Seiten.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0). Die Session
übernimmt die Werte und glättet sie nach den Stufenbildern. Was nur für eine
Seite steht, gibt es unter (x, y) → (−x, −y) auch für die andere.

- **Fluss** als `[[river]]`, etwa 170 m breit: Westrand (−1800, 760) →
  (−950, 800) → (−500, 800) → Knie (−150, 650) → (0, 400) → Mitte (0, 0) →
  (0, −400) → Knie (150, −650) → (500, −800) → (950, −800) → Ostrand
  (1800, −760). Die Knie weit ausgerundet. **Kaimauern** auf beiden Ufern,
  dahinter Uferstraßen mit Baumreihen; Fahrzeuge kommen nur über Brücken hinüber.
- **Zwei Landmassen:** Das **Westland** ist alles westlich der Mittelstrecke und
  nördlich der Südweststrecke, dazu der **Nordstreifen** (etwa 300 m tief)
  nördlich der Nordoststrecke bis zum Ostrand. Das **Ostland** ist das
  Spiegelbild mit dem **Südstreifen**. Jede Seite hat damit einen schmalen
  Streifen, der hinter die Flanke des Gegners führt.
- **Fünf Brücken** (`[[crossing]]`): **Mittelbrücke** West-Ost bei (0, 0);
  Nord-Süd bei (450, −800) und (950, −800), sie verbinden den Nordstreifen mit
  dem Ostland; gespiegelt (−450, 800) und (−950, 800). Mit Prompt 115 lange
  Brücken über die volle Breite, sonst verengt der Generator das Wasser an den
  Brücken wie bisher (als Entscheidung festhalten). Brücken sind sprengbar.
- **Westland, Wahrzeichen:**
  - **Palastpark**, rund, etwa 750 m Durchmesser, um (−1000, −300): Wege im
    Stern, Rasen, Baumgruppen, zwei Teiche; darin der **Stadtpalast** bei
    (−1150, −260) mit Vorplatz nach Osten;
  - **Ratsbau mit Uhrturm** am Kai der Mittelstrecke, Linie von (−330, 30) bis
    (−330, 230), Uhrturm am Nordende bei (−330, 20), davor ein Grünplatz bis
    x −600 (der „Ratsgarten“);
  - ein **Kirchturm** (vorhandene Kirche) als Gegenstück zum Schornstein bei
    (−1050, 600).
- **Ostland, Wahrzeichen:**
  - **Riesenrad** am Kai bei (260, −400), darunter ein kleiner Park bis
    (200, −150);
  - **Bahnhof**: Halle aus vier bis fünf Stücken, Längsachse Südwest–Nordost
    bzw. achsparallel (einfachste Lösung), um (520, −300), Kopfbau zur
    Mittelbrücke hin, **Gleise** (`[[rail]]`) aus der offenen Stirnseite nach
    Osten bis zum Kartenrand, mit Bahnübergängen an den Boulevards und einem
    Streifen Bahngelände (Schuppen, Lagerplatz);
  - **Kraftwerk mit Schornstein** am Südufer der Nordoststrecke um
    (1050, −620), mit Kohlenhof und Anleger.
- **Runde Parks** (Rondell mit Brunnen oder Pavillon, Ringweg, Bäume; je etwa
  250–300 m Durchmesser): bei (−430, −850) und (430, 850); bei (820, 150) und
  (−820, −150) (der westliche liegt im Rand des Palastparks). Baumreihen an
  allen Boulevards.
- **Stadtviertel:** der Rest aus `[[city_block]]` mit der Fassadenfamilie
  **Backstein** (137), etwa 110–130 Blöcke von 70–150 m Kantenlänge, vier bis
  fünf Geschosse:
  - **Ringstraßen** um Palastpark und runde Parks, daran gebogene Häuserzeilen;
  - **Boulevards** (etwa 25–30 m breit): von jedem Start zur Mittelbrücke und zu
    den äußeren Brücken, die Uferstraßen, ein großer Bogen von Brücke zu Brücke
    durch jedes Land; kleine **Kreisel** an ihren Kreuzungen;
  - dazwischen Nebenstraßen (12–15 m) in leicht gebogenen Rastern, wie in der
    Vorlage: kaum eine Straße ist ganz gerade;
  - Nordstreifen und Südstreifen: eine Zeile Blöcke zwischen Uferstraße und
    Kartenrand, dazwischen Lagerhäuser und Anleger.
- **Trümmer:** etwa jeder zehnte Block ganz oder zur Hälfte als Ruine bzw.
  beschädigt (Schalter aus 134), punktsymmetrisch verteilt, in Gruppen von zwei
  bis drei wie in der Vorlage (Bombenschneisen).
- **Parkecken:** alle vier Ecken sind baumbestandene Parks (etwa 300 m × 250 m,
  ohne Häuser); in zweien liegen Starts, zwei sind Ziele.

## 4. Spiel

- **Starts** (60 m eben und frei): Team West (−1550, −950) in der Nordwestecke
  und (−1550, 250) am Westrand (kleiner Park); Team Ost gespiegelt (1550, 950)
  und (1550, −250). **Abweichung von der Vorlage:** Dort liegen Lager in allen
  vier Ecken. Die Ecken Nordost und Südwest liegen aber auf dem Nord- bzw.
  Südstreifen, nur 300 m vom gegnerischen Ufer; ein Start dort wäre vom ersten
  Tick an unter Feuer. Sie werden Ziele. Spielerzuordnung wie bei den anderen
  Skizzenkarten (Team 0 West: Spieler 0, 2; Team 1 Ost: 1, 3). 1 gegen 1 nutzt
  den ersten Start jedes Teams.
- **Versorgungspunkte** (elf): Mittelbrücke (0, 0), umkämpft; Ratsgarten
  (−530, 150) und Bahnhofsplatz (530, −150); Palastpark (−820, −285) und Park
  Ost (820, 285); Rundparks (−430, −850) und (430, 850); Kraftwerk (1030, −650)
  und Kirchplatz (−1030, 650); Parkecken Nordost (1500, −1000) und Südwest
  (−1500, 1000).
- **Die Streifen:** Der Nordstreifen gehört zum Westland und endet in der
  Parkecke Nordost, zwei Brücken führen von ihm ins Ostland nahe dem zweiten
  Start des Ostens. Wer den Streifen hält, bedroht die Flanke; wer ihn verliert,
  hat den Gegner im Rücken. Gespiegelt für den Süden.
- **Stürzende Türme:** Uhrturm (am Kai neben der Mittelbrücke) und Schornstein
  (am Wasser) können mit ihrem Trümmerband Uferstraße oder Brückenzufahrt
  sperren. Die Wege so legen, dass danach kein Start und kein Ziel abgeschnitten
  ist (Test mit Fall in jede Richtung).
- **Reichweiten** (indirekt 1/8): Tabelle in `docs/MAPS.md` wie in Prompt 123.
  Ziel: aus den Startparks die eigenen Brückenköpfe, nicht die gegnerischen
  Starts; vom Nordstreifen aus das gegnerische Kraftwerk und der Bahnhof.

## 5. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/clock_tower.toml` nach Abschnitt 2 bis
4, erzeugt mit `just gen-sketch clock_tower`. Zuerst Fluss, Brücken, Parks,
Boulevards und Wahrzeichen; dann die Blöcke. Bestehende Skizzenkarten müssen Bit
für Bit gleich bleiben.

**Stufe 2 – Prüfen:**

- `just map-check clock_tower`, `just map-reach clock_tower`: alle Starts und
  Ziele für Fahrzeuge erreichbar, keine zugebauten Straßen, keine Blöcke im
  Wasser oder auf Straßen; die Streifen vom eigenen Land aus ohne Brücke
  erreichbar.
- **Symmetrietest:** unter Punktspiegelung gleiche Höhen, Gelände, Wege und
  Objekte; die Wahrzeichen sind als Ausnahme benannt.
- **Sichtprüfung:** Sichtfeld-Bilder von der Mittelbrücke, vom Ratsgarten, vom
  Bahnhofsplatz, aus dem Palastpark, vom Nordstreifen über den Fluss und aus
  einer Nebenstraße.
- **Zerstörungsprobe:** Lauf mit schwerem Beschuss eines Viertels, des Uhrturms
  und des Schornsteins; Wege und Sicht stimmen danach, die Wegsuche bleibt im
  Zeitbudget.
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`), auch über
  dem dichtesten Viertel. Reicht das Budget nicht: Blöcke vergrößern,
  Randviertel durch Parks und Bahngelände ersetzen oder die Karte auf
  3,0 km × 2,0 km verkleinern; als Entscheidung festhalten.
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, **auch mit vertauschten Seiten**,
  Ergebnis in `docs/AI.md`: Kommt die KI durch die gebogenen Straßen und über die
  Brücken, nutzt oder sichert sie die Streifen, ist eine Seite durch ihre
  Wahrzeichen im Vorteil (Ziel: höchstens 60 % für eine Seite)? Probleme als
  offene Punkte; die KI nicht umbauen.

**Stufe 3 – Abschluss:** Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md` mit der Reichweitentabelle, Test wie bei den
anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
`just selftest`. Der Golden Hash ändert sich nicht (neue Karte; die Regeln kamen
mit 134 und 137); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Kähne, Anlegerkräne, Laternen, Doppeldeckerbusse, Züge (Kulisse, offene
  Punkte).
- Regen und Nebel als Wetter, Nacht, Verdunkelung.
- Eine Mission oder ein geschichtliches Szenario; die Karte ist freies Gefecht.
- Eine lagegetreue Karte nach Geodaten (Muster Prompt 127), Boote (Prompt 119).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
