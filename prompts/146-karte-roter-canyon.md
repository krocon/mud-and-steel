# Prompt 146 – Karte nach Bildvorlage: Roter Canyon (Plateaus, Schluchten, vier Ebenen, zwölf Spieler)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 126 (Tafelberge: `[[mesa]]` mit Abbruchkanten und Rampen), 115 (lange Brücken:
> Stahlbogen um 140 m und 240 m) und 143 (sehr große Karten, zwölf Spieler;
> seine Empfehlung für Größe und Einheitengrenze gilt hier) sind erledigt
> (`prompts/done/…`). Empfohlen nach 132 oder 133 (Auffahrten entlang einer
> Weglinie, Fahrprobe als Test) und 129 (gestufte Höhen). Lies zuerst
> `CLAUDE.md`, dann diese Prompts und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[mesa]]`, `[[hill]]`,
> `[[river]]`, `[[crossing]]`, `[[shoal]]`, `[[road]]`, `[[scatter]]`),
> „Bridges“ (eine Brücke ist offen, was auch immer darunter liegt) und die neue
> Größengrenze; in `docs/ARCHITECTURE.md` **Sicht** („Seeing downhill“: weiter
> sehen nach unten, Sichtlinie über Kanten), indirektes Feuer, **Wegfindung an
> Hängen** (`max_grade_pct`, Steigungsfaktor) und Brücken; `data/terrain.toml`,
> `data/vision.toml`, `data/scale.toml` (`[range]`), `docs/TEAMS.md`,
> `data/map_sketches/mesa_desert.toml`, `crates/cli/src/gen_sketch.rs`,
> `docs/AI.md`, `docs/PERFORMANCE.md`.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/canyon/canyon.webp`. Genau ansehen. Ein mit KI erzeugtes,
> frei erfundenes Bild einer Canyonlandschaft. Der Titelkasten nennt 12 Spieler
> und 4 km × 4 km, das Bild ist aber breiter als hoch; der Maßstabsbalken ist
> unstimmig und wird ignoriert. Die Karte bekommt einen neutralen Namen; der
> Name aus dem Titelkasten erscheint nicht im Spiel.
>
> **Wunsch des Nutzers, wörtlich:** „Große Plateaus, tiefe Schluchten, mehrere
> Ebenen und mindestens zwei Panzerzufahrten zu jedem Plateau. Das würde eine
> ganz eigene Spielmechanik ermöglichen.“ Abschnitt 3 und 5 sind danach gebaut.
>
> **Dieser Prompt läuft in vier Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte `red_canyon` (im Spiel „Roter Canyon“ / „Red canyon“) für **zwölf
Spieler, 6 gegen 6** (Team West gegen Team Ost; auch kleinere Besetzungen): ein
Hochland aus rotem Fels, das eine **tiefe Schlucht mit Wildfluss** von Nord nach
Süd teilt. Die Schlucht gabelt sich um einen Inselberg in der Mitte. Vier hohe
Bogenbrücken sind die einzigen schnellen Wege hinüber. Das Gelände hat **vier
Ebenen**, und der Kampf entscheidet sich an Kanten, Auffahrten und Brücken.

Die Vorlage zeigt:

- eine **Schlucht mit türkisfarbenem Wildfluss**, Stromschnellen und
  Wasserfällen, von Norden nach Süden, die sich um einen **Inselberg** teilt
  und danach wieder vereint;
- **vier Bogenbrücken** hoch über dem Fluss: eine im Norden, je eine über den
  West- und den Ostarm am Inselberg, eine im Süden;
- **Tafelberge mit senkrechten, gebänderten Felswänden** auf beiden Seiten,
  dazwischen tiefer liegende Senken, durch die die Wege laufen;
- **zwölf Lager auf Tafeln** an den Rändern: sechs westlich, sechs östlich der
  Schlucht;
- **sieben Sterne**: Inselberg, je zwei Tafeln mit Ruinen auf jeder Seite, eine
  **Mine mit Förderturm** im Westen, ein **Werk mit Tanks und Hallen** im Osten;
- geschwungene Pisten von Lager zu Lager, zu den Brücken und auf die Tafeln;
- spärlichen Bewuchs: Büsche und niedrige Bäume auf den Flächen, sonst Fels und
  Sand.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe und Look

