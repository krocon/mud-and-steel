# Prompt 129 – Skizzenkarte: Grubenhügel (eigener Entwurf, starkes Relief)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` und
> `prompts/done/060-berge-taeler.md` (jeweils Ziel, Entscheidungen, Ergebnis), in
> `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz (besonders
> `[[hill]]`, `[[lake]]`, `[[rail]]`, `[[village]]`, `[[wood]]` mit `rock`),
> „Relief profiles and mountains (prompt 060)“, die Hangregeln und den Abschnitt zur
> Punktsymmetrie von `city_large`; `crates/cli/src/gen_sketch.rs`,
> `crates/cli/src/gen_map/relief.rs`, `data/terrain.toml` (`[slope]`),
> `data/objects.toml` (Felsen), `data/scale.toml` (`[range]`), `docs/AI.md`. Sind
> die Prompts 126 (`[[mesa]]`) oder 128 (Kesselsee, ggf. `[[rock_ring]]`) erledigt,
> auch deren Ergebnis.
>
> **Vorlage** (nicht im Repo, nur lesen):
> `docs/.vorlagen/grubenhuegel/grubenhuegel_entwurf.svg`, ein eigener Entwurf aus
> der Unterhaltung, vom Nutzer so angenommen. Die Farben sind **Höhenstufen**: Tal
> `#c9ad78`, Hang `#a8925f`, Hochfläche `#86774f`, Fels `#8a8579`, Gipfel
> `#d4d1c8`, Grube `#5b4f38`, See `#3d6f8f`. Maßstab: das Kartenquadrat (x 60–620,
> y 20–580) ist 2 km, 1 px = 3,57 m, Mitte (340, 300) = Kartenmitte; x nach Osten,
> y nach Süden.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 Bilder aus dem Spiel (auch
> schräg von der Seite, damit das Relief zu sehen ist) neben dem Entwurf zeigen und
> ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `pit_hill` (im Spiel „Grubenhügel“ / „Pit hill“), 2 km × 2 km,
**punktsymmetrisch** um die Mitte, als Skizze für `just gen-sketch`. Der Nutzer
brachte eine Minimap aus einem kommerziellen Spiel als Idee; die wird **nicht**
nachgebaut. Diese Karte ist ein eigener Entwurf mit eigener Anordnung.

Der Nutzer legt Wert auf **schöne Höhenunterschiede**: Die Karte lebt vom Relief.
Talboden, Hänge, Hochflächen, Felsbänder und Gipfel müssen im Spiel deutlich zu
sehen und zu spüren sein (Sicht, Tempo, Wege), nicht nur als sanfte Wellen.

## 2. Relief nach dem Entwurf

Richtwerte in Metern (x Osten, y Süden, Mitte 0/0); Südosten und Süden sind das
Spiegelbild (x, y) → (−x, −y). Höhen relativ zum Talboden (0 m).

- **Grubenhügel in der Mitte:** Ellipse um (0, 0) in drei Stufen. Fuß (Hang)
  Halbachsen etwa 457 m × 421 m, Hochfläche etwa 343 m × 307 m auf rund 35 m,
  Kuppe etwa 250 m × 221 m auf 60–70 m. Hänge zwischen den Stufen 25–50 %, mit
  **Felsbändern** (über 60 %, unpassierbar) auf etwa der Hälfte des Umfangs, sodass
  Fahrzeuge vor allem über die **zwei Rampen** hinaufkommen: Nordost von etwa
  (357, −279) nach (186, −86), Südwest gespiegelt; Rampen unter 12 %, breit genug
  für Panzer. Infanterie kommt überall hinauf, wo keine Felsbänder sind. Auf der
  Kuppe ein **Bergbaudorf** (Häuser, Hallen, ein Förderturm-Platzhalter) und zwei
  **Gruben** bei (−93, −114) und (93, 114), je etwa 64 m × 43 m und 8–12 m tief,
  mit steilen Wänden und einer Einfahrt. Ein Fußweg quer über die Kuppe von
  (0, −307) nach (0, 307).
- **Ringtal:** Talboden um den Hügel, der **Ringweg** als Ellipse etwa 636 m ×
  579 m. Der Talboden ist nicht ganz eben (leichte Wellen bis 3–5 m).
- **Seitenrücken:** je ein langer Rücken westlich des Hügels von etwa (−536, −464)
  über (−339, 0) bis (−500, 357), 30–40 m hoch, Hochfläche in der Mitte, mit
  Wäldchen am Nordende. Zwischen Rücken und Hügel bleibt eine Talsenke, damit es
  zwei getrennte Höhen sind. Spiegelbild im Osten.
- **Bergkamm Nord:** Polygon etwa (−500, −1000), (714, −1000), (679, −821),
  (536, −696), (286, −650), (0, −671), (−250, −643), (−446, −732), (−518, −875); in
  Stufen Hang, Hochfläche, Felsband, Gipfel; zum Kartenrand hin 100–120 m (Gipfel um
  (143, −975)). Am Fuß ein Weg (etwa (−357, −579) → (0, −607)). Nur der Fuß ist
  befahrbar. Spiegelbild im Süden.
