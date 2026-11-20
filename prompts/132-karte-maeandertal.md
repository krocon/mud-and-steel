# Prompt 132 – Karte nach Bildvorlage: Mäandertal (Tafelberge über einem Schlingenfluss)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 124 (Grabenlinie: `[[trench]]`, `[[wire]]`, Geschützringe) und 126 (Tafelberge:
> `[[mesa]]` mit Abbruchkanten und Rampen) sind erledigt (`prompts/done/124-*`,
> `prompts/done/126-*`). Lies zuerst `CLAUDE.md`, dann diese beiden Prompts und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[river]]`, `[[crossing]]`,
> `[[shoal]]`, `[[mesa]]`, `[[wood]]`, `[[village]]`, `[[farm]]`, `[[field]]`,
> `[[patchwork]]`, `[[trench]]`), „Bridges“, „Objects“ (`fort_*`) und die
> Punktsymmetrie von `city_large`; in `docs/ARCHITECTURE.md` Sicht („Seeing
> downhill“), indirektes Feuer und Wegfindung; `data/scale.toml` (`[range]`),
> `data/vision.toml`, `data/map_sketches/river_town.toml` und `mesa_desert.toml`
> als Muster, `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`.
> Sind die Prompts 115 (lange Brücken), 123 (Reichweitentabelle und
> Reichweitenprüfung), 130 (Omegaschleife: enge Flussschleifen) oder 131
> (Batteriehöhen: 4-km-Skizze, Feuerstellungen) erledigt, auch deren Ergebnis;
> Bausteine und Werkzeuge wiederverwenden statt neu bauen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/maeandertal/maeandertal.webp`. Genau ansehen. Ein mit KI
> erzeugtes, frei erfundenes Kartenbild. Es ist die zweite Fassung: Der Nutzer hat
> sie eigens machen lassen, damit **Fahrwege in Serpentinen auf jeden Berg und
> wieder herunter** führen (Abschnitt 3, „Fahrwege auf die Berge“). Die erste
> Fassung ohne diese Wege liegt als `maeandertal_erste_fassung.webp` daneben und
> gilt nicht mehr. Laut Beschriftung 4 km × 4 km mit Raster
> 0,5 km; der Maßstabsbalken unten links passt nicht dazu und wird ignoriert. Der
> Flusslauf ist im Nordosten nicht schlüssig (zwei verschlungene Arme); er wird
> durch den eigenen Lauf aus Abschnitt 3 ersetzt.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `meander_valley` (im Spiel „Mäandertal“ / „Meander valley“)
nach der Vorlage, für **2 gegen 2** (Team Nordwest gegen Team Südost, auch 1 gegen
1): Ein Fluss schlängelt sich in engen Schlingen schräg von Nordost nach Südwest
durch ein grünes Tal mit Dörfern und Feldern. Auf beiden Seiten stehen
**Tafelberge** mit Felswänden und flachen Kuppen, auf denen Stellungen liegen. Die
Schlingen bilden Halbinseln, die abwechselnd der einen und der anderen Seite
gehören; gekämpft wird um sieben Brücken und um die Sicht von den Kanten.

Die Vorlage zeigt:

- einen **stark mäandernden Fluss** von der Nordostecke zur Südwestecke, mit
  Halbinseln in den Schlingen, kleinen Stränden und Steilufern;
- **sieben Brücken**: Steinbogenbrücken und stählerne Fachwerkbrücken im Wechsel,
  dazu ein schmaler Steg;
- **Tafelberge** im Nordwesten und Südosten (und einzelne im Süden und Osten):
  flache Graskuppen mit Felswänden, auf den Kuppen **Geschützringe und Bunker**;
  dazwischen dunkler Nadelwald in den Schluchten;
- **breite Serpentinenwege auf jeden Berg**: Sie steigen in Kehren durch Lücken
  in den Felswänden, laufen über die Kuppe und auf einer anderen Seite wieder
  hinunter, oft weiter zum Nachbarberg; kein Berg ist eine Sackgasse;
- im Tal **Dörfer und Weiler** mit roten Dächern, ein **Gutshof mit Kapelle** auf
  einer Halbinsel, eine Kirche im Südwesten, ein Anleger am Fluss;
- **Felder im Flickenmuster** (Getreide gelb, Grünland, Brache) an den Dörfern;
- **kurze Zickzack-Gräben** an den Ufern, an den Halsstellen der Halbinseln und an
  den Brückenköpfen;
- geschwungene Wege und Straßen, die den Brücken und Hangfüßen folgen.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung. Nichts daraus übernehmen,
alles mit den Bausteinen des Spiels nachbauen.

