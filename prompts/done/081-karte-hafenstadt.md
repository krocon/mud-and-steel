# Prompt 081 – Karte nach echten Geodaten: Hafenstadt (Vorbild Monaco)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `docs/MAPS.md` ganz (Format mit Höhenkodierung, Geländearten, Objekte, Brücken,
> alle Generatoren, „Relief profiles and mountains (prompt 060)“, Karteneditor), in
> `docs/ARCHITECTURE.md` die Abschnitte zu Wegfindung und Hangregeln, „Vision and fog
> of war“ und „Map objects and decoration“, dazu `docs/PERFORMANCE.md`, `docs/AI.md`
> und `docs/SOURCES.md` (Abschnitt zu Karten). Dann `crates/cli/src/gen_land.rs` und
> `gen_town.rs` als Muster, `data/terrain.toml` (`[slope]`), `data/objects.toml` und
> die Gebäude in `data/catalog/neutral.toml`.
>
> **Dieser Prompt läuft in drei Stufen.** Nach Stufe 1 den Zwischenstand zeigen
> (Bild, Steigungskarte, Erreichbarkeit) und ohne Halt weiterarbeiten.

**Vorlagen zur Orientierung** (nicht im Repo, `.temp/` ist ignoriert):
`.temp/4k_monaco_view_001.webp` bis `_009.webp`, `.temp/monaco_3d_1.png` bis
`_3.png`, `.temp/Monaco.mp4` (2 Minuten). Bilder und Film eines käuflichen
3D-Stadtmodells von Monaco: weiße Gebäudekörper auf Gelände mit Straßen und Meer.

## 1. Ziel

Eine große Gefechtskarte namens `harbor_city`, deren Gelände, Küste, Straßennetz und
Bebauung dem heutigen Monaco folgen, in vereinfachter Form: der Felsen mit der
Altstadt, die Hafenbecken, die Stadt am Hang darüber. Sie entsteht aus
OpenStreetMap-Daten und einem freien Höhenmodell über ein Importwerkzeug, das sich
auch für andere Orte verwenden lässt.

Fertig ist der Prompt, wenn

- ein Werkzeug aus den Geodaten eine Karte im Format des Spiels schreibt, bei jedem
  Lauf Bit für Bit gleich, und `just map-check` sie annimmt,
- man die Stadt aus der Luft wiedererkennt: Küstenlinie, Hafenbecken, Felsen, Hang,
  die großen Straßenzüge, Dichte und Ausrichtung der Bebauung,
- das Gelände steil bleiben darf: Fahrzeuge fahren, wo es flach genug ist,
  Infanterie kommt weiter, und ein Gefecht Mensch gegen KI läuft darauf zu Ende,
- Herkunft und Lizenz aller Daten in `docs/SOURCES.md` und `CREDITS.md` stehen,
- `just check`, `just selftest` und `just editor-selftest` grün sind.

## 2. Was geht und was nicht

Ehrlich vorab, damit niemand das Falsche erwartet:

| Geht | Geht nicht (mit dem heutigen Spiel) |
|---|---|
| Echtes Gelände, echte Küste, die großen Straßenzüge im Maßstab 1:1 | Jedes Haus mit seinem echten Grundriss: Gebäude sind Katalogmodelle mit festen Maßen, keine frei geformten Körper |
| Gebäude dort, wo echte stehen, in echter Ausrichtung, aus dem Bestand gewählt nach Grundfläche und Höhe | Wahrzeichen und moderne Bauten (Palast, Kasino, Stadion, Türme): dafür gibt es keine Modelle |
| Dichte und Gliederung der Stadtteile | Tunnel, Hochstraßen, Treppenwege, Straßen übereinander |
| | Gassen unter 8 m Breite für Fahrzeuge (Wegzellen sind 4 m groß) |

Das Ergebnis ist die heutige Stadt mit den Häusern des Spiels: richtige Lage,
stellvertretende Gebäude. Frei geformte Gebäude aus Grundrissen wären ein eigener,
großer Umbau (Abschnitt 11).

## 3. Datenquellen und Lizenzen