- **6,0 km × 4,0 km**, wenn Prompt 143 das trägt; sonst die dort empfohlene
  Größe (alle Maße aus Abschnitt 4 im gleichen Verhältnis verkleinern, nicht
  unter 4,0 km × 2,664 km). Die Höhen bleiben dabei gleich.
- Biom **`desert`** mit einem **Rotfels-Look nur für diese Karte** (roter und
  ockerfarbener Fels mit waagrechten Bändern, rötlicher Sand, graugrüne Büsche,
  türkisfarbenes Wasser): über das Look-Feld je Karte, falls es das schon gibt
  (Prompt 122 bzw. 124), sonst als kleines Preset; `mesa_desert` und die anderen
  Wüstenkarten bleiben, wie sie sind. Wetter klar.
- **Nicht symmetrisch**, aber beide Seiten bekommen dasselbe Inventar (Abschnitt
  4); Ausgleich über Fahrzeittabelle und Turnier.

## 3. Die vier Ebenen

Höhen über dem Fluss; die Session darf sie nach Sicht- und Fahrproben anpassen,
die Staffelung bleibt.

| Ebene | Höhe | Was | Wer kommt hin |
|---|---|---|---|
| **Grund** | 0–10 m | Fluss mit Kiesbänken und Stromschnellen am Boden der Schlucht | über wenige Abfahrten von den Terrassen; Fluss nur an Furten zu queren |
| **Terrasse** | 55–65 m | breite Felsbänke in der Schlucht, 60–150 m tief, auf beiden Seiten, nicht durchgehend | über Abfahrten von der Hochebene |
| **Hochebene** | 110 m | die Senken zwischen den Tafeln: hier laufen die Pisten, hier liegen Mine und Werk, hier setzen die Brücken an | überall verbunden |
| **Tafel** | 145–160 m | die Tafelberge: zwölf Lagertafeln, vier Ruinentafeln, die Kuppe des Inselbergs | nur über Auffahrten |

- **Schlucht:** Rand zu Rand 250–450 m, an den Brücken auf die Spannweite der
  Bogenbrücke aus 115 verengt (höchstens etwa 240 m; die genaue Länge aus 115
  ablesen). Wände senkrecht und für alle gesperrt, außer an Abfahrten.
- **Felswände** an Tafeln und Schlucht entstehen aus der Geländehöhe (Steigung
  über `max_grade_pct` sperrt von selbst) plus Felsoberfläche. Das Raster hat
  2 m: eine Wand von 40 m Höhe braucht mindestens zwei bis drei Zellen Breite,
  damit die Darstellung nicht verzerrt. Prüfen, wie 126 das gelöst hat, und es
  auf 110 m Wandhöhe übertragen (gestufte Wand mit schmalen Bändern wie in der
  Vorlage).
- **Generator:** `[[mesa]]` trägt die Tafeln. Für Schlucht und Terrassen eine
  neue Tabelle **`[[canyon]]`** (Mittellinie, Randbreite, Tiefe, Terrassen als
  Abschnitte mit Seite, Breite und Höhe, Engstellen) oder `[[mesa]]` mit
  negativer Höhe, was einfacher ist; in `docs/MAPS.md` beschreiben. Bestehende
  Skizzenkarten bleiben Bit für Bit gleich.

## 4. Aufbau

Richtwerte in Metern für 6 km × 4 km (x nach Osten, y nach Süden, Mitte 0/0),
aus der Vorlage abgegriffen. Die Session übernimmt die Werte und glättet sie
nach den Stufenbildern.

- **Schlucht** (Mittellinie des Flusses): Nordrand (160, −2000) → (200, −1480)
  → (40, −1000) → Gabelung (200, −720). **Westarm:** (−440, −240) → (−560, 80)
  → Vereinigung (40, 480). **Ostarm:** (840, −320) → (840, 160) → (40, 480).
  Weiter (−240, 880) → (−320, 1120) → (−120, 1600) → Südrand (−40, 2000).
  Fluss 40–60 m breit, schnell; ein **Wasserfall** (Stufe von 10–15 m im Grund)
  an der Vereinigung, Stromschnellen an den Engstellen (Wasserfall als Bild aus
  122, falls erledigt; sonst steile Stromschnelle und offener Punkt).
