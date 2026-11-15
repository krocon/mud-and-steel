# Prompt 021 – Verformbares Gelände

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Pathfinding, Combat, Vision, Destruction, Terrain
> rendering, Map objects, Effects), `docs/DETERMINISM.md`, `docs/MAPS.md` und
> `docs/INFANTRY.md`.

## 1. Ziel

Einschläge schwerer Geschosse im Boden hinterlassen Krater. Krater sind Spielregel:
Sie ändern in der Simulation Höhe und Geländeart des Höhenfelds, bremsen Fahrzeuge
(Schlamm), verändern Sichtlinien geringfügig und dienen der Infanterie als Deckung.
Wegfindung und Sicht werden nur im betroffenen Bereich aktualisiert. Godot zeigt das
Ergebnis: Das Terrain liest nur den geänderten Bereich neu, Deko-Vegetation im Krater
verschwindet, Einschlag-Effekte aus 019 bleiben.

## 2. Ausgangslage

- `sim::Map` (2-m-Raster, `i32`-Millimeter, `u8`-Geländeart) wird per `Arc` geteilt und
  ist bisher bis auf Brücken (`Sim::set_bridge_intact`, copy-on-write über
  `Arc::make_mut`) unveränderlich. Der Inhalts-Hash der Karte bleibt der geladene.
- Bodentreffer entstehen in `Sim::fly_projectiles` (`HitKind::Ground`, `terrain_hit`).
  Indirektes Feuer gibt es noch nicht; Bodentreffer kommen von Fehlschüssen und
  Bodenangriffen direkt schießender Waffen.
- Wegfindung: `PathGrid::rebuild_cells` + `relabel_components` + `Sim::replan_through`
  (wie in `apply_destruction`). Die Komponenten-Flutfüllung kostet auf `test_large`
  etwa 8 ms.
- Sicht: `VisionGrid` speichert die Bodenhöhe pro 8-m-Zelle (Zellmitte);
  `Sim::requeue_viewsheds` stellt betroffene Sichtfelder neu ein.
- Infanterie-Deckung (022) hängt an Objekten (`Soldier::cover`, `choose_spots`,
  `soft_cover_absorbs`).
- Terrain (011): `terrain.heights_changed(rect)` liest ein Rechteck über
  `SimHost.terrain_layers` neu (lädt allerdings jeweils die ganzen Texturen hoch).
  `SimHost.debug_dent` ist eine reine Darstellungs-Delle (Selbsttest, Terrain-Bench).
- Deko (013): `SimHost.scatter_deco` streut pro 64-m-Quadrat aus Geländeart und
  Rastern; `map_objects.gd` baut Quadrate um die Kamera.
- Die Bridge hält eine eigene `Arc<Map>` in `Game::map` neben der Sim.

## 3. Umfang

1. **Regeln** in `data/craters.toml` (eingebaut wie `data/destruction.toml`, nicht in
   `SimConfig`, im State-Hash): Mindestkaliber, Radius/Tiefe/Randwall pro mm Kaliber,
   Mindestradius (ein Raster-Abstand), maximale Absenkung und Anhebung pro Sample
   gegenüber der geladenen Karte, Geländeart im Krater (Schlamm), maximale Kraterzahl
   pro Tick, Deckungsregeln (Mindesttiefe, Abfangchance pro Haltung).
2. **Sim (`crates/sim/src/sim/craters.rs`):** Bodentreffer ab Mindestkaliber merken
   (höchstens N pro Tick, Rest ohne Krater); nach `apply_destruction` anwenden:
   Höhenprofil (Schüssel + Randwall) rein ganzzahlig auf die Samples im Radius, mit
   Begrenzung über ein Delta-Raster (`i16` pro Sample, erst beim ersten Krater
   angelegt); Geländeart in der Schüssel wird Schlamm (außer Wasser und unter
   Brückendecks); keine Krater im Wasser oder auf Brückendecks. Karte per
   copy-on-write (einmalige Kopie). Danach: Pfadzellen neu (`rebuild_cells`),
   Komponenten nur neu, wenn sich Passierbarkeit geändert hat, und nur dann
   Neuplanung betroffener Routen; Sichtzellen-Bodenhöhe neu, Sichtfelder nur bei
   geänderter Höhe neu einreihen; Höhe von Einheiten und Soldaten im Bereich
   nachführen. Events pro Tick (abgeleitet, nicht gehasht). `Sim::queue_crater` für
   Spielsysteme und Debug (wie `set_path_obstacle`, nie aus Spielerbefehlen).
