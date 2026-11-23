# Prompt 121 – Karte und Mission: Landung im Golf (Vorbild Marina di Campo, Elba, Juni 1944)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzungen:** Prompt 120
> (Mittelmeer-Biom) und Prompt 119 (Landungsboote) sind erledigt (`prompts/done/`).
> Ohne 119 nur Stufe 1 und 2 (freie Karte), die Mission als offener Punkt. Lies
> zuerst `CLAUDE.md`, dann `prompts/done/120-mittelmeer-biom.md`,
> `prompts/done/119-landungsboote.md` und `prompts/done/081-karte-hafenstadt.md`
> ganz, die Entscheidungen der erledigten Prompts 112–116 (falls in
> `prompts/done/`, besonders die Frage, ob Missionen echte Orte nennen), in
> `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und `harbor_city`,
> `docs/MISSIONS.md` und eine vorhandene Mission mit `.de.po`, `docs/SOURCES.md`,
> `docs/AI.md`, `docs/PERFORMANCE.md` („Harbor city“), `tools/geo_import.py` ganz,
> `assets-src/geo/harbor_city/site.json`, `data/catalog/us.toml` und `de.toml`,
> `data/objects.toml` (`fort_*`).
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe den Zwischenstand zeigen
> und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte um den **Golf von Campo** an der Südküste von Elba aus echten Geodaten,
Stand Juni 1944, und eine **Landungsmission** dazu.

Historischer Rahmen (Operation Brassard, 17.–19. Juni 1944): Französische Truppen
(Kommandos, Kolonialinfanterie, marokkanische Goumiers) landeten mit britischer und
amerikanischer Marineunterstützung gegen die deutsche Besatzung; der Golf von Campo
war eine der Hauptlandezonen, Marina di Campo wurde hart verteidigt, Küstenbatterien
in den Hängen beschossen die Boote. Die Session prüft die Einzelheiten an Quellen
(`docs/SOURCES.md`) und erzählt nur, was belegt ist. Ereignisse an der
Zivilbevölkerung nach der Landung gehören nicht ins Spiel.

Es gibt keine französische Nation. Die Freien Franzosen waren 1944 fast ganz
amerikanisch ausgerüstet; sie werden mit **US-Einheiten** dargestellt. Die deutsche
Besatzung stellt der Katalog `de` wie überall, ohne Abzeichen (`docs/SOURCES.md`).

Die Karte heißt wie die anderen ohne echten Namen `landing_gulf` (im Spiel „Landung
im Golf“ / „Landing gulf“). Ob die Mission den
echten Ort nennt, folgt der Entscheidung aus Prompt 116; ist die noch nicht
gefallen, den Nutzer am Ende von Stufe 1 fragen.

## 2. Ausschnitt und Geodaten

Richtwerte, die Session darf sie nach den Stufenbildern anpassen:

- **Ausschnitt** etwa 4 km × 4 km um den Golf: der Strand und das Dorf Marina di
  Campo, die Ebene von Campo dahinter, die Hänge links und rechts des Golfs mit den
  Stellungen, im Nordwesten der Anstieg zum Monte Capanne (der Gipfel liegt etwa
  6 km entfernt und bleibt draußen). Im Süden ein Streifen Meer, tief genug für die
  Landungsboote (Prompt 119), vom Kartenrand bis an den Strand.
- **Rahmen** so gedreht, dass das Meer unten im Bild liegt (Blick von See wie bei
  `harbor_city`).
- **Höhen:** Copernicus DEM wie in 081 (0 bis einige hundert Meter). Die steilen
  Flanken prüfen (`just map-reach`): Die Hänge mit den Stellungen müssen für
  Infanterie erreichbar sein, für Fahrzeuge die Straßen ins Hinterland.
- **Meeresboden:** wie in 081 in Ringen, aber flacher auslaufend am Sandstrand, damit
  Landungsboote nahe am Ufer auflaufen; an den Felsküsten tiefer und steil.
  Felsküste als Kante (nicht passierbar), Sandstrand als `sand`.
- **Stand 1944:** weglassen, was später kam: der Flugplatz in der Ebene, Hotels,
  Campingplätze, Ferienanlagen, der heutige Jachthafen mit Stegen, moderne
  Straßenführungen. Das Dorf war 1944 deutlich kleiner; Häuser am Hafen und an der
  Hauptstraße behalten, Neubaugebiete am Rand weglassen (Höhengrenze und Lage wie in
  Prompt 112; als Entscheidung festhalten). Felder und Weinberge in der Ebene als
  Feldarten (`field_*`) wie in Prompt 120, Macchia als `tall_grass`.
- **Biom:** `mediterranean` (Prompt 120). Der Importer setzt damit Pinien,
  Steineichen, Olivenhaine, Macchia und die mediterranen Häuser; Trockenmauern und
  Terrassen an den Hängen, wo OSM sie zeigt (`barrier=wall`, `retaining_wall`) oder
  wo Olivenhaine und Weinberge am Hang liegen. Die Kirche des Dorfes mit Campanile.

## 3. Befestigungen

- Küstenbatterien und Stellungen in den Hängen beiderseits des Golfs und am Dorf:
  Orte aus OSM (`military=bunker`, `historic=*`) und Quellen, sonst in sinnvoller
  Lage festgehalten. Platzhalter: `fort_bunker_mg`, `fort_trench_section`,
  `fort_sandbag_position`, Minen- und Panzerhindernisse am Strand
  (`fort_hedgehog`, `fort_mine_*`) dort, wo Quellen sie belegen.
- Die Geschütze der Batterien als deutsche Geschütze aus dem Katalog in
  Stellung (passende Kaliber; schwere Küstengeschütze gibt es nicht, als offener
  Punkt).

## 4. Karte (frei spielbar)

- Zwei Teams: West gegen Ost über den Golf, oder Hinterland gegen Küste. Die Session
  wählt, was nach `just map-reach` und Weglängen ausgeglichener ist, und begründet
  es.
- Versorgungspunkte: Dorf/Hafen (Mitte), Strand, die Höhen über dem Golf, ein Punkt
  in der Ebene, die Straßenkreuzung ins Hinterland.
- Wenn Boote im Gefecht ohne Mission verfügbar sind (Entscheidung in Prompt 119),
  den Strand so anlegen, dass Landungen möglich sind.
- `just map-check`, Tests wie bei `harbor_city`, Messungen, KI-Turnier.

## 5. Mission

- **Seite:** Alliierte (US-Ausrüstung), gegen die KI.
- **Ablauf (Vorschlag):** In der Dämmerung fahren Landungsboote vom Kartenrand in
  den Golf (Prompt 119, Boote mit Ladung über das Missionsskript), die Batterien
  eröffnen das Feuer; erste Welle Infanterie an den Strand, zweite Welle mit Jeeps
  und Geschützen, dritte mit Panzern (LCM). Ziele: Strand halten, die Batterien in
  den Hängen ausschalten (eine nach der anderen, bis dahin beschießen sie die
  späteren Wellen), das Dorf nehmen, die Straße ins Hinterland öffnen.
- Eine Gruppe Kommandos landet vorab an einer Felsbucht seitlich und greift eine
  Batterie von hinten an (historisch landeten Kommandos vor der Hauptlandung an den
  Flanken; nach Quelle prüfen).
- Gegner: Festungstruppen in Stellungen, ein Gegenstoß mit wenigen Fahrzeugen aus
  dem Hinterland.
- Briefing, Meldungen, Ziele auf Deutsch und Englisch, Missionsliste, Durchlauf-Test.
  Schwierigkeit so abstimmen, dass die erste Welle nicht chancenlos ist.

## 6. Stufen

**Stufe 1 – Geodaten, Gelände, Küste, Straßen:** Abfrage, Extrakt, `site.json`,
Höhen, Strand, Felsküste, Meeresboden, Straßen, Stufenbilder, `just map-reach`.

**Stufe 2 – Dorf, Bewuchs, Befestigungen, freie Karte:** Stand 1944, Gebäude,
Felder, Macchia, Wald, Befestigungen, Starts, Versorgungspunkte, Menüname, Tests,
Messungen, KI-Turnier, Doku (`docs/MAPS.md` mit Tabelle „was die Vereinfachung
behielt“, `docs/SOURCES.md`, `docs/PERFORMANCE.md`, `docs/AI.md`).

**Stufe 3 – Mission** (nur mit Prompt 119): Skript, Texte, Test, Doku in
`docs/MISSIONS.md`. Danach `just check`.

Golden Hash: Ein reiner Kartenzusatz ändert ihn nicht.

## 7. Nicht Teil dieses Prompts

- **Französische Nation** und eigene Modelle (Uniformen der Kolonialtruppen,
  Goumiers).
- **Schwere Küstengeschütze** und Schiffsartillerie.
- Die übrigen Landezonen und Portoferraio.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 23.11.2026)

Niemand war erreichbar; diese Punkte hat die Session selbst entschieden
(alle vorläufig). Die Frage nach dem echten Ort ist mit Prompt 116
(vorläufig) entschieden: Karte, Mission, Menü und Übersetzungen tragen
keinen echten Namen, Ort und Datum stehen nur in `docs/SOURCES.md`,
`docs/MAPS.md`, `docs/MISSIONS.md` und `assets-src/geo/landing_gulf/README.md`.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Rahmen um 42,7444 N / 10,2449 E, nicht gedreht (x Osten, Meer unten), **5,0 × 4,4 km** statt 4 × 4 km. | Die Golfmündung ist 2,5 km breit; so liegen Dorf und Hafen, der ganze Strand, die Ebene bis zum heutigen Flugplatz, die Höhe über dem Leuchtturm (Batterie Poro) und die Höhe über der Ostspitze (Fonza) im Rahmen; der Monte Capanne bleibt draußen. |
| 2 | Höhen aus Copernicus GLO-30 (Kachel N42 E010), Öffnung 5 × 5 Pixel, Unschärfe 14 m zweimal; kein amtliches Geländemodell der Toskana gelesen. | Wie 081/113; Lizenz klar, ohne Anmeldung. |
| 3 | **Meeresboden:** neue Option `coast.beach_bed` (vor Sandstränden −0,3 m, 4 cm je 2 m tiefer, Flachwasser bis −1,0 m, 200 m Reichweite), Felsküste −2 m am Ufer, 0,6 m je Ring (Kante); `beach_rings` 30 führt den Strandanstieg (10 %) ins Land weiter. | LCVP läuft 10–15 m, LCM ~30 m vor dem Ufer auf, die Männer waten; Fels bleibt Kante. |
| 4 | **Stand 1944:** Flugplatz (Bahn nicht gelesen, Hallen/Terminal per Tag weg), Hotels, Campingplätze und die heutige Stadt um den alten Ort (Polygon, 394 Blöcke, 14,9 km Nebenstraßen) entfallen; alter Ort am Hafen und an der Hauptstraße als Polygon (~600 × 400 m, Reihenhäuser, Kirche mit Campanile); sonst Casali, wo OSM Gebäude hat. Keine Höhengrenze nötig (keine hohen Bauten). Jachthafen: Schwimmstege entfallen, die Mole bleibt (1944 genannt). | Nach allgemeiner Ortskenntnis, keine historische Karte. |
| 5 | **Felder** (neue Option `areas.fields`): OSM-Ackerland und Flächen unter Flugplatz/späterer Stadt als Parzellen 96 × 64 m (Stoppeln, Reihen, Pflug, Wiese), höchstens 15 % Steigung; Weinberge wie 120. | Die Ebene war 1944 Feld und Weinbau. |
| 6 | **Mauern und Terrassen:** OSM `barrier=wall` als Feldsteinmauer, `retaining_wall` als Terrassenmauer (Erdseite zur Höhe); Terrassen in Olivenhainen/Weinbergen am Hang (> 12 %) längs der Höhenlinien alle 3 m Höhe. | Prompt Abschnitt 2; neue Optionen `green.walls_from_osm`, `green.terraces`. |
| 7 | **Befestigungen:** sechs Stellungen (zwei an der Batterie Poro, eine bei Fonza nach Quelle; Osthang, Hafen, Strandende von Hand), 253 Drahtstücke in Läufen mit Lücken und zwei Schützenminen-Felder am Strand („minée et barbelée“); keine Igel (keine Quelle). OSM hat im Rahmen keinen Bunker. | Prompt Abschnitt 3. |
| 8 | **Teams Hinterland (Nord, Ebene) gegen Küste (Süd, Dorfrand und Küstenebene)** statt West gegen Ost. | Der Osten hat keine ebene Fläche für einen Start (jede 100-m-Zelle steigt > 10 m auf 64 m), seine Wege zum Strand laufen über die Hügelstraßen; so starten beide auf ebenem Land; Weglängen über alle Punkte ausgeglichen (Nord +90 m Fahrzeug, +370 m zu Fuß). |
| 9 | Sechs Versorgungspunkte: Strand (umkämpft), Hafen/Straßenkreuzung, Höhe über dem Leuchtturm/Osthang, Ebene; neutrale Markernamen. | Prompt nennt Dorf, Strand, Höhen, Ebene, Kreuzung. |
| 10 | Wälder 10 m statt 8 m Abstand. | „Hoch“ über dem dichtesten Pinienwald 45 → 60 FPS nah. |
| 11 | **Mission** „Landing in the gulf“ (`gulf_landing`, US): Kommandos landen in der Felsbucht und nehmen die Westbatterie von hinten (historisch: Batterie Poro ergab sich den Kommandos), erste Welle an beiden Strandenden, Batterien schießen per `barrage`, solange ihr Ziel offen ist; Dorf (Versorgungspunkt) und Ostbatterie gleichzeitig; zweite Welle 57-mm-Pak + Halbkette, dritte zwei Sherman; Gegenstoß StuG + Sd.Kfz. 251 (nicht belegt); Straße nördlich des Dorfes öffnen. Batterie „ausgeschaltet“, wenn zwei Einheiten drinstehen (Bedienungen geben auf, Feuer halten). Entfernungen gestaucht (Reichweiten im Spiel 1/8), leFH 18 statt 90/102 mm. | Erste Fassungen mit Zielen nacheinander gewann der Autopilot nicht in 20 min. |
| 12 | Der Test-Autopilot (`crates/ai/tests/missions.rs`) lässt schwimmende Boote aus seinen Befehlen (aufgelaufene bekommen wieder Befehle); Boote steuert nur das Skript. Der Selbsttest des Spiels bleibt unverändert (mit derselben Änderung verlor er `beach_landing`); er gewinnt die Mission auf Normal nach 1 746 s. | Ein Bewegungsbefehl zu einem Landziel landet ein Boot dort; `beach_landing` wird weiter gewonnen. |
| 13 | Keine Boote im freien Gefecht (Entscheidung 119); der Strand bleibt für die Mission angelegt. | – |
| 14 | KI unverändert. | Turnier ausgeglichen, nichts hängt. |

## Ergebnis

- **Stufe 1:** Overpass-Abfrage (7,0 MB, 17:49 UTC) und Copernicus-Kachel
  N42 E010 (5,6 MB) unter `build/geo121/raw/`; Site
  `assets-src/geo/landing_gulf/` (site.json, osm.json.gz 0,6 MB,
  dem.json.gz 0,2 MB, README). Neue Importer-Optionen `coast.beach_bed`,
  `bed_rings`, `beach_rings`, `areas.fields`, `green.walls_from_osm`,
  `green.terraces`, `fortifications.wire`/`mines`; alle anderen
  Geodatenkarten Bit für Bit gleich (`just gen-geo-check`). Bilder
  `build/geo121/s1/` (stage1, stage1_grid, geodata), `build/geo121/reach*.png`.
- **Stufe 2 – Karte** `data/maps/landing_gulf` („Landung im Golf“ /
  „Landing gulf“, `MAP_LANDING_GULF`): 5,0 × 4,4 km, −10 … 353 m,
  53,9 km Straßen, 538 Gebäude (144 im alten Ort), 7 412 Bäume,
  12 225 Macchia, 331 Terrassen- und 131 Mauerstücke, 6 Stellungen,
  253 Draht, 2 Minenfelder, 162 ha Felder; `just map-check` ok;
  `just map-reach`: 86,8 % Fahrzeuge, 1,4 % nur Infanterie, 98,4 %
  zusammenhängend, alle Starts/Punkte im Hauptteil; Test
  `landing_gulf_plays_and_carries_no_real_names`; Szenario
  `data/scenarios/bench_gulf.toml`; Bilder `build/geo121/report/`
  (stage2, reach, map_reach.txt) und aus dem Spiel `build/geo121/ingame/`
  (25 Ansichten, `SHOT_SET=land just terrain-shots … landing_gulf`).
- **Messungen** (`docs/PERFORMANCE.md` „Landing gulf“): Tick 3,46 ms
  Mittel, p99 6,1, max 8,5 ms; Sicht 0,68 ms; Bildrate hoch nah 60,2 /
  mittel 55,7 / Übersicht 86,8 / Dorf 71,6 FPS, mittel 67–120 FPS;
  path-bench auf dieser Karte nicht nutzbar (Kartenmitte ist Meer).
- **KI** (`docs/AI.md`): 1v1 hard/normal 50/50/0, hard/passive 80/10/1,
  normal/easy 80/20/0; 2v2 hard/normal 50/50, normal/normal 50/50; Basis
  hard/passive 3 von 4; Stuck-Audit 0 von 160.
- **Stufe 3 – Mission** `data/maps/landing_gulf/missions/gulf_landing.toml`
  mit `.de.po`; Autopilot gewinnt Easy (1 085 s) und Normal (1 110 s),
  Hard nicht (nur berichtet); `just selftest` gewinnt sie auf Normal nach 1 746 s; Doku `docs/MISSIONS.md`.
- **Goldener Hash unverändert** (keine Regel geändert, nur Karte, Mission,
  Importer und Test-Autopiloten).
- **Offen:** schwere Küstengeschütze, Kasematten, Flakschiff am Kai als
  Modelle; Bildrate „hoch“ über dem dichtesten Wald bei 120 m 55,7 FPS
  (Schatten); historische Karte von Dorf und Ebene 1944; amtliches
  Geländemodell der Toskana; Boote im freien Gefecht; Gegenstoß nicht
  belegt; Hard der Mission vom Autopiloten nicht gewonnen; kein Jeep im
  US-Katalog (zweite Welle mit Halbkette); `path-bench` setzt Einheiten in
  die Kartenmitte (hier Meer).