- **Vier Bogenbrücken** (`[[crossing]]`, Stahlbogen aus 115, alle West-Ost, auf
  Höhe der Hochebene): **Nord** bei (40, −1440), **West** über den Westarm bei
  (−360, 80), **Ost** über den Ostarm bei (740, 180), **Süd** bei (−300, 1140).
  Die Schlucht läuft an jeder Brücke Nord-Süd und ist dort verengt. Brücken sind
  in Teilen sprengbar (115).
- **Inselberg** zwischen den Armen um (240, −300), etwa 800 m × 900 m: ein
  **Sattel auf Höhe der Hochebene** im Süden, über den die Piste von der
  Westbrücke zur Ostbrücke läuft, und darüber eine **Kuppe als Tafel** (150 m,
  etwa 500 m × 400 m um (240, −500)) mit Ruinen, über **zwei Auffahrten** (von
  Südwesten und Südosten) erreichbar. Wer den Inselberg hält, hält zwei der vier
  Brücken.
- **Ruinentafeln** (je etwa 500 m × 400 m, Ruinen aus vorhandenen Wüstenbauten
  und Mauern): West-Nord (−790, −940), West-Süd (−1170, 640), Ost-Nord
  (1580, −1130), Ost-Süd (940, 640).
- **Mine** im Westen um (−1200, −100) und **Werk** im Osten um (1820, −80), beide
  auf der Hochebene in einer weiten Senke: Mine mit Förderturm, Halde, Schuppen
  und einer Feldbahn; Werk mit Hallen, zwei Wassertürmen bzw. Tanks, Rohrbrücke.
  Bauten aus vorhandenen Objekten (Stahlhalle, Lagerhaus, Depot, Schuppen); für
  **Förderturm** und **Wasserturm** je ein einfaches Modell aus dem Baukasten
  (`env_headframe`, `env_water_tower`), zerstörbar wie ein schwaches Gebäude,
  mit Vorschaubild; gibt es den Wasserturm schon (Prompt 133), ihn verwenden.
- **Lagertafeln** (je etwa 450 m × 350 m, Rand mit Felswand, oben eben):
  - **West:** (−720, −1760), (−2000, −1460), (−2650, −800), (−2650, 260),
    (−2350, 1380), (−980, 1420);
  - **Ost:** (1100, −1780), (2200, −1780), (2700, −840), (2700, 300),
    (2320, 1400), (1000, 1400).
- **Auffahrten: mindestens zwei Panzerzufahrten zu jeder Tafel.** Das gilt für
  alle zwölf Lagertafeln, die vier Ruinentafeln und die Kuppe des Inselbergs:
  - auf **verschiedenen Seiten**, über die Tafel verbunden, sodass man
    durchfahren kann und nie eingeschlossen ist;
  - für **alle Fahrzeuge**, auch den schwersten Panzer und Zugmittel mit
    gezogenem Geschütz: Steigung und Breite aus `data/terrain.toml` und der
    Wegfindung ablesen (nicht raten), mit Reserve; Fahrbahn mindestens 8 m, in
    Kehren breiter, sodass zwei Panzer aneinander vorbeikommen;
  - bei 40–50 m Höhenunterschied als **Serpentine** oder als **Rampe in einer
    Felsrinne** (Auffahrt entlang einer Weglinie aus 132/133, falls erledigt;
    sonst hier bauen: der Weg schneidet sich in den Hang, das Gelände wird
    längs der Linie auf gleichmäßige Steigung gebracht);
  - je Lagertafel eine Auffahrt zum Hinterland bzw. Nachbarlager und eine zur
    Front.
- **Abfahrten in die Schlucht:** je Ufer **drei** von der Hochebene auf die
  Terrassen (auch für Panzer) und von dort **zwei** in den Grund; der Inselberg
  hat je eine Abfahrt in jeden Arm. Im Grund **drei Furten** durch den Fluss
  (`[[shoal]]`, für Fahrzeuge langsam, für Infanterie gangbar): nördlich der
  Gabelung, südlich der Vereinigung, eine im Ostarm. Der Grund ist an den
  Brücken-Engstellen **nicht** durchgängig (dort füllt der Fluss die Klamm von
  Wand zu Wand): Er zerfällt in Abschnitte, jeder mit eigenen Abfahrten. Das
  hält die Regel einfach, dass nichts unter einer Brücke hindurchfährt; hat 115
  die Unterführung gebracht, darf der Grund durchgehen.
