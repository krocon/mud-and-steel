# Prompt 116 – Karte und Mission: Waalbrücke (Vorbild Nijmegen, September 1944)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 115
> (lange Brücken) ist erledigt (`prompts/done/115-*`). Lies zuerst `CLAUDE.md`, dann
> `prompts/done/115-lange-bruecken.md` und `prompts/done/081-karte-hafenstadt.md`
> ganz, die Entscheidungen der erledigten Prompts 112–114 (falls in
> `prompts/done/`), in `docs/MAPS.md` „Maps from real geodata (prompt 081)“,
> „Bridges“ und `harbor_city`, `docs/MISSIONS.md` ganz und eine vorhandene Mission
> (`data/maps/twin_meadow/missions/meadow_crossing.toml` mit `.de.po`). Dann
> `docs/SOURCES.md` (Karten aus Geodaten, Regeln für Abzeichen), `docs/AI.md`,
> `docs/PERFORMANCE.md` („Harbor city“), `tools/geo_import.py` ganz und
> `data/catalog/us.toml` sowie `de.toml` (welche Einheiten es gibt).
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe den Zwischenstand zeigen
> und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte um die **Waalbrücke von Nijmegen** aus echten Geodaten, Stand September
1944, und eine **Mission** dazu. Die Brücke von 1936 steht heute noch; Straßen,
Ufer und ein Teil der Stadt passen daher besser zu 1944 als in Arnhem.

Historischer Rahmen (Operation Market Garden): Amerikanische Fallschirmjäger der
82nd Airborne und britische Panzer der Guards Armoured Division nahmen die Brücke am
20. September 1944, nachdem Fallschirmjäger westlich davon die Waal in Booten unter
Feuer überquert hatten. Die Brücke fiel unbeschädigt in alliierte Hand. Briten gibt
es im Katalog noch nicht (Prompt 117); hier stehen **US-Panzer** für die
Gardetruppen. Die deutschen Verbände stellt der Katalog `de` wie überall, ohne
Abzeichen von SS-Verbänden (`docs/SOURCES.md`).

Die Karte heißt `river_crossing` (im Spiel „Flussübergang“ / „River crossing“), die
Mission ist die erste mit historischem Vorbild. Der Nutzer entscheidet, ob Mission
und Briefing den echten Ort und das Datum nennen (Frage am Ende von Stufe 1 stellen;
bis dahin wie bei `harbor_city` kein echter Name in den Dateien).

## 2. Ausschnitt und Geodaten

Richtwerte, die Session darf sie nach den Stufenbildern anpassen:

- **Ausschnitt** etwa 3,5 km × 3 km: im Süden die Altstadt mit dem Valkhof und dem
  Hunnerpark am Brückenkopf, die Waal (gut 250 m breit) mit der Brücke, im Norden
  die Polder bei Lent mit dem Deich. Die Eisenbahnbrücke westlich nur, wenn sie
  ohne Mehraufwand in den Ausschnitt fällt (Eisenbahnbrücken sind nicht Teil von
  115; dann als Kulisse ohne Fahrbahn oder draußen lassen).
- **Rahmen** so gedreht, dass die Brückenachse auf einer Kartenachse liegt
  (Prompt 115).
- **Höhen:** Copernicus DEM wie in 081 oder, wenn Prompt 114 einen Weg für
  amtliche Geländemodelle gebaut hat, das niederländische AHN (offen, RD-Koordinaten
  EPSG:28992; Lizenz prüfen und in `docs/SOURCES.md` festhalten). Die Deiche
  (wenige Meter hoch, 30–50 m breit) müssen in der Karte als Deiche erkennbar sein;
  mit 30 m DEM notfalls aus OSM (`man_made=dyke`, `embankment=yes`) aufschütten.
- **Stand 1944:** Die Innenstadt wurde im Februar 1944 bei einem Luftangriff schwer
  getroffen und nach dem Krieg teils neu bebaut. Moderne Großbauten, die neue
  Stadtbrücke („De Oversteek“, 2013) und Neubaugebiete im Norden (Lent ist heute
  stark bebaut, 1944 Dorf und Polder) weglassen. Wo das Baujahr fehlt, die
  Höhe begrenzen wie in Prompt 112 und die Entscheidung festhalten. Historische
  Topokarten der 1940er (Kadaster, Lizenz prüfen) und RAF-Luftbilder von 1944
  (Crown Copyright erloschen) als Referenz für das Gebiet am Brückenkopf und im
  Polder nutzen.

## 3. Karte (frei spielbar)

- Zwei Teams: Süd (Stadt) gegen Nord (Polder), je ein oder zwei Starts.
- Versorgungspunkte: Brückenkopf Süd (Hunnerpark), Brückenkopf Nord, Valkhof,
  Deich bei Lent, ein Punkt am Ufer westlich (Bootsübergang).
