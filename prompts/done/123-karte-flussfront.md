# Prompt 123 – Karte nach Bildvorlage: Flussfront, Artillerie auf beiden Seiten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` (Ziel, Entscheidungen, Ergebnis),
> in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz, „Bridges“,
> „Objects“ (`fort_*`) und den Abschnitt zur Symmetrie der großen Stadtkarte
> (`city_large`, Punktsymmetrie), `data/map_sketches/river_fields.toml` und
> `valley_fields.toml` als Muster, `crates/cli/src/gen_sketch.rs`. In
> `docs/ARCHITECTURE.md` die Abschnitte zu indirektem Feuer, Sicht („Seeing
> downhill“) und Wegfindung; `data/scale.toml` (`[range]`: indirektes Feuer ein
> Achtel der echten Weite), `data/vision.toml`, die Geschütze und Mörser in
> `data/catalog/*.toml` (`range_effective_m`), `docs/AI.md`. Sind die Prompts 115
> (lange Brücken) oder 122 (Inselreich, farbenfroher Look) erledigt, auch deren
> Ergebnis.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/flussfront/flussfront.webp`. Genau ansehen.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel neben
> der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `river_front` (im Spiel „Flussfront“ / „River front“) nach der
Vorlage, gebaut für ein **Artillerieduell über einen breiten Fluss**: Beide Seiten
haben auf ihrem Ufer Höhen und vorbereitete Feuerstellungen, von denen aus sie den
Fluss, die Inseln und die Brücken beschießen können. Wer übersetzen will, muss unter
Feuer über Brücken und Inseln.

Die Vorlage zeigt:

- einen **breiten Fluss** von West nach Ost quer durch die Mitte, mit Stromschnellen,
  Felsen im Wasser und **vier bis fünf bewaldeten Inseln** in der Mitte;
- **etwa zehn Steinbrücken**: je eine lange Brücke an beiden Enden (West und Ost),
  eine Kette von Nord nach Süd über die Mittelinseln, dazu schräge Brücken zwischen
  Ufern und Inseln; an vielen Brückenenden kleine **Brückentürme bzw. Bunker**;
- je eine **Stadt mit Platz und großem Gebäude** in der Mitte des Nord- und des
  Südufers, verbunden durch die gerade Hauptstraße über die Mittelinseln;
- je eine **Ringstraße** auf beiden Ufern, die die Brückenköpfe verbindet, mit
  Gehöften, Feldern (gold und grün) und Wäldern;
- **Felsberge** in den vier Ecken und Hügel hinter den Städten.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung. Nichts daraus übernehmen,
alles mit den Bausteinen des Spiels nachbauen.

## 2. Spielgedanke: Artillerie auf beiden Seiten

- **Streng spiegelsymmetrisch** an der Flussachse (Nord gegen Süd), damit beide Seiten
  gleiche Chancen haben: Gelände, Höhen, Straßen, Brücken, Stellungen, Starts und
  Versorgungspunkte gespiegelt (wie die Punktsymmetrie von `city_large`, hier als
  Spiegelung; die Session wählt Spiegelung oder Punktsymmetrie danach, was die
  Vorlage besser trägt, und begründet es). Das Licht ist nicht symmetrisch; das
  ist in Ordnung.
- **Reichweiten zuerst rechnen:** Im Spiel reicht indirektes Feuer ein Achtel der
  echten Weite (105-mm-Haubitze etwa 1,4 km, 15-cm-Kaliber und schwere Mörser
  weniger). Die Abstände so wählen, dass
  - leichte Haubitzen aus den Feuerstellungen den ganzen Fluss, die Inseln und die
    gegnerischen Brückenköpfe erreichen,
  - die gegnerischen Feuerstellungen nur aus vorgeschobenen Stellungen oder mit den
    weitesten Geschützen erreichbar sind (Gegenbatteriefeuer kostet Bewegung und
    Risiko),
  - Mörser nur vom Ufer aus über den Fluss wirken.
  Die Rechnung mit den echten Katalogwerten und dem Faktor als Tabelle in
  `docs/MAPS.md` festhalten (Geschütz, Reichweite im Spiel, was es von welcher
  Stellung erreicht).
- **Feuerstellungen:** je Seite drei bis vier vorbereitete Stellungen am
  Hinterhang der Hügel bzw. an den Bergen in den Ecken (`fort_sandbag_position`,
  `fort_trench_section`), auf ebenen Flächen, für gezogene Geschütze erreichbar,
  vom Gegenufer aus nicht direkt einsehbar (Sichtfeld aus der Simulation prüfen).
- **Beobachtung:** Höhen am Ufer und die Brückentürme sind Beobachtungspunkte mit
  Sicht über den Fluss; Versorgungspunkte dort, damit um Sicht gekämpft wird.
- **Brückenköpfe:** Bunker (`fort_bunker_mg`) und Gräben an den Brückenenden
  beider Seiten, gespiegelt. Brücken sind sprengbar (Pioniere, Prompt 020/055).
- **Zweiter Weg:** eine bis zwei Furten bzw. Untiefen (`[[shoal]]`) an den
  Stromschnellen, damit gesprengte Brücken das Spiel nicht beenden.
- **Starts:** 1 gegen 1 und 2 gegen 2: Team Nord und Team Süd, je zwei Starts hinter
  der Ringstraße zwischen Stadt und Eckbergen, 60 m eben und frei, außerhalb der
  Reichweite leichter gegnerischer Artillerie aus deren Feuerstellungen.

## 3. Skizze (Stufe 1)

`data/map_sketches/river_front.toml`, erzeugt mit `just gen-sketch river_front`:

- **Größe** 3,6 km × 2,0 km (Seitenverhältnis der Vorlage; Höhe nach der
  Reichweitenrechnung anpassen, höchstens 4 km), Biom `forest`, Wetter klar.
- **Fluss** als `[[river]]` bzw. `[[lake]]`-Polygon in der Breite der Vorlage
  (mehrere hundert Meter mit Inseln), Inseln als `[[island]]` oder Land-Polygone,
  Stromschnellen und Felsen im Wasser.
- **Brücken:** Ist Prompt 115 erledigt, mehrbogige Steinbrücken in voller Länge (60–
  150 m, Teile aneinandergereiht); sonst Dämme (`[[causeway]]`) mit kurzen
  Steinbrücken wie `island_chain`, und die langen Brücken als offener Punkt.
  Achsen nur Nord-Süd oder West-Ost (Brücken sind achsparallel); die schrägen
  Brücken der Vorlage werden zu achsparallelen mit kurzem Knick der Straße.
- **Brückentürme:** vorhandene Steinhäuser bzw. Bunker an den Brückenenden als
  Stellvertreter; ein eigenes Torhaus-Modell als offener Punkt.
- **Städte:** je ein `[[town]]` mit `[[square]]` und großem öffentlichem Gebäude
  (Rathaus, Kirche), gespiegelt.
- **Ringstraßen, Feldwege, Gehöfte** (`[[farm]]`), **Felder** (`[[patchwork]]`,
  gold und grün), **Wälder** (Nadel- und Mischwald), **Felsberge** in den Ecken
  (`[[hill]]` mit `[[wood]]` `rock`, `env_rock_massif`), Hügel hinter den Städten
  für die Feuerstellungen. Die Berge brauchen mehr Höhe als bisherige Skizzen (50–
  100 m); den Generator erweitern, falls nötig.

## 4. Prüfen (Stufe 2)

- `just map-check river_front`, `just map-reach river_front`: alle Starts,
  Feuerstellungen und Versorgungspunkte für Fahrzeuge mit gezogenem Geschütz
  erreichbar; Weglängen beider Seiten gleich (Symmetrie im Test prüfen: gespiegelte
  Höhen, Gelände und Objekte gleich).
- **Sichtprüfung:** Bild des Sichtfelds von jeder Feuerstellung und jedem
  Beobachtungspunkt; die Feuerstellungen sehen das Gegenufer nicht direkt.
- **Reichweitenprüfung:** Test oder Werkzeug, das für jede Stellung zeigt, welche
  Versorgungspunkte, Brücken und gegnerischen Stellungen in Reichweite welcher
  Geschütze liegen.
- **KI-Turnier** auf der Karte, Ergebnis in `docs/AI.md`: Nutzt die KI Artillerie
  aus den Stellungen, greift sie über die Brücken an, endet es im Stillstand?
  Probleme beschreiben und als offenen Punkt notieren; die KI nicht umbauen.
- Messung der Bildrate in jeder Ansicht, `docs/PERFORMANCE.md`.

## 5. Darstellung und Abschluss (Stufe 3)

- Ist der farbenfrohe Look aus Prompt 122 erledigt und vom Nutzer angenommen,
  diese Karte ebenfalls damit; sonst `forest` wie die anderen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md` mit der Reichweitentabelle, Test wie bei den anderen Skizzenkarten
  (zweimal erzeugt, gleiche Dateien).
