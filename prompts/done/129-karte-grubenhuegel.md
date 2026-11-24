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

### Entscheidungen (autonom, Nachtlauf 24.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Baustein für gestufte Höhen | `[[mesa]]` aus 126 erweitert statt `[[terrace_hill]]`/`[[pit]]`: `slope_grade` (gleichmäßiger Hang statt Schutt + Wand), `cliffs_m` + `cliff_m` (Felsband in Sektoren, oberer Teil des Hangs 180 %, Hang bleibt gleich breit), `center_m`, `ramp_paths_m` (Rampen entlang Linien, Bett steigt gleichmäßig), `stop_vehicles` (zwei Reihen großer Felsbrocken auf offenen Hangteilen); dasselbe auf `[[mesa.tier]]`. Grube = Mulden-Stufe mit Rampe. **[vorläufig]** | Prompt („ist `[[mesa]]` da, ihn verwenden und erweitern“); ein Code für Stufen, Rampen, Gruben. Opt-in: alle anderen Skizzenkarten bitgleich (geprüft). |
| 2 | Hangsteigung | Unterer Hang 32 %, Kuppenhang 34 %, Rücken 32 % (statt bis 50 %); der Umriss eines Hangs franst nur mit der groben Rauschoktave aus. **[vorläufig]** | Das Wegraster zählt schräg √2 × Steigung, die feinen Oktaven machten den Hang stellenweise steiler: bei 36–38 % war er für Infanterie gesprenkelt gesperrt (erstes Erreichbarkeitsbild). |
| 3 | Maße des Hügels | Fuß 452 × 416 m, Hochfläche 35 m ab 343 × 307 m, Kuppe 60 m (Fuß 312 × 282, oben 235 × 206 m), Hochflächenring gut 30 m breit. **[vorläufig]** | Prompt-Ellipsen; mit ≤ 34 % Hang bleibt für Hochfläche und Kuppe nur so Platz (Kuppe 65–70 m hätte den Ring geschlossen). |
| 4 | Felsbänder und Sperren | Unterer Hang: Nord- und Südviertel Felsband (oben 18 m); Kuppenhang: Ost- und Westviertel Felsband (oben 14 m), Nord- und Südviertel zwei Felsbrockenreihen. Kuppe für Fahrzeuge nur über die Rampen (Test), Hochfläche auch über die offenen unteren Hänge (32 %, bergauf 36 % Tempo). **[vorläufig]** | „Hälfte des Umfangs“; Infanterie und Fahrzeuge trennt nur ein Objekt (gleiche Hangregel), wie bei 128. |
| 5 | Rampen | Zwei Rampen in je zwei Abschnitten, um den Hügel gewunden: Nordost-Fuß → Norden auf die Hochfläche (398 m, 8,8 %), weiter um den Kuppenhang zum Nordwestrand der Kuppe (294 m, 8,5 %); Bild von Südwesten; 18 m Bett mit Feldweg. **[vorläufig]** | Gerade vom Fuß zur Kuppe (258 m für 60 m) wären 23 %; unter 12 % braucht es über 500 m. |
| 6 | Gruben | 64 × 43 m, 10 m tief, Schutt + Wand, Einfahrt 20 % (Nordgrube von Osten, Südgrube von Westen); Fußweg über die Kuppe mit Lücke an den Felsbrockenreihen. **[vorläufig]** | Ein Feldweg durch die Reihen ließ Panzer hinab (Test zeigte den Weg). |
| 7 | Seitenrücken und Ringweg | Rücken als Sichel zwischen Ringweg und Hügel, 30 m, Oberfläche 35–50 m breit, Talsenke ~60 m; Ringweg Ellipse 750 × 590 m statt 636 × 579. **[vorläufig]** | Im Entwurf überlappen Rücken und Hügelfuß; nur so bleiben zwei getrennte Höhen und ein Rücken mit Hochfläche. |
| 8 | Bergkämme | Umriss des Entwurfs bis 250 m hinter den Rand verlängert; 40-m-Stufe mit Felsband rundum über einem 20-%-Fuß (nur der Fuß befahrbar), Felsstufe 75 m, Gipfelblock 100 m (35–45 m nach Süden gerückt, damit er in der Karte liegt), Felsspitze +10 m: höchster Punkt 115 m. **[vorläufig]** | Der Hang wird vom nächsten Umrissstück gemessen; am Kartenrand lag sonst der Fuß. |
| 9 | Höhenkodierung, Look | Relief 115 m (< 120 m), Höhenschritt 2 mm (automatisch über 60 m); kein Look (Wald normal). **[vorläufig]** | Ab 120 m Relief schaltet der Terrain-Shader den Gebirgslook (Schnee auf dem Gipfel, Gras bis 42 %); ohne Look zeigen 32-%-Hänge Erde: die Hänge lesen sich als braune Bänder wie die Höhenstufen des Entwurfs. Look `rock` ließ sie grün (Stufe-1-Bild). |
| 10 | Seebucht | Normaler tiefer See; Halbinsel y −690 … −470 bis x −716 mit 17-m-Höcker, angebunden über einen 55 m breiten Landstreifen am Westrand; Versorgungspunkt (−843, −575) statt (−821, −436); Inselchen (−660, −400) statt (−614, −371). **[vorläufig]** | Im Entwurf liegt der Versorgungspunkt im Wasser und das Inselchen an Land/auf dem Ringweg; ohne Landbrücke wäre die Halbinsel unerreichbar. |
| 11 | Punkte | Rücken-Versorgungspunkt auf dem Rücken (−607, −71) statt (−443, −71) (läge in der Talsenke); Weiler-Punkt (465, −443) wie im Prompt; alle Starts und Marker auf Wegzellenmitten; Sichtmarker am Kuppenrand (`height_*_rim`, `_rim_w`), Hochfläche, Rückenrand, Kammfuß, Tal. **[vorläufig]** | Symmetrietest; die Kuppenmitte sieht über die 235 m flache Kuppe nichts unterhalb. |
| 12 | Bahn, Orte, Platzhalter | Bahn als `[[rail]]` bis zur Verladestelle außerhalb des Ringwegs (Halle, Kisten, Fässer), Bahnhof aus Stadthaus + Lagerhalle; Bergbaudorf mit Häusern, zwei Lagerhallen, Steinturm (`env_tower_coast`) als Förderturm-Platzhalter; Weiler fünf Häuser. **[vorläufig]** | Keine eigenen Modelle (Abschnitt 6). |
| 13 | Teams | Team 0 West: Spieler 0 (−715, 285), 2 (−751, 921); Team 1 Ost: Bilder; 1 gegen 1 Spieler 0 gegen 1. **[vorläufig]** | Prompt. |