- Die Brücke ist der einzige Weg für Fahrzeuge über die Waal. Infanterie kann nicht
  schwimmen (Prompt 055: Ertrinken); Boote gibt es nicht. Das ist für die freie
  Karte zu einseitig, wenn eine Seite die Brücke hält. Die Session schlägt eine
  Lösung vor (z. B. Brückenleger und gelegte Brücken nach 055 an einer ruhigen
  Stelle zulassen, oder Starts so legen, dass beide Seiten auf beiden Ufern Fuß
  fassen) und prüft sie im KI-Turnier.
- `just map-check`, `just map-reach`, Messungen und KI-Turnier wie in 081.

## 4. Mission

- **Seite:** Alliierte (US), gegen die KI.
- **Ablauf (Vorschlag):** Phase 1 Häuserkampf in der Altstadt bis an den
  Brückenkopf (Valkhof, Hunnerpark); Phase 2 eine Gruppe Fallschirmjäger erscheint
  am Nordufer westlich der Brücke (die Überquerung in Booten wird nicht gespielt,
  Missionsaktion `spawn`, eine kurze Meldung erzählt sie); Phase 3 Panzer stoßen
  über die Brücke vor, die Fallschirmjäger nehmen den Nordkopf; Sieg, wenn beide
  Brückenköpfe gehalten werden und die Brücke steht.
- **Sprengung:** Die Verteidiger haben die Brücke zur Sprengung vorbereitet. Ein
  Missionsauslöser kann sie sprengen, wenn der Spieler zu lange braucht (Zeit oder
  Bedingung); dann ist die Mission verloren. Die genaue Regel schlägt die Session
  vor.
- Briefing, Meldungen und Ziele auf Deutsch und Englisch (`.de.po` wie die anderen
  Missionen), Missionsliste im Menü, ein Durchlauf-Test wie bei den anderen
  Missionen.

## 5. Stufen

**Stufe 1 – Geodaten, Gelände, Brücke:** Abfrage, Extrakt, Höhen, Waal, Deiche,
Straßen, Brücke nach 115, Stufenbilder, `just map-reach`. Frage an den Nutzer zur
Nennung des echten Ortes.

**Stufe 2 – Stadt, Polder, freie Karte:** Gebäude (Stand 1944), Bewuchs, Starts,
Versorgungspunkte, Menüname, Tests, Messungen, KI-Turnier, Doku.

**Stufe 3 – Mission:** Skript, Texte, Test, Doku in `docs/MISSIONS.md`. Danach
`just check`.

## 6. Nicht Teil dieses Prompts

- Britische Einheiten (Prompt 117), danach die Gardetruppen statt US-Panzern.
- Sturmboote und die Flussüberquerung als spielbarer Teil.
- Eisenbahnbrücke und Züge.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 23.11.2026)