Die Bilder und der Film in `.temp/` sind **keine Datenquelle**. Sie zeigen ein
käufliches Modell; daraus wird nichts abgemessen, abgezeichnet oder übernommen. Sie
dienen nur dazu, das Ergebnis mit dem Augenmaß zu vergleichen.

- **Grundrisse, Straßen, Küste, Flächennutzung:** OpenStreetMap, vom Nutzer als
  Quelle festgelegt. Lizenz ODbL: Sie verlangt die Namensnennung („©
  OpenStreetMap-Mitwirkende“) im Spiel und in `CREDITS.md`, und sie gilt auch für
  vereinfachte und abgeleitete Daten. Halte in `docs/SOURCES.md` fest, dass die
  Kartendateien von `harbor_city` aus ODbL-Daten abgeleitet sind, mit Abrufdatum
  und Abfrage.
- **Höhen:** ein freies Höhenmodell, etwa SRTM (30 m Raster, als `.hgt` ohne
  Zusatzbibliothek lesbar) oder Copernicus DEM. Lizenz und geforderte Nennung
  nachlesen und eintragen.

Regeln:

- **Herunterladen ist für diesen Prompt freigegeben.** Die Session lädt die
  OpenStreetMap-Daten des Ausschnitts und die Höhendaten ohne Rückfrage, nennt
  aber im Bericht Quelle, Adresse, Dateiname und Größe. Anfragen an die Dienste
  tragen keine persönlichen Angaben (allgemeiner Client-Name, keine E-Mail), und
  sie halten sich an deren Nutzungsregeln (ein Abruf, kein Dauerfeuer).
- **Software installieren weiterhin nur nach Rückfrage.** `gdal` und `osmium` sind
  nicht auf dem Rechner, Python 3 ist da. Das Werkzeug kommt mit der
  Standardbibliothek aus oder die Session fragt.
- Die Rohdaten liegen mit Abrufdatum und genauer Abfrage vor. Sind sie klein
  (wenige MB), kommen sie ins Repo (`assets-src/geo/`), damit die Karte neu erzeugt
  werden kann; sonst in einen ignorierten Ordner, und `docs/MAPS.md` sagt, wie man
  sie wieder holt.
- Keine Daten aus Google Maps, Kartendiensten mit Nutzungsbedingungen gegen Abzug
  oder aus dem käuflichen Modell.

## 4. Festlegungen

| Frage | Festlegung | Grund |
|---|---|---|
| Name | `harbor_city` (Ordner `data/maps/harbor_city`), im Menü „Hafenstadt“ | Die Regel in `docs/SOURCES.md` bleibt: Karten tragen keine echten Ortsnamen. Der Ort steht nur als Datenherkunft in `docs/SOURCES.md`, `docs/MAPS.md` und in der Beschreibungsdatei des Werkzeugs, nicht im Spiel. |
| Zeit | Der Zustand von heute, wie er in den Daten steht: heutige Küste mit aufgeschüttetem Land, heutige Hafenbecken, heutige Bebauung | Vom Nutzer festgelegt. Die Häuser bleiben die des Spiels; moderne Bauten bekommen Stellvertreter (Abschnitt 7). |
| Vereinfachung | Die Daten werden vor dem Umsetzen kräftig vereinfacht (Abschnitt 5) | Vom Nutzer festgelegt; macht die Karte spielbar und das Werkzeug überschaubar. |
| Geschichte | Ein Gefecht hier ist erfunden. Keine Hoheitszeichen, Wappen oder Flaggen des Ortes, keine Namen von Straßen, Geschäften oder Personen | Wie bei allen Karten. |
| Befahrbarkeit | Die steile Stadt muss nicht befahrbar sein. Das Gelände bleibt, wie es ist; was für Fahrzeuge zu steil ist, bleibt ihnen verschlossen | Vom Nutzer festgelegt. Die Karte wird ein Gefecht um Hafen und Unterstadt mit Infanterie am Hang, kein Panzergelände. |
| Maßstab | 1:1, 2 m Raster | Grundsatz des Spiels. |
| Ausschnitt | Etwa 3 km × 2,5 km: Felsen, Hafenbecken, die Stadt am Hang, ein Streifen Hinterland; das Meer nur als schmaler Rand. Rechteckige Karten erlaubt das Format | Der Ort ist rund 2 km² groß und passt hinein. |
| Biom | `forest` | Ein Mittelmeer-Biom gibt es nicht (offener Punkt). |
| Werkzeug | `tools/geo_import.py` mit einer Beschreibungsdatei je Ort (Ausschnitt, Vereinfachung, Starts), aufgerufen über `just gen-geo harbor_city` | Python wie die anderen Datenwerkzeuge; ein zweiter Ort braucht nur eine zweite Beschreibung. Ganzzahlen und feste Rundung, damit das Ergebnis überall gleich ist. |