## 2. Größe und Symmetrie

- **4 km × 4 km**, Biom `forest`, Wetter klar. Halten Bildrate oder Wegsuche die
  4 km nicht im Budget (`docs/PERFORMANCE.md`), alle Maße mit 0,8 auf 3,2 km
  verkleinern; als Entscheidung festhalten.
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y): Der Fluss läuft
  durch die Mitte, die Nordosthälfte seines Laufs ist das Bild der
  Südwesthälfte, und jeder Berg, jedes Dorf, jede Stellung der einen Seite hat
  ein Gegenstück. Die Vorlage ist nur ungefähr so gebaut; wo sie abweicht, gilt
  die Symmetrie. Das Licht ist nicht symmetrisch; das ist in Ordnung.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0), an die Vorlage
angelehnt, aber eigener Entwurf. Die Session übernimmt die Werte und glättet sie
nach den Stufenbildern. Alles, was hier nur für eine Seite steht, gibt es
gespiegelt auch für die andere.

- **Fluss** als `[[river]]`, Breite 90–110 m. Südwesthälfte der Mittellinie ab
  der Mitte: (0, 0) → (0, 250) → (−250, 420) → (−550, 330) → (−700, 100) →
  (−950, 0) → (−1150, 150) → (−1150, 450) → (−1000, 700) → (−700, 800) →
  (−600, 1050) → (−800, 1300) → (−1150, 1300) → (−1400, 1100) → (−1650, 1250) →
  (−1700, 1600) → (−1900, 2000) am Südrand. Die Nordosthälfte ist das Spiegelbild
  und endet bei (1900, −2000) am Nordrand. Die Schlingen rund ausziehen (Punkte
  verdichten, wie bei der Omegaschleife); Prallhänge als kurze Steilufer,
  Gleithänge als flache Sand- und Kiesstreifen.
- **Halbinseln:** Die Schlinge um (−350, 150) gehört zum Nordwestufer (Hals nach
  Norden), die Schlinge um (−850, 400) zum Südostufer (Hals nach Osten), die um
  (−1000, 1050) wieder zum Nordwestufer; gespiegelt entsprechend. So hat jede
  Seite Land, das in das gegnerische Ufer hineinragt.
- **Sieben Brücken** (`[[crossing]]`, achsparallel; der Fluss läuft an jeder
  Brücke kurz gerade):
  - **Mittelbrücke** West-Ost bei (0, 0);
  - West-Ost bei (−1150, 300) und gespiegelt (1150, −300);
  - Nord-Süd bei (−400, 380), vom Gutshof auf das Südufer, und gespiegelt
    (400, −380);
  - West-Ost bei (−1680, 1450) und gespiegelt (1680, −1450).
  Die Vorlage zeigt stählerne Fachwerkbrücken; das Spiel kennt bisher Stein und
  Holz. Alle sieben als Steinbrücken bauen (mit 115 mehrbogig über die volle
  Breite, sonst verengt der Generator das Wasser wie bisher) und die
  Fachwerkbrücke als eigenes Modell als offenen Punkt notieren. Brücken sind
  sprengbar.
- **Furten** (`[[shoal]]`, nur Infanterie) statt des Stegs bei (−850, 760) und
  gespiegelt (850, −760): der zweite Weg, wenn Brücken gesprengt sind.
- **Tafelberge der Nordwestseite** (`[[mesa]]`, flache Kuppe mit Felswänden;
  Mitte, Größe, Höhe über dem Tal; Auffahrten siehe „Fahrwege auf die Berge“):
  - große Tafel (−800, −1150), 1100 m × 450 m, 60 m; Wand nach Süden zum Tal;
    Auffahrten von Norden, von Westen und eine dritte von Südosten aus dem Tal;
  - Westtafel (−1450, −500), 850 m × 600 m, 50 m; Auffahrten von Norden und von
    Südosten;
  - (−1300, −1600), 500 m × 350 m, 70 m; Auffahrten von Süden und von Osten;
  - Randtafel (−400, −1800), 700 m × 300 m, 80 m; Auffahrten von Westen und von
    Süden;
  - Vorberg (150, −1700), 500 m × 400 m, 45 m; Auffahrten von Westen und von
    Süden.
  Die Südostseite bekommt die Spiegelbilder. Umrisse unregelmäßig wie in der
  Vorlage, keine Rechtecke. In den Ecken Nordwest und Südost dürfen die Berge bis
  an den Kartenrand reichen (dort unzugänglicher Fels). Zwischen den Tafeln
  Schluchten mit dichtem Nadelwald und je einem Weg. Die Tafeln stehen nirgends
  näher als 150 m am Fluss.
