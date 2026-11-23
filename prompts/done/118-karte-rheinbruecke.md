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

### Entscheidungen (autonom, Nachtlauf 23.11.2026)

Niemand war erreichbar; diese Punkte hat die Session selbst entschieden
(alle **vorläufig**). Wie in 116 entschieden: Karte, Mission, Menü und
Übersetzungen tragen keinen echten Orts-, Einheiten- oder Personennamen
und kein Datum außer dem Monat; nur `docs/SOURCES.md`, `docs/MAPS.md`,
`docs/MISSIONS.md` und `assets-src/geo/rhine_bridge/README.md` nennen
Arnhem. Der Menüname „Rheinbrücke“ / „Rhine bridge“ nennt nur den Fluss,
wie der Prompt es vorgibt.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Rahmen um 51,97856 N / 5,91526 E (250 m nördlich des Nordendes), x-Achse 121°, 3,0 × 3,2 km: die Brücke läuft auf der y-Achse (oben ist 31° östlich von Nord), Stadt oben, Fluss quer, Polder unten. | Prompt: Brückenachse auf einer Kartenachse; so passen Altstadt, Anstieg zum Wald im Nordwesten, Fluss und Polder hinein. |
| 2 | Höhen aus AHN4 DTM (CC0, PDOK) wie 116, 25 Kacheln, Versatz −8 m. | Weg aus 116, Lizenz geklärt. |
| 3 | **Kern 1944 halbautomatisch:** Topotijdreis-Kacheln (Kadaster, RD-Kachelschema Stufe 11, 1,59 m/px) in den Kartenrahmen entzerrt, Blöcke/Teiche/Parks nach Farbe nachgezeichnet (`tools/geo_trace_core.py`, numpy + ImageMagick, einmaliges Hilfsmittel); Halbkreis 640 m um das Nordende. | Lage passt (Fluss und Brückenachse decken sich auf wenige Meter; die 300-m-Abweichung aus 116 kam vom falschen Kachelschema). Von Hand Blöcke abzuzeichnen wäre ungenauer. |
| 4 | **Straßen im Kern:** heutige Achsen bleiben, wo die alte Karte freien Boden zeigt (≤ 25 % über alten Blöcken), sonst entfallen sie (293 Wege, 11,8 km); die alten Straßen sind Straßenfläche zwischen den Blöcken (Schließung der Blockmaske, 9,8 ha). Mittellinien aus dem Skelett versucht und verworfen. | Das Skelett zerfiel an Straßenrändern und Schrift in Stücke; die Fläche ist für Bewegung und Häuserabstand gleichwertig. |
| 5 | Die Topotijdreis-„1944“-Ausgabe zeigt hier einen Stand **vor 1935** (keine Brücke; 1938–1950 dasselbe Blatt). Rampe und Umfeld von 1935 daher von Hand: Rampenkorridor geräumt (Graben/Park), Straße die Rampe hinab zum Ring, Straßen beiderseits der Rampe, drei Häuserblöcke an der Rampe (Umrisse geschätzt, Luftbild vom 13.9.1944 bestätigt Häuser beiderseits). | Keine spätere Ausgabe im Dienst gefunden. |
| 6 | Brücke: x = 1, y = 305…845: 2 × `Span60` (Nordufer), `Arch120` über dem Fluss (142 m breit an der Achse), 5 × `Span60` über das Vorland = 540 m (Quelle 601 m mit Widerlagern); Deck 15 m; Rampe 6 %. Die lange Rampe von 1935 bis zum Kreisel ist im Spiel eine Straße auf dem Boden (das Deck endet über dem Ufer, keine Unterführung nach 115). | `Arch120` nach 115; die Stadt liegt 10–12 m über dem Fluss, der Norddamm ist kurz. |
| 7 | Bauten nach 1945 südlich des Flusses und auf den alten Feldern östlich der Stadt („broek“) entfallen nach Baujahr (8 390), dazu die Nebenstraßen der Nachkriegsviertel im Süden (26,1 km); 17 spätere Seen/Teiche südlich des Flusses nicht gelesen; die Stadt außerhalb des Kerns behält alle Grundrisse als Stellvertreter. | Wie 116; topografische Karte zeigt dort Felder. |
| 8 | Eusebiuskirche als ein `env_church` „Damaged“ in der Mitte ihres OSM-Grundrisses, ausgerichtet Ost-West; OSM-Kirchengrundrisse im Kern nicht gelesen (sonst stellte der Blockrand mehrere Kirchenmodelle). | Prompt: vorhandenes Kirchenmodell als Wahrzeichen. |
| 9 | Freie Karte wie 116: **West gegen Ost auf beiden Ufern** (Starts 0/1 in der Stadt, 2/3 im Polder), sechs Versorgungspunkte (Brückenkopf Nord und Süd umkämpft, Altstadt/Werke Ost, Deich West/Ost), Pfadlängen auf ±90 m ausgeglichen. | Prompt: Lösung aus 116 übernehmen. |
| 10 | Mission `bridge_hold` „Holding the bridge“ / „Die Brücke halten“ (uk gegen passive KI): 4 Airborne-Gruppen, Vickers, 3 PIAT, 2 × 6-Pfünder, 2 Jeeps; Ziel 1 Häuser besetzen (darf scheitern), dann 16 min halten; Wellen 150 s Spähwagen/Schützenpanzer über die Brücke von Süden, 360 s Infanterie Ost, 420 s Granatwerfer, 600 s Pz IV + Infanterie West, 780 s StuG + Infanterie Nord; verloren, wenn der Feind den Versorgungspunkt am Nordende nimmt. | Prompt-Vorschlag; erste Fassung (3 Spähwagen, 2 StuG, 2 PIAT, Besetzziel an einem 12-Mann-Haus) verlor der Autopilot auf Normal. |
| 11 | **Nachschub nur per Abwurf als Skript** (neue Missionsaktion `resupply`: Munition für Einheiten im Umkreis, kostenlos); drei Abwürfe, der mittlere landet beim Feind und versorgt ihn. Keine Abwurf-Schaltfläche in der Luftleiste. | Eine neue Luftmission hätte die Luftregeln jeder Partie und den Golden Hash geändert. |
| 12 | Zweites Ende „was wäre wenn“: nach der Haltezeit fahren Firefly + 2 Cromwell von Süden über die Brücke, Texte ausdrücklich „Not historical“ / „Nicht historisch“, Hinweis, dass der Entsatz historisch nie kam. | Prompt: optional, deutlich gekennzeichnet. |
| 13 | Schwierigkeit: Easy Wellen 60 %, 30 % später; Hard Computer `normal`, Wellen 140 %, 15 % früher. | Autopilot gewinnt alle drei Stufen. |
| 14 | Lizenz Topotijdreis: im Dienst nicht genannt; als Kadaster CC BY 4.0 behandelt und in `CREDITS.md` genannt; nur Umrisse, kein Kartenbild im Repo. Luftbild (Nationaal Archief, gemeinfrei) nur unter `build/` zum Vergleich. | Daten der BRT/TOPraster sind CC BY 4.0; Lagen sind Tatsachen. |

