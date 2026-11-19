# Prompt 118 – Karte und Mission: Rheinbrücke (Vorbild Arnhem, September 1944)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzungen:** Prompt 115
> (lange Brücken), 116 (Waalbrücke) und 117 (Großbritannien) sind erledigt
> (`prompts/done/`). Lies zuerst `CLAUDE.md`, dann diese drei Prompts ganz
> (besonders Entscheidungen und Ergebnis), `prompts/done/081-karte-hafenstadt.md`,
> in `docs/MAPS.md` „Maps from real geodata (prompt 081)“, „Bridges“ und die Karte
> aus 116, `docs/MISSIONS.md` und die Mission aus 116, `docs/SOURCES.md` (Karten aus
> Geodaten, Abzeichen), `data/catalog/uk.toml`, `tools/geo_import.py` ganz.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe den Zwischenstand zeigen
> und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte um die **Rheinbrücke von Arnhem**, Stand 17.–20. September 1944, und
eine **Mission** dazu: britische Fallschirmjäger halten die Häuser am Nordende der
Brücke gegen deutsche Gegenangriffe, bis Entsatz kommt (der historisch nicht kam).

Schwierigkeit: **Das Arnhem von 1944 gibt es nicht mehr.** Die damalige Brücke wurde
im Oktober 1944 zerstört, die heutige (1948) steht an derselben Stelle. Die
Innenstadt am Nordende wurde in der Schlacht weitgehend zerstört und anders wieder
aufgebaut. Heutige OSM-Daten taugen für Fluss, Ufer, Höhen und die Polder im Süden,
nicht für die Häuser und Straßen am Brückenkopf.

Die Karte heißt `rhine_bridge` (im Spiel „Rheinbrücke“ / „Rhine bridge“); ob Mission
und Briefing den echten Ort nennen, hat der Nutzer in Prompt 116 entschieden, das
gilt hier genauso.

## 2. Geodaten und Stand 1944

- **Ausschnitt** etwa 3 km × 3 km: Innenstadt nördlich der Brücke mit dem
  ansteigenden Gelände zum Rand der Veluwe, der Nederrijn (100–150 m breit), im
  Süden die Polder mit Deichen. Rahmen so gedreht, dass die Brückenachse auf einer
  Kartenachse liegt.
- **Höhen, Fluss, Polder, Außenbezirke** aus OSM und DEM wie in 116 (AHN, falls
  verfügbar).
- **Kern am Brückenkopf (Stand 1944) von Hand:** Straßen und Häuserblöcke im Umkreis
  von etwa 600 m um das Nordende der Brücke nach historischen Quellen nachzeichnen:
  niederländische Topokarten der 1930er/40er (Kadaster, Lizenz prüfen),
  RAF-Luftbilder vom September 1944 (Crown Copyright erloschen), Lagepläne aus der
  Literatur nur als Referenz. Ablage als Daten in `assets-src/geo/rhine_bridge/`
  (z. B. `core_1944.json` mit Straßenachsen und Blockumrissen), die der Importer
  statt der OSM-Daten in diesem Bereich verwendet. Jede Quelle in
  `docs/SOURCES.md`.
- **Die Brücke von 1935** nach Prompt 115: Hauptbogen rund 140 m, Vorlandbrücken
  und die Rampe am Nordufer, die über die Uferstraßen läuft; die Häuser beiderseits
  der Rampe sind die Stellungen der Verteidiger. Maße nach Quelle.
- Bauten nach 1945 weglassen; die Eusebiuskerk (damals stark beschädigt) als
  Wahrzeichen mit vorhandenem Kirchenmodell.

## 3. Karte (frei spielbar)

Wie in 116: zwei Teams auf beiden Ufern, Versorgungspunkte an beiden Brückenköpfen,
in der Altstadt und auf dem Deich im Süden. Dieselbe Lösung für das Problem „eine
Brücke, kein anderer Übergang“ wie in 116 übernehmen. `just map-check`,
`just map-reach`, Messungen, KI-Turnier wie in 081.

## 4. Mission

- **Seite:** Großbritannien (Airborne), gegen die KI.
- **Ablauf (Vorschlag):** Der Spieler beginnt mit wenigen Trupps
  Fallschirmjägern, einer oder zwei 6-Pfünder-Pak und Jeeps am Nordende; Ziel 1:
  die Häuser an der Rampe besetzen, bevor der erste Gegenangriff kommt. Dann
  Angriffswellen: Spähwagen und Halbkettenfahrzeuge über die Brücke von Süden
  (historisch am Morgen des 18. September), Infanterie und Panzer aus der Stadt,
  Granatwerfer- und Artilleriebeschuss. Munition ist knapp, Nachschub kommt nur als
  Abwurf (Luftleiste aus 055), teilweise in falscher Lage.
- **Sieg und Niederlage:** Die Mission ist gewonnen, wenn der Spieler das Nordende
  eine bestimmte Zeit hält; die Zeit und die Wellen stimmt die Session so ab, dass
  es schwer, aber schaffbar ist. Optional ein zweites Ende, in dem XXX Corps mit
  Panzern über die Brücke kommt (das historische „was wäre wenn“), deutlich
  gekennzeichnet als nicht historisch.
- Deutsche Verbände als `de` ohne Abzeichen von SS-Verbänden (`docs/SOURCES.md`).
- Briefing, Meldungen, Ziele auf Deutsch und Englisch, Missionsliste, Durchlauf-Test.

## 5. Stufen

**Stufe 1 – Geodaten, Gelände, Brücke:** Ausschnitt, Höhen, Fluss, Polder, Brücke
von 1935, Stufenbilder, `just map-reach`.

**Stufe 2 – Kern von 1944, Stadt, freie Karte:** Quellen sichten, Kern nachzeichnen,
Gebäude, Bewuchs, Starts, Versorgungspunkte, Menüname, Tests, Messungen,
KI-Turnier, Doku. Bild aus dem Spiel vom Brückenkopf neben einem Luftbild von 1944
(nur zur Prüfung, das Luftbild nicht ins Repo, außer die Lizenz erlaubt es).

**Stufe 3 – Mission:** Skript, Texte, Test, Doku in `docs/MISSIONS.md`. Danach
`just check`.

## 6. Nicht Teil dieses Prompts

- Oosterbeek, die Landezonen westlich der Stadt, die Fähre bei Driel und die
  gesprengte Eisenbahnbrücke (eigene Karte, falls gewünscht).
- Luftlandung mit Lastenseglern als spielbarer Anfang.
- Ein Feldzug aus mehreren Missionen (Nijmegen → Arnhem).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.