- `just check`, `just selftest`. Der Golden Hash ändert sich nicht (neue Karte);
  wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Eigene Modelle für Brückentürme und Torhäuser.
- Vorgegebene Startarmeen mit Artillerie für diese Karte (das wäre eine Mission
  oder ein eigenes Szenario).
- KI-Verbesserungen für Gegenbatteriefeuer und Flussübergänge.
- Boote (Prompt 119) über den Fluss.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 23.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Symmetrie | **Spiegelung** Nord gegen Süd, nicht Punktsymmetrie: neues Skizzenfeld `mirror = "north"`; der Generator erzeugt die Nordhälfte und kopiert Höhen, Gelände, Brücken und Objekte gespiegelt nach Süden. Achse ist die Wegzellen-Mittellinie y = 1 m (bildet Wegzellen, Höhenpunkte und West-Ost-Brückenachsen auf sich ab). **[vorläufig]** | Die Vorlage ist an der Flussachse gespiegelt (Stadt gegen Stadt auf einer geraden Hauptstraße); Spiegelung trägt das besser als Punktsymmetrie. Das 8-m-Sichtraster liegt dadurch 2 m neben der Achse (keine Achse passt für beide Raster); Sicht ist bis auf eine Zelle gleich. |
| 2 | Größe | 3,6 × 2,0 km wie vorgeschlagen. **[vorläufig]** | Die Reichweitenrechnung passt in 2 km: Stellungen bei \|y\| = 805–822 m liegen sich 1 612–1 642 m gegenüber, zwischen leFH 18 (1 541 m) und Hummel (1 666 m); Starts ≥ 1 649 m von den gegnerischen Stellungen. |
| 3 | Flussbreite | 290–400 m von Ufer zu Ufer (mit Inseln), an den Endbrücken 130 m, an den Furten 100 m – schmaler als die Vorlage (~750 m). **[vorläufig]** | Sicht ist 250 m (Aufklärer 350 m): so sehen die Beobachtungshügel die Inseln, die Inseln das Gegenufer, mittlere Mörser wirken vom Ufer über den Fluss. |
| 4 | Fluss im Generator | `[[lake]] kind = "river"`: eine Wasserhöhe unter dem ebenen Grund, Hügel fallen als Steilufer zum Wasser; Inseln als Polygone (`[[island]] polygon_m`), Untiefen (`[[shoal]]`) auch im Fluss. **[vorläufig]** | `[[river]]` hat eine feste Breite; Hügel am Ufer hätten das Wasser angehoben. Alte Skizzenkarten bleiben Bit für Bit gleich. |
| 5 | Brücken | Prompt 115 ist erledigt: zehn lange Steinbrücken (`stone_long`, Ketten aus `ArchHigh`, 100–160 m, 60 Teile): je eine an den Enden, die Hauptstraße über die Mittelinsel (Nord, Süd), Ufer–West-/Ostinsel (4), West-/Ostinsel–Mittelinsel auf der Achse (2). Die schrägen Brücken der Vorlage wurden achsparallel. **[vorläufig]** | Achsparallele Brücken; jede Seite hat vier Wege über den Fluss plus zwei Furten. |
| 6 | Brückentürme | Steinhäuser (`env_house_stone`) an den Uferenden als Stellvertreter, dazu MG-Bunker und Gräben (`[[works]] bridgehead`). **[vorläufig]** | Eigenes Torhaus-Modell ist nicht Teil des Prompts (offen). |
| 7 | Feuerstellungen | Je Seite vier (`battery_n1`–`n4`, `battery_s1`–`s4`): zwei hinter den Hügeln hinter der Stadt (x = ±370 m), zwei hinter Hügeln vor den Eckbergen (x = ±1 130 m); 35 m eben (`flat_m`), neues `[[works]] battery` (vier Sandsack-Geschützstände, zwei Grabenstücke), Weg von Straße und Start. **[vorläufig]** | Hinterhang, eben, für gezogene Geschütze erreichbar; Sichtprüfung mit dem Sichtfeld der Simulation: sehen nichts vom Gegenufer, werden von niemandem dort gesehen. |
| 8 | Beobachtung und Versorgung | Vier Beobachtungshügel (22 m) an den Ufern bei den Inselbrücken mit Versorgungspunkten, dazu Mittelinsel, West- und Ostinsel (auf der Achse) und die beiden Stadtplätze: neun Versorgungspunkte. **[vorläufig]** | Um Sicht wird gekämpft; die Mitte ist neutral. |
| 9 | Starts | (±720, −790) Nord, (±720, 792) Süd, hinter der Ringstraße zwischen Stadt und Eckbergen; 1 gegen 1 = Spieler 0 (Nordwest) gegen 1 (Südwest). **[vorläufig]** | 60 m eben und frei, außerhalb der leichten Haubitzen der Gegenstellungen (≥ 1 649 m), nur die Hummel reicht. |
| 10 | Look | `forest` ohne `vivid`. **[vorläufig]** | Der farbenfrohe Look aus 122 ist erledigt, aber vom Nutzer noch nicht angenommen. |
| 11 | Berge | Eckberge 100–110 m (`peak`, r = 450 m), zwei 50–55-m-Gipfel am West- und Ostrand; keine Generator-Erweiterung nötig (122 trägt > 60 m). **[vorläufig]** | Prompt verlangt 50–100 m; etwas höher, weil sie in der Schrägansicht sonst kaum wirken. |
| 12 | Symmetrietest | Höhen, Gelände, Objekte, Starts, Marker und offene Wegzellen exakt gespiegelt; Wege beider Seiten über die **Wegkosten** (±2 %) statt die Länge. **[vorläufig]** | Die Wegsuche wählt zwischen fast gleich teuren Routen je Richtung verschieden (erster Entwurf: 784 gegen 624 m bei 0,4 % Kostenunterschied). In der fertigen Karte sind auch alle Längen von `map-reach` gleich. |
| 13 | Reichweitenprüfung | `just map-sight` erweitert (Marker `battery_*`): Sicht jeder Stellung auf die andere Seite, wer sie sieht, und je Stellung/Versorgungspunkt Entfernung und Zahl der indirekten Waffen zu gegnerischen Starts, Punkten, Stellungen, Brücken und Furten; Test `river_front_ranges_and_sight`. **[vorläufig]** | Werkzeug von 113 lag nahe; die Tabelle in `docs/MAPS.md` stammt daraus. |