## Ergebnis

- **Stufe 1 – Geodaten, Gelände, Brücke:** Overpass-Abfrage (47 MB,
  14:07 UTC), 25 AHN-Kacheln, 100 Topotijdreis-Kacheln, Luftbild unter
  `build/geo118/raw/`; Bilder `build/geo118/stage1/` (`stage1.png`,
  `stage1_grid.png`, `whole_cmp.png` Topokarte neben Karte,
  `core_topo_s.png`, `core_cmp_s.png`, `reach.png`, `reach_bridge.png`).
- **Stufe 2 – Karte** `data/maps/rhine_bridge` („Rheinbrücke“ / „Rhine
  bridge“, `MAP_RHINE_BRIDGE`): 3,0 × 3,2 km, −6 … 61,5 m, 118,9 km
  Straßen, 4 924 Gebäude + Kirche, 7 547 Bäume, Brücke aus 8 Teilen;
  Kern `assets-src/geo/rhine_bridge/core_1944.json` (107 Blöcke, 2 Teiche,
  2 Parks, Handteile); Importer: `core` (`apply_core`, `core_streets`,
  `core_landmarks`), alle anderen Geodatenkarten unverändert;
  `just map-check` ok; `just map-reach`: 83,3 % Fahrzeuge, 2,2 % nur
  Infanterie, 3,0 % niemand, 11,5 % Gebäude, 98,7 % zusammenhängend;
  Test `rhine_bridge_plays_and_carries_no_real_names`; `just
  gen-geo-check rhine_bridge` in `just check` und CI. Bilder
  `build/geo118/report/` (`stage1.png`, `stage2.png`, `reach.png`,
  `map_reach.txt`), Vergleich Spiel/Luftbild `build/geo118/compare/` (`game_vs_aerial_1944.png`: Spielbild vom Brückenkopf neben dem Luftbild vom 13.9.1944; `top_vs_aerial_1944.png`: Draufsicht gedreht neben dem Luftbild).
