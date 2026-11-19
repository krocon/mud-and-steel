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
