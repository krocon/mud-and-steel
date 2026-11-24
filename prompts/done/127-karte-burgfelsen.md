# Prompt 127 – Karte nach echten Geodaten: Burgfelsen (Vorbild Edinburgh)

> In einer frischen Session im Repo-Ordner ausführen. **Empfohlen vorher:** Prompt 112
> (Backsteinviertel: Häuser in Zeilen entlang der Blockkanten, Stand-1944-Filter).
> Lies zuerst `CLAUDE.md`, dann `prompts/done/081-karte-hafenstadt.md` ganz und die
> Entscheidungen der erledigten Prompts 112–114 und 116 (falls in `prompts/done/`,
> besonders Gebäude in Zeilen, Höhengrenze, zweite DEM-Quelle, Nennung echter Orte).
> In `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und `harbor_city` ganz,
> „Bridges“, „Objects“ und die Hangregeln; in `docs/ARCHITECTURE.md` Wegfindung,
> Sicht („Seeing downhill“), Infanterie in Gebäuden und Zerstörung. Dann
> `docs/SOURCES.md` (Karten aus Geodaten), `docs/ASSETS.md`, `docs/PERFORMANCE.md`
> („Harbor city“), `docs/AI.md`, `tools/geo_import.py` ganz,
> `assets-src/geo/harbor_city/site.json`, `data/objects.toml` (Gebäude, Kirche,
> Mauern) und die Gebäudemodelle im Baukasten (`tools/blender/kit/buildings.py`,
> `desert.py` als Muster für eine Gebäudefamilie).
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe den Zwischenstand zeigen
> (Stufenbilder, Bilder aus dem Spiel) und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Stadtkarte aus echten Geodaten nach dem Vorbild der **Innenstadt von
Edinburgh**, Stand um 1944, vereinfacht wie `harbor_city`. Kein historisches
Gefecht: In Edinburgh wurde nicht gekämpft. Die Karte ist eine frei erfundene Lage
an einem echten Ort und heißt wie die anderen neutral `castle_rock` (im Spiel
„Burgfelsen“ / „Castle rock“). Der echte Ort steht nur in `docs/SOURCES.md`,
`docs/MAPS.md` und `assets-src/geo/castle_rock/`.

Was die Karte besonders macht:

- **Burgfelsen:** ein Vulkanschlot mit Felswänden auf drei Seiten, oben die Burg;
  der einzige Zugang führt von Osten über den Grat der Altstadt.
- **Altstadt auf dem Grat:** hohe Steinhäuser (fünf bis acht Geschosse) an einer
  Hauptstraße, enge Gassen, steil abfallend nach Nord und Süd.
- **Talgraben:** zwischen Altstadt und Neustadt ein Tal mit Park und Bahnlinie,
  nur über wenige Brücken und einen Damm zu queren.
- **Neustadt:** georgianisches Straßenraster mit geschlossenen Häuserzeilen,
  Plätzen und Gärten in der Mitte.
- **Hügel im Osten:** ein kleiner, steiler Hügel mit Denkmälern am Ostende der
  Neustadt; im Südosten ein großer, kahler, felsiger Berg (gut 250 m) mit
  Felsbändern über einem Park, der weit über die Stadt sieht.
- **Im Norden:** das eingeschnittene Tal eines kleinen Flusses mit Steinbrücken,
  dahinter Parks, Sportplätze und Villenviertel.

## 2. Ausschnitt und Geodaten (Stufe 1)

Richtwerte, die Session darf sie nach den Stufenbildern anpassen:

- **Begrenzungsrechteck** etwa 55,938–55,968° N, 3,232–3,155° W: im Norden bis über
  das Flusstal und die Parks (Inverleith), im Westen bis knapp hinter den
  Burgfelsen, im Osten und Südosten mit dem großen Berg (Arthur's Seat) und dem
  Park darum.
- **Rahmen:** nicht oder leicht gedreht, Größe etwa 3,6 km × 2,6 km (höchstens 4 km;
  wenn nötig den Norden kürzen, nicht den Berg).
- **Höhen:** Copernicus DEM wie in 081. Die Gebäude werden wie dort abgezogen.
  Die **Felswände des Burgfelsens** und die Felsbänder am Berg verwischt das 30-m-
  Modell. Sie nachschärfen: Kanten aus OSM (`natural=cliff`) als steile Stufen
  ins Höhenfeld einarbeiten (steiler als 60 %, unpassierbar), Höhenunterschied nach
  Quelle. Optional, wenn Prompt 114 einen Weg für amtliche Geländemodelle gebaut
  hat: die offenen LiDAR-Daten Schottlands (Open Government Licence, British
  National Grid EPSG:27700; Lizenz prüfen und in `docs/SOURCES.md` festhalten).
- **Wasser:** der kleine Fluss im Norden als Fluss mit Steinbrücken; kleine Seen im
  Park am Berg; das Meer liegt außerhalb.
- **Bahn:** die Bahnlinie im Talgraben als `[[rail]]`-Ähnliches (Gleisbett, wie die
  Skizzen-Generatoren sie bauen), der große Bahnhof im Tal als offene Fläche mit
  Hallen aus dem Katalog. Brücken über das Tal nach Prompt 014 bzw. 115 (falls
  erledigt), sonst Dämme mit kurzen Brücken.
- **Stand 1944:** Die Innenstadt ist seit dem 19. Jahrhundert weitgehend gleich
  geblieben. Weglassen: das Parlamentsgebäude am Fuß des Berges (2004), große
  Einkaufszentren und Bürobauten nach 1945, moderne Hotels, Straßenbahn und
  ihre Haltestellen. Wo das Baujahr fehlt, Höhengrenze und Lage wie in Prompt 112
  und als Entscheidung festhalten.

## 3. Gebäude und Burg (Stufe 2)

- **Häuser in Zeilen** entlang der Blockkanten wie in Prompt 112 (geschlossene
  Steinzeilen, Höfe innen frei). Höhen aus OSM (`height`, `building:levels`),
  sonst je Bezirk: Altstadt 5–8 Geschosse, Neustadt 3–4, Villenviertel im Norden 2–3.
  Platzhalter aus dem Katalog (Stadthäuser, Mietshäuser, Kirchen, öffentliche
  Bauten); eigene Modelle für schottische Steinhäuser sind ein offener Punkt.
- **Burg als Wahrzeichen:** ein vereinfachtes Modell aus dem Baukasten, aus mehreren
  Objekten zusammengesetzt (damit es nach den vorhandenen Regeln zerstörbar und
  betretbar ist): Ringmauer mit Torhaus am Ostzugang, eine Batteriebastion, zwei bis
  drei große Steinbauten, eine kleine Kapelle, Plätze innen. Maße nach Quelle
  (Grundriss aus OSM, Höhen aus Fotos nur als Referenz). Infanterie kann Torhaus und
  Bauten besetzen; die Mauern geben Deckung wie Steinmauern. Vorschaubilder
  (`just thumbnails --only=…`), `just units-web-sync`.
- **Weitere Wahrzeichen** nur als vorhandene Modelle: Kirchen mit Turm, das Denkmal
  auf dem kleinen Hügel als Turm-Platzhalter.
- **Grün:** Parks, Gärten in der Neustadt, Bäume im Talgraben, Gras und Ginster
  (`tall_grass`) am Berg, keine Bäume auf den Felsbändern.

## 4. Spiel (Stufe 3)

- **Starts:** zwei Teams, Nord (Villenviertel und Parks hinter dem Flusstal) gegen
  Südost (Park am Fuß des Berges) oder West gegen Ost. Die Session wählt nach
  `just map-reach` und Weglängen, was ausgeglichener ist, und begründet es.
- **Versorgungspunkte:** Burg (nur über den Grat erreichbar, stark, aber eine
  Sackgasse), Altstadt, Bahnhof im Talgraben, Platz in der Neustadt, der kleine
  Hügel, der Berg (Sicht über die Stadt), Brücke über den Fluss im Norden.
- **Prüfen:** `just map-check castle_rock`, `just map-reach castle_rock` (Burg für
  Fahrzeuge über den Grat erreichbar, Felswände wirklich zu, Berg für Infanterie
  erreichbar, für Fahrzeuge nur über die Straße im Park), Sichtfeld-Bilder von Burg,
  kleinem Hügel und Berg, Reichweitentabelle wie in Prompt 123 (indirekt 1/8).
- **KI-Turnier**, Ergebnis in `docs/AI.md` (bleibt die KI im Talgraben oder in den
  Gassen hängen?). Probleme als offene Punkte; die KI nicht umbauen.
- **Messungen:** Bildrate in jeder Ansicht (viele Gebäude), Tick mit vielen
  Einheiten, Speicher (`docs/PERFORMANCE.md`).
- Menüname und Übersetzung, Tests wie bei `harbor_city` (laden, Editor-Rundweg,
  Wege, kein echter Name in den Kartendateien), `just gen-geo-check`, Doku in
  `docs/MAPS.md` mit Tabelle „was die Vereinfachung behielt“. `just check`,
  `just selftest`. Der Golden Hash ändert sich nicht durch eine neue Karte; ändern
  neue Objekte Regeltabellen, die als Ganzes gehasht werden, ist das beabsichtigt
  und gehört mit Grund in den Commit.

## 5. Nicht Teil dieses Prompts

- Eine Mission auf dieser Karte (z. B. Verteidigung der Burg, sinnvoll nach Prompt
  117 mit britischen Truppen).
- Eigene Modelle für schottische Steinhäuser und weitere Wahrzeichen.
- Der Hafen im Norden (Leith) und das Meer.
- Gassen schmaler als 4 m als befahrbare Wege (sie bleiben Infanterie vorbehalten
  bzw. entfallen wie in 081).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle wie in 081 eintragen
(vorläufig), mit Begründung.

### Entscheidungen (autonom, Nachtlauf 24.11.2026)

Niemand war erreichbar; diese Punkte hat die Session selbst entschieden
(alle vorläufig):

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Rahmen um 55,95333 N / 3,18064 W, x-Achse 73°, 3,6 × 3,4 km (1 801 × 1 701 Samples, 15,3 MB) statt 3,6 × 2,6 km; Abfrage über 55,930–55,972 N, 3,245–3,140 W. | Das Raster der Neustadt und die Stahlbrücke über das Tal liegen so auf den Kartenachsen (Brücken nur achsparallel); die Diagonale Burg–Berg–Flusstal braucht in diesem Rahmen 3,4 km Höhe. Der Norden ist gekürzt (Inverleith Park nur als Ecke), der Berg nicht. |
| 2 | Höhen nur aus Copernicus GLO-30 (Kachel N55 W004); die schottischen LiDAR-Daten nicht verwendet. | Das Portal ist eine Skriptseite, seine Lizenz ließ sich nicht lesen (nicht bestätigt), und der Importer hat keinen Leser für das British National Grid. Offener Punkt. |
| 3 | Fehler behoben: `dem_heights` nahm für Copernicus-Kacheln über 50° die Breitengrad-Auflösung auch für die Länge (die Karte war flach). | Betrifft keine andere Karte (bisher nur amtliche Modelle so weit nördlich); `just gen-geo-check` aller Karten gleich. |
| 4 | Burgfelsen: Oberflächenmodell dort ohne Median/Öffnung (`raw_polygons_m`), Plateau auf 121 m gehoben (`lifts`; Gipfel nach Quelle 130 m, Modell 116 m), dann die OSM-Felslinien geschärft (`cliffs`: Band 30 m, Stufe ≥ 4 m, Lücken bis 12 m geschlossen). | Der Felsen fällt in einer Stufe 40–60 m auf drei Seiten; `map-reach`: Burg nur über die Esplanade erreichbar. |
| 5 | Berg ohne Gipfelanhebung (233 m statt 251 m). | Eine Anhebung um 18 m machte die oberen Flanken steiler als 60 % und schnitt den Gipfel ab. |
| 6 | Berg für Fahrzeuge über die Grashänge offen, nicht nur über die Parkstraße. | Die Regeln kennen kein Gelände nur für Infanterie außer Objekten (Wald, Hecken, Mauern); Bäume auf dem kahlen Berg wären falsch. Offener Punkt. |
| 7 | Der Park um den Berg ist Heide (`heath_polygons_m`, 206 ha hohes Gras, keine gepflanzten Bäume, `scrub_spacing_m: 0`); kartierte Wälder bleiben. | Abschnitt 3: Gras und Ginster, keine Bäume auf den Felsbändern; vorher stand der ganze Park voller Parkbäume. |
| 8 | Bahn aus OSM (`rails`: `rail`, `disused`, `abandoned`, `razed`, ohne Tunnel und Straßenbahn) als 7-m-Schotterbett der Straßenklasse `rail`, Masten alle 50 m; Bahnhof als offene Schotterfläche mit einer Halle (mehr passen neben die Gleise nicht). | Wie die Skizzen-Generatoren; die später stillgelegten Linien bestanden 1944. |
| 9 | Hauptbrücke über das Tal als drei Stahl-Vorlandfelder `Span60` (180 m) auf x = −507 m mit Damm am Nordende; Waverley Bridge entfällt (Rampen in den Bahnhof), der Damm (Mound) ist Gelände. | Prompt 115; unter dem Deck ist nichts passierbar: Talboden und die Straße darunter sind dort getrennt (offener Punkt). |
| 10 | Brücken über die Gasse im Süden der Altstadt (George IV, South Bridge) und über die Calton Road bleiben (`keep_bridges_at_m`), beide Straßen auf dem Boden. | Sonst wären Hauptstraßen unterbrochen; das Modell zeigt den Einschnitt nicht. |
| 11 | Vier Steinbrücken über den Fluss im Norden (`stone_bridges`, Arch1/Arch3 achsparallel), keine Dämme nötig. | Abschnitt 2. |
| 12 | Fußgängerstraßen der Alt- und Innenstadt sind wieder Straßen (`pedestrian_polygons_m`); Gassen (closes) bleiben Fußwege und entfallen. | Stand 1944; Abschnitt 5. |
| 13 | Stand 1944: Parlament, Dynamic Earth und Büros daneben als Industriebezirk (Brauerei), Einkaufszentren als Häuserzeilen, über 45 m oder `start_date` ab 1946 Bezirksschätzung; 1 % Ruinen, 3 % beschädigt. | Allgemeine Ortskenntnis, keine historische Karte; die Stadt wurde kaum beschädigt. |
| 14 | Bezirke: Altstadt 5–7 Geschosse, Neustadt 3–4, Norden 2–3, Rest 3–4; Stellvertreter Steinhaus/Stadthaus/Mietshaus, Kirchen, Rathaus; keine Backsteinhäuser. | Abschnitt 3; Steinstadt. |
| 15 | Burg aus fünf neuen Modellen (Mauer mit Turm, Torhaus, Halbmondbastion, Burggebäude, Kapelle) nach dem OSM-Grundriss gesetzt (`landmarks`); Turm auf dem kleinen Hügel = Küstenwachturm. | Abschnitt 3; zerstörbar und betretbar nach den vorhandenen Regeln. Golden Hash dadurch geändert (Objekt-, Zerstörungs-, Sicht-, Infanterietabellen), Demo-Einheiten enden gleich. |
| 16 | Starts Nord (Parks hinter dem Fluss) gegen Südost (Park am Fuß des Berges). | West gegen Ost: der Westen läge über acht Punkte rund 6 km näher (Burg, Altstadt, Bahnhof, Neustadt 0,9–1,3 km statt 2,7–3,3 km); Nord gegen Südost: 0,5 km Unterschied mit Fahrzeugen und zu Fuß. |
| 17 | Acht Versorgungspunkte: Burg, Altstadt, Bahnhof, Neustadt-Platz, kleiner Hügel, Berggipfel, Steinbrücke Nord, Schlossvorplatz (als Gegenstück zur Brücke). | Abschnitt 4; paarweise ausgeglichen. |
| 18 | Wälder 10 m statt 8 m auseinander. | Bildrate „hoch“ über dem dichtesten Wald. |
| 19 | KI unverändert, Biom `forest`. | Prompt. |

## Ergebnis

- **Karte** `data/maps/castle_rock` („Burgfelsen“ / „Castle rock“,
  `MAP_CASTLE_ROCK`): 3,6 × 3,4 km, um 73° gedreht, Höhen −3 … 233 m,
  177 km Straßen und Gleisbetten (22,5 km Bahn), 10 052 Häuser in Zeilen
  aus 13 449 Grundrissen, Burg aus 59 Teilen, 9 633 Bäume, vier
  Steinbrücken, Stahlbrücke über das Tal; vier Starts Nord gegen Südost,
  acht Versorgungspunkte, vier Geschützstellungen (`height_*`).
  `just map-check` ok; `just map-reach`: 75,4 % Fahrzeuge, 3,2 % nur
  Infanterie, 5,0 % niemand (Felsen), 16,3 % Gebäude; Burg nur über die
  Esplanade. Doku `docs/MAPS.md` („Options of the eighth site“,
  „castle_rock“ mit Tabelle der Vereinfachung, Sicht- und
  Reichweitentabelle), `docs/SOURCES.md`, `CREDITS.md`,
  `assets-src/geo/castle_rock/README.md`.
- **Modelle:** `env_castle_wall` (Segment, Turm), `env_castle_gatehouse`,
  `env_castle_bastion`, `env_castle_hall`, `env_castle_chapel` (Baukasten
  `tools/blender/kit/castle.py` und `house`/`church`), Regeln in
  `data/objects.toml`, `destruction.toml`, `infantry.toml`, `vision.toml`,
  Editor-Gruppe „Burg“, Vorschaubilder, Einheitenseite.
- **Messungen:** `docs/PERFORMANCE.md` „Castle rock“, KI `docs/AI.md`
  „The castle rock“; Bilder unter `build/castle_rock/` (`stage1/`,
  `stage2/`, `final/`, `ingame/`), Hauptbild
  `build/castle_rock/final/compare_oblique.png`.
- **Golden Hash** (Demo und Wirtschaft) geändert: neue Objekte in den
  gehashten Regeltabellen; die Demo endet mit jeder Einheit wie vorher.