## 5. Vereinfachen

Die Rohdaten sind viel feiner, als das Spiel sie brauchen kann. Das Werkzeug
vereinfacht sie in festen, einstellbaren Schritten; die Werte stehen in der
Beschreibungsdatei und sind Anhaltswerte.

| Daten | Vereinfachung |
|---|---|
| **Straßen** | Nur das tragende Netz: Hauptstraßen und so viele Nebenstraßen, dass etwa alle 60 bis 100 m eine Fahrstraße liegt und jeder Stadtteil erreichbar ist. Zufahrten, Parkplätze, Fußwege, Treppen und Radwege entfallen. Getrennte Fahrbahnen einer Straße werden zu einer Achse. Linien geglättet (Abweichung bis etwa 4 m), kurze Stummel entfernt, Kreuzungen zusammengezogen. |
| **Gebäude** | Aneinandergebaute Grundrisse werden zu Blöcken verschmolzen, Anbauten und alles unter etwa 60 m² entfallen, Umrisse auf Rechtecke in Richtung der längsten Kante gebracht. Eine Obergrenze für die Zahl der Gebäude (etwa 1 500), über die Mindestgröße gesteuert. |
| **Küste und Hafen** | Küstenlinie geglättet (bis etwa 4 m), Molen und Kais unter 8 m Breite entfallen, Stege und Bojen entfallen, kleine Becken werden geschlossen. |
| **Gelände** | Höhenmodell geglättet, Ausreißer entfernt; unter Straßen quer eingeebnet, unter Bauplätzen eben (Abschnitt 6). |
| **Grün und Flächen** | Parks, Gärten, Wald und Fels als wenige große Flächen; alles Kleine entfällt. |
| **Namen und Merkmale** | Alle Namen, Adressen, Hausnummern und sonstigen Angaben entfallen sofort beim Einlesen. Behalten werden nur Geometrie, Art (Straße, Gebäude, Wasser, Grün) und Geschosszahl oder Höhe. |

Das Werkzeug gibt aus, wie viel vorher und nachher da war (Straßenkilometer,
Gebäude, Küstenpunkte), damit man sieht, was die Vereinfachung gekostet hat.

## 6. Stufe 1: Gelände, Küste, Straßen

- **Steilheit bleibt.** Die Stadt steigt auf wenigen hundert Metern um mehr als
  hundert Meter, der Felsen hat Klippen. Die Hangregel (`data/terrain.toml`,
  `[slope]`) lässt höchstens 60 % Steigung zu. Was steiler ist, bleibt
  unpassierbar; das Gelände wird dafür nicht abgeflacht und nicht gestaucht.
- **Höhenkodierung.** Die Höhen stehen in 16 Bit; `height_step_mm` und
  `height_base_mm` müssen zum Höhenunterschied des Ausschnitts passen
  (`mountain_pass` nutzt 6 mm je Stufe).
- **Straßen liegen auf dem Gelände.** Sie werden als Geländeart gemalt und quer
  eingeebnet, damit sie nicht schief am Hang hängen, mit kurzen Böschungen. Ihr
  Längsgefälle folgt dem Gelände; steile Strecken bleiben steil. Kein Umbau des
  Hangs, um Serpentinen befahrbar zu machen. Achsen auf den Mittellinien der
  Wegzellen, Fahrstraßen mindestens 8 m, Hauptstraßen 12 bis 16 m breit.
