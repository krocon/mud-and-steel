# Prompt 155 – Industriebauten: Fabriken, Schornsteine, Hafenkräne, Silos, Tanks, Güterzüge (zerstörbar)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 134 (Großstadthäuser: Blockrand-Baukasten, `[[city_block]]`, vorab zerstörte
> Häuser) und 137 (Backstein-Fassaden, Bahnhofshalle, Kraftwerk, Schornstein,
> Fallregel für hohe Bauten) sind erledigt (`prompts/done/…`). **Empfohlen nach
> 141** (`[[rail_yard]]`, Lokschuppen, Stellwerk, Bahnsteigdach) und **151**
> (`[[canal]]`, Kai); was davon fehlt, baut dieser Prompt in der einfachsten Form
> selbst. Lies zuerst `CLAUDE.md`, dann diese Prompts,
> `prompts/done/078-hochhaeuser.md` und `prompts/done/020-zerstoerung.md`; in
> `docs/ASSETS.md` Gebäude, Baukasten, Budgets; in `docs/ARCHITECTURE.md`
> „Destruction“, Brand und **explodierende Lager** (Zünder, `blast_caliber_mm`,
> Kettenreaktion), Sicht, Wegfindung, Besetzen, Wracks; `data/destruction.toml`
> (`env_warehouse`, `env_steel_hall`, Lager), `data/objects.toml`,
> `docs/SOURCES.md`, `docs/PERFORMANCE.md`; in `docs/MAPS.md` die
> Skizzen-Tabellen (`[[rail]]`, `[[building]]`, `[[row]]`).
>
> **Vorlage** (nicht im Repo, nur lesen):
> `docs/.vorlagen/fabrikstadt/fabrikstadt.webp`, ein mit KI erzeugtes Bild einer
> Industriestadt. Hier zählen nur die **Bauten**: Backsteinfabriken mit
> Schornsteinen und Rohrbrücken, Hallen, ein Ringlokschuppen mit Drehscheibe,
> eine Bahnhofshalle, Hafenkräne, Lagerhallen, Silos und Tanks, Güterzüge,
> Fachwerk- und Steinbrücken, ein Rathaus mit Uhrturm, viele Ruinen. Die Karte
> baut Prompt 156.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Die Bauten einer **Industriestadt um 1944**, als zerstörbare Kartenobjekte.
Vorbilder sind **Bautypen**, keine bestimmten Werke; Maße als Bautyp mit Quelle
nach `docs/SOURCES.md`, nichts aus anderen Spielen. Was schon da ist
(Stahlhalle, Lagerhaus, Depot, Treibstofflager, Bahnhofshalle, Kraftwerk,
Schornstein, Lokschuppen, Stellwerk), wird verwendet und nicht neu gebaut.

## 2. Bauten

- **Shedhalle** `env_factory_shed`: Fabrikhalle mit Sägezahndach (Glasbänder
  nach Norden), Backsteinwände, als **Feld von etwa 30 m × 20 m**, in Reihen und
  Zeilen aufreihbar zu Hallen von 60–150 m; Giebelstück mit großem Tor.
  **Durchfahrbar** durch die Tore, wenn die Regel für offene Hallen bzw.
  Torhäuser (134/137) das trägt: Infanterie und Fahrzeuge kämpfen dann **in**
  der Halle, die Wände sperren Sicht und Feuer. Sonst nur besetzbar, und die
  Durchfahrt als offener Punkt.
- **Geschossfabrik** `env_factory_block`: Backsteinbau mit vier bis fünf
  Geschossen, großen Sprossenfenstern, Treppenturm mit Wasserbehälter, etwa
  60 m × 18 m; Mittel- und Endstück.
- **Kesselhaus** mit kurzem Schornstein und **Fabrikschornstein** (der hohe aus
  137; dazu eine mittlere Variante um 45–60 m), Fallregel mit Trümmerband aus
  137.
- **Werksturm** `env_works_tower`: massiver Turmbau (Kühlturm, Kohlenturm oder
  Hochbunker der Vorlage als schlichter Backstein- bzw. Betonkörper, um 35 m),
  als Landmarke; nicht besetzbar.