- **Pisten** auf der Hochebene: je Seite eine Längspiste von Nord nach Süd durch
  die Senken, die alle sechs Lager über ihre Auffahrten anbindet; Querpisten zu
  jeder Brücke; alles geschwungen. Felsblöcke, Büsche und niedrige Bäume
  (`[[scatter]]`) als Deckung auf Hochebene und Tafeln, dichter an
  Wasserstellen.

## 5. Spielmechanik: Höhe, Kante, Engstelle

Keine neuen Regeln; die Karte holt die Wirkung aus den vorhandenen (Sicht nach
unten, Sichtlinie über Kanten, Steigung, Brücken, indirektes Feuer). Jede der
folgenden Aussagen wird in Stufe 4 mit einem Bild oder Test belegt; trägt eine
Regel sie nicht, das als offenen Punkt melden statt die Regel hier zu ändern.

- **Tafel schlägt Hochebene:** Von der Kante sieht und schießt man weiter nach
  unten als von unten herauf; wer oben steht, ist nur über zwei Auffahrten zu
  erreichen und kann sie sperren. Dafür liegt er frei, und Artillerie von einer
  Nachbartafel erreicht ihn.
- **Die Kante verdeckt:** Wer von der Kante zurücktritt, ist von unten nicht zu
  sehen (Hinterhang auf der Tafel); wer dicht unter der Wand steht, ist von oben
  nur von der Kante aus zu sehen. Feuerstellungen für Geschütze liegen
  zurückgesetzt, Beobachter an der Kante.
- **Brücken sind Lebensadern:** vier Übergänge für zwölf Spieler. Wer eine
  sprengt, schützt sich und sperrt sich aus. Bleibt keine, führt der Weg durch
  die Schlucht.
- **Die Schlucht ist der gedeckte, langsame Weg:** vom Hochland aus kaum
  einzusehen, aber von jeder Kante und jeder Brücke darüber beherrscht; Abfahrt,
  Furt und Auffahrt kosten ein Vielfaches der Brückenfahrt. Die Terrassen sind
  Zwischenstellungen für Infanterie und leichte Geschütze mit Blick auf Furten
  und Brückenfüße.
- **Der Inselberg** ist die Mitte: zwei Brücken, eine Kuppe mit Rundumsicht,
  von beiden Seiten gleich weit.
- **Versorgungspunkte** (elf): Inselberg-Kuppe (240, −510), umkämpft; die vier
  Ruinentafeln; Mine (−1200, −100) und Werk (1820, −80); Nordbrücke West- und
  Ostkopf (−120, −1440) / (200, −1440); Südbrücke West- und Ostkopf
  (−460, 1140) / (−140, 1140). Damit hat jede Seite fünf eigene und die Mitte
  einen.
- **Starts:** die zwölf Lagertafeln; gerade Spielernummern Team West, ungerade
  Team Ost, in einer Reihenfolge, bei der kleinere Besetzungen gegenüberliegende
  Lager nutzen (1 gegen 1: West (−2650, 260) gegen Ost (2700, 300), usw.);
  Zuordnung als Tabelle in `docs/MAPS.md`. Andere Teilungen (4 × 3: Nordwest,
  Südwest, Nordost, Südost) beschreiben, soweit 143 sie trägt.
- **Einheitengrenze** je Spieler aus 143 übernehmen.

## 6. Stufen

**Stufe 1 – Relief:** `data/map_sketches/red_canyon.toml` mit Schlucht,
Terrassen, Hochebene, Tafeln und Fluss, erzeugt mit `just gen-sketch red_canyon`;
Höhenbild, Schnittbilder quer zur Schlucht (die vier Ebenen sichtbar),
Rotfels-Look; Erzeugungszeit und Ladezeit nennen.

**Stufe 2 – Wege:** Brücken, alle Auffahrten und Abfahrten, Furten, Pisten.
Gleich danach die Fahrprobe (Stufe 4, erster Punkt), bevor irgendetwas
verschönert wird.