- **Was es im Spiel nicht gibt:** Tunnel, Hochstraßen, Treppen. Tunnelstrecken
  entfallen; Straßen, die übereinander liegen, behalten nur die untere.
- **Küste und Hafen.** Meer als tiefes Wasser, ein Saum flaches Wasser, Kaimauern
  und Molen als Land mit Kante. Keine Schiffe.

**Zwischenstand zeigen:** die Karte nur mit Höhen, Geländearten (Straße, Feldweg,
Gras, Wasser, Fels über den Hang-Shader) und Markern, ohne Gebäude. Dazu

1. ein Bild der ganzen Karte von oben neben einer der Vorlagen,
2. eine Karte der Steigungen: wo fahren Fahrzeuge, wo kommt nur Infanterie hin, wo
   niemand,
3. die Erreichbarkeit als Zahlen: Anteil der Landfläche für Fahrzeuge, für
   Infanterie, für niemanden; welche Stadtteile zusammenhängen
   (`just path-bench harbor_city` und die Kartenprüfung),
4. die Zahlen der Vereinfachung und eine Liste dessen, was vom echten Netz
   entfallen ist (Tunnel, Gassen, Treppen).

Dann weiter mit Stufe 2. Anhalten nur, wenn die Karte für ein Gefecht nicht taugt:
wenn auch Infanterie die Stadt nicht durchqueren kann oder die beiden Seiten
einander gar nicht erreichen (Abschnitt 8).

## 7. Stufe 2: Bebauung

- **Zuordnung.** Jeder vereinfachte Block wird durch Gebäude aus dem Katalog
  ersetzt: gewählt nach Grundfläche, Seitenverhältnis und Höhe (Geschosszahl aus
  den Daten, sonst aus der Lage geschätzt), gedreht in die Richtung der längsten
  Kante. Große Blöcke werden mit mehreren Gebäuden in Reihe gefüllt.
- **Hohe und moderne Bauten.** Der heutige Ort hat Hochhäuser, ein Stadion und
  große Hallen; das Spiel hat dafür keine Modelle. Hohe Häuser bekommen das höchste
  Gebäude des Bestands (die Wohnblöcke aus Prompt 078, wenn sie da sind, sonst das
  Stadthaus), sehr große Grundrisse mehrere davon oder Lagerhäuser und Hallen. Das
  Stadion wird eine freie Rasenfläche mit Bebauung ringsum. Die Stadt sieht dadurch
  aus wie heute angelegt und wie im Spiel gebaut; das ist gewollt.
- **Straßen gehen vor.** Ein Gebäude, das eine Fahrstraße oder ihren Rand berührt
  oder ein anderes überlappt, wird verschoben, kleiner gewählt oder entfällt. Die
  Altstadt auf dem Felsen wird dadurch lichter als in Wirklichkeit.
- **Hang.** Gebäude stehen auf dem Gelände, wie das Spiel es vorsieht. Prüfe, wie
  Häuser am Steilhang sitzen (Sockel, Lücken unter Ecken), und ebne den Bauplatz
  ein, wo es nötig ist.
- **Stellvertreter für Wahrzeichen.** Kirche für eine Kirche, Rathaus für einen
  großen öffentlichen Bau, Lagerhäuser am Hafen. Kein Nachbau wirklicher Gebäude.
- **Grün.** Parks, Gärten und Baumreihen als Bäume und Hecken; Sportplätze und
  Ähnliches als Gras.
- **Kennzahlen** ausgeben: wie viele Blöcke, wie viele Gebäude gesetzt, verschoben,
  entfallen, je Stadtteil.

## 8. Stufe 3: Gefechtskarte

- **Starts.** Zwei Teams, vier Starts, an den Enden des Küstenstreifens im Westen
  und Osten, die Stadt dazwischen. Jeder Start braucht 60 m ebene, freie Fläche
  und Platz für den Basis-Modus; die gibt es hier nicht von selbst, also am Start
  einebnen und in der Beschreibungsdatei festhalten. Vom Start müssen Fahrzeuge
  wenigstens ein Stück weit in die Karte kommen.
