# Prompt 202 – Fehlende Modelle aus der Reihe 112–133

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `docs/ASSETS.md` ganz (Konventionen, Budgets, Baukasten, Vorschaubilder),
> `docs/SOURCES.md`, `docs/SCALE.md`, den Baukasten unter `tools/blender/kit/`
> (besonders `buildings.py`, `med.py`) und die Baupläne unter `assets-src/blueprints/`.
> Für jede Gruppe unten zusätzlich den genannten Prompt in `prompts/done/`
> (Abschnitte „Offen“/„Nicht Teil dieses Prompts“ und Entscheidungen).
>
> **Dieser Prompt läuft in Gruppen.** Jede Gruppe ist für sich fertig (Modelle,
> Katalog, Vorschaubilder, Prüfungen, Commit), bevor die nächste beginnt.
> Nach jeder Gruppe kurz Bilder zeigen und ohne Halt weiterarbeiten.

## 1. Wunsch des Nutzers (2026-11-23)

Während der Prompts 112–133 haben die Sessions Modelle als „offen“ gemeldet, die
heute durch Platzhalter oder vorhandene Katalog-Modelle vertreten werden. Der Nutzer
möchte sie gesammelt in einem Prompt bauen lassen.

## 2. Weg

- Modelle entstehen **wie bisher aus Skripten** (Baukasten und Baupläne, Blender ohne
  Fenster), damit sie wiederholbar sind und `validate`/Budgets greifen. Das
  Blender-MCP darf zum Ansehen, Prüfen und für den Vergleich mit Bildvorlagen dienen,
  aber kein Modell darf nur als Handarbeit in einer `.blend` ohne Skript entstehen.
- Neue Katalogeinträge nur mit echten Werten und Quelle (`docs/SOURCES.md`; keine
  Daten aus anderen Spielen, keine Hakenkreuze/SS-Runen, kein Rotes Kreuz).
- Nach jeder Modelländerung: `just thumbnails --only=<id>,…`, `just units-web-sync`,
  bei Katalogänderung `just catalog-doc`; Fahrzeuge zusätzlich `just vehicle-db-check`.
- Ersetzt ein neues Modell einen Platzhalter auf einer Karte (Geodaten- oder
  Skizzenkarte), den Importer bzw. die Skizze umstellen, Karte neu erzeugen,
  `just gen-geo-check`, `just map-check`, `just map-reach` und im Commit sagen, dass
  sich die Karte beabsichtigt ändert. Golden Hash: neue Regeldaten sollen eine Partie
  nur dann berühren, wenn sie vorkommen (Muster `economy::keeps_faction`, Prompt 117,
  und die Boot-Regeln aus 119); ändert er sich doch, Grund finden und nennen.
- Bildrate auf den betroffenen Karten vorher/nachher messen (`just objects-bench`
  bzw. `just bench-view`), wenn Gebäude oder Pflanzen in großer Zahl ersetzt werden.

## 3. Gruppen

### A. Fahrzeuge und Flugzeuge

- **US-Jeep** (Willys MB): fehlt im US-Katalog; Prompt 121 musste die zweite Welle
  mit Halbkette fahren lassen. Katalog, Modell, Wirtschaft/Baulisten wie beim
  britischen Jeep aus 117; Mission `gulf_landing` danach prüfen.
- **Spitfire IX, Typhoon Ib, Mosquito FB VI** (117): heute aus US-Bauplänen skaliert,
  Silhouetten nur ungefähr. Nach Zeichnungen neu, Maße gegen Katalog.
- **Britische Vereinfachungen** (117): Universal Carrier hinten offen, 25-Pfünder
  mit Bettung (Feuerplattform), Churchill-Ketten über den Rumpf geführt.
- **Horsa-Lastensegler** (117): Modell. Die Landeregel (Luftlande-System kennt nur
  Absprung) nur, wenn sie klein bleibt; sonst Modell bauen und die Regel als offenen
  Punkt mit Vorschlag notieren.
- **LCT und Marinefährprahm** (119): Modelle nach dem Boots-Baukasten aus 119
  (Baugruppe `boat`, Bugklappe `Ramp`), Katalog mit Quellen; nur in Missionen.

### B. Befestigungen

- **Schwere Küstengeschütze, Kasematten, Flak am Kai** (113, 121): heute
  `fort_bunker_mg`, Gräben und Sandsäcke als Platzhalter; in 121 stehen leFH 18
  für 90- und 102-mm-Geschütze.
- **Forts, Batterien und Kasernen** der Küstenhöhen (113): heute normale
  Katalog-Häuser.

### C. Gebäude

- **Backsteinviertel** (112): Brownstone-Reihenhaus, Mietshaus mit Feuertreppen,
  gusseisernes Loft (SoHo), Lagerhaus am Kai; Wahrzeichen: Bogen am Washington Square,
  Kirchen. Importer `brick_quarter` danach auf die neuen Modelle umstellen.
