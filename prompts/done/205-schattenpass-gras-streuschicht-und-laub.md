# Prompt 205 – Schatten-Passes für Wald, Gras und Streuschicht beschleunigen sowie Straßenränder glätten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/PERFORMANCE.md` (Abschnitte „Simulation“ zu Prompt 086 Wald-Bench, „Coastal heights“,
> „Dune coast“) und `docs/ARCHITECTURE.md` („Tall grass“, „Meadow“, Regel aus Prompt 071:
> keine kameraabhängigen Texturkoordinaten). Im Code:
> `godot/assets/shaders/env_foliage.gdshader`,
> `godot/assets/shaders/env_common.gdshaderinc`,
> `godot/assets/shaders/tall_grass.gdshader`,
> `godot/assets/shaders/meadow_tuft.gdshader`,
> `godot/assets/shaders/ground_scatter.gdshader`,
> `godot/scenes/world/graphics_quality.gd`,
> `godot/scenes/world/map_objects.gd`,
> `godot/scenes/world/ground_scatter.gd`,
> `godot/assets/materials/terrain.gdshader`.
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hash bleibt unverändert.

## 1. Problem

Laut `docs/PERFORMANCE.md` bricht die Bildrate auf Qualitätsstufe **High** in dichten
Wäldern und Hochgras-/Heidegebieten massiv ein, sobald Schatten aktiv sind:
- Wald (`test_large`, `objects-bench` 15 m): **88 FPS ohne Schatten → 23 FPS mit Schatten**.
- Heide/Hochgras (`dune_coast`, `objects-bench` 15 m): **78 FPS ohne Schatten → 30 FPS mit Schatten**.

Konkrete Ursachen im Code:
1. **Fragment-Farb-Lookups in allen 4 Schatten-Kaskaden:**
   - `tall_grass.gdshader:38` ruft `tall_patch(p)` (2 `macro_tex`-Fetches) im Vertex-Shader
     auch dann auf, wenn `shadow_pass` (`PROJECTION_MATRIX[3][3] > 0.5`, Zeile 48) wahr ist.
   - `meadow_tuft.gdshader:39-45` führt **5 `macro_tex`-Fetches + `m_vnoise` + `meadow_base`**
     pro Vertex in allen 4 Schatten-Kaskaden aus – rein für `v_base`, das Schatten nie lesen!
   - `ground_scatter.gdshader:86-128` führt **5 `macro_tex`-Fetches + 5-Tap-`crushed_at(p)`**
     pro Vertex schattenwerfender Sträucher (109 Tris) und Steine in allen 4 Kaskaden aus.
2. **Teures `inverse(mat3(MODEL_MATRIX))` pro Vertex:**
   `meadow_tuft.gdshader:56` und `ground_scatter.gdshader:144` berechnen pro Vertex eine volle
   3×3-Matrix-Invertierung, obwohl `tall_grass.gdshader:89` bereits zeigt, dass
   `transpose(mat3(MODEL_MATRIX)) * off / (scale * scale)` ausreicht.
3. **Teurer Laub-Schatten-Pass auf 8192²-Atlas:**
   - `env_foliage.gdshader:32-37` rechnet in allen 4 Kaskaden des **8192²-Shadow-Atlas**
     (`graphics_quality.gd:76`, 4× Fillrate gegenüber 4096²!) vollen 3-Lagen-Wind inkl.
     hochfrequentem Blattzittern (`flutter`) sowie 3D-`vnoise` + `discard` pro Schatten-Texel.
   - Bei kahlen Bäumen (`env_bare > 0.5`, Zeile 35) wird erst `wind_offset()` berechnet,
     bevor der Vertex auf `vec3(0.0)` kollabiert wird.
4. **Optische Artefakte an Wüstenstraßen & Streuschicht (sichtbar in `build/block-shots/desert-day-quarter-35.png`):**
   - Streuschicht-Objekte (kleine pyramidenförmige Steine/Büschel) erscheinen mitten auf
     Straßen (`ROAD`).
   - Am Übergang Straße/Sand entstehen schräg zum 2-m-Raster sichtbare Sägezahn-Stufen.

## 2. Ziel

1. **Schatten-Passes der Vegetations-Shader entschlacken:**
   - In `meadow_tuft.gdshader` und `ground_scatter.gdshader` `bool shadow_pass = PROJECTION_MATRIX[3][3] > 0.5;`
     einführen und im Schatten-Pass sämtliche Farb-/Paletten-/`macro_tex`-Abfragen überspringen
     (nur Geometrie-/Wind-/Plattdrück-Logik ausführen, die die Silhouette betrifft).
   - In `tall_grass.gdshader` `tall_patch()` und hochfrequentes `flutter` im Schatten-Pass überspringen.
   - `inverse(mat3(MODEL_MATRIX))` in `meadow_tuft.gdshader` und `ground_scatter.gdshader`
     durch die schnelle `transpose`/`scale²`-Formel ersetzen.
   - In `env_foliage.gdshader`:
     - `if (env_bare > 0.5) { VERTEX = vec3(0.0); return; }` **vor** `wind_offset()` ziehen.
     - Im Schatten-Pass `flutter` weglassen (nur grobes Stamm-/Astbiegen behalten) und für
       die Alpha-Maske im Schatten-Pass 3D-`vnoise` durch eine deutlich billigere 2D-Hash-/Formmaske
       ersetzen, ohne dass Baumschatten merklich „blockiger“ werden.
