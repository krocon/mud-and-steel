# Prompt 153 – Karte nach Bildvorlage: Landzunge (Flussschlinge mit befestigten Kuppen, vier Spieler)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[river]]`, `[[crossing]]` mit
> `stone`, `[[shoal]]`, `[[hill]]`, `[[ring]]` (ummauerte Höhe), `[[town]]`,
> `[[row]]`, `[[square]]`, `[[village]]`, `[[farm]]`, `[[field]]`,
> `[[patchwork]]`, `[[wood]]`, `[[scatter]]`) und „Bridges“; die Karten
> `river_town` und `river_fields` als Muster (`data/map_sketches/*.toml`); in
> `docs/ARCHITECTURE.md` Sicht und Wegfindung; `data/scale.toml` (`[range]`),
> `docs/SOURCES.md`, `crates/cli/src/gen_sketch.rs`, `docs/AI.md`,
> `docs/PERFORMANCE.md`. Sind die Prompts 130 (Omegaschleife: enge
> Flussschleifen), 123 (Reichweitentabelle) oder 134 (Stadtblöcke) erledigt,
> auch deren Ergebnis.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/landzunge/landzunge.webp`. Genau ansehen. Ein mit KI erzeugtes
> Kartenbild. **Aufmachung und Marken (Rauten mit Nummern, Kreis in der Mitte)
> erinnern an die Übersichtskarten eines kommerziellen Spiels.** Verwendet wird
> **nur die Landschaft als Anregung**; es wird keine Karte eines anderen Spiels
> nachgebaut, keine Marke, kein Name, kein Spielmodus übernommen
> (`docs/SOURCES.md`: nichts aus anderen Spielen). Der Aufbau ist deshalb ein
> **eigener Entwurf**, der von der Vorlage abweicht: anderer Flusslauf (Auslauf
> nach Westen statt nach Süden), strenge Diagonalspiegelung, kein Kanal am
> Südrand, eigene Orte und Ziele. Erkennt die Session in der Vorlage eine
> bestimmte Karte eines Spiels wieder, die Abweichungen vergrößern und das im
> Ergebnis sagen.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 Bilder aus dem Spiel
> zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte `river_spit` (im Spiel „Landzunge“ / „River spit“), **2 km × 2 km**,
für **vier Spieler, 2 gegen 2** (Team Südwest gegen Team Nordost; auch 1 gegen
1): Ein Fluss läuft in einem großen Haken um die Kartenmitte und schließt eine
**Landzunge** ein, auf der vier **befestigte Kuppen** um den Mittelpunkt liegen.
Beide Teams stehen auf demselben Ufer und erreichen die Landzunge über Land;
das Bauernland jenseits des Flusses ist nur über Brücken zu erreichen und
bietet den Weg in die Flanke.

Was die Vorlage an Landschaft zeigt und der Entwurf aufnimmt:

- einen **Fluss in weiten Schlingen**, der eine Halbinsel in der Mitte umfließt;
- auf der Halbinsel **vier Kuppen mit Steinwällen** und kleinen Bauten um einen
  Platz in der Mitte, eine Straße quer darüber;
- eine **Kleinstadt** im Südosten mit gebogenen Häuserzeilen, einem Raster,
  Gärten und einem Park mit Brunnen;
- **Bauernland** im Nordwesten: Felder in Gold und Grün, ein Straßendorf, Höfe,
  Silos, Hecken, Baumgruppen;
- **Gehöft und Hallen** im Nordosten, eine **Geschützstellung im Steinring** auf
  einer Kuppe;
- mehrere **Brücken** aus Stein und Holz, geschwungene Feldwege.

## 2. Symmetrie

**Spiegelsymmetrisch an der Diagonale Nordwest–Südost**, also (x, y) → (y, x)
bei x nach Osten und y nach Süden mit der Mitte als Ursprung. Die Diagonale
läuft durch das Bauernland, die Spitze der Landzunge, den Mittelpunkt und die
Stadt; die Teams sitzen in der Südwest- und der Nordostecke. Unter dieser
Spiegelung wird aus einer Nord-Süd-Brücke eine West-Ost-Brücke, beides ist
erlaubt (Brücken sind achsparallel). Gelände, Fluss, Brücken, Wege, Orte, Starts
und Ziele sind gespiegelt; Hausvarianten und Feldfrüchte dürfen sich
unterscheiden, solange Weggitter, Sicht und Objektklassen gleich sind. Den
Symmetrietest für diese Spiegelung schreiben bzw. den vorhandenen erweitern.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0). Zu jedem Punkt
(x, y) gehört das Spiegelbild (y, x). Die Session übernimmt die Werte und
glättet sie nach den Stufenbildern.

- **Fluss** (`[[river]]`, etwa 50 m breit, für Fahrzeuge und Infanterie nur an
  Brücken und Furten zu queren): Nordrand (300, −1000) → (350, −700) →
  (560, −450) → **Ostknie** (700, −180) → (600, −40) → (300, −120) →
  (100, −260) → (−120, −340) → **Scheitel** auf der Diagonale (−300, −300) →
  (−340, −120) → (−260, 100) → (−120, 300) → (−40, 600) → **Südknie**
  (−180, 700) → (−450, 560) → (−700, 350) → Westrand (−1000, 300). Knie und
  Scheitel weit ausgerundet (Punkte verdichten wie bei der Omegaschleife). Flache
  Ufer mit Wiesen, Baumgruppen an den Außenbögen.
- **Zwei Länder:**
  - das **Außenland** südöstlich des Flusses: beide Lager, die Stadt, die
    Landzunge; es reicht als Streifen östlich des Nordlaufs bis in die
    Nordostecke und südlich des Westlaufs bis in die Südwestecke;
  - das **Bauernland** nordwestlich des Flusses: ohne Lager, nur über Brücken
    und Furten zu erreichen.
- **Landzunge:** das Land zwischen den Flussbögen um die Mitte, etwa 600 m
  breit, nach Südosten offen zur Stadt hin. Darauf:
  - **vier befestigte Kuppen** (`[[hill]]` 8–12 m mit ebenem Rücken von etwa
    60 m, ringsum ein Trockenmauer- bzw. Steinwall mit zwei Lücken über
    `[[ring]]`, darin eine Sandsackstellung oder ein kleines Steinhaus): bei
    (20, −180) und (190, −40) und gespiegelt (−180, 20) und (−40, 190). Je eine
    Lücke zeigt zum Mittelplatz, eine nach außen; Fahrzeuge kommen hinauf.
  - der **Mittelplatz** bei (60, 60) auf der Diagonale: freie Fläche zwischen
    den Kuppen, von allen vieren einzusehen;
  - eine **Straße** von der Stadt über den Mittelplatz zur Spitze, die sich
    dort zu den beiden Brücken gabelt.
- **Brücken** (`[[crossing]]`; der Fluss läuft an jeder Brücke kurz gerade und
  ist auf die Brückenlänge verengt, wie bisher im Generator):
  - **Nordbrücke** (Stein, Nord-Süd) bei (100, −260) und gespiegelt die
    **Westbrücke** (Stein, West-Ost) bei (−260, 100): von der Spitze der
    Landzunge ins Bauernland;
  - **Holzbrücke** mit Lastgrenze über den Nordlauf bei (350, −700) (West-Ost)
    und gespiegelt über den Westlauf bei (−700, 350) (Nord-Süd): vom Lager des
    jeweiligen Teams direkt ins Bauernland;
  - je eine **Furt** (`[[shoal]]`) am Ostknie bei (680, −100) und gespiegelt
    (−100, 680), für Infanterie und langsam für Fahrzeuge.
  Vier Brücken und zwei Furten. Brücken sind sprengbar.
- **Bauernland:**
  - ein **Straßendorf** in zwei Armen entlang einer Nord-Süd-Straße
    (x ≈ −450, y −800 … −480) und gespiegelt entlang einer West-Ost-Straße, die
    sich an einer **Kreuzung auf der Diagonale** bei (−450, −450) treffen; dort
    eine Kapelle, ein Gasthof, ein Brunnen; je Arm sechs bis acht Häuser;
  - **Felder** (`[[patchwork]]`: Getreide gold, Rüben und Kohl grün, Brache
    braun) über das ganze Land, mit Hecken und Zäunen, Garben und Heuhaufen;
  - zwei **Kuppen mit Steinwall** an den Brückenköpfen bei (260, −320) und
    gespiegelt (−320, 260), die die Nord- bzw. Westbrücke decken;
  - **Gehöfte** (`[[farm]]`) bei (−100, −700), (−750, −750) und gespiegelt, mit
    Silos bzw. Schobern als Landmarken; Baumgruppen und zwei kleine Gehölze.
- **Stadt** im Südosten, symmetrisch zur Diagonale, um einen **Park mit
  Brunnen** bei (620, 620): zur Landzunge hin zwei **gebogene Häuserzeilen**
  (Viertelkreise um die Mitte, Radius etwa 450 m und 560 m, mit Lücken für die
  Straßen), dahinter ein Raster aus kleinen Blöcken, Gärten und zwei
  öffentlichen Bauten (Rathaus, Kirche) auf etwa 700 m × 700 m; rote
  Ziegeldächer. Mit `[[town]]` und `[[row]]` bzw. `[[city_block]]`, falls 134
  erledigt ist. Die Stadt ist die Deckung auf dem Weg zur Landzunge und der
  kürzeste Landweg zwischen den Teams (am Südostrand entlang).
- **Lagerecken:** im Nordosten ein **Gutshof mit Hallen** (Stahlhalle,
  Lagerhaus, Hofgebäude) um (800, −800) und eine **Geschützstellung im
  Steinring** auf einer Kuppe bei (880, −460); gespiegelt im Südwesten ein
  Gutshof um (−800, 800) und die Stellung bei (−460, 880).
- **Wege:** Landstraße von jedem Lager am Fluss entlang zur Stadt; Feldwege vom
  Lager zur Holzbrücke und zur Furt; die Straße über die Landzunge; im
  Bauernland die beiden Dorfstraßen von den Holzbrücken zur Kreuzung und weiter
  zu Nord- und Westbrücke. Alles geschwungen.
- **Relief:** sanft gewellt (bis 10 m), der Fluss 3–4 m eingetieft; Biom
  `forest`, Wetter klar.

## 4. Spiel

- **Starts** (60 m eben und frei): Team Nordost (540, −810) und (710, −650);
  Team Südwest gespiegelt (−810, 540) und (−650, 710). Team 0 Südwest (Spieler
  0, 2), Team 1 Nordost (1, 3). 1 gegen 1 nutzt den ersten Start jedes Teams.
- **Versorgungspunkte** (sieben): Mittelplatz (60, 60), umkämpft; Park der Stadt
  (620, 620), umkämpft; Dorfkreuzung (−450, −450), umkämpft; Brückenkuppen
  (260, −320) und (−320, 260); Geschützstellungen (880, −460) und (−460, 880).
- **Drei Wege zum Gegner:**
  - **über die Landzunge** (kurz, in der Mitte, unter den Augen der vier
    Kuppen);
  - **durch die Stadt** (Landweg am Rand, Häuserkampf, kein Wasser);
  - **durch das Bauernland** (Holzbrücke hinüber, über offene Felder, über die
    Nord- oder Westbrücke auf die Spitze der Landzunge oder weiter zur
    gegnerischen Holzbrücke in den Rücken). Die Holzbrücken tragen keine
    schweren Panzer: Die Flanke gehört leichten Kräften, es sei denn, man nimmt
    die Steinbrücken an der Spitze.
- **Prüfen, dass es so ist:** Fahrzeiten vom Lager zum Mittelplatz, zur Stadt
  und zur Dorfkreuzung für einen leichten und einen schweren Panzer als Tabelle
  in `docs/MAPS.md`; die Kuppen sehen den Mittelplatz und beide Steinbrücken,
  aber nicht in die Stadt.
- **Reichweitentabelle** (indirekt 1/8) wie in Prompt 123: Was erreichen
  Geschütze aus den Steinringen an den Lagern und von den Kuppen? Ziel: aus dem
  eigenen Steinring die Landzunge, nicht das gegnerische Lager.

## 5. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/river_spit.toml` nach Abschnitt 2 bis
4, erzeugt mit `just gen-sketch river_spit`. Bestehende Skizzenkarten müssen Bit
für Bit gleich bleiben. Bilder aus dem Spiel; dazu ein Satz, worin der Entwurf
von der Vorlage abweicht.