- **Messungen** (`docs/PERFORMANCE.md` „Rhine bridge“): Tick 4,33 ms
  Mittel, p99 5,9, max 9,3 ms; Wegsuche 40 600 Knoten je Anfrage, max
  6,2 ms; Sicht 5,2 ms; 12,0 MB Speicher; Bildrate nicht gemessen.
- **KI** (`docs/AI.md` „The rhine bridge“): 1v1 hard/normal 60/30/1,
  hard/passive 70/10/2, normal/easy 50/40/1, normal/normal 40/60/0; 2v2
  hard/normal 33/50/1, normal/normal 50/33/1; Basis hard/passive 4 von 4;
  Stuck-Audit 0 von 110 Befehlen.
- **Stufe 3 – Mission** `data/maps/rhine_bridge/missions/bridge_hold.toml`
  mit `.de.po`; Sim: Aktion `resupply` (`Sim::drop_supplies`), Test
  `missions::a_supply_drop_refills_the_ammunition_of_units_near_it`;
  Autopilot gewinnt Easy/Normal/Hard nach 1 005 s (6/9/10 eigene Verluste
  von 15); Tuning-Test `one_mission_every_level`; `just selftest` spielt sie aus dem Menü (gewonnen nach 1 005 s; Lauf vor dem Verschieben zweier Einfahrtspunkte auf befahrbare Straßen). Doku `docs/MISSIONS.md`.
- **Goldener Hash unverändert** (neue Aktion nur in Missionen, keine Regel
  geändert).
- **Prüfungen:** `just check` (714 Tests, alle sechs Geodatenkarten Bit
  für Bit gleich), `just selftest`, `just map-check`, `just map-reach`,
  Stuck-Audit, Turnier, Benchmarks.
- **Offen:** die lange Rampe von 1935 (Damm mit Viadukt bis zum Kreisel)
  ist im Spiel eine Straße auf dem Boden, das Deck endet über dem Ufer
  (keine Unterführung, 115); Straßenachsen des Kerns nicht nachgezeichnet
  (nur Fläche); Umrisse der Rampenhäuser geschätzt; Reihenhäuser im
  Süden standen 1944 teils schon (Luftbild), sind aber nach Baujahr
  entfernt; Lizenz der Topotijdreis-Ebene bestätigen; Abwurf aus der
  Luftleiste; Bildrate in Godot nicht gemessen; `just selftest` nicht
  erneut nach dem Verschieben der Einfahrten West/Nord gelaufen
  (Autopilot-Test grün).
