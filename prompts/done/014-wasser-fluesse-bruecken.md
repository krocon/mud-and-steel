# Prompt 014 – Wasser, Flüsse und Brücken

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Pathfinding, Terrain rendering, Map objects, Lighting),
> `docs/DETERMINISM.md`, `docs/MAPS.md` und `docs/ASSETS.md` (Environment objects).

## 1. Ziel

Flüsse und Bäche wie in den Stil-Vorlagen: sichtbares Wasser mit Strömung, Schaum an
Ufern und Steinen, Uferbewuchs, und **Brücken als taktische Engstellen**. Einheiten
queren tiefes Wasser nur über Brücken oder Furten. Holzbrücken tragen keine
mittleren und schweren Panzer, Steinbrücken alles. Brücken sind zerstörbar
vorbereitet (die Zerstörung selbst kommt mit Prompt 020).

Die Regeln (Brücken, Tragfähigkeit, Wegfindung) liegen in `crates/sim`,
deterministisch und ganzzahlig. Wasserfläche, Strömung, Schaum und Uferbewuchs sind
reine Darstellung.

## 2. Ausgangslage

- Geländearten (`data/terrain.toml`): `shallow_water` (id 6, 30 % Geländetempo,
  befahrbar = Furt) und `deep_water` (id 7, unpassierbar). Der Generator
  (`crates/cli/src/gen_map.rs`) schneidet ein 3,5 m tiefes Flussbett, Furten entstehen
  dort, wo Wege den Fluss queren. Das Bett ist fast flach: tiefes und flaches Wasser
  unterscheiden sich in der Höhe kaum.
- Wegfindung (`crates/sim/src/path.rs`): 4-m-Zellen, Größenklassen `Small`/`Large`
  (Large mit 2 m Abstand zu unpassierbarem Gelände), Kosten pro Einheitentyp,
  Komponenten pro Klasse für unerreichbare Ziele, Hindernisse änderbar
  (`set_obstacle`).
- Kartenobjekte (Prompt 013, `crates/sim/src/objects.rs`, `data/objects.toml`):
  `objects.csv`, Regeln mit Blockier-Klasse und Grundform, im Karten-Hash.
- Brückenmodelle (Prompt 007): `env_bridge_wood` (Varianten `Span8/12/16`,
  4,5 m breit, Deck 2 m über der Basis) und `env_bridge_stone` (`Arch1/Arch2` 20 m,
  `Arch3` 30 m, 6 m breit, Deck 5 m über der Basis). Extras `deck_height_m`,
  `span_m`, `deck_width_m` an jeder Variante. Noch keine Kartenobjekte.
- Darstellung: eigener Terrain-Renderer mit Splatting (Flussbett nur abgedunkelt
  „bis Prompt 014“), Objekte und Deko per MultiMesh (`map_objects.gd`), Biome
  (Prompt 015).

## 3. Umfang

### 3.1 Simulation: Brücken als Kartenobjekte

1. **Regeln** in `data/objects.toml`: Brücken-Arten bekommen eine Deck-Regel
   (`deck = { width_mm, max_load_kg, terrain }`): Deckbreite (Fahrbahn zwischen den
   Geländern), Tragfähigkeit in kg und die Geländeart, deren Tempo auf dem Deck gilt
   (`road`). Länge pro Variante (`length_mm`). Die Deck-Regel ist Teil des
   Karten-Hashes; Arten ohne Deck hashen wie bisher.
2. **Oberfläche der Karte:** `Map` leitet aus den Objekten die Brücken ab
   (`Map::bridges()`): Achse, halbe Länge und Breite, Deckhöhe an beiden Enden
   (Geländehöhe an den Endpunkten der Achse), Tragfähigkeit, Geländeart, intakt.
   `Map::height_at` und `Map::terrain_at` liefern auf einem intakten Deck die
   Deckhöhe (linear zwischen den Enden, nie unter dem Gelände) und die
   Deck-Geländeart. Die rohen Werte gibt es als `ground_height_at` /
   `ground_terrain_at`. So brauchen Bewegung und Tempo in `sim.rs` keine eigene
   Brückenlogik: Einheiten fahren auf das Deck, stehen in Deckhöhe und fallen nicht
   seitlich ins Wasser (Wasser neben dem Deck bleibt unpassierbar).
3. **Wegfindung:** Eine Zelle ist Deck-Zelle, wenn ihr Mittelpunkt auf einem
   intakten Deck liegt. Für sie gelten statt der Proben darunter: befahrbar, keine
   Steilheit, kein Rand-Abstand, Kosten der Deck-Geländeart, Steigung des Decks.
   Proben unter einem Deck zählen nicht als unpassierbar für den Rand-Abstand der
   Nachbarzellen. Bedingung beim Laden: die Zellen entlang der Brückenachse sind
   durchgehend Deck-Zellen (sonst Fehler mit Art und Position der Brücke); in der Praxis heißt das
   Achse ungefähr parallel zum Raster und auf einer Zellmitte.
