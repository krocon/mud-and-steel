# Prompt 134 – Großstadthäuser: Blockrand mit Mansarddach, Palastflügel, Hofbau (zerstörbar)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/078-hochhaeuser.md` ganz (Generator `apartment_block`, Zustände
> `Intact` / `Damaged` / `Ruin` mit sichtbarem Inneren, Regeln, Kamera,
> `city_blocks`, offene Punkte) und `prompts/done/020-zerstoerung.md`; in
> `docs/ASSETS.md` die Abschnitte zu Gebäuden (Baukasten `kit/buildings.py`,
> modulare Varianten, Budgets, Blaupausen), in `docs/ARCHITECTURE.md`
> „Destruction“ und die Regeln für Gebäude (Weg, Sicht, Besetzen, Brand),
> `data/destruction.toml`, `data/objects.toml`, `docs/SOURCES.md`,
> `docs/PERFORMANCE.md`; in `docs/MAPS.md` „Maps from picture sketches (prompt
> 088)“ (`[[town]]`, `[[row]]`, `[[ring]]`, `[[building]]`, `[[square]]`) und die
> Karte `river_city` als Muster für eine Stadt aus Skizzen-Tabellen;
> `crates/cli/src/gen_sketch.rs`.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/eisenturm/eisenturm.webp`,
> ein mit KI erzeugtes Stadtbild. Hier zählen nur die **Haustypen**: geschlossene
> Blöcke mit Innenhof, helle Steinfassaden, graue Mansarddächer, abgeschrägte
> Ecken; ein Palast aus zwei gebogenen Flügeln; ein großer Hofbau mit rotem
> Ziegeldach; einzelne Blöcke in Trümmern. Die Karte selbst baut Prompt 136, den
> Turm Prompt 135.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gebäudefamilie für eine **europäische Großstadt des 19. Jahrhunderts**
(Bautyp: Blockrandbebauung mit Mietshäusern, wie sie um 1944 in vielen Städten
stand), aus der der Skizzen-Generator ganze Straßenblöcke mit Innenhof setzt.
Alle Häuser sind **zerstörbar** wie die Wohnblöcke aus Prompt 078, besetzbar und
gelten für Weg und Sicht als Gebäude.

## 2. Haustypen

Als Blaupausen des vorhandenen Baukastens, soweit er reicht; fehlt ein Bauteil
(Mansarddach, Bogen, schräge Ecke), den Baukasten rückwärtsverträglich
erweitern (bestehende Modelle bleiben gleich). Maße als Bautyp mit Quelle nach
`docs/SOURCES.md` (Traufhöhe, Geschosshöhen, Haustiefe, Hofgröße); keine Maße
oder Formen aus anderen Spielen.

- **Blockrandhaus, gerades Stück** `env_block_city`: fünf bis sechs Geschosse
  plus Mansarde, Traufe etwa 18–20 m, Haustiefe etwa 12–14 m, Frontlänge etwa
  24 m; Ladenzone im Erdgeschoss, durchlaufende Balkone im zweiten und im
  obersten Vollgeschoss, heller Kalkstein, graues Zink- oder Schieferdach mit
  Gauben und Schornsteinreihen. Straßenseite reich, Hofseite schlicht.
- **Eckhaus** `env_block_city_corner`: mit abgeschrägter Ecke (Eingang oder Laden
  in der Schräge), schließt zwei gerade Stücke im rechten Winkel an.
- **Spitzes Eckhaus** `env_block_city_wedge`: für spitze Blockecken an schrägen
  Straßen (Winkel etwa 45–60°), mit runder oder abgeschrägter Spitze.
- **Torhaus** `env_block_city_gate`: gerades Stück mit Durchfahrt in den Hof
  (für Infanterie und Fahrzeuge bis zu einer Breite, die die Session nach der
  Wegfindung festlegt; sonst nur Infanterie und als Entscheidung festhalten).
- **Palastflügel** `env_palace_wing` und **Palastkopf** `env_palace_pavilion`:
  ein leicht gebogener, zwei- bis dreigeschossiger Flügel mit Kolonnade (etwa
  60 m lang, mehrere aneinandergereiht ergeben einen Viertelkreis) und ein
  kubischer Kopfbau; heller Stein, flaches Dach. Stil der 1930er Jahre, kein
  bestimmtes Gebäude nachbilden.
- **Hofbau** `env_quad_wing` und Eckstück: zwei- bis dreigeschossige Flügel mit
  rotem Ziegeldach, die einen großen Hof umschließen (Kaserne, Schule,
  Hospital; **ohne Rotes Kreuz**).

Jeder Typ in `Intact`, `Damaged` (Dach offen, Fassade teils eingestürzt,
Brandspuren) und `Ruin` (ein bis zwei Geschosse stehen, sichtbares Inneres,
Schuttkegel zur Straße), wie in 078. Dreiecke je Haus im Budget von 078
(`docs/ASSETS.md`); Fenster und Balkongitter im Shader bzw. als Textur, nicht
als Geometrie. Zwei bis drei Fassadenvarianten je geradem Stück (Farbe,
Balkone, Läden), damit eine Straße nicht geklont wirkt.

## 3. Regeln (Simulation)

- Einträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster der
  Wohnblöcke: Grundriss, Höhe, Sichtsperre, Besatzung, Brand; Trefferpunkte nach
  Masse (gerades Stück etwa wie der Mietshausblock aus 078, Palast und Hofbau
  nach ihrer Größe), Ruine blockiert nur Panzer bzw. wie bei 078 geregelt.
- **Blöcke fallen stückweise:** Jedes Haus ist ein eigenes Objekt mit eigenen
  Trefferpunkten. Ein zerstörtes Stück öffnet den Hof (neuer Weg für Infanterie,
  neue Sichtlinie); das als Test.
- Keine neue Regelart, wenn die vorhandenen reichen. **Der Golden Hash ändert
  sich**, weil die Regeltabellen wachsen (wie bei 078); das ist ein gewollter
  Regelzuwachs: Ursache prüfen, Hash neu setzen und im Commit nennen. Bestehende
  Karten spielen sich gleich (Vergleichslauf vorher/nachher mit festem Seed auf
  einer Karte ohne die neuen Häuser, gleiche Ergebnisse bis auf den Hash der
  Tabellen).

## 4. Blöcke im Skizzen-Generator

- Neue Tabelle **`[[city_block]]`** (in `docs/MAPS.md` beschreiben): ein
  Straßenblock als Polygon (vier bis sechs Ecken, auch schiefwinklig). Der
  Generator setzt den Rand lückenlos aus geraden Stücken, Eck- und Keilhäusern
  (Längen ausgleichen über die Varianten und kleine Fugen), ein bis zwei
  Torhäuser, und füllt den **Innenhof** mit Hofboden, einzelnen Bäumen, Schuppen
  oder einem Hinterhaus. Schalter: Geschosszahl, Fassadenfamilie, Anteil
  beschädigter und zerstörter Häuser.
- **Viertel aus Straßen:** eine Tabelle oder ein Schalter, der aus einem
  Straßennetz (gedrehte Raster, Diagonalen, Ringstraßen wie in `river_city`) die
  Blöcke selbst ableitet, damit die Karte nicht jeden Block einzeln beschreiben
  muss. Den einfachsten Weg wählen, der die Vorlage trägt.
- **Vorab zerstört:** Die Vorlage zeigt einzelne Blöcke in Trümmern. Prüfen, ob
  Kartenobjekte schon mit einem Schadenszustand starten können; wenn nicht, das
  als Feld der Kartenobjekte ergänzen (Standard `Intact`, bestehende Karten Bit
  für Bit gleich).
- **Palast und Hofbau** als Zusammenstellung: `[[palace]]` (Mittelpunkt,
  Öffnungsrichtung, Radius: zwei Viertelkreis-Flügel mit Kopfbauten um eine
  Terrasse) und der Hofbau als Rechteck aus Flügeln; oder beides über
  `[[building]]`-Listen, wenn das einfacher ist.
- Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben.

## 5. Stufen

**Stufe 1 – Modelle:** Blockrandhaus gerade, Eckhaus, Torhaus in drei Zuständen;
Bilder eines von Hand gesetzten Blocks neben der Vorlage.

**Stufe 2 – Regeln und Generator:** Abschnitt 3 und 4, Keilhaus, Palast, Hofbau;
eine Ansichtskarte `city_quarter` (etwa 800 m × 800 m, zwölf bis sechzehn Blöcke
an geraden und schrägen Straßen, ein Platz, ein Palast, ein Hofbau) nach dem
Muster von `city_blocks`.

**Stufe 3 – Prüfen und Abschluss:**

- Tests: Weg durch Torhaus und durch die Lücke eines zerstörten Hauses, Sicht
  über den Hof, Besetzen, Zerstörung in Stufen.
- Bild- und Gefechtsreihe wie `just block-shots` / `just block-fight` auf
  `city_quarter` (Panzer und Artillerie gegen einen besetzten Block).
- **Bildrate** über dem Viertel in jeder Ansicht und hochgerechnet auf eine Stadt
  von etwa 120 Blöcken (Prompt 136); reicht das Budget nicht, Stücke länger
  machen oder ferne Häuser zusammenfassen und festhalten
  (`docs/PERFORMANCE.md`).
- Vorschaubilder für jedes neue Modell (`just thumbnails --only=…`),
  `just units-web-sync`, `just catalog-doc`.
- `docs/ASSETS.md`, `docs/MAPS.md`, `docs/ARCHITECTURE.md` (Tabelle unter
  „Destruction“) ergänzen. `just check`, `just selftest`.

## 6. Nicht Teil dieses Prompts

- Der Gitterturm (Prompt 135) und die Stadtkarte (Prompt 136).
- Kämpfe von Geschoss zu Geschoss, Keller, Kanalisation.
- Straßenmöbel (Laternen, Kioske, Litfaßsäulen), Hausboote und Kähne am Kai.
- Innenräume über das sichtbare Innere der Ruinen hinaus.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
