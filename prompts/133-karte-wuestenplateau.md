# Prompt 133 – Karte nach Bildvorlage: Wüstenplateau (2 gegen 2, Hochflächen, Wadis, Oasen)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 126
> (Tafelberge: `[[mesa]]` mit Abbruchkanten und Rampen, Karte `mesa_desert`) ist
> erledigt (`prompts/done/126-*`). Lies zuerst `CLAUDE.md`, dann diesen Prompt und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[mesa]]`, `[[river]]`,
> `[[crossing]]`, `[[shoal]]`, `[[lake]]`, `[[hill]]`, `[[wood]]`, `[[scatter]]`,
> `[[village]]`, `[[building]]`, `[[ring_road]]`), „Bridges“, „Objects“, die
> Wüstenkarten `desert_lagoon` und `desert_village` (Oase, Wüstenhäuser, Palmen)
> und die Punktsymmetrie von `city_large`; in `docs/ARCHITECTURE.md` Sicht
> („Seeing downhill“), indirektes Feuer und Wegfindung; `data/scale.toml`
> (`[range]`), `data/vision.toml`, `data/map_sketches/mesa_desert.toml` und
> `desert_lagoon.toml` als Muster, `crates/cli/src/gen_sketch.rs`, `docs/AI.md`,
> `docs/PERFORMANCE.md`. Sind die Prompts 132 (Mäandertal: Serpentinen-Auffahrten
> entlang einer Weglinie, Fahrprobe), 131 (4-km-Skizze), 123 (Reichweitentabelle
> und Reichweitenprüfung) oder 115 (lange Brücken) erledigt, auch deren Ergebnis;
> Bausteine und Werkzeuge wiederverwenden statt neu bauen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/wuestenplateau/wuestenplateau.webp`. Genau ansehen. Ein mit KI
> erzeugtes, frei erfundenes Kartenbild, laut Beschriftung 4,0 km × 4,0 km, 2
> gegen 2, mit vier Startlagern P1 bis P4 und einer Legende (Straße, Piste,
> Auffahrt fahrbar, Plateau, Felsen, Oase). Der Maßstabsbalken unten links passt
> nicht zur Kartengröße und wird ignoriert.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Wüstenkarte `desert_plateau` (im Spiel „Wüstenplateau“ / „Desert plateau“)
nach der Vorlage, für **2 gegen 2** (Team West gegen Team Ost, auch 1 gegen 1):
eine Sandebene mit **neun Hochflächen**, auf denen Ruinen und alte Forts stehen,
durchzogen von **zwei trockenen Flussbetten (Wadis)** mit sechs Brücken, mit
**Oasen** als einzigem Grün und Felsgebirge an den Rändern. Jede Hochfläche ist
über Auffahrten für Fahrzeuge erreichbar. Gekämpft wird um die Höhen, die
Brücken und das Wasser.

Die Vorlage zeigt:

- **vier Startlager** in den Ecken (Zelte, Baracken, Wachturm), durch eine
  **Ringpiste** verbunden;
- **zwei Wadis**, die schräg von Ostnordost nach Westsüdwest laufen und die Karte
  in drei Streifen teilen: eines von der Nordmitte zum Westrand, eines vom
  Ostrand zur Oase im Südwesten; kiesige, eingetiefte Betten mit Palmen und
  Büschen an den Rändern, **sechs kurze Steinbrücken**;
- **Hochflächen** mit senkrechten, rotbraunen Felsbändern und flacher Kuppe,
  darauf **Ruinen, Forts und ein Turm** aus Lehm und Stein; auf jede führt
  mindestens eine geschwungene Auffahrt („Auffahrt (fahrbar)“ in der Legende);
- eine **langgestreckte Hochfläche in der Mitte** zwischen den Wadis, daneben ein
  Depot mit **Wasserturm** und ein **Bohrturm**;
- **Oasen** mit Teich, Palmen und Lehmhäusern: eine große mit Dorf im Osten,
  kleinere im Nordwesten und Südwesten, Palmgruppen an den Wadis, ein Lehmdorf
  im Westen;
- **Felsgebirge** am Nord- und Südrand und in Stücken an den Seiten, einzelne
  Felsblöcke und Dünenrippen in der Ebene;
