# Prompt 112 – Karte nach echten Geodaten: Backsteinviertel (Vorbild Greenwich Village)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/081-karte-hafenstadt.md` ganz (Ziel, Stufen, Entscheidungen, Ergebnis,
> Abschnitt 11), in `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und
> `harbor_city` ganz, in `docs/SOURCES.md` den Abschnitt zu Karten aus Geodaten,
> `docs/PERFORMANCE.md` („Harbor city“), `docs/AI.md` („The harbor city“), dann
> `tools/geo_import.py` ganz und `assets-src/geo/harbor_city/site.json`. Dazu die
> Gebäude in `data/catalog/neutral.toml` und `docs/ASSETS.md` (Gebäude, Budgets).
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 den Zwischenstand zeigen
> (Stufenbilder von `--report`, `just map-reach`, ein Bild aus dem Spiel) und ohne
> Halt weiterarbeiten.

## 1. Ziel

Eine zweite Karte aus echten Geodaten mit dem vorhandenen Importer (Prompt 081): ein
Ausschnitt von **Manhattan um Greenwich Village**, so wie er **um 1944** aussah,
vereinfacht wie `harbor_city`. Der Nutzer hat Manhattan als Ganzes angefragt; wir haben
uns auf diesen Ausschnitt geeinigt, weil

- die Bebauung niedrig ist (meist 3–6 Geschosse: Reihenhäuser aus Backstein und
  Brownstone, Mietshäuser, Lofts und Lagerhäuser). Bei der isometrischen Kamera
  verdecken die Häuser das Gefecht nicht, und sie bleiben als Deckung spielbar.
- das Straßennetz im Village vom Schachbrett von 1811 abweicht (schräge Straßen,
  Dreiecksplätze, kurze Blöcke), im Norden und Osten aber in das Raster übergeht.
  Beides gibt es also auf einer Karte.
- der Bestand weitgehend dem von 1944 entspricht (Denkmalschutzgebiet seit 1969).
- es offene Flächen und Achsen gibt: Washington Square Park, die Piers und Kais am
  Hudson, breite Avenues (7th, 8th, Hudson Street, Broadway) für Fahrzeuge.

Die Karte heißt `brick_quarter` (im Spiel „Backsteinviertel“ / „Brick quarter“). Wie
bei `harbor_city` steht der echte Ort nur in `docs/SOURCES.md`, `docs/MAPS.md` und
`assets-src/geo/brick_quarter/`, nie in Kartendateien, Markern, Menüs oder
Übersetzungen.

Fertig ist der Prompt, wenn

- `just gen-geo brick_quarter` die Karte schreibt, Bit für Bit wiederholbar und in
  `just gen-geo-check` aufgenommen,
- `just map-check brick_quarter` besteht, `just map-reach brick_quarter` alle Starts
  und Versorgungspunkte im zusammenhängenden Hauptteil zeigt (Fahrzeuge und
  Infanterie),
- keine nach 1945 gebauten Hochhäuser auf der Karte stehen (Abschnitt 4),
- Messungen und KI-Turnier wie in Prompt 081 dokumentiert sind,
- die Karte im Menü wählbar ist und die Tests wie bei `harbor_city` laufen (laden,
  Editor-Rundweg, Wege von Start zu Start und zu den Versorgungspunkten, kein echter
  Name in den Kartendateien).

## 2. Ausschnitt und Rahmen

Richtwerte, die Session darf sie nach den Stufenbildern anpassen:

- **Begrenzungsrechteck** für die Abfrage: etwa 40,718–40,750° N, 73,986–74,016° W
  (vom Hudson bis etwas östlich des Broadway, von SoHo bis zur 23rd Street).
- **Rahmen:** x-Achse entlang der Insel. Das Straßenraster von Manhattan ist um etwa
  29° gegen Nord gedreht, also `x_axis_bearing_deg` um 29°, sodass die Avenues
  waagerecht im Bild laufen und der Hudson am Rand liegt. Größe etwa 3,2 km × 2,2 km,
  davon ein schmaler Streifen Fluss mit den Piers. Das Ufer gegenüber (New Jersey,
  gut 1 km entfernt) liegt außerhalb.
- **Höhen:** Die Gegend ist fast flach (etwa 0–20 m). Das Copernicus DEM ist ein
  Oberflächenmodell und in dichter Stadt sehr unruhig. Die Gebäudehöhen also wie in
  081 abziehen und prüfen, ob die Straßen danach eben sind. Notfalls glätter filtern
  oder das Land auf eine sehr flache Fläche setzen; das als Entscheidung festhalten.
- **Wasser:** Prüfen, ob die OSM-Küstenlinie den Hudson als Meer abschließt (bei
  Manhattan ist das in der Regel so). Sonst als Fläche `natural=water` einlesen.
  Piers als Land wie die Molen in 081; schmale Piers unter 8 m entfallen wie dort.

Die Abfrage einmal gegen den Overpass-Endpunkt laufen lassen, das DEM-Kachelstück
laden, beides wie in 081 mit `geo_import.py extract` reduzieren (Rohdaten nur unter
`build/`, reduzierte Dateien in Git LFS). Keine persönlichen Daten in den Anfragen.

## 3. Bebauung

Das Village ist **Blockrandbebauung**: geschlossene Häuserzeilen an der Straße,
Hinterhöfe innen. Der Importer verschmilzt sich berührende Grundrisse zu Blöcken und
füllt sie mit Reihen aus Katalog-Gebäuden. Das muss hier anders aussehen als in
Monaco:

- **Straßenwände:** Die Gebäude sollen in Reihen entlang der Blockkanten stehen,
  nicht als Raster quer über den Block. Ein Abstand von 4 m zur Straße bleibt (Punkt 6
  in 081, Panzer hingen an Hauskanten), er wirkt hier wie ein Gehweg. Die Höfe
  bleiben frei oder bekommen Hecken, Mauern oder Bäume.
- **Höhe:** aus `height` bzw. `building:levels`, sonst eine Schätzung je Bezirk
  (Village 4–5 Geschosse, SoHo 5–7, Chelsea 4–8, Uferstreifen Lagerhäuser).
- **Anzahl:** Im Ausschnitt stehen grob 8 000–12 000 Grundrisse. Die Obergrenze
  (`max_buildings`) nach Messung festlegen. `harbor_city` hat 1 455 Gebäude und
  bleibt in jeder Ansicht über 60 FPS. Hier sind mehr erwünscht, aber nicht auf
  Kosten der Bildrate. Messen, bevor die Grenze steigt.
- **Bezirke** in `site.json`: Village, West Village, SoHo, Chelsea, Uferstreifen
  (Lager, Kais, Fleischmarkt) und die Gegend östlich der 5th Avenue.

## 4. Stand 1944

Die Karte zeigt die Zeit um 1944, nicht heute:

- **Gebäude nach 1945 entfernen.** OSM trägt das Baujahr selten. Die NYC Building
  Footprints (NYC Open Data, mit Baujahr `cnstrct_yr` und Dachhöhe) als zweite
  Quelle prüfen: Lizenz und Nutzungsbedingungen klären und in `docs/SOURCES.md`
  festhalten, bevor die Daten verwendet werden. Wo ein Grundriss nach 1945 gebaut
  wurde, steht stattdessen ein Gebäude der Bezirksschätzung. Ohne geklärte Quelle
  gilt eine Höhengrenze: alles über etwa 45 m wird auf die Bezirkshöhe gesetzt; das
  so festhalten.
- **Was es 1944 gab und was nicht:** Die Hochbahnen der 6th und 9th Avenue sind 1939/40
  abgerissen. Die Hochstraße am West Street (West Side Elevated Highway) und die
  Güterbahn auf Stelzen (die heutige High Line, 1934) standen. Hochstraßen und Bahnen
  über Straßen kann der Importer nicht (fallen weg wie in 081). Sie als offenen Punkt
  notieren, nicht bauen. Die Piers am Hudson standen alle.
- **Moderne Grünanlagen am Ufer** (der heutige Hudson River Park) gab es nicht: dort
  liegen Kais, Lagerhallen und Gleise. Das Ufer daher als befestigten Boden (`road`)
  mit Lagerhäusern anlegen, nicht als Park.
- Washington Square Park bestand, mit dem Bogen als Wahrzeichen (Modell: offener
  Punkt, Abschnitt 7).

## 5. Spiel

- **Starts:** zwei Teams an den Enden entlang der Insel, Nord (Chelsea) gegen Süd
  (SoHo), je zwei Starts, eben wie in 081 (34 m), 80 m frei von Gebäuden und Bäumen.
  Freie Plätze dafür in den Stufenbildern suchen (Lagerflächen, Bahnhöfe, Kais).
- **Versorgungspunkte** in Paaren nach Weglänge ausgeglichen, einer umkämpft in der
  Mitte (Washington Square). Weitere Vorschläge: Piers, Dreiecksplätze, Union Square
  am Ostrand, falls im Ausschnitt.
- **Wege:** Es muss eine Fahrzeugroute zwischen den Teams geben, die nicht nur über
  eine Avenue führt (mindestens zwei getrennte Achsen, z. B. Ufer und Broadway).
- **KI-Turnier** auf der Karte wie in 081 und das Ergebnis in `docs/AI.md`. Bleibt die
  KI in den engen Straßen hängen, das beschreiben und als offenen Punkt notieren,
  nicht im Rahmen dieses Prompts die KI umbauen.

## 6. Stufen

**Stufe 1 – Gelände, Wasser, Straßen, Rahmen:** Abfrage, Extrakt, `site.json`,
Höhen, Ufer, Straßen, Starts und Marker; Stufenbilder und `just map-reach`.
Zwischenstand zeigen, dann weiter.

**Stufe 2 – Bebauung, Grün, Stand 1944, Spiel:** Gebäude nach Abschnitt 3 und 4,
Bäume (Straßenbäume sparsam, Parks), Versorgungspunkte, Menüname und Übersetzung,
Tests, Messungen (Bildrate in jeder Ansicht, Tick mit vielen Einheiten),
KI-Turnier, Doku (`docs/MAPS.md` mit Tabelle „was die Vereinfachung behielt“ wie bei
`harbor_city`, `docs/SOURCES.md`, `docs/PERFORMANCE.md`, `docs/AI.md`). Danach
`just check`.

Golden Hash: Ein reiner Kartenzusatz ändert ihn nicht. Ändert sich der Importer so,
dass `harbor_city` anders herauskommt, ist das beabsichtigt und muss im Commit stehen
(neu erzeugen, `just gen-geo-check`, KI-Turnier auf `harbor_city` vorher/nachher).

## 7. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Eigene Modelle** für Brownstone-Reihenhaus, Mietshaus mit Feuertreppen,
  gusseisernes Loft (SoHo), Lagerhaus am Kai, dazu Wahrzeichen (Bogen am Washington
  Square, Kirchen). Heute stehen die vorhandenen Katalog-Gebäude dafür.
- **Frei geformte Gebäude aus Grundrissen** (Abschnitt 11 von 081): erst damit stehen
  wirklich die echten Häuser.
- **Hochstraße am West Street und Güterbahn auf Stelzen** (Objekte über der Straße).
- **Ausblenden von Gebäuden vor der Kamera**, falls höhere Teile (Chelsea, Lofts) das
  Gefecht verdecken. Das als Beobachtung mit Bild festhalten.
- **Weitere Ausschnitte von Manhattan** (Midtown mit Hochhäusern, Lower Manhattan).
  Sie brauchen Lösungen für Gebäude über 100 m bei der isometrischen Kamera.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 23.11.2026)

Niemand war erreichbar; diese Punkte hat die Session selbst entschieden
(alle vorläufig):

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Rahmen um 40,73232 N / 74,00186 W, x-Achse 29°, 3,2 × 2,2 km. Das Ufer läuft rund 24° schräg zu den Avenues: der Fluss ist ein Keil im Nordwesten (0,94 km², im Süden am breitesten), Kais und Piers gibt es nur in der Südhälfte; das Chelsea-Ufer liegt außerhalb. | Die Avenues liegen so auf dem 4-m-Weggitter (Panzer in engen Straßen); ein Rahmen, der das ganze Ufer mitnimmt, hätte ein Drittel Wasser oder das Ufer von New Jersey im Bild gehabt. |
| 2 | Abfrage über 40,712–40,754 N, 74,028–73,976 W (größer als der Richtwert, damit der gedrehte Rahmen samt Rand hineinpasst) und eine zweite kleine Abfrage für Pier-Multipolygone (`osm_query_extra`). | Die Abfrage von 081 kennt nur Pier-Wege; die großen Piers sind hier Relationen. |
| 3 | Zwei DEM-Kacheln (N40 W074 und W075) als Mosaik; `extract` nimmt jetzt mehrere Kacheln und mehrere Overpass-Antworten. | Der Ausschnitt liegt über 74° W. |
| 4 | Höhen: das Oberflächenmodell ist in der dichten Stadt unbrauchbar (Gruben bis −39 m nach Abzug der Gebäude, Buckel bis 45 m ohne). Gebäudehöhen abgezogen, Öffnung über 9 × 9 Pixel, Boden auf 2–18 m begrenzt (`clamp_m`), dreimal über 50 m geglättet: sanfter Anstieg vom Fluss (2 m) nach Osten (17 m), Straßen eben. | Prompt: „notfalls glätter filtern“; echte Höhen der Gegend etwa 2–20 m. |
| 5 | NYC Building Footprints (Baujahr) geprüft und **nicht** verwendet: der Datensatz trägt keine Lizenz, die Nutzungsbedingungen verweisen auf die von NYC.gov („all rights reserved“). Stattdessen Höhengrenze 45 m und OSM-`start_date` ab 1946 → Bezirksschätzung (in `docs/SOURCES.md`). | Prompt: ohne geklärte Quelle die Höhengrenze. |
| 6 | Blockrand als neues Layout `perimeter`: Reihen an allen vier Blockkanten, jedes Haus an der Kante seines Abschnitts, Straßenseite nach außen, 1 m Abstand, alle 90 m ein 8-m-Durchgang in den Hof, bis 4 m nach innen versetzt; Klasse je Grundriss (Höhe, Kirche, öffentlicher Bau, Industrie); Höfe mit Bäumen und Büschen (18 m). 4 m Abstand zur Straße bleibt. | Straßenwände statt Raster; Durchgänge, damit Infanterie in die Höfe kommt (99 % der freien Fläche hängen zusammen). |
| 7 | Keine Obergrenze nötig: alle Blöcke ab 400 m², 5 628 Gebäude (davon 54 Lagerhäuser am Kai). Gemessen: über dem Viertel 86–120 FPS in jeder Stufe, Schlacht 62–107 FPS; nur die Ansichten mit den stehenden Armeen liegen bei 55–60 FPS (hoch), `harbor_city` in derselben Sitzung 59–62. | Die stehenden Armeen (Infanterieskript 3 ms) sind dort die Grenze, nicht die Häuser; offen, siehe Ergebnis. |
| 8 | Stellvertreter: Mietshaus (`env_block_town`) ab 14 m Höhe, Stadthaus (`env_house_town`) und Backsteinhaus (`env_house_farm_brick`) darunter, Lagerhaus am Kai und auf Industriegrund, Kirche und Rathaus nach Grundriss; keine Wüsten-Modelle. Ruinen 2 %, beschädigt 6 % wie in 081. | Backstein-Look; das Gefecht ist erfunden. |
| 9 | Ufer: Bezirk `waterfront` ganz gepflastert, ohne Park und Bäume; die heutigen Grundrisse dort (spätere Bauten) entfallen; 54 Lagerhäuser (`sheds`) entlang der Kaimauer und auf drei Piers. Die heutigen Pierflächen stehen für die Piers von 1944. | Prompt Abschnitt 4; die Pierschuppen von 1944 stehen nicht in heutigen Daten. |
| 10 | Flächen über dem Boden (Park auf der Güterbahn, Park-Pier von 2021, Dachgärten: 102) und Flächen mit `start_date` ab 1946 (32) entfallen. | Stand 1944; Objekte über der Straße kann das Spiel nicht. |
| 11 | Straßenbreiten: Nebenstraßen 10 m, Avenues 18 m, Uferstraße 20 m. | Mit 4 m Abstand stehen die Häuser etwa an der echten Baulinie. |
| 12 | Starts: Süd (−1450, −60) am Kai und (−1420, 600) im Loftviertel, Nord (1222, −884) auf dem Block des Seminars und (1508, 503) am Platz am Ostrand; je 34 m eben, 80 m frei geräumt. | Lagerflächen gibt es kaum; Kai, Seminarhof und Platz sind die offensten Flächen an den Enden. |
| 13 | Sieben Versorgungspunkte: der große Platz in der Mitte (umkämpft), Pier Süd / Markt Nord, zwei Dreiecksplätze, Kreuzung Süd / Platz am Ostrand. | Paarweise nach Weglänge ausgeglichen (Ergebnis). |
| 14 | Biom `forest`. | Wie `harbor_city`. |

## Ergebnis

- **Karte** `data/maps/brick_quarter` („Backsteinviertel“ / „Brick
  quarter“, Übersetzung `MAP_BRICK_QUARTER`): 3,2 × 2,2 km, um 29°
  gedreht, Höhen −6 … 17,7 m, 123,5 km Straßen (von 392,8 km OSM-Wegen),
  5 574 Katalog-Gebäude in Straßenwänden aus 8 218 Grundrissen (652
  Blöcke ab 400 m²) plus 54 Lagerhäuser an Kai und Piers, 769 Bäume, 436
  Büsche, 498 Heckenstücke; vier Starts (Süd gegen Nord), sieben
  Versorgungspunkte, alle für Fahrzeuge erreichbar. `just map-check
  brick_quarter` ok; `just map-reach`: 66,9 % des Landes für Fahrzeuge,
  32,0 % Gebäude, 98,6 % der freien Fläche hängen zusammen, alle Starts
  und Punkte im Hauptteil. Paare nach Weglänge: über alle vier Reihen
  Nord mit Fahrzeugen 85 m, zu Fuß 176 m kürzer.
- **Werkzeug:** `just gen-geo brick_quarter` (Bit für Bit wiederholbar,
  in `just gen-geo-check`, `just check` und CI); neue Optionen in
  `tools/geo_import.py` (mehrere Overpass-Antworten und DEM-Kacheln,
  Pier-Multipolygone, `clamp_m`, `skip_raised`, `built_before_year`,
  `max_tagged_height_m`, Layout `perimeter`, Bezirke `paved` /
  `skip_blocks`, `sheds`, Hofbäume), alle ohne Wirkung auf
  `harbor_city` (dort `just gen-geo-check` unverändert grün).
- **Test** `map_reach::tests::brick_quarter_plays_and_carries_no_real_names`
  (gemeinsam mit `harbor_city`): laden, Editor-Rundweg, Infanterie und
  Panzer von Start zu Start, Infanterie zu jedem Versorgungspunkt,
  Fahrzeuge zu mindestens der Hälfte, kein echter Name in den Dateien.
- **Messungen** (`docs/PERFORMANCE.md` „Brick quarter“): über dem Viertel
  86–120 FPS in jeder Stufe, Schlacht 65–107 FPS, stehende Armeen in
  „hoch“ 55 / 60 / 79 FPS (harbor_city in derselben Sitzung 59 / 62 / 76);
  Tick 2,7 ms Mittel, p99 6,6 ms mit 254 Einheiten; Pfade 5,4 ms je
  Planungstick, Sicht 4,0 ms.
- **KI** (`docs/AI.md` „The brick quarter“): unverändert; Schlachtmodus
  hard/normal 10/50/4, hard/passive 50/10/4, normal/easy 20/70/1 (der
  Angreifer verliert in den Straßen), Basismodus hard/passive 4 von 4;
  Stuck-Audit ohne hängende Einheit.
- **Quellen:** `docs/SOURCES.md`, `CREDITS.md`,
  `assets-src/geo/brick_quarter/README.md`; NYC-Baujahre geprüft und
  wegen ungeklärter Lizenz nicht verwendet.
- **Zwischenstand Stufe 1** (Bilder unter `build/geo112/stage1/`:
  `geodata.png`, `stage1.png`, `stage1_grid.png`, `reach.png`,
  `map_reach.txt`, zwei Bilder aus dem Spiel); Endstand unter
  `build/geo112/report/` und `build/geo112/shots/`.
- **Goldener Hash unverändert** (keine Regel, keine Testkarte geändert;
  `harbor_city` Bit für Bit gleich).
- **Offen:** eigene Modelle (Brownstone, Mietshaus mit Feuertreppen,
  gusseisernes Loft, Kai-Lagerhaus, Bogen am Platz, Kirchen); Gebäude aus
  Grundrissen; Hochstraße am Ufer und Güterbahn auf Stelzen; Piers und
  Schuppen von 1944, wo heutige Daten keine haben (das Ufer nördlich der
  Kartenmitte liegt außerhalb des Rahmens); NYC-Baujahre nach geklärter
  Lizenz; KI greift in dichte Stadt ohne Überlegenheit an und verliert;
  stehende Armeen in „hoch“ knapp unter 60 FPS (Zahl der Häuser nicht
  gesenkt, auf ruhiger Maschine nachmessen); Ausblenden von Gebäuden vor
  der Kamera hier nicht nötig (Beobachtung in PERFORMANCE.md); weitere
  Ausschnitte der Insel.
