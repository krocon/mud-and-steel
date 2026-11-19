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