- Straßen über die Brücken, Pisten zu jedem Ziel.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung. Nichts daraus übernehmen,
alles mit den Bausteinen des Spiels nachbauen. Die Beschriftungen (P1–P4,
Legende, Titelkasten) erscheinen nicht in der Karte; der Name „Wüstenplateau“ ist
ein gewöhnliches Wort und darf bleiben.

## 2. Größe, Symmetrie und Abgrenzung

- **4 km × 4 km**, Biom `desert`, Wetter klar. Halten Bildrate oder Wegsuche die
  4 km nicht im Budget (`docs/PERFORMANCE.md`), alle Maße mit 0,8 auf 3,2 km
  verkleinern; als Entscheidung festhalten.
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y). Das trägt die
  Vorlage gut: Das zweite Wadi ist fast das Spiegelbild des ersten, die Lager
  liegen über Kreuz. Wo sie abweicht, gilt die Symmetrie. Das Licht ist nicht
  symmetrisch; das ist in Ordnung.
- **Abgrenzung zu `mesa_desert` (Prompt 126):** Jene Karte ist klein (2,4 km),
  ein Becken mit einem Ring aus Zeugenbergen, ohne Wasserläufe. Diese ist groß,
  für vier Spieler, mit Wadis als Hindernis, Oasen und Ruinen als Zielen und
  durchfahrbaren Hochflächen. `mesa_desert` bleibt Bit für Bit gleich.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0), aus der Vorlage
abgegriffen und symmetrisch gemacht. Die Session übernimmt die Werte und glättet
sie nach den Stufenbildern. Was nur für eine Seite steht, gibt es unter
(x, y) → (−x, −y) auch für die andere.

- **Wadi Nord** (Mittellinie): Quelle bei (807, −1330) → (552, −1043) → Brücke
  (297, −788) → (−214, −597) → Brücke (−597, −278) → (−1043, −86) → Brücke
  (−1490, 105) → Westrand (−2000, 424). **Wadi Süd** ist das Spiegelbild: vom
  Ostrand (2000, −424) über die Brücken (1490, −105), (597, 278), (−297, 788)
  zum Ende bei (−807, 1330). Die Wadis teilen die Karte in einen Nordstreifen,
  einen Mittelstreifen und einen Südstreifen.
- **Wadi als Gelände:** trockenes Bett, 40–60 m breit, 3–4 m eingetieft, Kies
  und Sand, an den Rändern Büsche und einzelne Palmen, **kein Wasser**. Die
  Ränder sind kurze Steilstufen: **Infanterie quert überall** (langsam, im Bett
  mit Deckung gegen flaches Feuer), **Fahrzeuge nur an den Brücken und an zwei
  Furten je Wadi**, wo die Ränder flach auslaufen (Wadi Nord bei (−214, −597)
  und (−1043, −86), gespiegelt für Wadi Süd). Der Skizzen-Generator kennt bisher
  nur Wasserläufe: den einfachsten Weg wählen (ein trockener `[[river]]` mit
  eigenem Schalter oder eine neue Tabelle `[[wadi]]`) und in `docs/MAPS.md`
  beschreiben. Kein neues Gelände in der Simulation, wenn vorhandene Geländearten
  (Kies, Sand, Fels) und die Steigungsregeln der Wegfindung reichen.
- **Sechs Steinbrücken** (`[[crossing]]` `stone`, achsparallel, das Wadi läuft an
  jeder Brücke kurz gerade) an den sechs genannten Punkten. Sie sind kurz genug
  für die vorhandene Steinbrücke, wenn das Bett dort auf deren Länge verengt
  wird. Brücken sind sprengbar; die Furten bleiben dann der Weg für Fahrzeuge.
- **Hochflächen** (`[[mesa]]`, flache Kuppe mit Felsbändern, unregelmäßiger
  Umriss; Mitte, Größe, Höhe über der Ebene, Bauwerk):
  - **Mittelplateau** (0, 0), 900 m × 400 m, längs zu den Wadis gedreht
    (Ostnordost–Westsüdwest), 35 m; keine Bauten, nur Felsblöcke; drei
    Auffahrten (West, Ost, Mitte der Nordseite);
  - **Westfort** (−1394, −500), 860 m × 540 m, 45 m; Fort mit Hof und Mauern;
  - **Nordwestplateau** (−612, −804), 800 m × 410 m, 35 m; Ruinen eines Gehöfts;
  - **Nordfort** (−214, −1362), 770 m × 480 m, 50 m; das größte Fort, mit Turm;
  - **Südwestplateau** (−1150, 900), 700 m × 450 m, 40 m; Ruine mit Mauerresten.
  Die Ostseite bekommt die Spiegelbilder (Ostfort, Südostplateau mit Turm,
  Südfort, Nordostplateau). Zusammen neun. Zwischen den Hochflächen bleiben
  Täler von mindestens 100 m mit einer Piste; keine Hochfläche näher als 120 m
  an einem Wadi.