- **Was ein Gefecht braucht.** Infanterie beider Seiten erreicht einander und jeden
  Versorgungspunkt. Eine durchgehende Fahrzeugroute zwischen den Seiten (etwa die
  Küstenstraße) ist erwünscht, aber keine Pflicht; gibt es keine, sag es im
  Bericht, statt das Gelände umzubauen.
- **Versorgungspunkte** an Orten, die man sieht: Hafen, Platz auf dem Felsen, ein
  Platz in der Oberstadt, ein Punkt im Hinterland. Mindestens die Hälfte liegt dort,
  wo auch Fahrzeuge hinkommen. Die Stadt ist nicht symmetrisch, also über Lage und
  Zahl der Punkte ausgleichen: beide Seiten haben gleich weit, zu Fuß und mit
  Fahrzeugen. Marker mit neutralen Namen wie bei den anderen Karten.
- **KI.** Die KI muss mit Zielen umgehen, die ihre Fahrzeuge nicht erreichen: keine
  Kolonnen, die vor dem Hang stehen bleiben. Sieh nach, wie sie das heute löst
  (`docs/AI.md`), und behebe, was auf dieser Karte hängt.
- **Messen** wie bei jeder großen Karte: `just bench-view`, `just bench`,
  `just path-bench harbor_city`, `just vision-bench harbor_city`,
  `just ai-tournament` auf der Karte, Ladezeit, Dateigrößen, Zahl der Objekte. Über
  60 FPS in jeder Grafikstufe, Tickzeit im Rahmen aus `docs/PERFORMANCE.md`, jedes
  KI-Spiel endet.
- **Von Hand:** Infanterie vom einen Start zum anderen schicken, über den Hafen,
  über den Hang und auf den Felsen; Fahrzeuge an Ziele am Hang schicken und
  ansehen, was sie tun (umkehren, davor halten, hängen bleiben).

## 9. Grenzen

- Die Karte ist neu, an ihr hängt kein Test und keine Messreihe: der goldene Hash
  ändert sich nicht. Keine Regel der Simulation wird für diese Karte geändert; die
  Hangregel bleibt, und was sie sperrt, bleibt gesperrt.
- Das Werkzeug schreibt nur Kartendateien im vorhandenen Format. Kein neues
  Kartenformat, keine neuen Geländearten, keine neuen Objektarten.
- Ein Test erzeugt die Karte zweimal aus denselben Rohdaten und vergleicht.
- Kein echter Name kommt in die Kartendateien, die Marker, das Menü oder die
  Übersetzungen.

## 10. Prüfen und Abschluss

1. Bilder der ganzen Karte und der Stadtteile neben den Vorlagen, aus ähnlichen
   Blickwinkeln (`SHOT_SET=land just terrain-shots <dir> harbor_city`).
2. Die Messungen und Kennzahlen aus den Stufen als Tabellen.
3. `just map-check harbor_city`, `just check`, `just selftest`,
   `just editor-selftest`; die Karte im Editor öffnen und speichern.
4. Menü: Name „Hafenstadt“, Größe, Vorschau und Übersetzungen wie bei den anderen
   Karten.

Die Bilder und Tabellen dem Nutzer zeigen. Was nicht erreicht wurde, steht im
Abschlussbericht mit Grund, nicht als erledigt.

- `docs/MAPS.md`: Abschnitt „Maps from real geodata (prompt 081)“ (Werkzeug,
  Beschreibungsdatei, Vereinfachung, Quellen, was entfällt, wie man einen zweiten
  Ort macht). `docs/SOURCES.md` und `CREDITS.md`: Quellen, Lizenzen, Nennung, der
  Ort als Datenherkunft. `docs/PERFORMANCE.md`: Messwerte. `CLAUDE.md` und
  `justfile`: `just gen-geo`. Auf Englisch, ohne die Bilder der Vorlagen.
- Die Namensnennung für OpenStreetMap steht dort, wo das Spiel seine Quellen
  zeigt.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, je Stufe mindestens einer. Kein `git push`.

