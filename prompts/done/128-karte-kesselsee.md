# Prompt 128 – Skizzenkarte: Kesselsee (eigener Entwurf)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` (Ziel, Entscheidungen, Ergebnis),
> in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz (alle Tabellen,
> besonders `[[lake]]`, `[[shoal]]`, `[[hill]]`, `[[wood]]` mit `rock`,
> `[[village]]`, `[[ring_road]]`, `[[patchwork]]`), „Relief profiles and mountains
> (prompt 060)“ und den Abschnitt zur Punktsymmetrie von `city_large`;
> `data/map_sketches/island_chain.toml` als Muster, `crates/cli/src/gen_sketch.rs`,
> `data/terrain.toml` (`[slope]`), `data/objects.toml` (Felsen), `data/scale.toml`
> (`[range]`), `docs/AI.md`. Ist Prompt 126 (Tafelberge, `[[mesa]]`) erledigt, auch
> dessen Ergebnis: die Felsen des Kessels können damit gebaut werden.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/kesselsee/kesselsee_entwurf.svg`,
> ein eigener Entwurf aus der Unterhaltung mit dem Nutzer, von ihm so angenommen.
> Maßstab: das Kartenquadrat (x 60–620, y 20–580 im SVG) ist 2 km, 1 px = 3,57 m,
> Mitte (340, 300) = Kartenmitte; x nach Osten, y nach Süden wie in den Skizzen.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 Bilder aus dem Spiel neben
> dem Entwurf zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `kessel_lake` (im Spiel „Kesselsee“ / „Kessel lake“), 2 km ×
2 km, **punktsymmetrisch** um die Mitte, gebaut als Skizze für `just gen-sketch`.
Herkunft: Der Nutzer brachte eine Minimap aus einem kommerziellen Spiel als Idee;
die wird **nicht** nachgebaut. Diese Karte ist ein eigener Entwurf mit ähnlichen
Zutaten (Seeufer, Felsen in der Mitte, Straßendörfer, Bergrand), aber eigener
Anordnung. Nur der Entwurf oben ist Vorlage.

## 2. Aufbau nach dem Entwurf

Richtwerte in Metern (x Osten, y Süden, Mitte 0/0); alles im Südosten ist das
Spiegelbild (x, y) → (−x, −y) des Nordwestens, die Session übernimmt die Werte und
glättet sie nach den Stufenbildern.

- **Felskessel in der Mitte:** Felsring um (0, 0), Mittellinie Radius etwa 250 m,
  Ring etwa 70–80 m breit, 15–25 m hoch, Außen- und Innenseite steil (über 60 %
  an den meisten Stellen, Felsbrocken und `env_rock_massif`), nur stellenweise für
  Infanterie kletterbar. **Zwei Durchlässe** im Norden und Süden (bei (0, −250) und
  (0, 250)), je etwa 60–80 m breit, eben, durch beide läuft ein Feldweg von
  (0, −371) nach (0, 371). Innen eine Wiese mit Radius etwa 185 m und ein **Weiler**
  aus vier bis sechs Häusern um die Mitte.
- **Ringweg** um den Kessel, Radius etwa 370 m (`[[ring_road]]` oder Weg-Polygon).
- **See Nordwest:** Polygon etwa (−1000, −1000), (−1000, −136), (−857, −100),
  (−679, −171), (−550, −350), (−636, −536), (−600, −743), (−743, −886),
  (−786, −1000); Sandstrand und eine Sandzunge am Ostufer (um (−600, −640) und
  (−630, −180)), ein Inselchen bei (−757, −536), eine kleine Sandbank bei
  (−857, −321). **Furt:** eine Untiefe (`[[shoal]]`) von der Sandzunge zum Ufer
  bei etwa (−679, −464) → (−550, −436) als Umweg für Fahrzeuge am See entlang.
  Spiegelbild im Südosten.
- **Berge Nordost:** Polygon etwa (464, −1000), (1000, −1000), (1000, −339),
  (879, −407), (743, −564), (643, −671), (543, −821), Höhe zum Rand hin bis 80–120 m,
  Gipfelbereich in der Ecke (um (850, −850)) am höchsten, Fuß sanft (Bergfuß für
  Starts nutzbar), Flanken mit Felsbändern, Nadelwald am Fuß. Spiegelbild im
  Südwesten.
- **Straßendorf Nord** an der Kreuzung bei (321, −571): Hauptstraße nach Norden zum
  Start (321, −893), Abzweig nach Südosten zum Ringweg bei (0, −371) über
  (179, −500), Nebenweg nach Osten (643, −393) → (786, −250). Felder (gold und grün)
  östlich und südöstlich des Dorfes, ein Feld bei (96, −643). Spiegelbild: Dorf Süd
  bei (−321, 571).
- **Seeweg:** ein Feldweg vom Westeingang des Ringwegs (−364, 57) nach Norden am
  See vorbei: (−446, −179), (−436, −536), (−250, −714), (−71, −893), (−36, −1000).
  Spiegelbild im Südosten.
- **Hauptstraße West:** vom Start West (−857, 107) über (−536, 86) zum Ringweg bei
  (−364, 57). Spiegelbild Ost.
- **Wald:** Wäldchen nördlich des Seewegs (um (−230, −680)), kleine Gruppen am
  Bergfuß und bei den Starts; Spiegelbilder.
- **Gehöfte:** je ein Gehöft bei (643, −257) und (−643, 257).

## 3. Spiel

- **Starts** (60 m eben und frei): Team West (−857, 107) und (−321, 893), Team Ost
  (857, −107) und (321, −893). 1 gegen 1 nutzt die beiden ersten.
- **Versorgungspunkte:** Kessel (0, 0) umkämpft; Dorf Nord (321, −443) und Dorf Süd
  (−321, 443); Furt Nordwest (−514, −443) und Furt Südost (514, 443).
- **Prüfen:** `just map-check kessel_lake`, `just map-reach kessel_lake`: alle Starts
  und Punkte für Fahrzeuge erreichbar, der Kessel nur durch die beiden Durchlässe
  (für Fahrzeuge), die Furten wirklich befahrbar; Weglängen beider Seiten gleich
  (Symmetrietest: gespiegelte Höhen, Gelände, Objekte gleich).
- **Sicht und Reichweite:** Sichtfeld-Bilder vom Kesselrand, aus dem Weiler und vom
  Bergfuß; Reichweitentabelle wie in Prompt 123 (indirekt 1/8): welche Punkte von
  welchem Start bzw. Bergfuß aus erreichbar sind.
- **KI-Turnier**, Ergebnis in `docs/AI.md`: Kämpft die KI um den Kessel, nutzt sie
  die Furten, staut sie sich in den Durchlässen? Probleme als offene Punkte; die KI
  nicht umbauen.

## 4. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/kessel_lake.toml` nach Abschnitt 2 und 3,
erzeugt mit `just gen-sketch kessel_lake`, Biom `forest`, Wetter klar. Reicht der
Generator für den Felsring nicht (Ring mit Lücken, steile Flanken), ergänzen: ein
`[[hill]]`-Ring bzw. `[[mesa]]` aus Prompt 126, oder eine neue Tabelle
`[[rock_ring]]` (Mittelpunkt, Radius, Breite, Höhe, Lücken als Winkel und Breite).
Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben.

