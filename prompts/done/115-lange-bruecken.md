# Prompt 115 – Lange Brücken über große Flüsse

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/014-wasser-fluesse-bruecken.md`, `prompts/done/020-zerstoerung.md`
> und `prompts/done/055-bruecken-pioniertechnik-luftlande.md` (jeweils Ziel,
> Entscheidungen, Ergebnis), in `docs/MAPS.md` „Bridges“ ganz, in
> `docs/ARCHITECTURE.md` „Bridges, bridge-laying tanks and camouflage“ und die
> Abschnitte zu Wegfindung, Sicht und Zerstörung. Dann `crates/sim/src/bridges.rs`,
> `crates/sim/src/objects.rs`, die Brückenteile in `crates/sim/src/map.rs`,
> `crates/sim/src/path.rs` und `crates/sim/src/sim/crossings.rs`, `data/objects.toml`
> (`env_bridge_*`), `docs/ASSETS.md` und die Brückenmodelle unter
> `godot/assets/` (`env_bridge_*`). In `tools/geo_import.py` die Behandlung von
> `bridge` und `layer`.
>
> Dieser Prompt ist die Vorarbeit für Prompt 116 (Nijmegen) und 118 (Arnhem).

## 1. Ausgangslage

Kartenbrücken (Prompt 014) sind Objekte mit einer Fahrbahn: Holz bis 16 m, Steinbogen
bis 30 m. Sie liegen genau achsparallel (Richtung ein Vielfaches von 90°, Achse auf
einer Mittellinie des 4-m-Weggitters), ihre Fahrbahnhöhe läuft linear zwischen den
Ufern, unter einer Brücke ist nichts befahrbar (eine Wegzelle auf der Fahrbahn ist
Fahrbahn). Sprengen ist vorbereitet (`Sim::set_bridge_intact`,
`Sim::collapse_bridge` aus der Pioniersprengung).

Die Rheinbrücke von Arnhem (1935) und die Waalbrücke von Nijmegen (1936) waren
Stahlbogenbrücken: ein Hauptbogen von rund 140 m (Arnhem) bzw. 244 m (Nijmegen),
dazu Vorlandbrücken und lange Anfahrtsrampen, die am Ufer über Straßen hinweggehen.
Beides kann das Spiel heute nicht abbilden.

## 2. Ziel

- **Neue Brückenart** `env_bridge_steel_arch` (zweispurige Straßenbrücke mit
  Stahlbogen, Fahrbahn etwa 7 m plus Gehwege, Tragfähigkeit für alle Fahrzeuge des
  Katalogs) mit Varianten für den Hauptbogen (z. B. 140 m und 240 m) und einem
  **Vorlandstück** (Balkenbrücke auf Pfeilern, z. B. 30 m und 60 m), das sich
  aneinanderreihen lässt.
- **Lange Fahrbahnen** über mehrere Objekte: Brückenteile, die Ende an Ende liegen,
  bilden eine durchgehende Fahrbahn (gleiche Achse, Höhen stetig). Die Fahrbahnhöhe
  darf über den Fluss hoch liegen (10–20 m über dem Wasser), die Rampen an Land
  sind Erddämme aus der Karte wie bisher.
- **Unterführungen:** Unter einem Vorlandstück auf Pfeilern bleibt der Boden
  befahrbar, soweit die lichte Höhe reicht (Vorschlag: ab 4,5 m für alle Fahrzeuge).
  Das braucht im Weggitter zwei Ebenen an diesen Zellen oder eine andere Lösung;
  die Session schlägt die einfachste vor, die deterministisch bleibt, und begründet
  sie. Ist es zu aufwendig, bleibt es bei Dämmen ohne Unterführung, und Straßen
  unter der Rampe werden in der Karte umgelegt (als Entscheidung festhalten).
- **Sicht und Feuer:** Die Brücke verdeckt nicht (Fachwerk und Bogen sind
  durchsichtig), die Fahrbahn bietet keine Deckung außer dem Geländer für liegende
  Infanterie (Vorschlag: wie niedrige Mauer). Einheiten auf der Fahrbahn sehen und
  schießen von ihrer Höhe aus.
- **Zerstörung:** Jedes Teil einzeln sprengbar (Pioniere, schwere Treffer nach den
  Regeln aus 020); ein gesprengtes Teil reißt nur sich selbst heraus. Was auf dem
  Teil steht, stürzt wie in 055 ab. Das Modell zeigt den Zustand (eingestürztes
  Teil, Trümmer im Wasser).
- **Diagonal:** Brücken bleiben achsparallel. Eine Karte richtet ihren Rahmen so
  aus, dass die Brücke auf einer Achse liegt (`x_axis_bearing_deg` im Importer). Das
  in `docs/MAPS.md` beschreiben.

## 3. Importer

`tools/geo_import.py` erkennt in `site.json` eine **Brücke über Wasser**: Lage,
Achse und Teile werden in `site.json` angegeben (keine automatische Erkennung aus
OSM nötig), der Importer setzt die Objekte, legt die Rampen an (Dämme mit höchstens
etwa 12 % wie bisher) und prüft, dass die Achse auf dem Weggitter liegt. Die Straße
auf der Brücke wird dabei nicht als Straße gemalt. `harbor_city` muss Bit für Bit
gleich herauskommen.

## 4. Modelle

Zwei Modelle aus dem Baukasten (headless Blender wie die anderen Objekte), nach den
Regeln in `docs/ASSETS.md`: Hauptbogen (eine Länge skalierbar oder zwei Varianten)
und Vorlandstück, jeweils intakt und eingestürzt. Maße und Form nach den echten
Brücken (Quellen in `docs/SOURCES.md`; Fotos nur als Referenz). Vorschaubilder mit
`just thumbnails --only=…`, `just units-web-sync`.

## 5. Prüfen

- Testkarte (z. B. `cli gen-map` mit Fluss oder eine kleine Karte von Hand) mit
  einem 240-m-Bogen und zwei Vorlandstücken: Panzer fährt hinüber, Infanterie auch,
  Sprengen eines Vorlandstücks trennt die Ufer, Einheiten planen neu, Hash gleich in
  zwei Läufen.
- Unterführung (falls umgesetzt): Lkw fährt unter dem Vorlandstück durch, Panzer
  auf der Brücke darüber, beide gleichzeitig.
- KI: findet den Weg über die lange Brücke; mit gesprengter Brücke nimmt sie den
  Umweg oder bleibt stehen, ohne zu hängen.
- Golden Hash: Bleibt er gleich, ist das gut. Ändert er sich durch Umbau an
  Fahrbahn oder Weggitter, ist das beabsichtigt; Grund in den Commit.
- `just check`, `just selftest`, Messung der Wegfindung mit langer Brücke in
  `docs/PERFORMANCE.md`.

## 6. Nicht Teil dieses Prompts

- Eisenbahnbrücken und Züge.
- Klappbrücken, Pontonbrücken als Kartenobjekt (gelegte Brücken gibt es schon).
- Diagonale Brücken.

## 7. Entscheidungen (Nachtlauf 23.11.2026, alle vorläufig)

| # | Frage | Entscheidung | Warum |
|---|---|---|---|
| 1 | Lange Fahrbahn | Kartenbrücken, deren Enden sich (±0,25 m) auf parallelen Achsen treffen, bilden eine Kette (`Bridge::link_chains` beim Laden); Höhe linear zwischen dem Boden an den beiden **äußeren** Enden der Kette; einzelne Brücken wie bisher. **[vorläufig]** | Keine neuen Kartendaten; die Dämme der Karte bestimmen die Höhe wie bisher die Ufer; Hash alter Karten bleibt. |
| 2 | Innere Stöße | Enden, die nicht auf dem Boden aufliegen, „hängen“ (`Bridge::hangs`): Wegraster und Bewegung verlassen ein Deck dort nur auf das Nachbarteil; fehlt es, ist das Ende eine Sackgasse. Pioniere sprengen von einem Ende aus, auf dem sie stehen können. **[vorläufig]** | Sonst plant der Wegfinder nach einer Sprengung über das hängende Ende 18 m in die Tiefe (Test `nobody_drives_off_a_hanging_joint` fällt ohne die Regel). |
| 3 | Unterführung | **Nicht umgesetzt:** unter einem Deck ist nichts befahrbar, auch nicht unter Vorlandstücken über Land; Straßen unter Rampe und Vorland enden dort, der Verkehr fährt um das Dammende (Test). Der Importer zählt die abgeschnittenen Straßenflächen; umlegen per `roads.extra`. **[vorläufig]** | Zwei Ebenen je Wegzelle hätten A*, Komponenten, Sicht (2,5D) und `height_at` aller Systeme (Höhe aus der Position) betroffen; zu viel Risiko für die Vorarbeit zu 116/118. |
| 4 | Maße | `env_bridge_steel_arch` `Arch244` (Waalbrücke: 244,1 m zwischen den Pfeilern, 26,5 m breit, Bogen ~52 m über Wasser, Fahrbahn 18 m) und `Arch120` (Arnhem: größte Spannweite 120 m laut nl.wikipedia statt „~140 m“, 23,8 m, Fahrbahn 15 m); `env_bridge_steel_approach` `Span60`/`Span30` (Balken auf Pfeilern, 14 m breit, Fahrbahn 18,9 m über dem Pfeilerfuß; Einzelspannweiten geschätzt). Fahrbahn 12 m zwischen den Geländern (7 m Straße + Gehwege), 100 t. **[vorläufig]** | Echte Werte, wo die Quellen sie geben; Rest `uncertain`. |
| 5 | Sicht | Neues Deck-Feld `see_through`: das Sichtraster behält unter dem Deck den Boden als Horizont (`VisionGrid::deck_low_mm`), Einheiten auf dem Deck stehen, sehen und werden gesehen in Deckhöhe. **[vorläufig]** | Die Fahrbahn ist im 2,5D-Raster sonst eine 18 m hohe Wand. |
| 6 | Deckung | Neues Deck-Feld `railing_cover_pct` = 50: liegende Soldaten auf dem Deck, Schuss eher von der Seite als längs → 50 % fängt das Geländer. **[vorläufig]** | „Wie niedrige Mauer“ ohne neues Objekt; längs der Fahrbahn keine Deckung. |
| 7 | Schwere Treffer | `[shelling]` in `data/crossings.toml`: ab 10 cm kostet jede Explosion auf dem Deck (Granaten, Bomben, Raketen, Ladungen) und jeder Bodentreffer direkten Feuers Kaliber² × Fahrzeugfaktor wie in 020; Vorland 36 000, Bogen 60 000 Punkte (≈ 16 bzw. 27 Treffer 15 cm). Bei 0 stürzt nur dieses Teil ein (am Tick-Ende, wie die Pioniersprengung). Holz- und Steinbrücken bleiben nur sprengbar. **[vorläufig]** | Stahlbrücken sind Ziel eines schweren Beschusses, nicht weniger Panzergranaten; Zustand erst im Hash, wenn eine Brücke getroffen wurde. |
| 8 | Modell des Einsturzes | Je Variante `<Name>Down` im selben Rahmen (gleiche `deck_height_m`), `role = "collapsed"`; Godot tauscht beim Einsturz (`map_objects.gd` `set_bridge_down`). Karten setzen diese Varianten nicht (nicht in `objects.toml`). **[vorläufig]** | Kein neues Kartenformat; Variante liegt sofort richtig. |
| 9 | Importer | `long_bridges` in `site.json` (Achse x/y, Linie, Start, Deckhöhe, Teile); prüft die Linie auf dem Wegraster, schreibt die Teile zuerst, baut Dämme (12 %, 8 m Widerlager unter dem Deckende, Krone ±10 m mit Straße, Seiten 100 %), malt OSM-Brückenwege im Korridor nicht. Ohne den Schlüssel unverändert. **[vorläufig]** | Brücke wird angegeben, nicht erkannt (Prompt). |
| 10 | Katalog | Länge von Umgebungsobjekten bis 300 m statt 100 m (Bogen 244 m). | Nötig für den Hauptbogen. |

## 8. Ergebnis

- **Sim:** `crates/sim/src/bridges.rs` (Ketten, hängende Enden),
  `map.rs` (`deck_step_ok`), `path.rs` (`bridge_hangs` im
  Geländertest), `vision.rs` (`deck_low_mm`, `refresh_decks`),
  `objects.rs` (Deck-Felder `see_through`, `railing_cover_pct`),
  `crossings.rs` + `sim/crossings.rs` (`[shelling]`, `shell_bridge`,
  `bridge_hp`, `railing_catches`), Aufrufe in `sim/combat.rs`,
  `sim/indirect.rs`, Pionierstandort in `sim/works.rs`, Sichtraster bei
  `set_bridge_intact`.
- **Daten:** `data/objects.toml`, `data/catalog/neutral.toml` (zwei
  Einträge mit Quellen), `data/crossings.toml` `[shelling]`.
- **Modelle:** `tools/blender/kit/buildings.py` (`bridge_steel_arch`,
  `bridge_steel_approach`), Baupläne, `.blend`/`.glb`, je vier Varianten
  (intakt, eingestürzt), 948 bzw. 396 Dreiecke, `validate-env` ok;
  Vorschaubilder, `units-web-sync`; Aufstellung
  `build/renders/env115/`.
- **Godot:** `map_objects.gd` `set_bridge_down`, `works_view.gd`.
- **Importer:** `tools/geo_import.py` `long_bridges`; Probe auf einer
  Kopie von `harbor_city` mit 304-m-Brücke über das Hafenbecken
  (`build/geo115/`: map-check ok, map-reach über den Damm, Bilder
  `reach_crop.png`, `stage2_crop.png`); alle vier Geodatenkarten Bit für
  Bit gleich.
- **Tests:** `crates/sim/tests/long_bridges.rs` (Kette und Höhen, Panzer
  und Infanterie hinüber, Sprengung trennt die Ufer + Neuplanung +
  gleicher Hash, Pioniere sprengen ein Teil, keine Unterführung, Sicht
  unter dem Deck, hängender Stoß über Land, Beschuss bringt ein Teil zu
  Fall + gleicher Hash), `crates/ai/tests/long_bridge.rs` (KI fährt über
  die Brücke; mit gesprengtem Vorland bleibt sie diesseits ohne
  Dauerplanung).
- **Messung** (`docs/PERFORMANCE.md` „Long bridges“): 100 Einzelbefehle
  über die Brücke 13 100 Knoten je Anfrage, alle Wege nach 41 Ticks,
  3,2 ms je Tick; gesprengt 6 100 Knoten, 19 Ticks.
- **Golden Hash unverändert** (alte Brücken und Karten ohne Ketten
  rechnen wie vorher).
- **Offen:** Unterführung (zwei Ebenen); eigener Test der
  Geländerdeckung; Treffer auf Pfeiler/Bogen zählen nicht, nur auf der
  Fahrbahn; Einsturzmodell nicht im Spiel angesehen (nur Blender-
  Aufstellung und Vorschaubild); keine echte Karte mit langer Brücke
  (116/118).