3. **Infanterie-Deckung:** stehende Trupps nutzen Krater-Samples (Tiefe ab
   Mindesttiefe) in der Nähe als Deckungsplätze, wie Objekt-Deckung; Treffer auf einen
   Soldaten im Krater werden mit einer Chance nach Haltung abgefangen (Rundum-Deckung,
   unabhängig von der Schussrichtung). `in_cover` im Snapshot gilt auch dort.
4. **Bridge:** liest die Karte immer aus der Sim (`Game::map()` statt eigener `Arc`),
   damit Krater ohne zweite Kopie sichtbar sind; `take_crater_events()` mit
   Sample-Rechteck, Punkt, Radius, Tiefe; `scatter_deco` lässt verformte Samples aus;
   `debug_crater(x, z, caliber)` reiht einen Sim-Krater ein (nur Einzelspieler).
5. **Godot:** `world/crater_view.gd` sammelt die Rechtecke eines Frames und baut das
   Terrain gedrosselt neu (ein Upload für viele Krater), wirft betroffene Deko-Quadrate
   weg (sie bauen sich ohne Deko im Krater neu auf); das Debug-Menü setzt einen echten
   Sim-Krater. Krater-Decal und Erdbrocken kommen aus den Bodentreffer-Effekten von 019.
6. **Messung:** Rust-Benchmark (CLI) der Krater-Kosten pro Tick auf `test_large`;
   Godot-Bench mit 100 Einschlägen in kurzer Zeit (Bildrate, Upload-Zeit).
7. **Doku:** `docs/ARCHITECTURE.md` (neuer Abschnitt „Terrain deformation“, Verweise in
   Vision/Destruction/Terrain/Effects „Open“), `docs/MAPS.md`, `docs/INFANTRY.md`.

## 4. Akzeptanzkriterien

1. `just check` grün, Golden-Hash bewusst aktualisiert, Grund im Commit.
2. Test: Ein Krater ändert die Höhe an erwarteten Samples exakt (Mitte, Rand, Wall,
   unverändert außerhalb), Geländeart wird Schlamm, Wasser bleibt unverändert,
   Begrenzung der Tiefe greift bei wiederholten Treffern.
3. Test: Zwei Sims (zwei Peers per Loopback) mit Beschuss ergeben identische Hashes
   und Höhen; ein Krater, der Passierbarkeit ändert, plant Routen neu.
4. Test: Soldaten im Krater gelten als in Deckung.
5. Bildrate bei 100 Einschlägen in kurzer Zeit gemessen und dokumentiert.
6. Alle Selbsttests grün: `just selftest` (mit Flow-Test), `just lan-selftest`,
   `just editor-selftest`, `just gallery-check`, Demo-Lauf (`--demo`).

## 5. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Wo die Höhen leben | In der `Map` der Sim (copy-on-write, eine Kopie pro Match) plus `i16`-Delta-Raster | Alle Systeme (Bewegung, Ballistik, Sicht, Darstellung) lesen dieselben Höhen ohne Sonderfälle; Speicher bleibt konstant (Karte + Delta). |
| Hash | Laufende Prüfsumme über jeden Krater und jedes geänderte Sample, plus Anzahl | Ein Hash über 4 M Samples pro Tick wäre zu teuer; die Prüfsumme erfasst jede Änderung in Reihenfolge. |
| Neue Geländeart? | Nein, Schlamm (Id 4) | Der Splatting-Shader hat genau 8 Gewichte; Schlamm bremst schon (40 %). Deko verschwindet über das Delta-Raster. |
| Radius auf 2-m-Raster | Mindestens ein Raster-Abstand | Kleinere Krater träfen oft kein Sample; die Darstellung braucht mindestens eine Zelle. |
| Neuplanung | Nur bei geänderter Passierbarkeit | Kosten ändern sich bei jedem Krater; ständige Neuplanung aller Routen wäre teuer und unruhig. |
| Deckung | Rundum-Abfangchance nach Haltung, ab Mindesttiefe | Bei 2-m-Samples reicht die Geometrie allein nicht, damit eine Mulde Rumpf und Kopf verdeckt. |
| Terrain-Upload | Pro Frame gesammelt, gedrosselt | Godot 4 lädt Texturen nur ganz hoch; 100 Krater dürfen nicht 100 Uploads kosten. |

## 6. Arbeitsweise

Mit dem Nutzer Deutsch, im Repo Englisch. Kein `git push`. Keine Tests abschwächen.
Commits pro Schritt. Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`,
`self_test.gd`, `justfile`, Doku) nur klein und additiv ändern (parallel laufen 016,
023, 026). Am Ende den aktuellen lokalen `main` übernehmen, diese Datei mit `git mv`
nach `prompts/done/` verschieben, Status in `prompts/README.md` auf „erledigt“ setzen,
committen.
