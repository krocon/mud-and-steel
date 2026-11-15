# Prompt 019 – Effekte

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Abschnitte „Combat“ und „Lighting, atmosphere and
> biomes“) und `docs/ASSETS.md` (Sockets).

## 1. Ziel

Das Gefecht sieht aus wie in den Stil-Referenzen: weiße Pulverdampfwolken und
Mündungsfeuer beim Abschuss, Leuchtspur, Erdfontänen und Funken bei Einschlägen,
Feuerbälle mit Lichtblitz und Trümmern bei Zerstörungen, danach brennende Wracks
mit dichten Rauchsäulen, Staubfahnen hinter fahrenden Fahrzeugen je nach
Geländeart, Auspuffrauch und Einschlagspuren am Boden.

Alles ist **reine Darstellung** in Godot. Effekte werden von den Kampf-Ereignissen
der Simulation (Prompt 017) und vom Snapshot (Positionen, `unit_moving`,
Geländeart) ausgelöst und wirken nie zurück: kein Effekt liest oder schreibt
etwas, das die Simulation beeinflusst, und Godot-Zufall ist hier erlaubt, weil
nichts davon in den State-Hash eingeht.

## 2. Ausgangslage

- `SimHost.take_combat_events()` liefert pro Frame Ereignisse als Dictionaries:
  `shot` (Einheit, Waffe, Mündungsposition, Kaliber), `hit` (Position, Oberfläche
  `unit`/`ground`/`object`, Ziel, Fläche, `penetrated`, Kaliber) und `destroyed`
  (Einheit, Position). `scenes/world/combat_view.gd` liest sie und sendet sie als
  Signal `combat_event` weiter (für Effekte und Ton vorgesehen). Es zeichnet
  schon Leuchtspuren (gestreckte Quader je Geschoss), Lebensbalken und dunkle
  Wracks.
- Modelle haben Sockets: `Socket_Muzzle` (Hauptwaffe), `Socket_Muzzle_02…` (MGs),
  `Socket_Exhaust_NN`, `Socket_Fire_NN`.
- `SimHost` liefert schon alles Nötige: `unit_moving()`, `unit_aim_yaws()`,
  `unit_type_ids()`, `unit_footprints()`, `terrain_at(x, z)`, `height_at(x, z)`.
  **Keine Rust-Änderung nötig.**
- Grafikstufen `low`/`medium`/`high` in `scenes/world/graphics_quality.gd`,
  angewandt von `atmosphere.gd`; Biome `forest`/`desert`/`winter` als
  `BiomePreset` (mit Windrichtung).
- Parallel: Prompt 018 (Nebel des Krieges) und 028 (Karteneditor). Gemeinsame
  Dateien (`main.gd`, `self_test.gd`, `justfile`, `graphics_quality.gd`, Doku)
  nur klein und additiv ändern.

## 3. Umfang

1. **Effekt-System** `godot/scenes/world/effects.gd` (von `main.gd` erzeugt,
   hört auf `combat_view.combat_event`), Bausteine in
   `godot/scenes/world/effect_library.gd` (Partikel-Systeme, Materialien) und
   prozedurale Texturen in `godot/scenes/world/effect_textures.gd` (Rauch-Puff aus
   Rauschen mit radialem Abfall, Funke, Feuer, Brandfleck; zur Laufzeit erzeugt,
   keine Downloads).
2. **Abschuss:** Mündungsblitz (kurzes leuchtendes Billboard plus Lichtblitz bei
   Kanonen) am nächstgelegenen `Socket_Muzzle*` des Schützen, weiße
   Pulverdampfwolke in Schussrichtung, bei Kanonen zusätzlich Staub vom Boden
   vor der Mündung. Größe nach Kaliber (Gewehr, MG, Kanone).
3. **Einschlag:**
   - Boden: Erdfontäne (Erdklumpen und Staub nach oben), Farbe nach Geländeart
     und Biom (Erde, Sand, Schnee, Schlamm), Wasser als Wasserfontäne;
     Brandfleck-Decal ab Kanonenkaliber.
   - Einheit: Funken; bei Durchschlag zusätzlich kurzer Feuerstoß und Rauch,
     bei Abprallern nur Funken.
   - Objekt: Splitter und Staub.
   - Kleine Kaliber (< 20 mm) nur kleine Staubwölkchen.
4. **Zerstörung:** Feuerball, Lichtblitz, Funken, Trümmer, Stoßstaub am Boden,
   großer Brandfleck, Kamera-Wackeln abhängig von der Entfernung. Fahrzeugwracks
   **brennen** danach an `Socket_Fire_NN` (Feuer, dichte schwarze Rauchsäule mit
   Wind des Bioms, flackerndes Licht), gehen nach einer Weile in Schwelrauch über
   und hören dann auf. Infanterie: nur Staub, kein Feuer.