Niemand war erreichbar; diese Punkte hat die Session selbst entschieden
(alle vorläufig). Die Frage aus Stufe 1 (echten Ort und Datum nennen?)
blieb unbeantwortet: Karte, Mission, Menü und Übersetzungen tragen
keinen echten Namen, nur `docs/SOURCES.md`, `docs/MAPS.md`,
`docs/MISSIONS.md` und `assets-src/geo/river_crossing/README.md` nennen
Nijmegen.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Rahmen um 51,85334 N / 5,87009 E (Mitte der Brücke), x-Achse 70,5°, 3,6 × 3,2 km: die Brücke läuft auf der y-Achse (oben ist 19,5° westlich von Nord), Altstadt unten links, Polder und Dorf oben, Wiesen rechts. | Prompt 115: Brückenachse auf einer Kartenachse; so passen Altstadt mit Burgberg und Park, die ganze Brücke, Polder mit Dorf und der Altarm im Osten hinein. |
| 2 | Höhen aus **AHN4 DTM 0,5 m** (CC0) über den PDOK-Kachel-Dienst (WCS) in 30 Kacheln à 1 km bei 1 m, umgerechnet über ein RD-Polynom (Schreutelkamp/Strang van Hees, ≈ 0,25 m, nur `+ - *`) auf dem Gitter aus 114; Lücken unter Häusern und Wasser ringweise aufgefüllt; Höhenversatz −8 m (Waal ≈ NAP +8 m als Wasserstand 0). | Prompt Abschnitt 2 (AHN statt Copernicus); Deiche (wenige Meter, 30–50 m breit) sind damit da, ohne sie aufzuschütten. |
| 3 | Fluss aus den OSM-Wasserflächen (kein Meer, `sea_seeds_m` leer), Seen und Teiche ab 3 000 m². | Die Waal ist in OSM ein Multipolygon; Küstenlinien gibt es nicht. |
| 4 | Stand 1944 am Nordufer: die Nebenrinne von 2013–16, ihr Einlass und zwei spätere Teiche entfallen (`skip_water_at_m`); Rinne, Insel und verlegter Deich als Polder (höchstens +2,7 m, `dry_polygons_m`, Polygon aus dem Umriss der Rinne + 130 m nach Norden); der alte Deich auf der früheren Insel bleibt und trifft den Brückendamm. | Prompt: Stand 1944; die Daten zeigen den alten Deich auf der Insel. |
| 5 | Gebäude: OSM trägt (aus dem Gebäuderegister) fast überall das Baujahr. Nordufer und östliche Wiesen: alles ab 1946 entfällt (4 675 Grundrisse, die Neustadt im Norden; 157 Häuser des alten Dorfs bleiben); Stadt: alle Grundrisse als Stellvertreter auf dem alten Straßenplan, getaggte Höhe höchstens 22 m; Altstadt 12 % Ruinen, 20 % beschädigt (Luftangriff Februar 1944 und Kämpfe). | Baujahr vorhanden; in der Stadt hätte Weglassen die nach dem Krieg wieder bebauten Blöcke leer gelassen. |
| 6 | Nebenstraßen der Neubaugebiete im Norden entfallen (Polygon, 240 Wege, 27,8 km), die Hauptstraßen bleiben. | Wie 114 („Straßen später gebaut“). |
| 7 | Eisenbahnbrücke (liegt im Ausschnitt, 0,9 km westlich) weggelassen, ihre Dämme enden am Fluss; moderne Stadtbrücke und Radbrücken entfallen. | Prompt: Eisenbahnbrücke nur ohne Mehraufwand; es gibt kein Modell. |
| 8 | Brücke auf x = 9 m von y = −24 bis 580 m: 3 × `Span60` + `Span30` (Vorland und nördlicher Flussteil), `Arch244` über der Fahrrinne, 2 × `Span60` + `Span30` über Kai und Hang = 604 m wie das Vorbild; Nordende Damm 14 m über dem Fluss, der in den alten Deich übergeht; Südende auf der Terrasse der Stadt (18,4 m) mit eingeebneter Fläche davor (neue Option `level_high_ends`). | Prompt 115; ohne die Fläche fuhren schwere Panzer seitlich an das Deckende und blieben hängen (Stuck-Audit 3 → 1 von 100 Befehlen). |
| 9 | Der Altarm östlich der Brücke trennte die Wiesen von der Stadt: ein Damm an der Straßenquerung (`land_strips_m`) und allgemein `causeway_max_m` (Straßen über Wasser bis 60 m laufen auf einem Damm). | Kleine Brücken aus OSM gibt es im Spiel nicht. |
| 10 | Freie Karte **West gegen Ost auf beiden Ufern** statt Süd gegen Nord: Start 0 (Stadt) und 1 (Wiesen) am Südufer, 2 (Polder West) und 3 (Polder Ost) am Nordufer; im 1v1 beide am Südufer, im 2v2 hat jedes Team auf jedem Ufer einen Start. Brückenleger helfen nicht (Fluss 250–400 m breit). | Prompt Abschnitt 3, Vorschlag „Starts auf beiden Ufern“; Turnier: Spiele enden, beide Seiten fahren über die Brücke. |
| 11 | Sechs Versorgungspunkte: Brückenkopf Süd (Park) und Nord umkämpft, Paare Burgberg / Wiese Ost und Westufer (Landestelle der Boote) / Deich Nord; neutrale Namen. Über alle Punkte Ost mit Fahrzeugen 329 m länger. | Prompt nennt fünf Orte; der sechste gleicht die Wiese für Ost aus. |
| 12 | Mission „The great bridge“ / „Die große Brücke“ (`bridge_assault`): Spieler beginnt 300 m südlich der Brücke (4 Sherman, 3 Schützen, MG), Südwache auf der Straße zwischen Burgberg und Park, Fallschirmjäger erscheinen 200 m westlich des Nordendes, drei Ziele am Nordende. | Erste Fassungen (Wache auf dem Burgberg-Plateau, Spieler in der Altstadt) gewann der Autopilot nicht in 20 Minuten (steile Hänge, Trupps im Park nicht sichtbar). |
| 13 | **Sprengregel:** Zünder 10 Minuten nach der Landung (Easy 15, Hard 7,5), solange der Sprengtrupp (Pioniere hinter dem Damm) lebt; dann stürzen Bogen und Nachbarfeld ein; jedes Brückenteil unten (auch durch eigenen Beschuss) = Niederlage. Neue Skriptbausteine `demolish` (Aktion) und `bridge_down` (Bedingung). | Prompt Abschnitt 4; den Trupp ausschalten ist eine Handlung, die der Spieler sieht (Zeit im Panel). |
| 14 | Historische Karte 1944 (Kadaster Topotijdreis) nur angesehen; ihre Kacheln ließen sich nicht genau über den Rahmen legen (≈ 300 m und Maßstab daneben); nichts daraus abgezeichnet. RAF-Luftbilder nicht verwendet. | Lizenz im Dienst nicht genannt; Lage unsicher. |
| 15 | Biom `forest`; KI unverändert. | Wie 112–114. |