- **Auffahrten für Fahrzeuge:** **jede Hochfläche hat mindestens zwei Auffahrten
  auf verschiedenen Seiten**, über die Kuppe verbunden, sodass man durchfahren
  kann und keine Hochfläche eine Sackgasse ist. Für alle Fahrzeuge, auch den
  schwersten Panzer und Zugmittel mit gezogenem Geschütz: Steigung und Breite aus
  den Regeln der Wegfindung ablesen (nicht raten), mit Reserve; Fahrbahn
  mindestens 8 m, in Kehren breiter. Geschwungen wie in der Vorlage (ein bis
  drei Kehren). Ist Prompt 132 erledigt, dessen Auffahrt entlang einer Weglinie
  verwenden; sonst sie hier bauen (der Weg schneidet sich in den Hang, das
  Gelände wird längs der Linie auf gleichmäßige Steigung gebracht). Abseits der
  Auffahrten sind die Felsbänder für alle gesperrt. Je eine Auffahrt zeigt zum
  eigenen Hinterland, eine zur Mitte.
- **Ruinen und Forts:** aus vorhandenen Wüstenhäusern, Mauern und Türmen
  zusammengesetzt (Hof, Ecktürme, Lücken in der Mauer als Tore), als Deckung für
  Infanterie und als Beobachtungsposten an der Kante. Eigene Ruinenmodelle sind
  ein offener Punkt.
- **Mitte:** Depot mit **Wasserturm** am Nordfuß des Mittelplateaus bei
  (40, −370) und sein Gegenstück bei (−40, 370); **Bohrtürme** bei (520, −565)
  und (−520, 565). Fehlen Modelle für Wasserturm oder Bohrturm, Stellvertreter
  aus vorhandenen Objekten (Turm, Tank, Schuppen) setzen und die Modelle als
  offenen Punkt notieren; keine Modelle in diesem Prompt bauen, die mehr als
  eine einfache Zusammenstellung sind.
- **Oasen** (Teich aus flachem Wasser wie in `desert_village`, Palmen, Büsche,
  Lehmhäuser):
  - **Oasendorf Ost** bei (1190, −437): Teich etwa 200 m × 130 m, Palmenhain,
    zehn bis zwölf Häuser; Gegenstück **Oasendorf West** bei (−1190, 437);
  - **kleine Oase** bei (−947, −1139) mit drei Häusern und Gegenstück
    (947, 1139);
  - **Quelloasen** an den Wadi-Enden bei (−807, 1330) und (807, −1330): Teich
    mit Palmen, zwei bis drei Häuser;
  - Palmgruppen entlang der Wadis, sonst kein Baumbestand.
- **Felsgebirge** (unpassierbar, 80–120 m): ein Riegel am Nordrand von x −1200
  bis 250 und sein Gegenstück am Südrand, kürzere Stücke am West- und Ostrand
  zwischen den Lagern, lose Felsgruppen in den Ecken hinter den Lagern. Die
  Ebene dazwischen leicht gewellt, mit Dünenrippen und verstreuten Felsblöcken
  (`[[scatter]]`) als Deckung.
- **Wege:** eine **Ringpiste** (`[[ring_road]]` oder Pisten) durch alle vier
  Lager; **Straßen** (`road`) über jede Brücke; eine Straße von Lager zu Lager
  quer durch die Mitte am Mittelplateau vorbei; Pisten (`track`) zu jeder
  Auffahrt, Oase und Furt. Alles geschwungen.

## 4. Spiel

- **Starts** (60 m eben und frei, mit Zelten und Baracken am Rand als Kulisse):
  Team West (−1600, −1250) im Nordstreifen und (−1580, 1250) im Mittelstreifen;
  Team Ost gespiegelt (1600, 1250) im Südstreifen und (1580, −1250) im
  Mittelstreifen. Spielerzuordnung wie bei den anderen Skizzenkarten (Team 0
  West: Spieler 0, 2; Team 1 Ost: 1, 3). 1 gegen 1 nutzt den ersten Start jedes
  Teams. Jedes Team hat damit einen Spieler hinter einem Wadi und einen im
  offenen Mittelstreifen; ob das trägt, zeigt das Turnier (sonst Starts
  verschieben und festhalten).