## 11. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Frei geformte Gebäude aus Grundrissen:** Körper je Grundriss mit Dach, eigener
  Blockierform in der Simulation, Schadensstufen. Das wäre der Weg zu „wirklich
  jedem Haus“ und ein eigener großer Auftrag.
- **Modelle für Wahrzeichen und moderne Bauten** (Hochhaus, Stadion), ein
  **Mittelmeer-Biom** (Pinien, Zypressen, helle Häuser, Stützmauern, Terrassen).
- **Tunnel, Hochstraßen, Treppen, Schiffe, Hafenkräne.**
- **Weitere Orte** mit demselben Werkzeug.

## Entscheidungen (autonom, Nachtlauf 17.11.2026)

Der Prompt hatte keine offenen Fragen; diese Punkte hat die Session beim
Umsetzen selbst entschieden (vorläufig):

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Ausschnitt 3,6 km × 2,4 km statt etwa 3 × 2,5 km, um 37° gedreht: die x-Achse läuft entlang der Küste, das Meer liegt im Süden. | Der Ort ist entlang der Küste etwa 3,5 km lang; gedreht passt er mit schmalem Meeresrand und einem Streifen Hinterland, und der Blick von Süden entspricht dem der Vorlagen (vom Meer aus). |
| 2 | Höhen aus dem Copernicus DEM GLO-30 (AWS, ohne Anmeldung) statt SRTM. | Eine Quelle mit klarer Lizenz und Nennung. Es ist ein Oberflächenmodell: die Höhe der OSM-Gebäude wird je Pixel abgezogen (sonst stünde das flache Neuland 20 m hoch), dann Median und graue Öffnung. |
| 3 | Rohdaten reduziert im Repo (`assets-src/geo/harbor_city/osm.json.gz` 1,4 MB, `dem.json.gz` 80 KB, Git LFS), die Downloads selbst nur unter `build/`. | Die Karte lässt sich ohne Netz neu erzeugen; die reduzierten Daten tragen keine Namen. |
| 4 | Beschreibungsdatei `site.json` (JSON) statt TOML. | Python 3.10 hat kein `tomllib`; keine Installation. |
| 5 | Obergrenze 1 600 Gebäude über die Mindestgröße der verschmolzenen Blöcke (700 m²): 1 455 Gebäude. | „Etwa 1 500“; große Blöcke werden in Reihen mit mehreren Gebäuden gefüllt, daher ist die Mindestgröße höher als 60 m². |
| 6 | Gebäude halten 4 m (eine Wegzelle) Abstand von Straßen. | Mit 2 m blieb im KI-Turnier ein Panzer an einer Hauskante neben der Straße hängen: der Hauskasten sperrte die Randzelle. |
| 7 | Bauplätze werden nur eingeebnet, wo das keinen Weg abschneidet (Prüfung der Wegzellen ringsum mit einem Nachbau des Weggitters auf den gespeicherten Höhen); Gebäude, die zusammenhängendes Gelände trennen, entfallen (48). | Am Steilhang machen Sockel sonst Wände; ohne die Prüfung war das Hinterland vom Rest abgeschnitten. |
| 8 | Straßenböschungen mit 40 %, wo der Hang steiler ist, mit einer Kante am Rand (Stützmauer). | 40 % bleibt in jeder Richtung des 4-m-Gitters passierbar (\|dx\| + \|dy\| ≤ 60 %); mit 100 % zerfiel der Hang in Inseln. |
| 9 | Starts je Team an den Küstenenden: West (-1470, 540) und (-1290, 800), Ost (1560, 720) und (1320, 470); 34 m eben, 80 m frei von Gebäuden und Bäumen. | Ebene Neulandflächen gibt es nur dort; Platz für den Basis-Modus. |
| 10 | Sieben Versorgungspunkte: Hafen West/Ost, Felsen/Oberstadt Ost, Hügel West/Platz Ost, Hinterland. | In Paaren nach Weglänge ausgeglichen; das Hinterland ist nur über die Ostseite erreichbar (für Fahrzeuge ~500 m kürzer), insgesamt Ost mit Fahrzeugen ~630 m kürzer, zu Fuß West ~40 m. |
| 11 | Stadtboden bis 6 m um die Häuser als Straßenbelag (`road`), Villengärten bleiben Gras. | Sonst stand die Stadt auf Rasen; die Vorlage ist eine gepflasterte Stadt. |
| 12 | Menüname aus einer Übersetzung `MAP_<NAME>` (nur `harbor_city`: „Hafenstadt“ / „Harbor city“), sonst wie bisher der Ordnername; die Größe im Menü nimmt beide Seiten. | Andere Karten haben keine Anzeigenamen; die Größenangabe nahm nur `samples[0]`. |
| 13 | KI unverändert. | Ein Bodenmodell der KI aus dem Weggitter (Hang, Gebäude) änderte auf dieser Karte die Patts nicht und verschob Ergebnisse auf anderen Karten; verworfen. Was hing, war die Hauskante (Punkt 6). |
| 14 | Biom `forest`, kein Mittelmeer-Biom. | Wie festgelegt (offener Punkt). |