- **Rathaus mit Uhrturm** `env_city_hall_tower`: Backsteinbau mit Mittelturm
  (vorhandenes Rathaus erweitern oder Ratsbau und Uhrturm aus 137 kleiner).
- **Hafenkran** `env_crane_harbor`: Wippkran auf Portal, auf Schienen am Kai, um
  25–30 m; Gitterwerk als Durchsicht-Textur. Hindernis sind nur die Portalfüße,
  darunter frei; sperrt keine Sicht; fällt bei Zerstörung zur Seite (kleines
  Trümmerband, Fallregel).
- **Silo** `env_silo` (Zylinder aus Beton oder Blech, um 20 m, als Gruppe von
  vier bis sechs mit Förderhaus) und **Großtank** `env_tank_storage` (stehender
  Stahltank, 12–20 m Durchmesser, mit Auffangwall).
- **Rohrbrücke und Förderband** `env_pipe_bridge`: aufgeständert zwischen
  Bauten; Hindernis sind nur die Stützen (Kulisse mit Stützen, über Straßen
  hoch genug für alle Fahrzeuge).
- **Güterzug als Gelände:** `env_wagon_box` (gedeckter Wagen), `env_wagon_flat`
  (Flachwagen, auch mit Ladung: Kisten, Träger, Stämme), `env_wagon_tank`
  (Kesselwagen), `env_wagon_hopper` (Schüttgutwagen), `env_locomotive`
  (Schlepptenderlok als schlichtes Modell). Sie **stehen** auf den Gleisen und
  fahren nicht.
- **Kaimauer, Poller, Stapel** (Kisten, Fässer, Stückgut aus vorhandenen
  Objekten) für den Hafen; ein **Frachter** am Kai nur, wenn ein schlichtes
  Modell wenig kostet (er liegt im Wasser und hat keine Regelwirkung), sonst
  offener Punkt.

Alle Bauten in `Intact`, `Damaged`, `Ruin` mit sichtbarem Inneren wie in
078/134: Hallen mit eingestürztem Dach und stehenden Wänden, Geschossfabrik als
Skelett, Wagen ausgebrannt bzw. umgestürzt. Dreiecke je Bau im Budget von 078
(`docs/ASSETS.md`); Fenster, Sprossen und Gitter als Textur. Vorschaubild für
jedes neue Modell (`just thumbnails --only=…`), `just units-web-sync`,
`just catalog-doc`.

## 3. Regeln (Simulation)

- Einträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster
  der vorhandenen Bauten. Richtgrößen: Shedfeld wie das Lagerhaus,
  Geschossfabrik wie ein Blockrandhaus oder mehr, Silo und Werksturm zäh
  (Beton), Kran und Rohrbrücke schwach.
- **Besetzbar:** Geschossfabrik, Shedhalle (falls nicht durchfahrbar gelöst),
  Rathaus, Kesselhaus. Nicht besetzbar: Schornsteine, Werksturm, Kran, Silo,
  Tank, Wagen.
- **Güterwagen:** sperren den Weg für Fahrzeuge und die Sicht bis zu ihrer
  Höhe, geben Infanterie Deckung; **Infanterie kommt unter bzw. zwischen den
  Wagen durch** (Lücke an jeder Kupplung; oder die Wagen sperren nur große
  Einheiten, je nachdem, was die Objektregeln hergeben). Schwere Panzer schieben
  bzw. walzen einen einzelnen Wagen **nicht** um (zu schwer; keine neue Regel).
  Zerstört bleibt ein Wrack, das weiter sperrt. Flachwagen ohne Ladung sperren
  die Sicht nicht.
