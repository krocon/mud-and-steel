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