- **Fahrwege auf die Berge** (Kernforderung des Nutzers): Fahrzeuge müssen auf
  jeden Berg hinauf- und wieder herunterfahren können.
  - **Jede Tafel hat mindestens zwei Auffahrten auf verschiedenen Seiten**, die
    über die Kuppe miteinander verbunden sind. So kann man durchfahren, ausweichen
    und sich zurückziehen; eine gesperrte oder beschossene Auffahrt schließt
    niemanden ein.
  - **Für alle Fahrzeuge**, auch für den schwersten Panzer und für Zugmittel mit
    gezogenem Geschütz: Steigung und Breite nach den Regeln der Wegfindung wählen
    (größte befahrbare Steigung und Wenderadius aus `docs/ARCHITECTURE.md` bzw.
    dem Code ablesen, nicht raten), mit Reserve. Die Fahrbahn mindestens 8 m
    breit, in den Kehren breiter, damit zwei Fahrzeuge aneinander vorbeikommen.
  - **Serpentinen** wie in der Vorlage: Die Auffahrt steigt in zwei bis vier
    Kehren durch eine Lücke in der Felswand. Reichen die Rampen von `[[mesa]]`
    (gerade Rampe) dafür nicht, den Baustein um eine Auffahrt entlang einer
    Weglinie erweitern (der Weg schneidet sich in den Hang, das Gelände wird
    längs der Linie auf gleichmäßige Steigung gebracht); `mesa_desert` bleibt
    Bit für Bit gleich.
  - **Verbindungen von Berg zu Berg:** Wo zwei Tafeln nebeneinanderliegen (große
    Tafel und Westtafel, große Tafel und Randtafel), führt ein Weg über einen
    Sattel auf halber Höhe direkt hinüber, ohne ganz ins Tal abzusteigen.
  - **Auf der Kuppe** ein Rundweg, der Auffahrten, Geschützring und Posten
    verbindet.
  - Die Auffahrten sind Engstellen: je eine zum eigenen Hinterland (sicherer
    Nachschub) und eine zum Tal hin (kurzer Weg zur Front, vom Gegner
    angreifbar). Abseits der Wege bleiben die Felswände für alle gesperrt.
- **Stellungen auf den Kuppen:** je Tafel ein Geschützring (aus 124) auf einer
  ebenen Fläche, von der Kante zurückgesetzt, und ein Bunker (`fort_bunker_mg`)
  oder Unterstand an der Talkante als Beobachtungsposten. Je Team also fünf
  Feuerstellungen.
- **Dörfer und Höfe:**
  - **Gutshof mit Kapelle** auf der Halbinsel um (−350, 150): Herrenhaus (größtes
    vorhandenes Wohnhaus), Scheunen, Kapelle, ummauerter Hof, ein Feld;
  - **Uferdorf** der Nordwestseite um (450, −600), sechs bis acht Häuser, mit
    Anleger (Holzsteg) am Fluss;
  - **Kirchdorf** am Westrand um (−1500, 650), Kirche und fünf Häuser, an der
    Brücke (−1150, 300);
  - **Weiler** am Hangfuß um (−300, −750), drei bis vier Häuser;
  - einzelne Gehöfte (`[[farm]]`) an den Wegen.
- **Felder** (`[[patchwork]]`, Getreide, Grünland, Brache) um Uferdorf, Kirchdorf
  und Gutshof und auf den Halbinseln; Wiese mit Baumgruppen und Hecken dazwischen.
  Das Tal bleibt offen, der Wald steht an den Bergen.
- **Gräben** (`[[trench]]`, kurze Zickzack-Stücke von 100–250 m, Draht zum
  Wasser hin): quer über den Hals jeder Halbinsel, an jedem Brückenkopf beider
  Ufer und am Ufer vor den Dörfern. Keine durchgehende Linie.
- **Wege, alle geschwungen:** je Seite eine Talstraße (`road`) am Hangfuß entlang
  von der Nordost- zur Südwestecke, die die Brückenköpfe verbindet; eine Straße
  aus der Nordwestecke durch die Schlucht zur Mittelbrücke und weiter in die
  Südostecke; Feldwege (`track`) zu jeder Auffahrt, jedem Dorf und über jede
  Halbinsel.

## 4. Spiel