- **Brand und Explosion:** **Großtanks und Kesselwagen** nach der vorhandenen
  Regel für Treibstofflager (Zünder, Explosion, Kettenreaktion), mit Werten
  nach ihrer Größe; ein Tanklager ist damit eine Stellung, die man nicht halten
  will, wenn Artillerie fällt. Hallen und Wagen mit Holz brennen. Vorsicht mit
  Kettenreaktionen in einem Zug aus zwölf Kesselwagen: mit Test begrenzen (die
  vorhandene Regel anwenden und messen; wenn ein Tick zu teuer wird, Abstände
  der Wagen vergrößern statt die Regel zu ändern).
- **Hohe Bauten** (Schornsteine, Kran, Werksturm) mit der Fallregel aus 137.
- **Der Golden Hash ändert sich** (neue Einträge in den Regeltabellen);
  gewollter Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit nennen.
  Vergleichslauf auf einer Karte ohne die neuen Bauten zeigt gleiche Ergebnisse
  wie vorher.

## 4. Zusammenstellungen im Skizzen-Generator

In `docs/MAPS.md` beschreiben; bestehende Skizzenkarten bleiben Bit für Bit
gleich.

- **`[[factory]]`:** ein Werksgelände als Polygon mit Art (`steel`, `machine`,
  `chemical`, `mill`) und Größe: Der Generator setzt Shedhallen in Zeilen,
  eine Geschossfabrik an der Straße, Kesselhaus und ein bis drei Schornsteine,
  je nach Art Werksturm, Silos oder Tanks, Rohrbrücken, Werkshof mit Stapeln,
  Mauer mit zwei Toren, Gleisanschluss. Schalter für den Anteil beschädigter
  und zerstörter Bauten.
- **`[[train]]`:** eine Wagenreihe auf einem Gleis (Anfang, Länge, Art der
  Wagen, mit oder ohne Lok, Lücken alle n Wagen); der Rangierbahnhof
  (`[[rail_yard]]` aus 141, sonst hier als parallele `[[rail]]`-Stränge) wird
  damit gefüllt: einige Gleise voll, einige leer, sodass Gassen entstehen.
- **`[[harbor]]`:** Kai entlang einer Uferlinie mit Kränen in regelmäßigem
  Abstand, Lagerhallen dahinter, Stapeln, Gleis am Kai.
- `[[city_block]]` mit der Backsteinfamilie und hohem Ruinenanteil für die
  Arbeiterviertel.

## 5. Stufen

**Stufe 1 – Modelle:** alle Bauten `Intact`; eine Ansichtskarte `works_test`
(etwa 900 m × 700 m: ein Werk mit Schornsteinen, ein Stück Hafen mit drei
Kränen, sechs Gleise mit Zügen, ein Tanklager, zwei Backsteinblöcke); Bilder
neben der Vorlage.

**Stufe 2 – Zustände, Regeln, Generator:** Abschnitt 3 und 4; Tests: Fahrt und
Gefecht durch die Shedhalle (oder Besetzen), Infanterie zwischen Wagen
hindurch und Panzer außen herum, Tanklager explodiert mit Kettenreaktion im
Zeitbudget, Kran und Schornstein fallen, zweimal gleicher Lauf ergibt gleichen
Hash.

**Stufe 3 – Abschluss:** **Bildrate** über `works_test` in jeder Ansicht und
hochgerechnet auf die Stadt aus Prompt 156 (`docs/PERFORMANCE.md`; viele kleine
Objekte: Wagen und Stapel als Instanzen, ferne Vereinfachung), Gefechtsreihe als
Bilder (Kampf im Rangierbahnhof, Beschuss des Tanklagers), `docs/ASSETS.md`,
`docs/MAPS.md`, `docs/ARCHITECTURE.md`, `docs/SOURCES.md`. `just check`,
`just selftest`.

## 6. Nicht Teil dieses Prompts

- Die Karte (Prompt 156).
- Fahrende Züge, arbeitende Kräne, Rauch als Sichtsperre aus Schornsteinen
  (Rauchfahnen nur als Bild).
- Produktion in Fabriken als Spielregel (Besitz gibt Einheiten oder Rohstoffe);
  die Bauten sind Stellung, Deckung und Ziel.
- Hochofenwerk im Detail, Zechen, Werften mit Schiffen auf Helling.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
