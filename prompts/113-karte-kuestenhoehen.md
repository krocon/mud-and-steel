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
