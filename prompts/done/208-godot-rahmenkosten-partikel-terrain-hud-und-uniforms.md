# Prompt 208 – Godot-Rahmenkosten senken: Partikel-Sharing, Globale Uniforms, Terrain-Uploads, HUD & Hygiene

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md` und
> `docs/PERFORMANCE.md`. Im Code:
> `godot/scenes/world/effect_library.gd`,
> `godot/scenes/world/effects.gd`,
> `godot/scenes/world/map_objects.gd`,
> `godot/scenes/world/env_look.gd`,
> `godot/assets/shaders/env_common.gdshaderinc`,
> `godot/project.godot`,
> `godot/scenes/terrain/terrain.gd`,
> `godot/scenes/world/unit_highlight.gd`,
> `godot/scenes/world/unit_pick.gd`,
> `godot/scenes/world/combat_view.gd`,
> `godot/scenes/world/fog_of_war.gd`,
> `godot/scenes/hud/hud.gd`,
> `godot/scenes/hud/minimap.gd`,
> `godot/scenes/camera/rts_camera.gd`,
> `godot/scenes/main.gd`.
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hash bleibt unverändert.

## 1. Problem

In der Godot-Präsentation summieren sich mehrere vermeidbare Per-Frame-Kosten,
GPU-Speicher-Duplikate und Wartungs-Schulden auf:

1. **966 doppelte Partikel-Materialien & ~1.500 GPU-Texturen (`effect_library.gd`, `effects.gd`):**
   - `POOL_SIZES` summiert sich auf **966 `GPUParticles3D`-Emitter** (High). Jeder Aufruf von
     `EffectLibrary.build()` erzeugt ein eigenes `ParticleProcessMaterial`, neue `CurveTexture`s,
     neue `GradientTexture1D`s, ein neues `QuadMesh` und ein eigenes `ShaderMaterial`
     (`_material()` cacht nur `_shaders`, nicht die `ShaderMaterial`-Instanz!).
   - Nur `muzzle_tongue` (40 Emitter) mutiert `pm.initial_velocity_max` (`effects.gd:1012`).
     Alle übrigen **926 Emitter** können pro Effekttyp Material, Mesh und Texturen teilen.
   - `effects.gd:1489` formatiert jeden Frame `"%s/%s" % [quality, _biome()]` zum Änderungsvergleich.
   - Laufzeit-`Shader.new()`-Strings (`effect_library.gd`, `fog_of_war.gd`, `atmosphere.gd`,
     `minimap.gd`) erschweren den Pipeline-Cache; erste Nutzung führt zu Hitches (z. B. 784 ms).
2. **16-MB-Instanz-Uniform-Puffer durch identische Biom-Uniforms (`env_common.gdshaderinc`, `map_objects.gd`):**
   - `env_foliage`, `env_dry`, `env_snow`, `env_dust` sind `instance uniform`s und werden
     identisch auf **12.821+ Map-Objekt-Nodes** kopiert (weshalb `project.godot:37` `buffer_size`
     auf `1_048_576` = 16 MB anheben musste).
   - `map_objects.gd:1000` (`_multimesh()`) setzt 6 `env_*`-Uniforms auch auf `tall_grass`,
     `meadow_tufts` und `ground_scatter`, obwohl keiner dieser 3 Shader diese Uniforms deklariert!
   - `map_objects.gd:948-965` (`_build_tall()`) zählt `deco_instances` doppelt (Near + Far).
3. **Terrain-MultiMesh- & Krater-Upload-Overhead (`terrain.gd`):**
   - `terrain.gd:239-270` (`update_chunks()`) lädt bei Kamerabewegung `multimesh.buffer = buffer`
     für **alle 5 LOD-Stufen** in voller Kapazität (~327 KB/Frame auf 4-km-Karten) hoch –
     auch wenn `lists[level].is_empty()`! Außerdem rechnet es `sqrt` + `log()/log(2.0)` für
     alle 1.024–1.269 Chunks („terrain chunks 0,9–1,3 ms“ in `PERFORMANCE.md`).
   - `terrain.gd:329-331` (`regions_changed()`) lädt bei jedem Krater alle 6 Vollkarten-Texturen
     (~88 MB auf `test_large`, ~112 MB auf `coastal_heights`) komplett neu auf die GPU hoch –
     inklusive `"field"`, das Krater gar nicht ändern.
4. **Unkonditionelle Per-Frame-Arbeit bei Stillstand:**
   - `unit_highlight.gd:244` ruft jeden Frame `pick.unit_at()` auf (7 FFI-Arrays + Projektion
     aller Einheiten), selbst wenn Maus & Kamera stillstehen; Zeilen 317–327 allokieren Arrays
     und aktualisieren Uniforms *vor* dem Early-Out; `_color_outlines():395` holt FFI-Arrays
     auch bei leerem `outlined` und kopiert `PackedInt64Array` via `Array(ids).find(id)`.
   - `combat_view.gd:342-345` schreibt `turret.rotation.y` jeden Frame (dirtyt 250+ Türme);
     `HealthBars._process():426` ruft jeden Frame `queue_redraw()` auf.
   - `hud.gd:215, 264-342` führt `_layout()` auf ~10 Controls **jeden Frame** aus.
   - `minimap.gd:300-407` holt `_colors()` jeden Frame, reallokiert `_dots.multimesh.instance_count`
     bei jeder Änderung der Einheitenanzahl, macht 4 Rust-Raycasts auch bei stehender Kamera
     und sucht in `_draw_overlay():404` in O(64 × 504).
   - `rts_camera.gd:439, 482-520` ruft `_apply()` (2–5 Höhenabfragen + `look_at()`) auch bei
     ruhender Kamera jeden Frame auf.
   - `fog_of_war.gd:44-128` nutzt `discard` in beiden Vollbild-Quads (schlecht für TBDR-GPUs
     auf Apple Silicon) und löst mit `FogDesaturate` auch ohne Wasser ein MSAA-`screen_texture`-Resolve aus.
5. **Code-Hygiene (`main.gd`, `effects.gd`):**
   - `main.gd:71-148` preloaded 23 Test-/Benchmark-/Shot-Skripte direkt im Produktionsstart.
   - 7+ Subsysteme greifen per `main._unit_nodes` oder String-Lookups `main.get("...")` in `_process` auf `main` zu.

## 2. Ziel

1. **Partikel-Ressourcen teilen & Shader auslagern:**
   - In `effect_library.gd` `ShaderMaterial`, `QuadMesh` und unveränderliche
     `ParticleProcessMaterial`s pro Effekttyp/Konfiguration cachen (nur `muzzle_tongue` behält
     eigene `ParticleProcessMaterial`s oder teilt den Rest).
   - `_configure()` in `effects.gd` über zwei Felder (`_last_quality`, `_last_biome`) prüfen.
   - Runtime-`Shader.new()`-Strings (mindestens `fog_of_war`, `effect_library`) in echte
     `.gdshader`-Dateien verschieben; optional ein kurzer 2-Frame-Prewarm während `LoadingScreen`
     gegen erste Pipeline-Hitches.
2. **Globale Biom-Uniforms & `map_objects.gd`:**
   - `env_foliage`, `env_dry`, `env_snow`, `env_dust` auf `global uniform` (in `project.godot`
     registriert, gesetzt durch `EnvLook`) umstellen (wie `env_roof_tint`). Prüfen, dass
     Galerie/Thumbnails (die ggf. mehrere Biome testen) die globalen Werte sauber setzen.
   - In `map_objects.gd`: `_apply_look()` nicht auf Gras-/Scatter-MultiMeshes anwenden;
     Doppelzählung von `deco_instances` in `_build_tall()` korrigieren; prüfen, ob
     `project.godot` `buffer_size` wieder verkleinert werden kann.
3. **Terrain-Uploads optimieren (`terrain.gd`):**
   - `multimesh.buffer = buffer` für leere LOD-Stufen überspringen (nur `visible_instance_count = 0`);
     `log()/log(2.0)` durch quadratische Distanz-Schwellenwerte ersetzen.
   - In `regions_changed()` `"field"` überspringen, wenn unverändert; prüfen, ob Godot 4.7
     Teil-Updates (`update_rect` / `RenderingDevice.texture_update`) für die Krater-Texturen
     erlaubt, ansonsten zumindest nur tatsächlich veränderte Layer hochladen.
4. **Per-Frame-Stillstands-Guards & Fog-Cleanup:**
   - Hover-Picking in `unit_highlight.gd` nur bei Änderung von `(mouse_pos, camera_xform, tick)`;
     Early-Out in `_color_outlines()` und `ids.find(id)` ohne `Array(ids)`.
   - Dirty-Guards in `combat_view.gd` (Turret-Rotation, `HealthBars`), `hud.gd` (`_layout`),
     `minimap.gd` (Kapazitätsverdopplung + `visible_instance_count`, Kamera-Dirty-Guard,
     schnelles ID-Lookup), `rts_camera.gd` (`_apply`).
   - `fog_of_war.gd`: `discard` entfernen (`ALBEDO = vec3(1.0)` bei `blend_mul`, `ALPHA = 0.0`
     statt `discard` in `FogDesaturate`, und `screen_texture` nur innerhalb `0 <= v < 1` lesen).
5. **Code-Hygiene:**
   - Die 23 Dev-/Bench-/Shot-Skripte in `main.gd` von `const ... := preload(...)` auf
     bedarfsweises `load(...)` beim jeweiligen CLI-Flag umstellen.
   - String-Lookups `main.get("...")` in `_process`-Pfaden durch typisierte Felder/Getter ersetzen
     und `effects.gd` (1.723 Zeilen) sinnvoll in Teilmodule (`burst_pool.gd`, `wreck_fire_view.gd`,
     `vehicle_ground_fx.gd`, `gun_kick_view.gd`) aufteilen, soweit es die Lesbarkeit klar verbessert.

## 3. Prüfen und zeigen

- **Messungen vorher/nachher direkt hintereinander:**
  - `just bench-view` (Skriptzeiten für `terrain chunks`, `unit markers`, `hud`, `combat view`, `effects` + Gesamt-FPS).
  - `just effects-bench` und `just crater-bench` (falls vorhanden) bzw. Krater-Hitch-Prüfung.
  - Speicher-/Ressourcenanzahl der Partikel-Pools (Materialien/Texturen vor/nach Sharing loggen).
  - Ergebnisse in `docs/PERFORMANCE.md` ergänzen.
- `just check`, `just selftest`, `just gallery-check`, `just editor-selftest` grün; Golden Hash unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen in `crates/sim` oder an 3D-Modellen.
- Infanterie-Skript-Umbau (Prompt 206) und `UV2`-Surface-Merge (Prompt 207).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/208-godot-rahmenkosten-partikel-terrain-hud-und-uniforms.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (2026-11-26)

- **Partikel-Sharing & Shader-Extraktion:** `effect_library.gd` teilt `ParticleProcessMaterial`, `QuadMesh`, `ShaderMaterial`, `CurveTexture` und `GradientTexture1D` pro `(kind, soft, clouds, wind)` (nur `muzzle_tongue` dupliziert sein `ParticleProcessMaterial`); auf `high` fallen 1 114 Emitter auf 69 ProcessMaterials, 30 Meshes, 23 ShaderMaterials und 54 Curve-/Ramp-Texturen (`medium`: 722 → 55/30/22/54, `low`: 392 → 43/30/19/54). Alle Partikel-, Fog-, Minimap- und Vignette-Shader wurden in `.gdshader`/`.gdshaderinc`-Dateien ausgelagert; `process_material`/`draw_pass_1` werden vor den übrigen `GPUParticles3D`-Eigenschaften gesetzt (0 `ParticlesShaderRD`-RID-Leaks beim Beenden). `_configure()` in `effects.gd` prüft über `_last_quality`/`_last_biome`; `grass_fire.gd` verhindert Material-Duplikate.
- **Globale Biom-Uniforms & `map_objects.gd`:** `env_foliage`, `env_dry`, `env_snow`, `env_dust` in `project.godot` als `global uniform` registriert (`buffer_size = 524288` [vorläufig]) und über `EnvLook.apply_global()` gesetzt (`EnvLook.global_snow()` als Headless-Fallback für `RendererDummy`). `map_objects.gd` wendet `_apply_look()` nicht mehr auf Gras-/Tuft-/Scatter-MultiMeshes an und zählt `deco_instances` nicht mehr doppelt.
- **Terrain-Uploads (`terrain.gd`):** Quadratische LOD-Schwellenwerte ohne `sqrt`/`log`, Überspringen leerer LOD-Uploads und Layer-Dirty-Prüfung bei `regions_changed()` (Krater-Upload auf `test_large`: 7,9 → 5,2 ms, −34 %, `worst` 33,38 → 29,12 ms).
- **Stillstands-Dirty-Guards & Fog:** Dirty-Guards in `unit_highlight.gd`, `combat_view.gd`, `hud.gd`, `minimap.gd` und `rts_camera.gd`; `fog_dim.gdshader`/`fog_desaturate.gdshader` ohne `discard`. In `bench-view --perf-quick --quality=high` (1280×720, M1 Max): 40/150/600 m 70,6/72,1/72,1 → 73,7/74,3/73,6 FPS, `hud` −48 % bis −51 %, `combat view` −36 % bis −39 %, `unit markers` −36 % bis −40 %.
- **Hygiene:** 26 Dev-/Bench-/Shot-Skripte in `main.gd` auf lazy `load(...)` umgestellt, heiße `main.get(...)`-Aufrufe durch typisierte Felder ersetzt, `gun_kick_view.gd` aus `effects.gd` ausgelagert.
- **Prüfung:** `just check` (756 Tests, Golden Hashes unverändert), `just selftest`, `just editor-selftest`, `just bench-view --perf-quick --quality=high`, `just crater-view-bench test_large` und `just effects-bench` grün und ohne RID-Leaks. (`just gallery-check` hat keine neuen Fehler aus Prompt 208; vorbestehende Prompt-202-Boot-/Geschütz-Meldungen unverändert.)