4. **Tragfähigkeit:** `UnitType` bekommt `mass_kg` aus dem Katalog (0 für
   Einträge ohne Masse, z. B. Infanterie). Deck-Zellen einer Brücke mit
   `max_load_kg < mass_kg` sind für diesen Einheitentyp gesperrt (Kosten 0).
   Komponenten für unerreichbare Ziele werden pro (Größenklasse, Laststufe)
   berechnet statt pro Klasse. Gruppenpfade nutzen das restriktivste Profil
   (Klasse, dann Laststufe, dann Größe), Formationsglieder prüfen Sichtlinien mit
   ihrem eigenen Profil.
5. **Zerstörbar vorbereitet:** `Sim::set_bridge_intact(index, intact)` schaltet das
   Deck ab oder wieder an (Karte copy-on-write über `Arc::make_mut`), baut die
   betroffenen Zellen neu und plant Einheiten neu, deren Route sie berührt, wie
   `set_path_obstacle`. Der Zustand ist Teil des State-Hashes. Noch kein Befehl ruft
   es auf (Prompt 020).
6. **Replay-Format** 3 (UnitType hat ein neues Feld).

### 3.2 Generator: Flüsse, Furten, Brücken

1. **Flussprofil** mit erkennbarer Tiefe: Ufer 3 m unter dem Umland, flaches Wasser
   bis 0,5 m unter der Wasserlinie, tiefe Rinne bis 1,8 m; Furten ohne Rinne
   (weich übergeblendet).
2. **Brücken:** pro angefangenem Kilometer Fluss eine Querung mit Brücke, mindestens
   zwei, abwechselnd Stein und Holz, zusätzlich zur bestehenden Furt. Jede Brücke
   liegt in Ost-West-Richtung (Heading 90) mit der Achse auf einer Zellmitte; der
   Fluss wird an der Brücke auf die Spannweite verengt, Rampen (Dämme, ≤ 12 %
   Steigung) führen auf Deckhöhe (Holz 1,5 m, Stein 3 m über dem Ufer). Ein Feldweg
   führt von der Startwiese über die Brücke zum Ostrand. Variante: die kürzeste, die
   Wasser plus 4 m auf jeder Seite überspannt.
3. **Steine im Fluss** (kleine und mittlere `env_rock`, blockieren nicht) für Schaum
   und Optik; Brücken, Rampen und Furten bleiben frei.
4. `test_small` und `test_large` neu erzeugen (Golden-Hash-Änderung, im Commit
   begründen). Marker `bridge_stone`, `bridge_wood`, `ford` in `map.toml` für Tests
   und Kameraansichten.

### 3.3 Darstellung (nur Präsentation)

1. **Wasserfläche:** `godot/scenes/world/water.gd` + `water.gdshader`. Die Bridge
   berechnet in Rust (Floats erlaubt) pro Wasserprobe: Wasserspiegel (aus den
   Uferhöhen, entlang des Flusses geglättet), Uferabstand, Fließvektor (entlang der
   Ufer, schneller in der Mitte, um Steine herum) und Tiefe, und baut daraus ein Mesh
   nur über Wasser (plus ein Probenring Überlappung, das Gelände verdeckt den Rest).
   Die Fließrichtung kommt aus `water_flow_deg` in `map.toml` (optional, wie `biome`
   nur Darstellung, nicht im Hash; Standard 180 = nach Süden).
2. **Shader:** Flow-Map mit zwei phasenversetzten Normal-Samples, Schaum an Ufern,
   in flachem Wasser und um Steine (mitfließend), Farbe nach Tiefe (Bildschirm-Tiefe
   plus Rasterwert), Brechung über die Screen-Textur, Fresnel-Spiegelung des Himmels
   und Sonnenglanz. Keine SSR (zu teuer für eine 4-km-Karte), keine Planar-Reflexion.
   Pro Biom eine Wasserfarbe (Winter: dunkler, kühler).
3. **Ufer:** Der Terrain-Shader bekommt den Wasserspiegel und zeichnet einen nassen,
   dunkleren Streifen knapp über der Wasserlinie; der alte Platzhalter („wet river
   bed until prompt 014“) wird auf das Bett unter Wasser reduziert.
4. **Uferbewuchs:** Variante `Reeds` von `env_grass_clump` (Blender-Skript,
   `just env env_grass_clump`), Deko-Schicht „reeds“ in `map_objects.gd`, deren
   Dichte vom Abstand zum Wasser abhängt (neuer Wert in den Deko-Rastern); dazu
   mehr Büsche und Steine an Ufern.
5. **Brückenmodelle** stehen so, dass ihr Deck (`deck_height_m` aus den Extras) auf
   der Deckhöhe der Simulation liegt.
6. **Werkzeuge:** `just terrain-shots` bekommt Ansichten der Steinbrücke, der
   Holzbrücke und der Furt; `just terrain-bench` misst eine Flussansicht.

### 3.4 Nicht im Umfang

Wasser als Spielmechanik jenseits der Geländearten (Strömung schiebt Einheiten,
Boote, Schwimmen), Pioniere und Behelfsbrücken (023), Zerstörungs-Effekte (019/020),
Brücken im Karteneditor (028), Seen mit eigener Optik (funktionieren aber über
denselben Pfad).

## 4. Akzeptanzkriterien