- **Versorgungspunkte** (elf): Mittelplateau (0, 0), umkämpft; Bohrtürme
  (520, −565) / (−520, 565); Oasendörfer (1190, −437) / (−1190, 437); Westfort
  (−1394, −500) / Ostfort (1394, 500); Nordfort (−214, −1362) / Südfort
  (214, 1362); mittlere Wadi-Brücken (−597, −278) / (597, 278).
- **Höhe gegen Ebene:** Von den Kuppen sieht man weit über den Sand (Sichtregeln
  mit Geländehöhe); die Ebene bietet nur Wadis, Oasen, Felsblöcke und
  Dünenrippen als Deckung. Wer eine Hochfläche hält, beherrscht die Brücken in
  ihrer Nähe.
- **Reichweiten** (indirekt 1/8, Sicht und direktes Feuer 1/4): mit den echten
  Katalogwerten eine Tabelle in `docs/MAPS.md` wie in Prompt 123 (Geschütz,
  Reichweite im Spiel, was es von welcher Hochfläche erreicht). Ziel: Vom
  Mittelplateau erreichen leichte Haubitzen beide mittleren Brücken und beide
  Bohrtürme; die Starts erreicht keine Hochfläche der Gegenseite. Abstände
  danach nachziehen.

## 5. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/desert_plateau.toml` nach Abschnitt 2
bis 4, erzeugt mit `just gen-sketch desert_plateau`. Bestehende Skizzenkarten
müssen Bit für Bit gleich bleiben.

**Stufe 2 – Prüfen:**

- `just map-check desert_plateau`, `just map-reach desert_plateau`: alle Starts,
  Kuppen und Versorgungspunkte für Fahrzeuge erreichbar; die Felsbänder abseits
  der Auffahrten und das Felsgebirge für alle zu; die Wadis für Fahrzeuge nur an
  Brücken und Furten, für Infanterie überall.
- **Fahrprobe auf die Hochflächen**, als Test: Für jede Hochfläche und jede ihrer
  Auffahrten einzeln findet die Wegsuche für den schwersten Panzer und für ein
  Zugmittel mit gezogenem Geschütz einen Weg vom Start auf die Kuppe und zurück;
  mit einer gesperrten Auffahrt bleibt die Kuppe über die andere erreichbar. In
  einem kurzen Lauf der Simulation je Hochfläche ein Fahrzeug hinauf- und über
  die andere Seite hinunterfahren lassen; Fahrzeiten in `docs/MAPS.md`.
- **Wadi-Probe:** Mit allen sechs Brücken gesprengt erreichen Fahrzeuge jeden
  Streifen noch über die Furten; ein Fahrzeug bleibt an den Steilrändern nicht
  hängen.
- **Symmetrietest:** unter Punktspiegelung gleiche Höhen, Gelände, Objekte,
  Weglängen.
- **Sichtprüfung:** Sichtfeld-Bilder von jeder Kuppe und aus einem Wadi.
- **Reichweitenprüfung** mit Test oder Werkzeug (aus 123, falls vorhanden).
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Findet die
  KI Auffahrten und Furten, besetzt sie Hochflächen und Oasen, sind die beiden
  Startlagen eines Teams etwa gleich stark, endet es im Stillstand? Probleme als
  offene Punkte; die KI nicht umbauen.
- Messung von Bildrate und Wegsuche in jeder Ansicht (`docs/PERFORMANCE.md`), bei
  Bedarf die Verkleinerung aus Abschnitt 2.

**Stufe 3 – Abschluss:** Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md` mit der Reichweitentabelle, Test wie bei den
anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
`just selftest`. Der Golden Hash ändert sich nicht (neue Karte, keine neue
Regel); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Eigene Modelle für Ruinen, Forts, Wasserturm, Bohrturm, Zelte (offene Punkte).
- Wasser im Wadi, Sturzfluten, Sandsturm als Wetter.
- Regeln für Wasser oder Öl als Rohstoff; die Punkte sind gewöhnliche
  Versorgungspunkte.
- KI-Verbesserungen für Auffahrten, Furten und Hochflächen.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