- **Dünenküste** (114): Reetdachhaus und weitere Inselbauten.
- **`env_church`** (120): alter Fehler, eine Turmwand reicht bis in die Kirchenhalle;
  im neuen Campanile behoben, das alte Modell neu bauen und alle Karten mit Kirche
  ansehen.

### D. Pflanzen

- **Feigenkaktus, Agave, Weinstöcke** (120), mit Baum-/Buschregeln wie die anderen
  Mittelmeer-Pflanzen; Weinberge im Importer danach als Weinstock-Reihen statt
  `field_green`.
- **Pinienkrone** (120): wirkt aus der Nähe wie Laub; Schirmform nachschärfen.

### E. Weitere Funde aus 122–133

Die Sessions der Prompts 122–133 laufen noch. Was sie als fehlende Modelle melden,
wird hier nachgetragen, bevor dieser Prompt startet.

- **Größere Kirche** (122): Im größten Dorf von `archipelago` steht heute eine
  normale Kirche plus Rathaus, weil der Katalog keine größere Kirche hat.
- **Brückenturm / Torhaus** (123): An den Brückenenden von `river_front` stehen
  heute `env_house_stone` und `fort_bunker_mg` als Stellvertreter.
- **Lkw als Kulisse** (124): Es gibt kein neutrales Lkw-Objekt; die Lager von
  `trench_line` haben deshalb keine abgestellten Lkw.
- **Bohlenweg** (124): Holzbohlen über Schlamm und Grabengrund fehlen auf
  `trench_line`. Falls dafür eine Regel nötig ist (Wegkosten), klein halten oder als
  offenen Punkt notieren.
- **Zelt `env_tent`** (124): wirkt im Spiel sehr hell; Farbe/Material prüfen.
- **Geschütz als Kulisse** (125): Die Batterien von `river_fork_front` bestehen nur
  aus Sandsackstellungen und Grabenstücken (`[[works]] battery`); ein neutrales,
  abgestelltes Geschützobjekt fehlt. Auch die Lager dort haben keine Lkw (siehe oben).
- **Wüstengestrüpp** (126, z. B. Kameldorn): auf `mesa_desert` steht heute
  `env_bush` (runde Hasel) im Wüstenlook.
- **Felsnadeln / Hoodoos** (126): heute nur Gelände; ein Felsmodell würde die
  schmalen Nadeln auf `mesa_desert` lesbarer machen.
- **Verlassenes Wüstengehöft** (126): heute aus beschädigten Wüstenhäusern,
  `env_wall_mud` und `env_well_desert` zusammengesetzt; prüfen, ob ein eigenes
  Ensemble lohnt.
- **Rampenbefestigung** (126): Rampen auf `mesa_desert` sind nur Gelände mit
  Feldweg; Stützmauer/Böschungskante als Objekt wäre möglich.
- **Burgfelsen** (127, `castle_rock`):
  - **Stein-Mietshaus (Tenement) und georgianisches Reihenhaus** mit Schieferdach.
    Heute stehen dort Katalog-Stadthäuser, teils mit roten Ziegeldächern. Das
    Mietshaus mit Gruppe C (Backsteinviertel) abstimmen.
  - **Bahnhofshalle** mit Dach über den Gleisen.
  - **Wahrzeichen:** Kathedrale mit Kronenturm, Palast am Fuß des Berges,
    Denkmalturm auf dem kleinen Hügel (heute Küstenwachturm), Sternwarte,
    Säulendenkmal. Ohne echte Namen in Karte und Katalog.
  - **Stahlbogen der Talbrücke:** heute Vorlandfelder aus Prompt 115.
- **Kesselsee** (128, `kessel_lake`, alles nur Verschönerung):
  - **Kletterpfad** an den `climb_*`-Stellen: Stufen, Seil oder Leiter. Heute ist
    dort nur Gelände, Felsbrocken sperren den Weg für Fahrzeuge.
  - **Furtpfähle** oder Markierungsstangen an den Untiefen.
  - **Bootssteg / Ruderboot** am See. Mit dem Boots-Baukasten aus 119 abstimmen.
- **Grubenhügel** (129, `pit_hill`):
  - **Förderturm:** heute `env_tower_coast` als Platzhalter.
  - **Grubenanlagen:** Gebäude und Gerät an der Grube, Lorenbahn, Abraumhalden.
  - **Bergbau-Werkshallen:** heute `env_warehouse`.
  - **Bahnhofsgebäude:** heute `env_house_town` mit Lagerhalle; zusammen mit der
    Bahnhofshalle aus 127 bauen.
  - **Laderampe der Verladestelle:** heute nur eine Halle mit Kisten und Fässern.