## Ergebnis

- **Stufe 1 – Geodaten, Gelände, Brücke:** Overpass-Abfrage (52 MB,
  11:01 UTC) und 30 AHN-Kacheln (61 MB) unter `build/geo116/raw/`;
  Importer `tools/geo_import.py`: `extract --ahn` (RD-GeoTIFF, Lücken
  füllen), RD-Umrechnung, Optionen `offset_m`, Fluss ohne Meer,
  `skip_water_at_m`, `dry_polygons_m`, `land_strips_m`,
  `causeway_max_m`, `minor_skip_polygons_m`, `drop_from_year`/
  `drop_postwar`, Bezirks-`layout` und Ruinenanteile,
  `level_high_ends`; alle anderen Geodatenkarten Bit für Bit gleich.
  Bilder `build/geo116/stage1/` (`stage1.png`, `stage1_grid.png`,
  `crop_north.png`, `crop_south.png`, `topo1944.png`,
  `topo1944_shift.png`, `reach.png`).
- **Stufe 2 – Karte** `data/maps/river_crossing` („Flussübergang“ /
  „River crossing“, `MAP_RIVER_CROSSING`): 3,6 × 3,2 km, −6 … 40,2 m,
  82 km Straßen, 3 944 Gebäude, 4 372 Bäume, Brücke aus 8 Teilen (604 m);
  `just map-check` ok; `just map-reach`: 85,4 % Fahrzeuge, 1,4 % nur
  Infanterie, 4,7 % niemand, 8,6 % Gebäude, 98,8 % zusammenhängend über
  die Brücke; Test `river_crossing_plays_and_carries_no_real_names`;
  `just gen-geo-check` in `just check` und CI; Menü-Vorschau im
  Selbsttest gebaut; Bilder `build/geo116/report/` (`stage2.png`,
  `stage2_bridge.png`, `reach.png`, `map_reach.txt`).
- **Messungen** (`docs/PERFORMANCE.md` „River crossing“): Tick 4,30 ms
  Mittel, p99 5,6, max 6,5 ms (254 Einheiten); Wegsuche 56 600 Knoten je
  Anfrage, max 6,3 ms; Sicht 4,8 ms; Bildrate mittel 69–120 FPS, hoch
  30–98 (Schatten am Wald); 14,4 MB Speicher.
- **KI** (`docs/AI.md` „The river crossing“): 1v1 hard/normal 10/50/4,
  hard/passive 70/30/0, normal/easy 60/40/0, normal/normal 30/30/4; 2v2
  hard/normal 16/33/3, normal/normal 33/0/4; Basis hard/passive 4 von 4;
  Stuck-Audit 1 von 100 Befehlen (Tiger am Südende des Decks).
- **Stufe 3 – Mission** `data/maps/river_crossing/missions/bridge_assault.toml`
  mit `.de.po`; Sim: Aktion `demolish`, Bedingung `bridge_down`
  (`crates/sim/src/mission.rs`, `sim/mission.rs`, `sim/crossings.rs`
  `demolish_bridges`, Bridge-Ziel in `mission_funcs.rs`); Tests
  `long_bridges::a_mission_blows_up_the_arch_and_is_lost` und Fehler in
  `missions::errors_name_what_is_wrong`; Autopilot gewinnt Easy, Normal
  und Hard; `just selftest` spielt sie aus dem Menü (gewonnen nach
  1 088 s). Doku `docs/MISSIONS.md`.
- **Goldener Hash unverändert** (Missionszustand nur mit Mission im
  Hash; keine Regel geändert).
- **Offen:** Eisenbahnbrücke als Kulisse; Boote und die Überquerung als
  spielbarer Teil (119); Britische Gardetruppen (117) statt US-Panzern;
  Obstgärten im Polder; historische Karte genau georeferenzieren
  (Stand 1944 von Deich und Dorf nur aus heutigen Daten); Unterführung
  der Kaistraße; ein schwerer Panzer bleibt im Stuck-Audit am Deckende
  hängen; Bildrate „hoch“ am Wald (Schatten); Schlacht-Bildrate in Godot
  nicht gemessen; KI nutzt Brücken nicht taktisch (Sprengen, Halten);
  West im Gleichstand leicht im Vorteil.
