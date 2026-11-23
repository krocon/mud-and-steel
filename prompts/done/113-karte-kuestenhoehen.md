# Prompt 113 – Karte nach echten Geodaten: Küstenhöhen (Vorbild Marin Headlands)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/081-karte-hafenstadt.md` ganz (Ziel, Stufen, Entscheidungen, Ergebnis,
> Abschnitt 11), in `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und
> `harbor_city` ganz, dazu „Relief profiles and mountains (prompt 060)“ und die
> Abschnitte zu Hangregeln, Gelände und Objekten (`fort_*`). In
> `docs/ARCHITECTURE.md` „Vision and fog of war“ (Sichtfeld mit Geländehöhe, „Seeing
> downhill“) und die Abschnitte zu indirektem Feuer und Wegfindung. Dann
> `docs/SOURCES.md` (Karten aus Geodaten), `docs/PERFORMANCE.md` („Harbor city“),
> `docs/AI.md` („The harbor city“), `data/scale.toml` (`[range]`),
> `data/vision.toml`, `data/terrain.toml` (`[slope]`), `data/objects.toml`,
> `tools/geo_import.py` ganz und `assets-src/geo/harbor_city/site.json`.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 den Zwischenstand zeigen
> (Stufenbilder von `--report`, Steigungskarte, `just map-reach`, ein Bild aus dem
> Spiel) und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte für ein **Artillerie- und Höhengefecht** aus echten Geodaten mit
dem vorhandenen Importer (Prompt 081). Vorbild sind die **Marin Headlands** nördlich
von San Francisco, um 1944 und vereinfacht wie `harbor_city`: grasbewachsene Kämme
bis gut 300 m, tiefe Täler, eine felsige Steilküste mit Buchten, wenige Straßen und
fast keine Gebäude.

Gespielt wird um **Sicht**. Die Simulation rechnet Sicht und Schusslinien über die
Geländehöhe. Wer die Kämme hält, sieht in die Täler; Geschütze stehen verdeckt am
Hinterhang. Indirektes Feuer reicht im Spiel nur ein Achtel der echten Weite
(`data/scale.toml`: eine leichte Feldhaubitze mit 10,6 km also rund 1,3 km), die
Karte ist größer. Stellungswechsel, Beobachter auf den Höhen und Wege über die
Kämme zählen also.

Die Karte heißt `coastal_heights` (im Spiel „Küstenhöhen“ / „Coastal heights“). Wie
bei `harbor_city` steht der echte Ort nur in `docs/SOURCES.md`, `docs/MAPS.md` und
`assets-src/geo/coastal_heights/`, nie in Kartendateien, Markern, Menüs oder
Übersetzungen.

Fertig ist der Prompt, wenn

- `just gen-geo coastal_heights` die Karte schreibt, Bit für Bit wiederholbar und in
  `just gen-geo-check` aufgenommen,
- `just map-check coastal_heights` besteht und `just map-reach coastal_heights` alle
  Starts und Versorgungspunkte im Hauptteil für Fahrzeuge zeigt (auch die auf den
  Höhen, Abschnitt 4),
- gezogene Geschütze von jedem Start auf mindestens einen Kamm mit Sicht ins
  gegnerische Tal kommen (Test),
- Messungen und KI-Turnier wie in Prompt 081 dokumentiert sind,
- die Karte im Menü wählbar ist und die Tests wie bei `harbor_city` laufen (laden,
  Editor-Rundweg, Wege, kein echter Name in den Kartendateien).

## 2. Ausschnitt und Rahmen

Richtwerte, die Session darf sie nach den Stufenbildern anpassen:

- **Begrenzungsrechteck** für die Abfrage: etwa 37,820–37,860° N, 122,540–122,470° W
  (von Point Bonita und Rodeo Beach im Westen bis zur 101 und Horseshoe Bay im
  Osten, im Norden bis über die Kämme hinter dem Rodeo Valley).
- **Rahmen:** nicht gedreht oder so gedreht, dass die Küste unten im Bild liegt
  (Blick vom Wasser wie bei `harbor_city`). Größe etwa 5 km × 4 km, mit einem
  schmalen Wasserstreifen im Süden und Westen. San Francisco und das Ufer gegenüber
  liegen außerhalb.
- **Speicher:** 5 km × 4 km im 2-m-Raster sind 2 501 × 2 001 Samples, etwa 25 MB im
  Speicher (test_large mit 4 × 4 km: 20 MB). Messen und festhalten. Ist das zu viel,
  den Ausschnitt verkleinern, nicht das Raster ändern.
- **Höhen:** Copernicus DEM wie in 081. Hier gibt es kaum Gebäude, die Glättung kann
  schwächer sein, damit Kämme, Sättel und Täler scharf bleiben. Die Höhenkodierung
  (`height_step_mm`) so wählen, dass 0 bis etwa 330 m und der Meeresboden passen.
  Vor der Küste ist das Wasser tief, die Steilküste soll als Kante bleiben
  (unpassierbar), Strände (Rodeo Beach, Black Sands, Kirby Cove) als `sand`.
- **Lagune** hinter Rodeo Beach als Wasser.

Abfrage einmal gegen den Overpass-Endpunkt, DEM-Kachelstück laden, beides wie in 081
mit `geo_import.py extract` reduzieren (Rohdaten nur unter `build/`, reduzierte
Dateien in Git LFS). Keine persönlichen Daten in den Anfragen.

## 3. Bewuchs und Boden

- Grasland ist der Normalfall (`grass`), Buschland (`natural=scrub`, `heath`) als
  `tall_grass`. Das ist Sichtschutz für Infanterie, aber kein Hindernis.
- Waldstücke (vor allem in den Tälern und an der Nordostseite) aus OSM wie in 081.
  Keine Bäume auf den Kämmen, wo OSM keine zeigt.
- Kein Mittelmeer- oder Kalifornien-Biom: `forest` wie bei `harbor_city`. Einen
  trockenen Sommeranblick (gelbes Gras) nur als offenen Punkt notieren.

## 4. Straßen, Steigung und Erreichbarkeit

Das ist der heikelste Teil. Hänge über 60 % sind gesperrt, viele Flanken liegen bei
30–50 %.

- **Straßen:** Conzelman Road (an der Küste bis Point Bonita), McCullough Road (über
  den Kamm), Bunker Road (durch das Rodeo Valley), Fahrwege zu den Batterien. Der
  Importer legt Straßen mit ebenem Querschnitt und Böschungen an. Das muss an den
  Serpentinen funktionieren; die Stufenbilder dazu ansehen.
- **Tunnel:** Bunker Road läuft durch einen einspurigen Tunnel zwischen Fort Baker
  und dem Rodeo Valley, die 101 durch den Waldo Tunnel. Der Importer lässt Tunnel
  weg. Ohne Tunnel muss es eine Fahrzeugroute zwischen Ost und West über den Kamm
  geben (McCullough, Conzelman). Fehlt sie, als Entscheidung festhalten, wie das
  gelöst ist. Kein neues Tunnelsystem in diesem Prompt.
- **Fahrwege auf den Kämmen** (heutige Wanderwege, damals Militärwege): Ob sie 1944
  bestanden, lässt sich nicht für jeden Weg belegen. `highway=track` übernehmen wie
  in 081 (`dirt_track`), Fußwege nicht.
- **Prüfen** mit `just map-reach`: Anteil Land für Fahrzeuge, nur Infanterie,
  niemand. Wenn Kämme für Fahrzeuge unerreichbar sind, mit Fahrwegen oder Rampen an
  geeigneten Sätteln nachhelfen (in `site.json`, als Daten, nicht im Code
  verstreut), bis jede Seite mindestens zwei Höhenstellungen für gezogene Geschütze
  erreicht.

## 5. Stand 1944 und Befestigungen

- **Weglassen:** die Raketenstellung und die Radarkuppe aus den 1950ern, das Museum
  und moderne Parkplätze, Aussichtsplattformen und heutige Wanderer-Infrastruktur.
- **Behalten:** die Forts (Barry, Cronkhite, Baker) mit ihren Bauten als Gebäude aus
  dem Katalog (Kasernen, Lagerhäuser). Die Golden Gate Bridge (1937) liegt am
  Ostrand. Sie wird nicht gebaut; der Kartenrand liegt so, dass keine halbe Brücke
  übrig bleibt (Straße endet am Rand).
- **Küstenbatterien** (Townsley, Spencer, Wallace, Mendell u. a.) als Objekte aus
  `data/objects.toml` an ihren Orten: `fort_bunker_mg` als Platzhalter für die
  Betonbauten, `fort_trench_section` und `fort_sandbag_position` daneben. Die Orte
  aus OSM (`military=bunker`, `historic=*`) oder von Hand in `site.json` mit
  Quelle. Die großen Küstengeschütze selbst sind nicht im Spiel.
- Sausalito und die Werft im Nordosten nur, soweit sie ohne Mehraufwand in den
  Ausschnitt fallen; sonst draußen lassen.

## 6. Spiel

- **Starts:** zwei Teams, West (Fort Cronkhite, Rodeo Beach) gegen Ost (Fort Baker,
  Horseshoe Bay), je zwei Starts, eben wie in 081 (34 m), 80 m frei.
- **Versorgungspunkte** in Paaren nach Weglänge ausgeglichen. Die meisten auf
  Höhen (Hawk Hill, Wolf Ridge, der Kamm über Battery Spencer, die Höhe über Point
  Bonita), dazu einer im Rodeo Valley oder Gerbode Valley als umkämpfter Talpunkt.
- **Sicht prüfen:** Für jeden Versorgungspunkt auf einer Höhe festhalten, welche
  Täler und Wege man von dort sieht (Sichtfeld aus der Simulation, Bild). Kein
  einzelner Punkt soll fast die ganze Karte einsehen.
- **KI-Turnier** wie in 081 und das Ergebnis in `docs/AI.md`: erreicht die KI die
  Höhen, stellt sie Geschütze sinnvoll auf, bleiben Kolonnen an Serpentinen hängen?
  Probleme beschreiben und als offenen Punkt notieren, die KI nicht umbauen.

## 7. Stufen

**Stufe 1 – Gelände, Wasser, Straßen, Rahmen:** Abfrage, Extrakt, `site.json`,
Höhen, Küste, Strände, Lagune, Straßen, Starts und Marker; Stufenbilder,
Steigungskarte, `just map-reach`. Zwischenstand zeigen, dann weiter.

**Stufe 2 – Bewuchs, Forts, Befestigungen, Spiel:** Abschnitt 3, 5 und 6,
Menüname und Übersetzung, Tests, Messungen (Bildrate in jeder Ansicht, Tick mit
vielen Einheiten, Speicher), KI-Turnier, Doku (`docs/MAPS.md` mit Tabelle „was die
Vereinfachung behielt“ wie bei `harbor_city`, `docs/SOURCES.md`,
`docs/PERFORMANCE.md`, `docs/AI.md`). Danach `just check`.

Golden Hash: Ein reiner Kartenzusatz ändert ihn nicht. Ändert sich der Importer so,
dass `harbor_city` anders herauskommt, ist das beabsichtigt und muss im Commit stehen
(neu erzeugen, `just gen-geo-check`, KI-Turnier auf `harbor_city` vorher/nachher).

## 8. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Tunnel** als Kartenelement (Bunker Road, Waldo Tunnel).
- **Eigene Modelle** für Küstenbatterien in Beton, Kasematten, Feuerleitstände,
  Kasernen der Forts; Küstengeschütze als feste, spielbare Stellungen.
- **Trockenes Sommerbild** (gelbes Gras, Küstenbuschland) als Biom.
- **Brücke über das Golden Gate** und das Gegenufer als Kulisse.
- **Beobachter und vorgeschobene Artillerie-Aufklärung** als eigene Spielregel, falls
  das Turnier zeigt, dass Geschütze auf dieser Karte ohne sie wenig bewirken.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 23.11.2026)

Niemand war erreichbar; diese Punkte hat die Session selbst entschieden
(alle vorläufig):

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Rahmen um 37,83396 N / 122,50906 W, x-Achse 78°, 6,0 × 3,4 km (3 001 × 1 701 Samples, 25,5 MB) statt 5 × 4 km. | Die Küste von Point Bonita bis Lime Point läuft rund 78°; so liegt sie unten im Bild. Von Rodeo Beach bis Horseshoe Bay sind es gut 5,5 km, mit 5 km fielen eine Startseite oder Fort Baker heraus. Gleiche Fläche und gleicher Speicher wie 5 × 4 km; Raster unverändert. |
| 2 | Abfrage über 37,810–37,864 N, 122,552–122,466 W (größer als der Richtwert), eine Kachel Copernicus N37 W123; dazu eine kleine Abfrage nach `military`/`historic`/`building=bunker` nur zum Ablesen der Batterie-Orte. | Der gedrehte Rahmen samt Rand passt hinein; die Batterie-Orte stehen mit Quelle von Hand in `site.json`. |
| 3 | Höhen: Öffnung 3 × 3 Pixel, Unschärfe 14 m zweimal (statt 18 m), Höhenkodierung automatisch (6 mm, −12 … 334 m); Meeresboden −1,5 m am Ufer, 0,8 m tiefer je 2 m, bis −12 m. | Kaum Gebäude; Kämme und Täler bleiben scharf, Steilküste bleibt Kante. |
| 4 | Kuppen sehen wenig (2–4 ha): die Kuppe ist gewölbt, die Flanken liegen hinter dem Hang. Geschützstellungen (`height_*`) daher am vorderen Hangrand 30–130 m unter den Kuppen, gesucht mit `map-sight` (Tal ≥ 20 m tiefer zum Gegner hin). | Wie echter Hinterhang/Vorderhang; die Simulation rechnet die Sicht so. |
| 5 | Neues Werkzeug `just map-sight` (cli `map-sight`, `VisionGrid::sight_from` in `sim`, nur lesend, kein Hash) und Test `coastal_heights_guns_reach_heights_with_sight`: von jedem Start erreicht ein Fahrzeug mindestens zwei `height_*`, die je ≥ 100 Zellen (0,64 ha) Tal ≥ 20 m unter dem Auge zum Gegner hin sehen. | Abschnitt 1 und 6 verlangen Test und Sichtbild; „Tal zum Gegner hin“ als Halbebene vor dem Punkt, von den eigenen Starts aus gesehen. |
| 6 | Buschland (`scrub`, OSM deckt 66 % der Karte) nur als `tall_grass`, ohne Bäume und Büsche (`scrub_spacing_m: 0`); Bäume nur in OSM-Wäldern und Parks. | Abschnitt 3: Sichtschutz, kein Hindernis, keine Bäume auf den Kämmen. |
| 7 | Lagune (und ein Teich) als Wasser über die neue Option `coast.lakes_min_m2`. | Abschnitt 2. |
| 8 | Brücke über die Meerenge: Wege über Wasser entfallen (`drop_bridges_over_water`), die Autobahn endet am Ufer am Nordpfeiler; der Rand liegt nicht vor der Brücke. | Fort Baker und Horseshoe Bay liegen östlich der Brückenachse; ein Rand westlich davon hätte die Ostseite abgeschnitten. Ein Brückenstumpf entsteht nicht. |
| 9 | Ohne Tunnel: Fahrzeugroute Ost–West über die Küstenstraße am Südkamm und die Straße vom Tal über den Sattel; keine zusätzlichen Rampen nötig (`roads.extra` leer). | `map-reach`: 75,4 % des Landes für Fahrzeuge, 97,3 % zusammenhängend, alle Starts, Versorgungspunkte und Stellungen im Hauptteil. |
| 10 | Stand 1944: Familiensiedlung im Tal (24 Blöcke, 7 Straßen) und Raketenstellung über Polygone weggelassen; Batterie-, Bunker- und Ruinen-Grundrisse (34) nicht als Häuser (`skip_tags`). | Nach allgemeiner Kenntnis des Orts, keine historische Karte. |
| 11 | Forts mit Katalog-Häusern (Stein-, Backstein-, Bauernhaus, Lagerhaus); `env_barracks` gibt es nicht als Kartenobjekt. Die Stadt im Nordosten (Sausalito-Süd) fällt in den Rahmen und bekommt Häuser wie ein Villenviertel (627). | Kein neues Objekt; die Stadt lag ohne Mehraufwand im Ausschnitt. |
| 12 | 15 Batterie-Orte im Rahmen (von 17): je ein `fort_bunker_mg`, drei Grabenstücke 9 m davor quer zur Schussrichtung, zwei Sandsackstellungen; Blickrichtung von Hand zum Meer. Battery 129 (1942–44) bleibt. | Abschnitt 5. |
| 13 | Starts: West (−2620, −410) Fort Cronkhite und (−2140, −180) Nordufer der Lagune bei Rodeo Beach; Ost (2710, 300) Exerzierplatz Fort Baker und (2760, 540) Ufer Horseshoe Bay; je 34 m eben, 80 m frei. | Die flachsten Stellen dort. |
| 14 | Sechs Versorgungspunkte: Tal (Ostende des Rodeo Valley) und Südkamm (Hawk Hill) umkämpft, Paare Nordkamm (Wolf Ridge) / Hügel nördlich Fort Baker und Höhe über Point Bonita / Kamm über Battery Spencer. Neutrale Markernamen (`supply_south_ridge` usw.). | Nach Weglänge: über die zwei Paare Ost mit Fahrzeugen 230 m länger, zu Fuß 260 m kürzer; der sechste Punkt statt Gerbode Valley (liegt außerhalb). |
| 15 | KI unverändert. | Abschnitt 6: beschreiben, nicht umbauen. |

## Ergebnis

- **Karte** `data/maps/coastal_heights` („Küstenhöhen“ / „Coastal
  heights“, `MAP_COASTAL_HEIGHTS`): 6,0 × 3,4 km um 78° gedreht, Höhen
  −12 … 334 m, 70,8 km Straßen und Fahrwege (von 136 km OSM-Wegen), 776
  Gebäude (149 in drei Forts, 627 in der Stadt), 15 Batterie-Orte mit 15
  Bunkern, 41 Grabenstücken, 30 Sandsackstellungen, 3 651 Bäume, 494
  Büsche; Lagune als Wasser; vier Starts West gegen Ost, sechs
  Versorgungspunkte, sieben Geschützstellungen (`height_*`).
  `just map-check` ok; `just map-reach`: 75,4 % Fahrzeuge, 0,5 % nur
  Infanterie, 23,0 % niemand (Kliffs, steile Flanken), 1,0 % Objekte;
  97,3 % zusammenhängend.
- **Werkzeug:** `just gen-geo coastal_heights` (in `just gen-geo-check`,
  `just check`, CI); neue Optionen in `tools/geo_import.py`
  (`extra_tags`, `lakes_min_m2`, `skip_polygons_m`,
  `drop_bridges_over_water`, `roads.extra`, `skip_tags`,
  `fortifications`, `scrub_spacing_m: 0`), alle ohne Wirkung auf
  `harbor_city` und `brick_quarter` (Bit für Bit gleich). Neu `just
  map-sight` (Sicht von Versorgungspunkten und Stellungen mit der
  Sichtberechnung der Simulation, Bild).
- **Tests:** `coastal_heights_plays_and_carries_no_real_names` (laden,
  Editor-Rundweg, Wege, kein echter Name) und
  `coastal_heights_guns_reach_heights_with_sight` (West sechs, Ost fünf
  Stellungen mit Sicht ins Tal zum Gegner).
- **Sicht:** jede Stellung sieht rund 10 ha (0,6–0,8 % des Landes), die
  Kuppen 2–13 ha; kein Punkt sieht viel mehr als die eigenen Hänge und
  das Tal darunter (Sichtweite 250 m im Spiel).
- **Messungen** (`docs/PERFORMANCE.md` „Coastal heights“): Tick 5,0 ms
  Mittel, p99 6,0 ms, aber Spitzen bis 31 ms in der Wegsuche (lange Wege
  über die Kämme); Wegsuche 163 000 Knoten je Anfrage; Sicht 5,1 ms;
  Speicher 25,5 MB. Bildrate mittel: Schlacht 61 / 61 / 70, stehende
  Armeen 58 / 55 / 104 FPS, über der Karte 67–104; hoch gestört (das
  Spiel lief parallel in einem zweiten Fenster, `harbor_city` im selben
  Zustand ebenfalls weit unter seinen Werten).
- **KI** (`docs/AI.md` „The coastal heights“): fast nur Zeitüberschreitungen
  (Schlacht hard/normal 10/0/9, hard/passive 0/0/10, normal/easy
  50/40/1; Basis hard/passive 0 von 4); der Angreifer hält 900 m vor
  dem Start auf dem Kamm oder läuft gruppenweise auf; Geschütze und
  Werfer bleiben an der Basis; Stuck-Audit ohne hängende Einheit.
- **Bilder:** Stufe 1 unter `build/geo113/stage1/` (`geodata.png`,
  `stage1.png`, `stage1_grid.png`, `reach.png`, `sight.png`,
  `map_reach.txt`, `map_sight.txt`, vier Bilder aus dem Spiel), Endstand
  unter `build/geo113/report/` und `build/geo113/shots1/`.
- **Goldener Hash unverändert** (keine Regel geändert; `sim` hat nur
  eine neue lesende Funktion).
- **Offen:** Tunnel als Kartenelement; eigene Modelle für Batterien,
  Kasematten, Feuerleitstände und Kasernen, Küstengeschütze als
  Stellungen; trockenes Sommerbild (gelbes Gras, Küstenbuschland);
  Brücke über das Golden Gate und Gegenufer; Beobachter und vorgeschobene
  Artillerie-Aufklärung als Regel (die KI stellt keine Geschütze auf die
  Höhen); Wegsuch-Spitzen bis 30 ms auf großen Bergkarten; Bildrate
  „hoch“ auf ruhiger Maschine nachmessen (Schatten des hohen Grases);
  KI-Patts (Angreifer hält vor dem Ziel, Nachzügler am Strand), längere
  Spielzeiten für Karten dieser Größe; Kaserne als Kartenobjekt.