**Stufe 2 – Prüfen und Abschluss:** Abschnitt 3, Messung von Bildrate und Wegsuche
(`docs/PERFORMANCE.md`), Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md`, Test wie bei den anderen Skizzenkarten (zweimal
erzeugt, gleiche Dateien). `just check`, `just selftest`. Der Golden Hash ändert
sich nicht (neue Karte); wenn doch, Ursache finden.

## 5. Nicht Teil dieses Prompts

- Winter- oder Wüstenvariante (über `base` in der Skizze leicht nachzuholen).
- Boote auf dem See (Prompt 119).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 24.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Felsring | Zwei C-förmige `[[mesa]]` (Bogenbänder um (1, 1), Fuß von Radius 212 bis 290 m, Mittellinie 251 m, 22 m hoch, Rauheit 4 m, doppelt so viele Felsbrocken auf dem Schutt), keine neue Tabelle `[[rock_ring]]`; die Enden liegen je 36 m neben der Nord-Süd-Achse, die Durchlässe sind am Fuß gut 70 m breit und eben. **[vorläufig]** | Der Tafelberg aus 126 hat schon Schutthang (35–60 %) und Wand (180 %); ein Ring mit Lücken ist zwei Bögen. Kein zweiter Code für dasselbe Profil. |
| 2 | Kletterstellen für Infanterie | Neue, wiederverwendbare Option `[[mesa]] climbs_m` (mit `climb_grade` 38 %, `climb_width_m` 12 m, hier 16 m): eine Rampe ohne Weg, zwei Reihen großer Felsbrocken (`env_rock` `Large`) quer im Einschnitt halten Fahrzeuge auf; Marker `climb_*`. Je Bogen ein Übergang (außen und innen): Ostbogen im Nordosten, Bild Westbogen im Südwesten. **[vorläufig]** | Hangregel gilt für alle gleich (60 %), Panzer halten Abstand nur zu unpassierbarem Gelände, nicht zu steilem: nur Objekte trennen Infanterie und Fahrzeuge. Schräg zum Wegraster zählt der Hang |dx| + |dy| (38 % → 54 %), und erst 16 m Breite lassen eine Treppe offener Zellen (12 m reichte nicht, ausprobiert). |
| 3 | Ringweg | Geschlossener Feldweg (`[[road]]` mit 49 Punkten, 7 m) statt `[[ring_road]]`. **[vorläufig]** | `[[ring_road]]` ebnet alles innerhalb seines Radius (Flachstelle) und hätte den Ring eingeebnet. |
| 4 | Symmetrie | Punktsymmetrie um (1, 1) wie `mesa_desert`: Ring und Ringweg um (1, 1), Südhälfte = Bild (2 − x, 2 − y) der Nordhälfte; die Werte des Prompts gelten für die Nordhälfte (Ost-Hauptstraße und Südost-Seeweg als Bild der West-Werte). Start Nord bei (321, −891) statt −893, Reichweitenmarker auf Wegzellenmitten. **[vorläufig]** | Nur so bilden sich Höhen- und Wegzellen aufeinander ab; mit −893 war der Weg vom Start zu einem 30-m-Marker auf einer Seite eine Zelle länger (Symmetrietest, Kosten 7000 gegen 8000). |
| 5 | Furten | See als `[[lake]] kind = "river"` (Wasser 3 m unter dem Grund, Ufer fallen in 45 m), weil nur dort `[[shoal]]` wirkt; zwei Untiefen-Arme (18 m breit): vom Ufer am Versorgungspunkt über (−679, −464) zum Inselchen und vom Inselchen zum Sandstrand im Norden. **[vorläufig]** | Der Entwurf zeigt nur einen Strich vom Ufer in den See; über das Inselchen wird daraus ein langsamer Weg am See entlang (Flachwasser 30 % Tempo) neben dem Seeweg, und die Insel ist erreichbar. |
| 6 | Aussehen | Neuer Karten-Look `rock` (`forest_rock.tres`): Wald-Biom mit vollem Berglook (graue Wände mit Schichten, Schutt) bei jedem Relief und ohne Schneegrenze. **[vorläufig]** | Ohne Look zeigten die Wände unter 120 m Relief braune Erde (Stufe-1-Bild), `vivid` legte Schnee ab 95 m auf den Gipfel (Bild), beides passt nicht zu Felsen im Sommer. |
| 7 | Berge | Drei Felsberge (`peak`) je Ecke: breit um (900, −900) 82 m, Gipfel um (870, −870) +42 m, Kamm an der Ostkante 55 m, an der Nordkante 40 m; höchster Punkt 123 m in der Ecke; Fels- und Klippenwald oben, Nadelwald am Fuß. **[vorläufig]** | Prompt 80–120 m, Fuß sanft (Start Nord liegt am Fuß); ein einzelner Kreis traf das Polygon des Entwurfs nicht. |
| 8 | Grundhöhe | Breite Anhebung um 4 m (`[[hill]]`, Radius 6 km). **[vorläufig]** | Werkzeuge zählen Grund ≤ 0 m als Wasser; die Mitte lag bei 0 m. Uferstreifen bleiben teils darunter (1,4 % des Landes). |
| 9 | Dörfer und Weiler | Straßendorf als `[[town]]` mit 26 m Abstand (je etwa zwölf Häuser an Hauptstraße und Abzweigen), Weiler drei Häuser je Hälfte (sechs). **[vorläufig]** | Der Entwurf zeigt ein lockeres Dorf von sieben Häusern; dichter Standardabstand (4 m) gab 24 Häuser in Reihe. |
| 10 | Marker | `height_*_rim` (Ringkrone, nur Infanterie), `height_*_foot`/`battery_*_foot` (Bergfuß), `battery_*_start1`/`_start3` (Reichweite von den Starts), `ford_*_shore`/`_islet`/`_beach`. **[vorläufig]** | `map-sight` rechnet Reichweiten nur von `battery_*` und `supply_*`. |
| 11 | Teams | Team 1 (Ost) ist die gezeichnete Nordhälfte (Start Ost (857, −107) Spieler 1, Start Nord (321, −891) Spieler 3), Team 0 (West) ihr Bild (Spieler 0 West, 2 Süd); 1 gegen 1 West gegen Ost. **[vorläufig]** | Prompt-Liste; „die beiden ersten“ Starts sind West und Ost. |

### Ergebnis

- **Karte** `data/maps/kessel_lake/` aus `data/map_sketches/kessel_lake.toml`
  (2,0 × 2,0 km, Biom `forest`, neuer Look `rock`, klar, punktsymmetrisch um
  (1, 1)), im Menü „Kesselsee“ / „Kessel lake“: Felskessel aus zwei
  Bogen-Tafelbergen (22 m, Schutt mit Felsbrocken, Wände, Klippenwald oben)
  mit Durchlässen Nord und Süd (gut 70 m, eben, Feldweg), Weiler aus sechs
  Häusern innen, Ringweg (370 m); Seen NW/SO mit Stränden, Inselchen,
  Sandbank und zwei Furt-Armen über das Inselchen; Berge NO/SW bis 123 m
  mit Fels und Nadelwald; Straßendörfer Nord/Süd mit Feldern, Gehöfte,
  Wäldchen; Kletterstellen für Infanterie über jeden Bogen.
- **Generator**: `[[mesa]] climbs_m` (+ `climb_grade`, `climb_width_m`),
  Look `rock` (`forest_rock.tres`, `sim::map::Look::Rock`, nicht im Hash);
  alle anderen Skizzenkarten bitgleich (geprüft). `just kessel-lake-shots`.
- **Prüfungen**: `map-check` ok; `map-reach`: alle Starts, Punkte,
  Furtenden, Bergfuß-Stellungen im Hauptteil (89,7 % für Fahrzeuge),
  Ringkrone nur für Infanterie, Weglängen beider Seiten gleich; Tests
  `kessel_lake_is_point_symmetric`, `kessel_lake_ring_has_two_passages`
  (Kessel für Fahrzeuge nur durch die Durchlässe, Durchlässe > 60 m, beide
  Furten für Panzer befahrbar), Karte im Test „zweimal erzeugt“;
  `stuck-audit` 144/144; Golden Hash unverändert.
- **Sicht und Reichweite**: Tabellen in `docs/MAPS.md`; vom Weiler aus
  liegen beide Dörfer (547 m) ab Grille/sIG 33 in Reichweite, kein Mörser
  und kein Werfer erreicht von Start oder Bergfuß einen gegnerischen Start.
- **KI-Turnier** (gleiche KI und Fraktion beidseitig, 419 entschiedene
  Spiele): 57 % West (normal 53 %, schwer 63 %; de/su ausgeglichen, us/uk
  schwer deutlich West). Die KI kämpft im Gefechtsmodus nicht um den
  Kessel und nutzt die Furten nicht; niemand staut sich in den Durchlässen.
- **Messungen**: path-bench 4,4 ms je Planungstick; Bildrate hoch 25–102
  FPS (dichter Wald darunter, wie auf anderen Skizzenkarten), mittel ≥ 62,8.
- **Bilder**: `build/kessel_lake/stage1*/`, `build/kessel_lake/final/`
  (`compare_{top,oblique,kessel,lake}.png`, `sight_three.png`, `reach.png`).
- **Offen**: KI um das Zentrum und über die Furten; West-Neigung harter
  us/uk-Spiele (KI plant um (0, 0) statt (1, 1), Sichtzellen nicht
  symmetrisch); Bildrate über dichtem Wald auf „hoch“; kleine dunkle Flecken
  im Wasser über den Furten (Wasserfläche aus den Uferhöhen).