### Ergebnis

- **Karte** `data/maps/pit_hill/` aus `data/map_sketches/pit_hill.toml`
  (2,0 × 2,0 km, Biom `forest` ohne Look, klar, punktsymmetrisch um (1, 1)),
  im Menü „Grubenhügel“ / „Pit hill“: gestufter Hügel (Hochfläche 35 m,
  Kuppe 60 m) mit Felsbändern auf der Hälfte jedes Hangs, Felsbrockenreihen,
  zwei gewundenen Rampen (8,5–8,8 %), Bergbaudorf und zwei Gruben (10 m);
  Seitenrücken (30 m) hinter einer Talsenke, Ringweg, Bergkämme N/S bis
  115 m (nur der Fuß befahrbar), Seebuchten mit Halbinsel und Inselchen,
  Bahn mit Bahnhof und Verladestelle, Weiler, Felsen, Wäldchen.
- **Generator**: `[[mesa]]`/`[[mesa.tier]]` `slope_grade`, `cliffs_m`,
  `cliff_m`, `center_m`, `ramp_paths_m`, `stop_vehicles` (in `docs/MAPS.md`);
  alle anderen Skizzenkarten bitgleich (geprüft). `just pit-hill-shots`.
- **Prüfungen**: `map-check` ok; `map-reach`: alle Starts, Punkte, Rampen,
  Gruben im Hauptteil (91,5 % Fahrzeuge), Weglängen beider Seiten gleich;
  Tests `pit_hill_is_point_symmetric`, `pit_hill_top_only_over_ramps`
  (Kuppe für Fahrzeuge nur über die Rampen, Felsband zu, Rampen < 12 %),
  Karte im Test „zweimal erzeugt“; `stuck-audit` 208/208; Golden Hash
  unverändert.
- **Sicht**: Kuppenrand sieht hinab ins Ringtal (8,6–10,7 ha), Kuppenmitte
  nur die Kuppe, Rückenrand die Talsenke und den unteren Hügelhang, nicht
  Hochfläche (280 m > 250 m Sichtweite) und nicht das Dorf, Kammfuß das
  eigene Tal. Reichweitentabelle in `docs/MAPS.md`.
- **KI-Turnier** (gleiche KI und Fraktion beidseitig): 53 % West über 212
  entschiedene Spiele (normal 50 %, schwer 57 %; hart su/us West-lastig,
  de Ost); de gegen de fast nur Zeitüberschreitungen (die Armeen kreisen
  gleichsinnig um den Hügel). Die KI steigt im Gefechtsmodus nicht auf den
  Hügel, niemand hängt an Felsbändern.
- **Messungen**: path-bench Planung 4,4 ms; Bildrate hoch ≥ 69,7 FPS außer
  Mitte über dem dichtesten Wald 58,9; mittel ≥ 93,4.
- **Bilder**: `build/pit_hill/stage1/`, `stage1b/` (Stufe 1, mit und ohne
  Look `rock`), `build/pit_hill/final/` (`compare_{top,oblique,side,hill}.png`,
  `details.png`, `sight_five.png`, `reach.png`).
- **Offen**: KI nimmt die Kuppe nicht und nutzt Höhen nicht; gleichsinniges
  Kreisen gleicher Armeen; Sicht 250 m kürzer als die Sichtachsen des
  Entwurfs; eigene Modelle für Förderturm, Grubenanlagen, Bahnhof,
  Laderampe; Wasserfläche zeigt einen dunklen Fleck in der Bucht.