5. **Fahrzeuge:** Staubfahne hinter fahrenden Fahrzeugen nach Geländeart unter
   der Einheit (Feldweg und Sand stark, Wiese schwach, Straße kaum, Schlamm
   dunkle Spritzer, Schnee Pulverschnee, Wasser Gischt; das Biom färbt um wie das
   Gelände), Auspuffrauch an `Socket_Exhaust_NN` (stärker beim Fahren).
6. **Leuchtspur:** die vorhandenen Leuchtspuren aus 017 bleiben; Mündungsblitz
   und Pulverdampf sitzen am Socket des Modells.
7. **Budget und LOD:**
   - Pools je Effektart (fester Satz `GPUParticles3D`, ältester wird
     wiederverwendet), Höchstzahl von Lichtern, Decals und brennenden Wracks.
   - Werte pro Grafikstufe additiv in `graphics_quality.gd`
     (Partikelfaktor, Pools, Lichter, Decals, weiche Partikel). Ein Wechsel der
     Stufe zur Laufzeit baut die Pools neu.
   - LOD: Effekte, die auf dem Bildschirm kleiner als ein paar Pixel wären
     (Entfernung zur Kamera, Zoom), werden ausgelassen oder ausgedünnt
     (`amount_ratio`); Staubfahnen und Auspuffrauch nur in Kameranähe.
8. **Sichtbarkeit (für Prompt 018):** eine Abfrage `effects.visible_at:
   Callable` (Standard: immer sichtbar). Neue Effekte werden nur an sichtbaren
   Punkten erzeugt; dauerhafte Effekte (brennende Wracks, Staub) prüfen sie
   regelmäßig und werden ausgeblendet.
9. **Effekt-Testszene:** `scenes/tools/effects_bench.gd`, gestartet über
   `main.tscn` mit `-- --effects-shots` (feste Ansichten aller Effekte als
   PNG-Frames, auch in Wüste und Winter) und `-- --effects-bench` (Bildrate bei
   50 gleichzeitigen Explosionen in jeder Grafikstufe, 1920 × 1080, V-Sync aus).
   `just effects-shots <dir>` und `just effects-bench`.
10. **Selbsttest:** der Kampftest prüft zusätzlich, dass Effekte erzeugt wurden
    (Mündungsfeuer, Einschläge, Zerstörung, brennendes Wrack) und dass die
    Simulation davon unberührt bleibt.

## 4. Akzeptanzkriterien

1. `just check` grün, Golden-Hash in `crates/sim/tests/determinism.rs`
   unverändert (keine Änderung an `crates/sim`).
2. `just selftest` (mit Effekt-Prüfung), der Flow-Selbsttest, `just
   lan-selftest` und `just gallery-check` grün; `-- --demo` läuft ohne Fehler und
   zeigt die Effekte.
3. Effekt-Testszene zeigt alle Effekte aus Abschnitt 3; Bildvergleich (Kontakt-
   bogen der Ansichten, Wald/Wüste/Winter) im Scratchpad.
4. Bildrate bei 50 gleichzeitigen Explosionen je Grafikstufe gemessen und in
   `docs/ARCHITECTURE.md` eingetragen; Ziel: `high` ≥ 60 FPS auf dem M1 Max.
5. `docs/ARCHITECTURE.md` hat einen Abschnitt „Effects (presentation only)“
   (Ereignis → Effekt, Budget je Stufe, LOD, Sichtbarkeitsabfrage, Messwerte,
   Entscheidungen).

## 5. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Wo leben Effekte? | Nur GDScript in `scenes/world/`, keine Rust-Änderung | Alle Daten sind schon im Snapshot und Ereignisstrom; Effekte sind Darstellung. Golden-Hash und Lockstep bleiben unberührt. |
| Partikel | `GPUParticles3D` mit `ParticleProcessMaterial`, Pools mit `restart()` | Bewährt (Schneefall aus 015), GPU-seitig, keine eigenen Shader nötig. Pools vermeiden Knoten-Erzeugung pro Schuss. |
| Texturen | Zur Laufzeit erzeugt (Rauschen × radialer Abfall) | Keine Downloads, keine Lizenzfragen, ein paar KB. |
| Mündungsposition | Nächster `Socket_Muzzle*` zur Mündung der Simulation | Robust ohne Abbildung Waffenindex → Socket; fällt auf die Sim-Position zurück. |
| Kamera-Wackeln | `Camera3D.h_offset`/`v_offset`, abklingend | Keine Änderung am Kamera-Rig, keine Wirkung auf Picking-Logik der Sim (Befehle nutzen den Bodenpunkt). |
| Einschlag-Decals | `Decal`-Knoten, Ring-Puffer pro Stufe, flache Projektionsbox | Optisch bis 021 (Krater); flache Box hält die Decals vom Fahrzeug fern. |
| Nebel des Krieges | `visible_at: Callable` | 018 kann es setzen, ohne dass sich hier etwas ändert. |

## 6. Arbeitsweise

Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe, keine Software ohne
mein Go installieren, kein Test wird abgeschwächt. Commits pro Schritt. Am Ende
diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
`prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was
gebaut wurde, wie ich es teste, Messwerte, was offen ist.