### Ergebnis

- **Skizze** `data/map_sketches/river_front.toml`, Karte `data/maps/river_front/`
  (3,6 × 2,0 km, 36 174 Objekte, 60 Brückenteile, 308 Gebäude), im Menü
  „Flussfront“ / „River front“, in der Kartenauswahl mit Vorschau (201).
- **Generator** (`gen_sketch.rs`): `mirror = "north"`, `[[lake]] kind =
  "river"`, Polygon-Inseln, Untiefen im Fluss, `flat_m` an Markern,
  `[[works]]` (`battery`, `bridgehead`); alle anderen Skizzenkarten Bit für
  Bit gleich (geprüft). Tests `river_front_is_mirror_symmetric`,
  `river_front_ranges_and_sight`; die Karte im Test „zweimal erzeugt“.
- **Prüfungen**: `map-check` ok; `map-reach`: alle Starts, Stellungen,
  Punkte, Brücken, Furten für Fahrzeuge erreichbar, Weglängen beider Seiten
  gleich; `stuck-audit`: 348 von 352 angekommen, einer gab auf (andere
  Einheit im Weg); Reichweitentabelle in `docs/MAPS.md`; KI-Turnier in
  `docs/AI.md`; Bildrate in `docs/PERFORMANCE.md` (hoch: Wald 33, Mitte 37,
  Übersicht 61 FPS; mittel 66–120). Golden Hash unverändert (neue Karte).