- **Starts** (60 m eben und frei): Team Nordwest (−1750, −1050) westlich der
  großen Tafel und (−950, −1700) zwischen den Tafeln am Nordrand; Team Südost
  gespiegelt (1750, 1050) und (950, 1700). Spielerzuordnung wie bei den anderen
  Skizzenkarten (Team 0: Spieler 0, 2; Team 1: Spieler 1, 3). 1 gegen 1 nutzt den
  ersten Start jedes Teams.
- **Versorgungspunkte** (elf): Mittelbrücke (0, 0), umkämpft; Gutshof
  (−350, 150) und gespiegelt; Brücke (−1150, 300) und gespiegelt; Brücke
  (−1680, 1450) und gespiegelt; Talkante der großen Tafel (−800, −980) und
  gespiegelt; Kirchdorf (−1500, 650) und gespiegelt.
- **Höhe gegen Tal:** Die Kuppen sehen weit ins Tal (Sichtregeln mit
  Geländehöhe), sind aber nur über die Auffahrten zu erreichen und liegen selbst frei.
  Wer im Tal steht, hat Dörfer, Hecken und Gräben als Deckung.
- **Reichweiten** (indirekt 1/8, Sicht und direktes Feuer 1/4): mit den echten
  Katalogwerten eine Tabelle in `docs/MAPS.md` wie in Prompt 123 (Geschütz,
  Reichweite im Spiel, was es von welcher Kuppe erreicht). Ziel: Von den vorderen
  Tafeln erreichen leichte Haubitzen die Mittelbrücke und die nächsten
  Halbinseln; die gegnerischen Tafeln und Starts erreicht keine eigene Kuppe.
  Abstände danach nachziehen.

## 5. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/meander_valley.toml` nach Abschnitt 2
bis 4, erzeugt mit `just gen-sketch meander_valley`. `[[mesa]]` stammt aus einer
Wüstenkarte: Fels, Bewuchs und Hangfuß für das Biom `forest` passend machen
(grauer Fels, Gras auf der Kuppe, Nadelwald am Fuß), ohne `mesa_desert` zu
ändern. Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben.

**Stufe 2 – Prüfen:**

- `just map-check meander_valley`, `just map-reach meander_valley`: alle Starts,
  Kuppen und Versorgungspunkte erreichbar; die Felswände abseits der Wege für
  alle zu; jede Halbinsel über ihren Hals und über eine Brücke erreichbar.
- **Fahrprobe auf die Berge**, als Test: Für jede Tafel und **jede** ihrer
  Auffahrten einzeln findet die Wegsuche für den schwersten Panzer und für ein
  Zugmittel mit gezogenem Geschütz einen Weg vom Start auf die Kuppe und zurück;
  mit einer gesperrten Auffahrt bleibt die Kuppe über die andere erreichbar.
  Dazu in einem kurzen Lauf der Simulation je Tafel ein Fahrzeug wirklich
  hinauf- und über die andere Seite hinunterfahren lassen (bleibt nicht hängen,
  kommt an) und die Fahrzeiten in `docs/MAPS.md` nennen. Bilder von zwei
  Serpentinen aus dem Spiel neben der Vorlage.
- **Symmetrietest:** unter Punktspiegelung gleiche Höhen, Gelände, Objekte,
  Weglängen.
- **Sichtprüfung:** Sichtfeld-Bilder von jeder Talkante, von der Mittelbrücke und
  vom Gutshof.
- **Reichweitenprüfung** mit Test oder Werkzeug (aus 123, falls vorhanden); die
  Ziele aus Abschnitt 4 als Test.
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Findet die KI
  die Auffahrten, besetzt sie Kuppen und Halbinseln, kommt sie über die Brücken, endet
  es im Stillstand? Probleme als offene Punkte; die KI nicht umbauen.
- Messung von Bildrate und Wegsuche in jeder Ansicht (`docs/PERFORMANCE.md`), bei
  Bedarf die Verkleinerung aus Abschnitt 2.

**Stufe 3 – Abschluss:** Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md` mit der Reichweitentabelle, Test wie bei den
anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
`just selftest`. Der Golden Hash ändert sich nicht (neue Karte); wenn doch,
Ursache finden.

## 6. Nicht Teil dieses Prompts

- Ein Modell für stählerne Fachwerkbrücken, für das Herrenhaus oder den Anleger
  (offene Punkte).
- Der kräftige, satte Farblook der Vorlage (über den Look aus Prompt 122
  nachzuholen, falls der Nutzer ihn annimmt).
- KI-Verbesserungen für Auffahrten, Kuppen und Flussübergänge.
- Boote auf dem Fluss (Prompt 119), Winter- oder Herbstvariante.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