## Ergebnis

- **Werkzeug** `tools/geo_import.py` (Python 3, nur Standardbibliothek),
  `just gen-geo harbor_city` (mit `--report DIR` Bilder der Stufen),
  `just gen-geo-check` (in `just check` und CI: zweimal erzeugt, Bit für
  Bit gleich, gleich der eingecheckten Karte). Beschreibung und reduzierte
  Rohdaten in `assets-src/geo/harbor_city/`. Neu: `just map-reach <karte>`
  (Anteile Fahrzeug/Infanterie/niemand, Zusammenhang, Weglängen Start →
  Versorgungspunkt, Bild). Aufbau: `docs/MAPS.md` „Maps from real geodata“.
- **Karte** `data/maps/harbor_city` („Hafenstadt“): 3,6 × 2,4 km, Höhen
  −6 … 673 m, 82 km Straßen (von 236 km OSM-Wegen), 1 455 Gebäude aus 5 862
  Grundrissen (397 Blöcke ab 700 m²), 10 133 Bäume und Palmen, 3 741
  Büsche, 1 354 Heckenstücke; vier Starts, sieben Versorgungspunkte, alle
  für Fahrzeuge erreichbar; `just map-check harbor_city` ok. Land: 56 %
  Fahrzeuge, 2,7 % nur Infanterie, 33,7 % niemand (zu steil, Kaikanten),
  7,6 % Gebäude.
- **Test** `map_reach::tests::harbor_city_plays_and_carries_no_real_names`:
  lädt, Editor-Rundweg mit gleichem Hash, Infanterie und Panzer von Start
  zu Start, Infanterie zu jedem Versorgungspunkt, Fahrzeuge zu mindestens
  der Hälfte, kein echter Name in den Kartendateien.
- **Messungen:** `docs/PERFORMANCE.md` „Harbor city“ (jede Ansicht über
  60 FPS in hoch/mittel, Tick 4,6 ms mit 254 Einheiten), KI:
  `docs/AI.md` „The harbor city“ (Spiele enden oder laufen in die
  bekannten Munitions-/Sichtpatts; keine Kolonne vor einem Hang).
- **Menü:** „Hafenstadt“ / „Harbor city“ (Übersetzung `MAP_HARBOR_CITY`),
  Größe „3.6 km × 2.4 km“; keine Vorschau (hat keine Karte).
- **Quellen/Lizenz:** `docs/SOURCES.md`, `CREDITS.md` (im Spiel unter
  Mitwirkende), `assets-src/geo/harbor_city/README.md`.
- **Goldener Hash unverändert** (keine Regel, keine Testkarte geändert).
- **Offen:** Mittelmeer-Biom, Gebäude aus Grundrissen, Wahrzeichen,
  Tunnel/Treppen/Schiffe, weitere Orte; Hinterland nur über die Ostseite
  erreichbar (Ost mit Fahrzeugen ~630 m kürzer über alle Punkte); Lizenztexte
  ODbL/Copernicus vor einer Veröffentlichung noch einmal prüfen; die
  Kamera der Bildreihe kommt nicht über ~600 m (Übersicht als Draufsicht
  aus `--report`).
