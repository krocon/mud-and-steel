# Prompt 141 – Bauten einer Alpenstadt: Dom, Laubenhäuser, Bergfestung, Flugplatz, Bahnanlagen, Bogenviadukt (zerstörbar)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 134
> (Großstadthäuser: Blockrand-Baukasten, `[[city_block]]`, vorab zerstörte
> Häuser) ist erledigt (`prompts/done/134-*`). **Empfohlen nach 139** (Burgteile,
> `[[castle_ruin]]`), **137** (Bahnhofshalle, Fallregel für hohe Bauten) und
> **115** (lange Brücken, Vorlandstücke auf Pfeilern); was davon erledigt ist,
> wird wiederverwendet, was fehlt, baut dieser Prompt in der einfachsten Form
> selbst. Lies zuerst `CLAUDE.md`, dann die genannten Prompts in `prompts/done/`,
> `prompts/done/078-hochhaeuser.md` und `prompts/done/020-zerstoerung.md`; in
> `docs/ASSETS.md` Gebäude, Baukasten, Brücken, Budgets; in
> `docs/ARCHITECTURE.md` „Destruction“, Sicht, Wegfindung, Brücken,
> Luftunterstützung („Air support“); `data/destruction.toml`,
> `data/objects.toml`, `docs/SOURCES.md`, `docs/PERFORMANCE.md`; in
> `docs/MAPS.md` die Skizzen-Tabellen und „Bridges“.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/alpenstadt/alpenstadt.webp`,
> ein mit KI erzeugtes Bild einer Stadt in einem Alpental. Hier zählen nur die
> **Bauten**: ein Dom mit zwei Türmen und grünen Hauben, dichte Blöcke mit roten
> Dächern, eine Festung mit Kirchturm auf einem Felshügel, ein Flugplatz mit
> Kontrollturm und Halle, ein Bahnhof mit Gleisfeld, Bogenviadukte. Die Karte
> baut Prompt 142.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Die Bauten einer **Stadt im Alpental um 1944**, als zerstörbare Kartenobjekte.
Vorbilder sind **Bautypen**, keine bestimmten Gebäude; neutrale Namen im Spiel;
Maße als Bautyp mit Quelle nach `docs/SOURCES.md`, nichts aus anderen Spielen.

## 2. Bauten

- **Dom** `env_cathedral`: barocke Kirche mit **Doppelturmfassade** (zwei Türme
  um 50–60 m mit grünen Kupferhauben), Langhaus mit Kuppel über der Vierung,
  etwa 70 m × 35 m. Aus Teilen: Turmpaar, Langhaus, Chor, damit sie einzeln
  Schaden nehmen.
- **Laubenhaus:** eine Fassadenfamilie **„Alpenstadt“** für die Haustypen aus
  134 (gerade, Eck-, Keil-, Torhaus): drei bis fünf Geschosse, farbiger Putz
  (Ocker, Rosa, Hellgrün, Weiß), rote Ziegeldächer mit Gauben, **Laubengang**
  (Bögen) im Erdgeschoss an Plätzen und Hauptgassen, Erker. Dazu ein schmales
  **Altstadthaus** `env_house_arcade` (zwei Fensterachsen, vier Geschosse, tiefes
  Grundstück) für geschlossene Zeilen am Markt.
- **Bergfestung** aus Teilen: Ringmauer, Rundturm, Torbau, Wohnbau, Kapelle mit
  Turm, mit Dächern und Wehrgängen (intakt, als Kaserne genutzt). Sind die
  Burgteile aus 139 bzw. 127 da, diese um die **intakte Variante** ergänzen;
  sonst die Teile hier bauen (Mauerstück 20 m, Rundturm 12 m, Torbau mit
  Durchfahrt, Wohnbau, Kapelle).
- **Flugplatz:**
  - `env_hangar`: Flugzeughalle, Stahlfachwerk mit Wellblech oder Holzbinder,
    etwa 40 m × 30 m, große Tore (offen und geschlossen);
  - `env_airfield_tower`: Flugleitungsbau der 1930er Jahre, zwei Geschosse mit
    verglastem Aufsatz, etwa 12 m hoch (nicht der moderne Kontrollturm der
    Vorlage);
  - Barackenzeile aus vorhandenen Baracken, Tanklager aus vorhandenen
    Treibstofflagern;
  - **Rollbahn** als Bodenart bzw. Straßentyp (befestigt, sehr breit) und
    Grasnarbe; abgestellte Flugzeuge nur als Wracks bzw. Kulisse, wenn es dafür
    schon Modelle gibt (sonst offener Punkt).
- **Bahnanlagen:**
  - `env_station_building`: Empfangsgebäude einer mittleren Stadt, drei
    Geschosse, Mittelteil und zwei Flügel, etwa 90 m lang (Kopfbau aus 137
    verwenden, falls vorhanden);
  - `env_platform_roof`: Bahnsteigdach auf Stützen, in Stücken von 30 m, offen
    (sperrt keine Sicht, gibt keine Deckung, für Infanterie begehbar);
  - `env_engine_shed`: Lokschuppen als Ringsegment oder Rechteckschuppen,
    Backstein;
  - `env_signal_box`: Stellwerk, klein, zwei Geschosse;
  - Güterwagen als Kulisse nur, wenn es Modelle gibt (sonst offener Punkt).
- **Bogenviadukt** `env_viaduct_arch`: Steinbogen-Stück von etwa 20 m Länge und
  wählbarer Höhe (10–25 m), aneinandergereiht zu einem Viadukt für **Gleis oder
  Straße**; unter den Bögen frei (Durchfahrt für Fahrzeuge unter einer
  Mindesthöhe, die die Session nach den Regeln aus 115 festlegt). Ist 115
  erledigt, als Variante der Vorlandstücke bauen. Ist 115 nicht erledigt und
  wäre die Unterführung eine neue Regelart, das Viadukt als **Damm mit zwei bis
  drei gemauerten Durchlässen** bauen (Durchlass = kurze Brücke über einen Weg)
  und das echte Viadukt als offenen Punkt notieren.

Alle Bauten in `Intact`, `Damaged`, `Ruin` wie in 078/134. Dom: Türme fallen
einzeln (Fallregel aus 137, falls vorhanden; sonst Ruine an Ort und Stelle wie
der Kirchturm). Viadukt: jedes Stück einzeln sprengbar wie eine Brücke.
Dreiecke je Bau im Budget von 078 (Dom höchstens das Doppelte; in
`docs/ASSETS.md` festhalten).

## 3. Regeln (Simulation)

- Einträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster der
  Gebäude. Richtgrößen: Dom-Langhaus etwa das Doppelte einer Kirche, Turmpaar
  wie eine Kirche, Festungsmauern zäh wie in 139, Hangar und Bahnsteigdach
  schwach (Hangar brennt), Empfangsgebäude wie ein Blockrandhaus, Viaduktstück
  wie eine Steinbrücke.
- **Besetzbar:** Laubenhäuser, Empfangsgebäude, Stellwerk, Lokschuppen, Wohnbau
  und Türme der Festung, Flugleitungsbau, Hangar (wenig Schutz). Nicht
  besetzbar: Domtürme, Bahnsteigdächer, Viadukt.
- **Laubengang:** nur Darstellung; der Grundriss des Hauses reicht bis zur
  Straße (keine begehbare Arkade als Regel; offener Punkt).
- **Rollbahn und Gleisfeld** sind offenes, festes Gelände: schnell für
  Fahrzeuge, ohne Deckung. Gleise bremsen Radfahrzeuge leicht, wenn es dafür
  schon eine Geländeart gibt; keine neue Regel dafür.
- **Flugplatz ohne Regelwirkung auf die Luftunterstützung** (sie kommt weiter vom
  Kartenrand); ein Bonus für den Besitzer wäre eine neue Regel und ist ein
  offener Punkt.
- **Der Golden Hash ändert sich** (neue Einträge in den Regeltabellen); gewollter
  Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit nennen. Vergleichslauf
  auf einer Karte ohne die neuen Bauten zeigt gleiche Ergebnisse wie vorher.

## 4. Zusammenstellungen im Skizzen-Generator

In `docs/MAPS.md` beschreiben; bestehende Skizzenkarten bleiben Bit für Bit
gleich.

- **Festung:** `[[castle_ruin]]` mit Schalter „intakt“ (oder eigene Tabelle).
- **`[[airfield]]`:** Rollbahn als Linie mit Breite, Grasfläche als Polygon,
  Vorfeld, Hallen, Flugleitungsbau, Baracken, Tanklager, Zaun.
- **`[[rail_yard]]`:** Gleisfeld aus n parallelen Gleisen zwischen zwei
  Punkten, Bahnsteige mit Dächern, Empfangsgebäude, Lokschuppen, Stellwerk,
  Güterschuppen; die Strecke schließt mit `[[rail]]` an.
- **Viadukt:** Schalter an `[[rail]]` bzw. `[[road]]` für einen Abschnitt „auf
  Bögen“ (Höhe aus dem Gelände), mit benannten Durchfahrten.
- `[[city_block]]` mit der Fassadenfamilie „Alpenstadt“.

## 5. Stufen

**Stufe 1 – Modelle:** alle Bauten `Intact`; eine Ansichtskarte
`alpine_town_test` (etwa 1000 m × 800 m: Domplatz mit Laubenhäusern, Festung auf
einer Kuppe, ein Stück Flugplatz, ein kleiner Bahnhof, ein Viadukt über eine
Senke); Bilder neben der Vorlage.

**Stufe 2 – Zustände, Regeln, Generator:** Abschnitt 3 und 4; Tests: Besetzen,
Zerstörung in Stufen, Durchfahrt unter dem Viadukt bzw. durch den Durchlass,
gesprengtes Viaduktstück unterbricht Gleis oder Straße, Fahrt über Rollbahn und
Gleisfeld, zweimal gleicher Lauf ergibt gleichen Hash.

**Stufe 3 – Abschluss:** Kamera am Dom (Regel aus 078), Bildrate über
`alpine_town_test` in jeder Ansicht (`docs/PERFORMANCE.md`), Gefechtsreihe als
Bilder, Vorschaubilder für jedes neue Modell (`just thumbnails --only=…`),
`just units-web-sync`, `just catalog-doc`, `docs/ASSETS.md`, `docs/MAPS.md`,
`docs/ARCHITECTURE.md`, `docs/SOURCES.md`. `just check`, `just selftest`.

## 6. Nicht Teil dieses Prompts

- Die Karte (Prompt 142).
- Stadion, Autobahnkreuz und moderner Kontrollturm der Vorlage (nach 1945).
- Startende und landende Flugzeuge, Flugplatz als Regel, Züge und Lokomotiven.
- Seilbahn, Skischanze, weitere Wahrzeichen.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