1. `just check`, `just selftest`, `just gallery-check` und `just lan-selftest` grün;
   `-- --demo` läuft weiter.
2. Golden-Hash bewusst aktualisiert (neue Testkarte, `mass_kg`), Grund im Commit.
3. Rust-Tests:
   - Deckhöhe und -geländeart auf, neben und unter einer Brücke; Fehler bei einer
     Brücke, deren Achse keine durchgehenden Deck-Zellen hat.
   - Testkarte mit Fluss, Holzbrücke, Steinbrücke und Furt: Infanterie und leichte
     Fahrzeuge nehmen die nächste Querung, ein Panzer über der Holzbrücken-Last
     meidet sie; keine Einheit steht je auf tiefem Wasser ohne Deck.
   - Brücke zerstört: Einheiten planen um und nehmen die Furt; wieder intakt: die
     Brücke ist wieder nutzbar. Hash zweier Läufe gleich.
   - Generator: `test_small` hat je eine Stein- und Holzbrücke und eine Furt, und jede
     Querung verbindet beide Ufer im Pfadraster.
4. Selbsttest: ein Panzer fährt per Rechtsklick über die Steinbrücke ans andere Ufer
   und steht dabei in Deckhöhe; das Wassermesh deckt die Wasserproben ab; die
   Brückenmodelle stehen in Deckhöhe.
5. Bildvergleich im Scratchpad (vorher/nachher: Fluss, Brücken, Furt, alle drei
   Biome) und Bildrate der Flussansicht gemessen (Ziel: ≥ 60 FPS bei 1080p, Kosten
   des Wassers angegeben).
6. Doku aktualisiert: `docs/MAPS.md` (Brücken, Furten, Generator, `water_flow_deg`),
   `docs/ARCHITECTURE.md` (Brücken in Wegfindung und Bewegung, Wasser-Darstellung,
   Messwerte), `docs/ASSETS.md` (Schilf).

## 5. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Brücke als was? | Kartenobjekt mit Deck-Regel in `objects.toml` | Brücken werden wie Häuser platziert, gehasht, gezeichnet und später zerstört; kein zweites Dateiformat. |
| Wo wirkt das Deck? | In `Map::height_at`/`terrain_at` (Oberfläche) | Bewegung, Tempo, Steigung und Darstellung nutzen schon diese Funktionen; `sim.rs` bleibt fast unverändert (parallele Prompts). |
| Deckhöhe | Linear zwischen den Geländehöhen an den Achsenenden | Passt immer an die Ufer, keine zusätzlichen Kartendaten; der Generator legt Rampen an. |
| Deck-Zellen | Zellmitte auf dem Deck, Achse muss durchgehend abgedeckt sein | Einfach und exakt; schräge Brücken würden Diagonalschritte ohne Eckenregel brauchen. Der Generator richtet Brücken am Raster aus. |
| Tragfähigkeit | Masse aus dem Katalog gegen `max_load_kg`, Holz 16 t, Stein 100 t (Spielregeln) | Echte Fahrzeugmassen sind im Katalog; Holzbrücke trägt Lkw, Halbketten, leichte Panzer und Spähwagen, aber keine Panzer IV, T-34 oder Sherman. |
| Komponenten | Pro (Klasse, Laststufe), nur so viele wie Profile sie brauchen | Ein Panzer, der nur über eine Holzbrücke käme, bekommt das nächste erreichbare Ziel statt einer vergeblichen Suche. |
| Vom Deck fallen | Geländer an den Deckseiten (über mehr als 0,5 m Höhenunterschied), Gefälle über der Steigungsgrenze auch bergab gesperrt | Beim Ausführen gefunden: die Separation schob einen Panzer vom Damm 3 m tief ins Flussbett, wo er feststeckte. |
| Zerstörung | Copy-on-write der Karte | `Map` bleibt für alle Leser unveränderlich; die Kopie (20 MB) entsteht nur beim seltenen Ereignis. |
| Wasserspiegel | In der Bridge aus Uferhöhen, nicht in der Karte | Keine neuen Kartendaten; die Simulation braucht ihn nicht. |
| Fließrichtung | Tangente des Uferabstands, Vorzeichen aus `water_flow_deg` | Eine Zahl pro Karte statt einer gemalten Flow-Map; reicht für Flüsse ohne Schleifen. |
| Spiegelung | Fresnel mit Himmels-Radiance und Sonnenglanz, keine SSR | SSR wirkt auf das ganze Bild und kostet mehrere ms; in der hohen Draufsicht spiegelt Wasser fast nur den Himmel. |
| Wassermesh | Ein Mesh aus 2-m-Zellen nur über Wasser, in Rust gebaut | Flüsse sind schmal: ein volles Raster wäre fast leer, Rust baut es in wenigen ms. |

## 6. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, Commits pro Schritt, `just check` grün. Parallel
laufende Prompts (017 Kampf, 027 Menüs) ändern `sim.rs`, `crates/bridge/src/lib.rs`,
`main.gd`, `self_test.gd`: dort nur kleine, additive Änderungen. Am Ende nach
`prompts/done/` verschieben und `prompts/README.md` aktualisieren.