- **Seebucht Nordwest:** Polygon etwa (−1000, −1000), (−571, −1000), (−600, −821),
  (−679, −679), (−636, −500), (−714, −304), (−875, −214), (−1000, −229), Wasser
  einige Meter unter dem Talboden, Ufer teils steil. Eine **Halbinsel** vom
  Westrand (y −671 … −507, bis x −714) mit einem Höcker von 15–20 m, ein Inselchen
  bei (−614, −371). Spiegelbild im Südosten.
- **Felsen** im Tal: einzelne Felsrücken und Blöcke (`env_rock_outcrop`,
  `env_rock_massif`) wie im Entwurf, z. B. bei (−321, −179), (−679, 429),
  (−143, 500); Spiegelbilder.

Die Höhen sollen mit Bedacht abgestuft sein: Wer auf der Kuppe steht, sieht ins
ganze Ringtal und auf die Rücken, aber nicht über die Kämme; wer auf einem Rücken
steht, sieht die Hochfläche des Hügels, aber nicht ins Dorf auf der Kuppe; vom
Kammfuß sieht man in das eigene Tal. Mit Sichtfeld-Bildern aus der Simulation
prüfen und die Höhen danach nachstellen.

## 3. Bahn, Wege, Orte

- **Bahn West:** Linie (`[[rail]]`) entlang des Westrands von (−929, 1000) nach
  (−929, 214), dann zur **Verladestelle** am Ringtal bei etwa (−679, 64) (Halle,
  Rampe, Kisten). **Bahnhof** bei etwa x −900 … −850, y 543 … 679. Spiegelbild im
  Osten.
- **Hauptweg West:** vom Ringweg bei (−636, 0) über (−750, 0), (−821, 143),
  (−793, 607) zum Start (−750, 929). Spiegelbild Ost.
- **Weiler:** je einer am Kammfuß, Süd bei (−464, 536) (Spiegel Nord (464, −536)),
  mit Wegen zum Ringweg und zum Hauptweg.
- **Wald:** kleine Gruppen am Rücken, am Kammfuß und an den Seeufern; sonst offenes
  Gras mit etwas `tall_grass` an Hängen. Kein Wald auf der Kuppe.

## 4. Spiel

- **Starts** (60 m eben und frei): Team West (−750, 921) und (−714, 286), Team Ost
  (750, −921) und (714, −286). 1 gegen 1 nutzt (−714, 286) und (714, −286).
- **Versorgungspunkte:** Kuppe (0, 0) umkämpft; Bahnhöfe (−875, 729) / (875, −729);
  Weiler (−464, 443) / (464, −443); Seitenrücken (−443, −71) / (443, 71);
  Halbinseln (−821, −436) / (821, 436).
- **Prüfen:** `just map-check pit_hill`, `just map-reach pit_hill` (alle Starts und
  Punkte für Fahrzeuge erreichbar; die Kuppe für Fahrzeuge nur über die Rampen;
  Felsbänder wirklich zu; Halbinseln erreichbar), Symmetrietest (gespiegelte Höhen,
  Gelände, Objekte gleich), Steigungskarte als Bild.
- **Sicht und Reichweite:** Sichtfeld-Bilder von Kuppe, Hochfläche, Seitenrücken,
  Kammfuß und aus dem Tal; Reichweitentabelle wie in Prompt 123 (indirekt 1/8).
- **KI-Turnier**, Ergebnis in `docs/AI.md`: Findet die KI die Rampen, besetzt sie die
  Kuppe, bleiben Kolonnen an den Felsbändern hängen? Probleme als offene Punkte;
  die KI nicht umbauen.

## 5. Stufen

**Stufe 1 – Skizze und Relief:** `data/map_sketches/pit_hill.toml`, erzeugt mit
`just gen-sketch pit_hill`, Biom `forest`, Wetter klar. Der Skizzen-Generator kann
heute nur Buckel (`[[hill]]`); für die Stufen braucht es einen Baustein für
**gestufte Höhen** mit Felsbändern und Rampen. Ist `[[mesa]]` aus Prompt 126 da,
ihn verwenden und um sanftere Hänge (Steigung als Wert statt nur Kante)
erweitern; sonst eine neue Tabelle `[[terrace_hill]]` (Umriss je Stufe als
Polygon oder Ellipse, Höhe je Stufe, Hangsteigung, Anteil und Lage der Felsbänder,
Rampen als Punkte) und `[[pit]]` (Grube: Umriss, Tiefe, Einfahrt). Bestehende
Skizzenkarten müssen Bit für Bit gleich bleiben. Höhenkodierung so wählen, dass
See, Tal und 120-m-Gipfel passen.

**Stufe 2 – Prüfen und Abschluss:** Abschnitt 4, Messung von Bildrate und Wegsuche
(`docs/PERFORMANCE.md`), Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md` (mit den neuen Tabellen), Test wie bei den anderen
Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`, `just selftest`. Der
Golden Hash ändert sich nicht (neue Karte); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Ein eigenes Modell für Förderturm und Grubenanlagen (Platzhalter aus Katalog und
  Objekten).
- Züge auf der Bahn.
- Winter- oder Herbstvariante (über `base` in der Skizze leicht nachzuholen).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
