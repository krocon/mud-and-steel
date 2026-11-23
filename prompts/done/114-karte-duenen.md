# Prompt 114 – Karte nach echten Geodaten: Dünen (Vorbild Kampen auf Sylt)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/081-karte-hafenstadt.md` ganz (Ziel, Stufen, Entscheidungen, Ergebnis,
> Abschnitt 11), in `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und
> `harbor_city` ganz, dazu die Abschnitte zu Geländearten, Hangregeln, Biomen und
> Objekten (`fort_*`). In `docs/ARCHITECTURE.md` „Vision and fog of war“ und die
> Abschnitte zu Wegfindung und Hängen. Dann `docs/SOURCES.md` (Karten aus Geodaten),
> `docs/PERFORMANCE.md` („Harbor city“), `docs/AI.md` („The harbor city“),
> `data/terrain.toml`, `data/objects.toml`, `tools/geo_import.py` ganz und
> `assets-src/geo/harbor_city/site.json`. Sind Prompt 112 oder 113 schon erledigt
> (`prompts/done/`), auch deren Entscheidungen lesen.
>
> **Dieser Prompt läuft in drei Stufen.** Nach Stufe 1 und nach Stufe 2 den
> Zwischenstand zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte in einer **Dünenlandschaft** aus echten Geodaten. Vorbild ist
**Kampen auf Sylt**, um 1944 und vereinfacht wie `harbor_city`: im Westen Strand,
Nordsee und das Kliff, in der Mitte Dünen und Heide mit dem Dorf, im Osten das
Wattenmeer.

Gespielt wird auf **kurze Sicht**: Dünen von 10–30 m Höhe, einzelne bis gut 50 m,
dazwischen Mulden und Täler im Abstand von 50–200 m. Die Sicht ist ständig
unterbrochen; Infanterie und Panzerabwehr kämpfen aus dem Hinterhalt, Fahrzeuge
kommen im Sand nur langsam voran und an steilen Dünenhängen nicht weiter. Das ist
gewollt das Gegenteil der Küstenhöhen aus Prompt 113 mit ihren weiten Blicken.

Dafür reicht das Copernicus DEM (30 m) nicht: Es macht aus den Dünen weiche Buckel.
Der Prompt erweitert deshalb den Importer um ein **amtliches Geländemodell mit 1 m
Auflösung** (DGM1 Schleswig-Holstein). Davon haben auch spätere Karten in
Deutschland etwas.

Die Karte heißt `dune_coast` (im Spiel „Dünenküste“ / „Dune coast“). Wie bei
`harbor_city` steht der echte Ort nur in `docs/SOURCES.md`, `docs/MAPS.md` und
`assets-src/geo/dune_coast/`, nie in Kartendateien, Markern, Menüs oder
Übersetzungen.

Fertig ist der Prompt, wenn

- `geo_import.py` ein DGM aus UTM-Kacheln lesen kann (Abschnitt 2) und
  `harbor_city` dabei Bit für Bit gleich bleibt,
- `just gen-geo dune_coast` die Karte schreibt, Bit für Bit wiederholbar und in
  `just gen-geo-check` aufgenommen,
- die Dünen in der Karte als Dünen erkennbar sind: Vergleich Copernicus gegen DGM1
  als Bild und Höhenprofil quer über die Insel (in `docs/MAPS.md`),
- `just map-check dune_coast` besteht und `just map-reach dune_coast` alle Starts und
  Versorgungspunkte im Hauptteil zeigt,
- Messungen und KI-Turnier wie in Prompt 081 dokumentiert sind,
- die Karte im Menü wählbar ist und die Tests wie bei `harbor_city` laufen (laden,
  Editor-Rundweg, Wege, kein echter Name in den Kartendateien).

## 2. Geländemodell DGM1

- **Quelle und Lizenz zuerst:** Das Landesamt für Vermessung und Geoinformation
  Schleswig-Holstein bietet das DGM1 als offene Daten an. Lizenz, Namensnennung und
  Bezugsweg prüfen und in `docs/SOURCES.md` festhalten, **bevor** Daten geladen
  werden. Ist die Lizenz nicht frei genug (Weitergabe der reduzierten Daten im Repo,
  Verwendung in einem Spiel), anhalten und den Nutzer fragen.
- **Format:** Das DGM1 kommt in Kacheln (meist 1 km × 1 km) in ETRS89 / UTM Zone 32N
  (EPSG:25832), als GeoTIFF oder XYZ-Text. Höhen über NHN in Metern.
- **Importer:** `geo_import.py extract` bekommt eine zweite DEM-Quelle: mehrere
  Kacheln einlesen, zusammensetzen, auf das Begrenzungsrechteck zuschneiden und in
  den Rahmen der Karte umrechnen (UTM → geographische Koordinaten → Kartenrahmen,
  oder direkt UTM → Rahmen, wenn das einfacher und genau genug ist; die Abweichung
  messen und festhalten). Nur Standardbibliothek, deterministisch wie bisher (Floats
  nur mit `+ - * /` und `sqrt`, eigene Reihen für Winkelfunktionen).
- **Reduzieren:** Für das 2-m-Raster der Karte reicht ein 2-m-Auszug des DGM1
  (Mittel aus 2 × 2). Er wird wie `dem.json.gz` in Git LFS abgelegt. Die Größe messen;
  bei mehr als etwa 20 MB auf 16-Bit-Werte mit Basis und Schritt packen.
- **Kein Gebäudeabzug:** Das DGM ist ein Geländemodell ohne Häuser und Bäume. Die
  Abzugsstufe aus 081 (Gebäudehöhen, Median, graue Öffnung) für diese Quelle
  abschalten, die Glättung nur so weit, dass die Wegfindung keine Treppen sieht.
- **Wasser:** Das DGM zeigt Land. Die Nordsee vor dem Strand und das Watt kommen aus
  OSM (Küstenlinie) wie in 081. Den Meeresboden wie dort in Ringen anlegen.

## 3. Ausschnitt und Rahmen

Richtwerte, die Session darf sie nach den Stufenbildern anpassen:

- **Begrenzungsrechteck:** etwa 54,935–54,980° N, 8,300–8,390° E (von der Nordsee
  über Kliff, Dünen und Dorf bis ins Watt; im Norden Richtung List, im Süden bis an
  den Rand von Wenningstedt).
- **Rahmen:** x-Achse entlang der Insel (etwa Nord-Süd), sodass die Nordsee am einen,
  das Watt am anderen Bildrand liegt. Größe etwa 4 km × 3 km.
- **Höhen:** etwa −2 bis gut 50 m an Land, dazu der Meeresboden. Die
  Höhenkodierung (`height_step_mm`) entsprechend wählen; 1 mm Schritte reichen für
  65 m Relief.

## 4. Boden und Bewuchs

- **Strand** (`sand`) am Westrand, das **Kliff** als steile Kante (nicht passierbar),
  die Abbruchkante aus dem DGM übernehmen.
- **Dünen:** offener Sand (`sand`) dort, wo OSM `natural=sand` oder `natural=dune`
  zeigt, sonst bewachsene Dünen mit Strandhafer und Heide (`tall_grass`). Steile
  Leehänge über 60 % bleiben gesperrt; das ergibt sich aus dem DGM und den
  Hangregeln, nicht von Hand.
- **Heide** (`natural=heath`) als `tall_grass`, Wiesen hinter dem Deich als `grass`.
- **Watt:** Das Watt wechselt mit den Gezeiten. Gezeiten gibt es nicht. Vorschlag:
  ein Streifen `mud` (Schlick) zwischen Ufer und tiefem Wasser, außen
  `shallow_water`, Priele als `deep_water`. Die Breite aus OSM (`natural=wetland`,
  `wetland=tidalflat`), sonst als Entscheidung festhalten.
- **Bäume:** wenige. Nur kleine Kiefernforste und Gehölze um das Dorf aus OSM, keine
  Bäume in den offenen Dünen.
- **Biom:** `forest` (es gibt kein Küsten-Biom). Prüfen, wie Sand und Heide damit
  aussehen; ein Bild aus dem Spiel festhalten. Ein eigenes Küsten-Biom ist ein
  offener Punkt.

## 5. Stand 1944, Dorf und Befestigungen

- **Dorf:** Die Häuser aus OSM wie in 081 (wenige, niedrig, verstreut). Platzhalter
  aus dem Katalog (kleine Häuser, Höfe). Reetdachhäuser als eigenes Modell sind ein
  offener Punkt.
- **Weglassen:** moderne Ferienanlagen, Parkplätze am Strand, Strandübergänge aus
  Holz, Hotels und alles, was offensichtlich nach 1945 gebaut wurde. OSM trägt das
  Baujahr selten; wo unklar, behalten und als Entscheidung festhalten.
- **Befestigungen:** Sylt war im Krieg befestigt (Bunker und Flakstellungen in den
  Dünen, der Fliegerhorst südlich von Kampen). Gekämpft wurde dort nicht; die Karte
  ist ein erfundenes Gefecht. Bunker aus OSM (`military=bunker`) an ihren Orten als
  `fort_bunker_mg`, dazu `fort_trench_section` und `fort_sandbag_position` in
  sinnvollen Gruppen. Keine Hoheitszeichen oder Symbole aus dieser Zeit an
  Gebäuden oder Objekten (Regeln in `docs/SOURCES.md`).
- **Fliegerhorst:** Liegt er im Ausschnitt, als offene Fläche mit Rollbahn (`road`)
  und Hallen aus dem Katalog; sonst draußen lassen.

## 6. Spiel

- **Starts:** zwei Teams an den Enden entlang der Insel (Nord gegen Süd), je zwei
  Starts, eben wie in 081 (34 m), 80 m frei. Plätze auf Heide oder Wiese, nicht
  mitten in den Dünen.
- **Wege:** mindestens zwei getrennte Fahrzeugrouten zwischen den Teams (die
  Hauptstraße über die Insel und eine zweite, z. B. am Watt oder über die Heide).
  Die Dünen dazwischen sollen für Infanterie offen sein, für Fahrzeuge
  stellenweise.
- **Versorgungspunkte** in Paaren nach Weglänge ausgeglichen: Dorf (umkämpft in der
  Mitte), höchste Düne, Kliff, Heide, Rand des Watts. Für die Punkte auf Dünen das
  Sichtfeld aus der Simulation als Bild festhalten.
- **KI-Turnier** wie in 081, Ergebnis in `docs/AI.md`: Bleiben Fahrzeuge im Sand
  stecken oder fahren sie endlose Umwege? Probleme beschreiben und als offenen Punkt
  notieren, die KI nicht umbauen.

## 7. Stufen

**Stufe 1 – DGM1 im Importer:** Lizenz klären, Kacheln laden, `extract` mit UTM-Kacheln,
Umrechnung, Reduktion; Vergleich Copernicus gegen DGM1 (Bild, Profil quer über die
Insel); `harbor_city` unverändert (`just gen-geo-check`). Zwischenstand zeigen.

**Stufe 2 – Gelände, Wasser, Straßen, Rahmen:** `site.json`, Strand, Kliff, Dünen,
Heide, Watt, Straßen, Starts und Marker; Stufenbilder, Steigungskarte,
`just map-reach`. Zwischenstand zeigen.

**Stufe 3 – Dorf, Bewuchs, Befestigungen, Spiel:** Abschnitt 4 bis 6, Menüname
und Übersetzung, Tests, Messungen (Bildrate in jeder Ansicht, Tick mit vielen
Einheiten, Speicher), KI-Turnier, Doku (`docs/MAPS.md` mit Tabelle „was die
Vereinfachung behielt“ und Abschnitt zur zweiten DEM-Quelle, `docs/SOURCES.md`,
`docs/PERFORMANCE.md`, `docs/AI.md`). Danach `just check`.

Golden Hash: Ein reiner Kartenzusatz ändert ihn nicht. `harbor_city` muss beim
Importer-Umbau gleich herauskommen; tut es das nicht, ist das ein Fehler, keine
Regeländerung.

## 8. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Küsten-Biom** (Strandhafer, Heide, helles Licht, Nordseefarben).
- **Reetdachhaus** und weitere Bauten der Insel als eigene Modelle.
- **Gezeiten** (Watt wird zu Wasser und wieder zu Schlick) als Spielregel.
- **Wandernder Sand** oder Spuren in den Dünen.
- **Weitere Karten mit DGM1** aus anderen Bundesländern (meist dasselbe Format).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 23.11.2026)

Niemand war erreichbar; diese Punkte hat die Session selbst entschieden
(alle vorläufig):

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Lizenz geklärt vor dem Laden: Nutzungsbedingungen des OpenGBD-Portals (Stand 1.8.2022) sind CC BY 4.0, Namensnennung „© GeoBasis-DE/LVermGeo SH/CC BY 4.0 (Quelle verändert)“; reicht für den reduzierten Auszug im Repo und die Verwendung im Spiel, daher kein Halt. | Abschnitt 2; Vermerk in `docs/SOURCES.md`, `CREDITS.md`, README des Orts. |
| 2 | Bezug als `x y z`-Text (die Portal-Kacheln für den Massendownload), 28 Kacheln à 1 km² (749 MB unter `build/`), Kachelliste aus dem GeoJSON-Index des Portals; 23 Kacheln Befliegung 2024, 5 am Rand von 2005. | Das Portal bietet DGM1 nur als XYZ an; GeoTIFF wäre nicht einfacher gewesen. |
| 3 | Umrechnung Kartenrahmen → Geographisch (exakte Umkehrung des Rahmens) → UTM (Snyder-Reihe, nur `+ - * / sqrt`, eigene Sinusreihe) auf einem Gitter alle 16 Samples (32 m), dazwischen bilinear. Gemessen: Gitter gegen exakt < 1 mm, eine einzige Affinabbildung wäre bis 1,0 m daneben; Formel gegen das EPSG-Beispiel (GN 7-2) 1 cm. | Konsistent mit OSM (das ebenfalls über den Rahmen projiziert wird); direktes UTM → Rahmen affin wäre 1 m daneben gewesen. |
| 4 | 2-m-Auszug als Mittel aus 2 × 2 Pixeln in cm, nur über dem Rahmen plus 100 m (Zellen weiter weg leer): `dem.json.gz` 4,2 MB, kein 16-Bit-Packen nötig. | Abschnitt 2 (Grenze etwa 20 MB). |
| 5 | Kein Gebäudeabzug, keine Median-/Öffnungsfilter; nur ein Kastenfilter über 6 m (Radius 1 Sample, ein Durchgang). | Das DGM hat keine Häuser und Bäume; der Filter nimmt nur die 1-cm-Stufen und Einzelpixel. |
| 6 | Rahmen um 54,9570 N / 8,3420 E, x-Achse 15° (entlang der Insel, Norden rechts), 4,6 × 3,2 km statt 4 × 3 km (2 301 × 1 601 Samples, 18,4 MB). | So liegen Strand und Nordsee oben, das Watt unten, die Dünen nördlich und südlich des Dorfs ganz im Bild; 4 km hätten den Dünengürtel im Norden abgeschnitten. |
| 7 | Watt: Das DGM hat dort nur eine Füllhöhe (um +1,3 m), also keine Priele. Statt dessen aus OSM `wetland=tidalflat` über dem Wasser: Schlick (`mud`) bis 240 m vom Ufer (0,6 m am Ufer bis 0,15 m außen), dann 80 m `shallow_water`, dann tiefes Wasser; Priele als `deep_water` entfallen. | Abschnitt 4 (Vorschlag), Breite als Entscheidung, weil weder OSM noch DGM eine Grenze hergeben. |
| 8 | Offener Sand nur, wo OSM `natural=sand`/`dune` zeigt (Option `areas.open_sand`): im Rahmen keine Fläche über 1 500 m², also bleiben die Dünen `tall_grass` (Heide und Strandhafer), Sand ist der Strand. | Abschnitt 4: aus den Daten, nicht von Hand. |
| 9 | Stand 1944: Häuser nur im alten Ortskern (Bezirk von etwa 1,5 × 0,9 km), höchstens 270 Katalog-Häuser (Bauernhäuser, Backstein, Stein; 260 stehen), alle übrigen Grundrisse (Ferienhäuser, Nordrand des Nachbarorts) entfallen; Wohnstraßen als Sandwege (`dirt_track`, 6 m), Straßen im Nordrand des Nachbarorts weggelassen (Polygon), Zufahrten, Parkplätze, Fuß- und Strandwege nach Klasse weg; Golfplatz bleibt offenes Gelände. | Nach Ortskenntnis, keine historische Karte; „wenige, niedrig, verstreut“. |
| 10 | Befestigungen: OSM kennt im Rahmen keinen Bunker (die eine Bunkerruine liegt 75 m nördlich außerhalb). Zehn erfundene Stellungen auf Dünenkämmen (je Bunker, drei Grabenstücke, zwei Sandsackstellungen, Blick zur See), in `site.json` als erfunden markiert. Fliegerhorst liegt außerhalb. | Abschnitt 5 verlangt Gruppen; ohne Daten nur als Platzhalter. |
| 11 | Starts: Süd (−2100, 200) und (−2100, 600) auf Feldern/Heide der Geest, Nord (2080, 300) und (1700, 470) auf Wiese/Marsch zwischen Dünen und Watt; je 34 m eben, 80 m frei. | Abschnitt 6: an den Enden, nicht mitten in den Dünen. |
| 12 | Acht Versorgungspunkte: Dorf und höchste Düne umkämpft (beide etwas näher am Süden), Paare Kliff/Düne Nord, Heide Süd/Heide Nord, Watt Süd/Watt Nord; neutrale Namen (`supply_high_dune` usw.). Über alle Punkte Nord 29 m kürzer mit Fahrzeugen, Süd 537 m kürzer zu Fuß. Die Punkte auf Kliff und Norddüne liegen 20–30 m neben der Kuppe am höchsten Platz, den Fahrzeuge erreichen (auf der Kuppe selbst wären sie ein eigener, abgeschnittener Teil des Wegenetzes). | Abschnitt 6, Paare nach Weglänge. |
| 13 | Zweite Fahrzeugroute: kein eigener Weg nötig; 94,6 % des Landes sind für Fahrzeuge offen, Heide und die Marsch am Watt tragen Fahrzeuge neben der Hauptstraße. | `map-reach`. |
| 14 | Neue Importer-Optionen nur bei gesetzten Schlüsseln (`extract --dgm`, `elevation` mit UTM-Quelle, `coast.tidal_flat`, `areas.open_sand`, `compare-dem`); Copernicus-Kacheln mit rechteckigen Pixeln (über 50° Breite) liest `extract_dem` jetzt auch; `harbor_city`, `brick_quarter`, `coastal_heights` bleiben Bit für Bit gleich. | Abschnitt 2 und 7. |
| 15 | Biom `forest`; KI unverändert. | Abschnitt 4 und 6. |

## Ergebnis

- **Stufe 1 – DGM1 im Importer:** `geo_import.py extract --dgm` liest
  UTM-XYZ-Kacheln (28 Kacheln DGM1, 749 MB unter `build/geo114/raw/`),
  2-m-Auszug `dem.json.gz` 4,2 MB (Git LFS), Umrechnung über ein
  32-m-Gitter (Fehler < 1 mm; eine einzige Affinabbildung wäre 1,0 m
  daneben); `generate` erkennt die Quelle am Format, kein Gebäudeabzug,
  ein 6-m-Kastenfilter. `harbor_city`, `brick_quarter` und
  `coastal_heights` bleiben Bit für Bit gleich. Neues `just
  gen-geo-compare` (`geo_import.py compare-dem`): Copernicus gegen DGM1
  als Bild und Profil durch die höchste Düne; Copernicus: Kuppe 44,9 m
  statt 50,3 m, mittleres Gefälle 5,6 % statt 8,1 %, steiler als 60 %
  0,0 % statt 0,61 % des Landes. Bilder unter `build/geo114/stage1/`
  (`dem_compare.png`, `dem_profile.png`, `.csv`, `dgm_raw.png`), Auszug
  in `docs/MAPS.md` (`docs/images/dune_coast/`).
- **Stufe 2 – Gelände, Wasser, Straßen, Rahmen:** `site.json`, Strand,
  Kliff und Dünen aus dem DGM, Heide als hohes Gras, Watt als Schlick
  und Flachwasser (Option `coast.tidal_flat`), 29,5 km Straßen und
  Sandwege, Starts und Marker; Bilder unter `build/geo114/stage2/`
  (`geodata.png`, `stage1.png`, `stage1_grid.png`, `stage2.png`,
  `reach.png` als Steigungskarte, `map_reach.txt`).
- **Stufe 3:** Karte `data/maps/dune_coast` („Dünenküste“ / „Dune
  coast“, `MAP_DUNE_COAST`): 4,6 × 3,2 km, −6 … 50,3 m, 260 Häuser im
  alten Ortskern, 1 753 Bäume, 10 erfundene Stellungen (10 Bunker, 29
  Grabenstücke, 20 Sandsackstellungen), 93,6 ha Schlick; vier Starts
  Süd gegen Nord, acht Versorgungspunkte. `just map-check` ok; `just
  map-reach`: 94,6 % Fahrzeuge, 0,7 % nur Infanterie, 4,1 % niemand
  (Kliff, Leehänge), 99,9 % zusammenhängend, alle Starts und Punkte im
  Hauptteil. `just gen-geo-check` in `just check` und CI; Test
  `dune_coast_plays_and_carries_no_real_names`; Menü-Selbsttest baut die
  Vorschau der Karte (`dune_coast 575x400 build 375 ms`); Benchmark
  `bench_dune.toml`; Bilder aus dem Spiel `build/geo114/shots_named/`;
  Sicht `build/geo114/report/sight.png`.
- **Messungen** (`docs/PERFORMANCE.md` „Dune coast“): Tick 4,15 ms Mittel,
  p99 6,6, max 7,1 ms; Wegsuche 126 000 Knoten je Anfrage, keine Spitzen;
  Sicht 4,7 ms; Speicher 18,4 MB. Bildrate mittel über der Karte
  66–114 FPS, Schlacht 72–98, stehende Armeen 49–50 FPS; hoch über dem
  Wald 30 FPS (Gras-Schatten, ohne 78).
- **KI** (`docs/AI.md` „The dune coast“): hard/normal 40/40/2,
  hard/passive 0/30/7, normal/easy 80/20/0; Basis hard/passive 0 von 4;
  Stuck-Audit 60 Befehle, 192 Einheiten, keine hängt; keine Umwege, der
  Angreifer fährt über die Hauptstraße und verliert am verteidigten
  Start.
- **Goldener Hash unverändert** (`sim` nicht berührt).
- **Offen:** Küsten-Biom; Reetdachhaus und weitere Inselbauten; Gezeiten;
  wandernder Sand und Spuren; Priele (weder OSM noch DGM haben sie hier);
  DGM1 anderer Länder; stehende Armeen unter 60 FPS; Gras-Schatten in
  „hoch“; KI greift verteidigten Start ohne Überlegenheit an und kennt
  keine Hinterhalte in Dünentälern.