2. **Schatten-Budget auf High sinnvoll ausbalancieren (`graphics_quality.gd`, `map_objects.gd`):**
   - A/B-Messung durchführen:
     a) Schattenwurf von `meadow_tufts` / `ground_scatter`-Sträuchern auf den nahen Bereich
        begrenzen (bzw. Fern-Varianten ohne Schatten).
     b) Vergleich **4096² vs. 8192²** Directional-Shadow-Atlas auf `High` (optischer Vergleich
        nah & Spielhöhe gegen FPS-Gewinn im Wald und auf `dune_coast`).
   - Nur übernehmen, was in Vorher/Nachher-Bildern optisch überzeugt und nachweislich FPS bringt.
3. **Streuschicht von Straßen fernhalten & Straßenkante prüfen:**
   - In `ground_scatter.gd` / `map_objects.gd` sicherstellen, dass auf `ROAD` (und ggf. sehr
     nah an Gebäudewänden) keine Streuschicht-Büschel/Steine platziert werden.
   - Prüfen, warum in `desert-day-quarter-35.png` deutliche 2-m-Treppenstufen am Straßenrand
     entstehen (`terrain.gdshader` `warp` / `splat`-Filterung / `blend_contrast`), und den
     Übergang ohne Verletzung von Prompt 071 (Weltkoordinaten) sauberer blenden.

## 3. Prüfen und zeigen

- **A/B-Benchmarks (auf ruhiger Maschine direkt hintereinander):**
  - `just objects-bench test_large --quality=high` (Ziel: deutlicher Sprung weg von 23 FPS).
  - `just objects-bench dune_coast --quality=high` (Ziel: deutlicher Sprung weg von 30 FPS).
  - `just bench-view --perf-quick --quality=high`.
  - Ergebnisse tabellarisch in `docs/PERFORMANCE.md` ergänzen.
- **Vorher/Nachher-Bilder:**
  - Wald 15 m / 120 m, `dune_coast` Hochgras, Wüstenstraße (`desert-day-quarter-35`),
    Schattenschärfe an Panzern und Bäumen.
- `just ground-motion` ohne Verschlechterung, `just check`, `just selftest` grün.

## 4. Nicht Teil dieses Prompts

- Änderungen an der Simulation oder an 3D-Modellen.
- Draw-Call-Reduktion durch `UV2`-Slot-Kodierung (Prompt 207).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/205-schattenpass-gras-streuschicht-und-laub.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis

- **Schatten-Pass-Bereinigung der Vegetations-Shader:**
  - `godot/assets/shaders/env_foliage.gdshader`: `env_bare > 0.5`-Kollaps vor `wind_offset()` gezogen; im Schatten-Pass entfällt `flutter` (`leaf = 0.0`), und `fragment()` nutzt einen frühen Schatten-Zweig ohne `CLOUD_FRAGMENT`, 3D-`vnoise`, Albedo/Schnee/Volumetrik, mit sin-/sqrt-freier `hash2_fast`/`leaves_shadow`-Maske.
  - `godot/assets/shaders/tall_grass.gdshader`: `tall_patch()` und `flutter` im Schatten-Pass übersprungen.
  - `godot/assets/shaders/meadow_tuft.gdshader`: alle 5 `macro_tex`-Fetches, `m_vnoise`, `meadow_base` und `flutter` im Schatten-Pass übersprungen; `inverse(mat3(MODEL_MATRIX))` durch `transpose / scale²` ersetzt.
  - `godot/assets/shaders/ground_scatter.gdshader`: Farb-/Paletten-/`macro_tex`-Berechnungen im Schatten-Pass übersprungen, `crushed_at()` auf 1-Tap (`crushed_at_shadow`) reduziert, `inverse(mat3(MODEL_MATRIX))` durch `transpose / scale²` ersetzt.
- **Shadow-Budget A/B (`godot/scenes/world/graphics_quality.gd`, vorläufig):**
  - `high` nutzt jetzt `shadow_size = 4096` (4 PSSM-Kaskaden, `SOFT_MEDIUM`). A/B-Vergleich (M1 Max, 1280 × 720, Details in `docs/PERFORMANCE.md`):
    - `test_large` 15 m: **22,0 → 43,3 (Shader @ 8192²) → 78,7 FPS (4096²)**; 120 m: **35,4 → 73,4 FPS**; Dorf 150 m: **47,5 → 63,7 FPS**.
    - `dune_coast` 15 m: **28,4 → 49,9 (8192²) → 78,7 FPS (4096²)**; 120 m: **45,7 → 78,2 FPS**.
    - `bench-view --perf-quick` (40/150/600 m): **63,7/63,0/66,2 → 67,3/63,3/69,4 FPS**.
    - Optischer Vergleich (`standing_40.png` / `standing_150.png`) zeigt bei 4096² praktisch identische Schattenschärfe an Panzern und Baumkronen. Dekorations-Schatten (`near` vs. `near-nodeco`) kosten nach der Shader-Bereinigung nur noch ~0,5 ms und bleiben daher aktiv.
- **Straßenränder & Streuschicht (`map_objects.gd`, `crates/bridge/src/objects.rs`, `terrain.gdshader`):**
  - Dichte auf `dirt_track` (id 1) für `grass`, `stones`, `scatter_stalks`, `scatter_stones` auf `0.0` gesetzt (wie `road` id 2) und in `objects::scatter()` Ausfransen (`fray`) auf Straße/Feldweg unterbunden (Schultern behalten über `edge` weiterhin Streuschicht).
  - In `terrain.gdshader` Warp entlang Straße/Feldweg gedämpft (`mix(1.0, 0.25, road_track_raw)`) und `blend_contrast` / `blend_window` für alle Schichten an Straßen-/Wegerändern geglättet, sodass Sandrippel keine Sägezähne mehr in den Straßenrand schneiden (geprüft mit `desert-day-quarter-35` und `desert-day-street-60`).
- Verifiziert mit `just ground-motion`, `just selftest` und `just check` (Simulation und Golden Hashes unverändert).