- **Bilder**: `build/river_front/final/compare_{top,oblique}.png` (Vorlage
  neben Spiel) und Einzelbilder, Zwischenstände `build/river_front/stage1/`,
  `stage1b/`, `build/river_front/{reach,sight}.png`,
  `build/river_front/picker/picker_river_front*.png`.
- **Offen**: Die KI nutzt die Feuerstellungen nicht und schießt kein
  Gegenbatteriefeuer; im Basismodus Stillstand (alle vier Spiele 30 min ohne
  Sieger, jede Seite bleibt am eigenen Ufer); gleiche KIs gewinnen je nach
  Fraktion auf einer Seite öfter (z. B. us/us 31 : 9 für Nord) trotz exakt
  gespiegelter Karte; Sichtraster 2 m neben der Spiegelachse; eigenes
  Torhaus-Modell; Bildrate im dichten Wald auf „hoch“ unter 60 FPS; das
  Gegenufer ist von den Beobachtungshügeln nicht zu sehen (460 m > Sicht),
  nur von den Inseln; Nutzer entscheidet über `vivid`.

### Überarbeitung (24.11.2026, auf Wunsch des Nutzers)

- **Fluss** näher an der Vorlage: an beiden Enden zwei Arme (je 140 m) um
  einen Landkopf, dazwischen 660 m breit, in der Mitte 770 m; Mittelinsel,
  West- und Ostinsel auf der Achse, je eine Uferinsel vor jeder Stadt an
  der Hauptstraße, vier kleinere Waldinseln, Felsinselchen. Entscheidung 3
  (schmaler Fluss) ist damit aufgehoben: Die Beobachtungshügel sehen nur
  noch den nahen Arm, schwere Mörser reichen von dort bis zu den
  Achseninseln, nicht mehr von Hügel zu Hügel (Test angepasst).
- **Brücken**: 44 statt 10 (260 Teile), keine länger als 160 m; wo das Wasser breiter
  ist, trägt eine Mini-Insel die Straße von Brücke zu Brücke (neues
  Skizzenfeld `reach_m` an `[[crossing]]`). Vier Furten.
- **Gebirge**: in jeder Ecke eine Bergkette mit drei Gipfeln (55–95 m),
  für Fahrzeuge befahrbar, zwei Wege zu jedem Hauptgipfel (`peak_*`).
- **Tafelberge** an den westlichen Armen: je Ufer zwei (20 m, Felswände,
  Rampe von der Ringstraße), dazwischen der Weg zur Brücke über den Arm. Keine
  Hochbrücke: Ein Deck über Land sperrt den Weg darunter.
- Nicht neu gemessen: KI-Turnier (`docs/AI.md`) und Bildrate
  (`docs/PERFORMANCE.md`) stammen von der ersten Fassung.