**Stufe 2 – Prüfen und Abschluss:**

- `just map-check river_spit`, `just map-reach river_spit`: alle Starts und
  Ziele für Fahrzeuge erreichbar; das Bauernland für schwere Panzer nur über
  die Steinbrücken; mit allen Brücken gesprengt bleibt es über die Furten
  erreichbar.
- **Symmetrietest** für die Diagonalspiegelung (Höhen, Gelände, Weggitter,
  Objektklassen, Weglängen).
- **Sichtprüfung:** Sichtfeld-Bilder von jeder Kuppe der Landzunge, vom
  Mittelplatz, von einer Brückenkuppe, aus einem Steinring, aus der Stadt.
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`).
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Welche der
  drei Wege nimmt die KI, kämpft sie um die Kuppen, nutzt sie das Bauernland?
  Probleme als offene Punkte; die KI nicht umbauen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md` mit Fahrzeit- und Reichweitentabelle, Test wie bei den anderen
  Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
  `just selftest`. Der Golden Hash ändert sich nicht (neue Karte); wenn doch,
  Ursache finden.

## 6. Nicht Teil dieses Prompts

- Der Kanal am Südrand der Vorlage und ihre Marken, Nummern und Spielmodi.
- Neue Modelle (Silos, Gewächshäuser, Bunker auf den Kuppen); vorhandene
  Objekte als Stellvertreter, Wünsche als offene Punkte.
- Winter- oder Herbstvariante (über `base` in der Skizze leicht nachzuholen).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