**Stufe 3 – Bauten und Bewuchs:** Lager, Ruinen, Mine, Werk, Förderturm und
Wasserturm, Felsblöcke, Büsche, Wasserfall, Starts und Ziele.

**Stufe 4 – Prüfen und Abschluss:**

- **Fahrprobe, als Test:** Für **jede** der siebzehn Tafeln und **jede** ihrer
  Auffahrten einzeln findet die Wegsuche für den schwersten Panzer und für ein
  Zugmittel mit gezogenem Geschütz einen Weg vom nächsten Lager hinauf und
  zurück; mit einer gesperrten Auffahrt bleibt die Tafel über die andere
  erreichbar. Je Tafel fährt in einem kurzen Lauf der Simulation ein Panzer
  wirklich hinauf und auf der anderen Seite hinunter. Dasselbe für jede Abfahrt
  in die Schlucht und jede Furt. Fahrzeiten in `docs/MAPS.md`.
- **Brückenprobe:** Mit allen vier Brücken gesprengt erreicht jedes Team die
  andere Seite noch durch die Schlucht; Fahrzeit über die Brücke und durch die
  Schlucht nebeneinander (Ziel: die Schlucht kostet das Drei- bis Fünffache).
- `just map-check red_canyon`, `just map-reach red_canyon`: alle Starts und
  Ziele für Fahrzeuge erreichbar; Felswände für alle zu; kein Fahrzeug kann
  über eine Kante geraten (auch nicht geschoben oder rutschend).
- **Sichtprüfung** zu Abschnitt 5: Sichtfeld-Bilder von einer Tafelkante, von
  einem zurückgesetzten Punkt auf derselben Tafel, vom Fuß der Wand, von einer
  Terrasse, aus dem Grund, von einer Brückenmitte, von der Kuppe des Inselbergs.
- **Reichweitentabelle** (indirekt 1/8) in `docs/MAPS.md`: Was erreichen
  Geschütze von Tafel zu Tafel, über die Schlucht, in den Grund? Kein Lager in
  Reichweite eines gegnerischen Lagers.
- **Ausgleich:** Fahrzeit von jedem Lager zur nächsten Brücke, zum Inselberg und
  zum nächsten Ziel; Summe je Team höchstens 15 % verschieden, sonst Lager,
  Auffahrten oder Ziele verschieben.
- **Bildrate, Tick, Wegsuche und Sicht** in jeder Ansicht mit zwölf Spielern
  (`docs/PERFORMANCE.md`); die Sicht nach unten kostet mehr Strahlenschritte:
  das Sichtbudget je Tick prüfen (Rückstau in der Warteschlange bei 1500
  Einheiten messen).
- **KI-Turnier** 6 gegen 6, 3 gegen 3 und 1 gegen 1, auch mit vertauschten
  Seiten (Ziel höchstens 60 % für eine Seite), Ergebnis in `docs/AI.md`: Findet
  die KI Auffahrten, Brücken und den Weg durch die Schlucht, besetzt sie Kanten,
  staut sie sich an Engstellen, endet es im Stillstand? Probleme als offene
  Punkte; die KI nicht umbauen.
- `just selftest`, `just lan-selftest` (zwölf Plätze), Menüname und
  Übersetzung, Eintrag in der Tabelle der Skizzenkarten in `docs/MAPS.md`, Test
  wie bei den anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien),
  Vorschaubilder der neuen Modelle, `just units-web-sync`, `just catalog-doc`.
  `just check`. Der Golden Hash ändert sich nur, wenn Förderturm und Wasserturm
  neue Einträge in den Regeltabellen brauchen; dann Ursache nennen, Hash neu
  setzen und im Commit sagen. Sonst bleibt er.

## 7. Nicht Teil dieses Prompts

- Neue Regeln für Höhe (Reichweitenbonus, Sturzschaden, Abstürzen von Kanten,
  Steinschlag), Seilbahnen, Hängebrücken für Infanterie, Klettern.
- Fahren unter Brücken, wenn 115 es nicht gebracht hat; Boote auf dem Wildfluss.
- KI-Verbesserungen für Engstellen und Höhenstellungen.
- Eine lagegetreue Karte eines echten Canyons nach Geodaten.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