## 4. Nicht Teil dieses Prompts

- Britische Stimmen, Boots- und Paddelgeräusche (Ton, keine Modelle).
- Hochstraße und Güterbahn auf Stelzen (112), Tunnel (113), Unterführungen (115):
  brauchen Regeln für zwei Ebenen, nicht nur Modelle.
- Kanada, Polen, Land Mattress (117).

## 5. Abschluss

Je Gruppe: `just check`, betroffene Selbsttests, Vorschaubilder, ein Commit mit
„(prompt 202)“. Am Ende Tabelle aller gebauten Modelle (Dreiecke, Budget, ersetzt
welchen Platzhalter, betroffene Karten) und offene Punkte in der README-Zeile,
Prompt nach `prompts/done/` verschieben.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle eintragen (vorläufig), mit
Begründung.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Start vor 131–133 (Nutzer, 2026-11-24) | Deren Funde fehlen in Gruppe E und müssen später nachgetragen werden. |
| 2 | `us_jeep` mit den Werten von `uk_jeep` (gleiche Quelle), am Ende der US-Fahrzeugliste | Gleiches Fahrzeug; die KI-Mischungen greifen über feste Plätze auf die Listen zu, neue Einträge gehören ans Ende. |
| 3 | Golden Hashes geändert (`determinism.rs`, `economy.rs`) | Die Baulisten gehen in den Zustands-Hash ein und „us“ wird in jeder Partie behalten (`ALWAYS_KEPT`); ohne den Jeep in der Liste stimmen beide alten Werte, der Katalogeintrag allein berührt sie nicht. Beabsichtigte Regeländerung. |
| 4 | `gulf_landing`: Jeep zusätzlich (zweiter LCVP neben der Pak), Halbkette bleibt | Mit Jeep statt Halbkette gewann der Autopilot Normal nicht mehr in 20 min; der Test wird nicht abgeschwächt. |
| 5 | Jeep offen (Scheibe hoch, Verdeck ab, Spriegel stehen), US und britisch; neuer Baustein `open_tub` | Bildvorlage des Nutzers; die Spriegel tragen die Kataloghöhe 1,77 m (Verdeck auf). |
| 6 | Flugzeuge waagerecht gezeichnet und als Ganzes aufs Fahrwerk gekippt (`ground_angle = "fit"`) | Vorher stieg nur der Rumpf, Flügel, Gondeln und Haube blieben waagerecht: falsche Winkel zueinander (Hinweis des Nutzers zur Spitfire). |
| 7 | Spitfire als Hochrücken (gerahmte Haube mit hinterer Verglasung), nicht mit Tropfenhaube | Dreiseitenriss des Nutzers; Maße daraus geschätzt (±5 %). |
| 8 | Kataloghöhe der Mosquito (4,65 m, unsicher) als Spornlage mit einem Blatt senkrecht gebaut | Wie bei den anderen Jägern; die Quelle sagt nicht, welche Lage gemeint ist. |
| 9 | Horsa als Katalogeintrag `aircraft_transport` hinter `uk_dakota`, ohne Landeregel | Lufteinsätze nehmen den ersten Katalogeintrag der Kategorie je Fraktion: die Dakota bleibt das Absetzflugzeug, kein Hash ändert sich. Offen: Landung eines Seglers (Vorschlag: eigener Einsatz „glider“, der Trupp und Jeep am Zielpunkt absetzt und das Modell als Wrack-Objekt liegen lässt). `ceiling_m` geschätzt (Pflichtfeld, keine Quelle). |
| 10 | Katalogprüfung für Boote erweitert (Länge bis 50 m, Breite 12 m, Masse 300 t, Besatzung 25, Zuladung 150 t) | LCT(5) und Marinefährprahm liegen über den Grenzen aus Prompt 119 (15-m-Boote). Passagiere bleiben bei höchstens 120 (ein Sockel je Platz). |
| 11 | LCT(5) (US) statt Mark 4; Marinefährprahm als Typ A mit den unteren Werten der Quellenspanne | Die Quelle nennt für den MFP nur Spannen über A–D. Höhen, Passagiere, Fahrzeugplätze geschätzt und als unsicher markiert. Aufbauten als Objekt `Crane` (zählt nicht zur Kataloghöhe des Rumpfs). |
| 12 | Große Boote nicht im Spiel erprobt | Nur Modelle und Katalog; ob Wegsuche und Landung mit 36–47 m Länge funktionieren, zeigt erst eine Mission. |
| 13 | Gruppe B: vier neutrale Befestigungsobjekte als Kulisse (`fort_casemate_gun`, `fort_coast_gun` mit Variante `Flak`, `fort_battery_concrete` mit Variante `Disarmed`, `fort_fire_control`); Geschütze sind Teil des Modells, keine Waffe | Spielbare Küstengeschütze wären neue Einheiten mit Regeln (Prompt 113 nennt sie als eigenen offenen Punkt); in `gulf_landing` schießen weiter die leFH 18. |
| 14 | Alle Größen der Befestigungen sind Schätzwerte (als unsicher markiert) | Die gelesenen Seiten (Regelbau, Atlantic Wall, Coastal artillery) nennen keine Maße, nur Wandstärken; Rohrlänge nach Cannone da 90/53. Keine Seite für das 102-mm-Geschütz gefunden (404). |
| 15 | Kaserne: vorhandenes Modell `env_barracks` als Kartenobjekt freigeschaltet (Objekt-, Zerstörungs-, Feuer-, Sicht-, Infanterieregeln wie das Blockhaus, 12 Plätze) statt eines neuen Modells | Das Modell gab es schon (Basisgebäude); es fehlte nur als Kartenobjekt. |
| 16 | Golden Hashes erneut geändert (Gruppe B) | Die neuen Kartenobjekte erweitern die gehashten Objekt-, Zerstörungs-, Feuer-, Sicht- und Infanterieregeln (wie bei Prompt 127). Die Demo nutzt keines: jede Einheit endet, wo sie vorher endete (`cli run --verbose`, alte gegen neue Regeln, nur die Hash-Zeile unterscheidet sich). |
| 17 | Zuordnung der Werke: `coastal_heights` 10 Batterien (5 bewaffnet, 5 entwaffnet), 3 Kasematten (Townsley, zweimal 129), 2 Feuerleitstände; `landing_gulf` 3 Küstengeschütze (Poro zweimal, Fonza), Flak am Hafen, Kasematte am Südende, der Höhenrücken behält den MG-Bunker | Welche Batterie 1944 noch bewaffnet war, ist geraten und nicht gegen eine Quelle geprüft. Boden unter den Werken wird wie unter Gebäuden eingeebnet. |
| 18 | Gruppe C: Brownstone, Mietshaus, Loft und Kai-Lagerhaus mit dem Wohnblock-Generator (neue Parameter `front`, `stoop`, `fire_escape`, `top_cornice`), nicht als eigene Generatoren | Fassadenraster, Schadensstufen und Regeln sind dort schon gelöst. Brownstone als dunkles Mauerwerk (kein Sandstein-Material), Gusseisenfront als heller Putz (kein Eisen-Material für Wände). |
| 19 | Das Mietshaus ist ein eigenes Modell (`env_block_tenement`, US-Stadt); das Stein-Mietshaus für `castle_rock` (Gruppe E) wird davon getrennt gebaut | Andere Bauweise (Schieferdach, Naturstein) als das Backstein-Mietshaus mit Flachdach. |
| 20 | Bogen als allgemeiner Gedenkbogen ohne Inschrift und Figuren mit den Hauptmaßen der Quelle; Standort 60 m nördlich des Platz-Markers von Hand | Kein echtes Denkmal nachgebaut; der Marker ist ein Versorgungspunkt und bleibt frei. |
| 21 | Eine große Kirche (`env_church_large`) für `brick_quarter` und zugleich für Gruppe E (`archipelago`) | Gleicher Bedarf. |
| 22 | `env_church`: Ursache waren die Glockenstuben-Öffnungen der Seitenwände (in Kirchenmitte statt Turmmitte), nicht nur das kleine Fenster | Erst der Vorschaubogen der großen Kirche zeigte es. Karten mit Kirche nicht im laufenden Spiel angesehen, nur im Vorschaubogen. |
| 23 | Dünenküste: nur das Reetdachhaus; „weitere Inselbauten“ offen | Der Prompt nennt keine bestimmten; Vorschläge: Leuchtturm, Strandhalle, Bootsschuppen. |
| 24 | `brick_quarter` hat jetzt rund 8 100 statt 5 628 Gebäude | Die Brownstones sind schmaler als die Stadthäuser. Bildrate unverändert (Übersicht 72,1 → 71,6 FPS, Dorfansicht 88,8 → 91,6 FPS). |

## Nebenbefunde (nicht Teil dieses Prompts, nicht geändert)

- `just validate-assets`: sechs Geschütze sind veraltet gebaut (`de_flak36`, `de_lefh18`, `de_pak40`, `su_m30`, `su_zis3`, `us_57mm_m1`; Bauplan oder Katalog nach dem letzten Bau geändert).
- `just gallery-check`: 13 Meldungen, alle schon vor diesem Prompt (Boote und `de_pilb39` ohne Kennzeichen-Sockel bzw. `Socket_Muzzle`, `uk_37in_aa`, `uk_75mm_pack`, und `env_tent`: Fläche 2 ohne Env-Shader – vermutlich der Grund, warum das Zelt so hell wirkt, siehe Gruppe E).

