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
