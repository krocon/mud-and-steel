# Prompt 207 – Einheiten-Material-Slots über `UV2` bündeln (3×–5× weniger Draw-Calls)

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach Prompt 204 (`Mount`)
> und idealerweise koordiniert mit Prompt 213 (falls Soldaten-Meshes neu gebaut werden).
> Lies zuerst `CLAUDE.md`, `docs/ASSETS.md` („Material slots“, „Material system“, „LOD“),
> `docs/INFANTRY.md` und `docs/PERFORMANCE.md` (Abschnitte „Presentation“, „Prompt 165“,
> „Brick quarter“, „Gun crews“). Im Code:
> `godot/scenes/world/unit_merge.gd`,
> `godot/assets/import/unit_post_import.gd`,
> `godot/assets/shaders/unit_camo.gdshader`,
> `godot/assets/shaders/soldier.gdshader`,
> `godot/scenes/tools/soldier_look.gd`,
> `godot/scenes/world/unit_highlight.gd`,
> `tools/blender/soldier/mesh.py`.
> Reine Darstellung / Asset-Pipeline – **keine Änderungen in `crates/sim`**, Golden Hash unverändert.

## 1. Problem

Sowohl Fahrzeuge als auch Soldaten nutzen heute ein eigenes `ShaderMaterial` pro Material-Slot
(`uniform int slot`). Dadurch zersplittern alle Meshes in viele einzelne Surfaces – und jeder
`material_overlay`-Pass (`unit_xray.gdshader`, `unit_outline.gdshader`, Freund-/Feind-Tönung)
sowie alle 4 Schatten-Kaskaden rendern **jede Surface erneut**:

- **Fahrzeuge:** Ein gemergter Panzer (`unit_merge.gd`) hat 5–8 Surfaces auf `Hull`, 2–4 auf
  `Turret` und 1–2 auf `Gun` → **10–18 Draw-Calls pro Panzer** (× 5 Pässe mit Schatten + X-Ray).
  Das erklärt u. a., warum `unit_xray.gdshader` bis zu **+1.230 Draw-Calls** bei 600 m erzeugt.
  Zusätzlich erzeugt `unit_merge.gd:120-133` beim ersten Spawnen jedes Typs synchron auf dem
  Main-Thread per `SurfaceTool.append_from` + `ImporterMesh.generate_lods()` die Meshes –
  ohne `st.index()` aufzurufen!
- **Soldaten:** Jeder Soldat hat **11 Surfaces** (4 auf `Body`, 5 auf `Gear`, 2 auf `Weapon`),
  plus 4 zusätzliche X-Ray-Draw-Calls pro eigenem Soldat. Bei 1.125 Soldaten entstehen so
  **4.500–6.100 Draw-Calls** bei 40–150 m. Dabei zeigt `soldier.gdshader:9` ausdrücklich:
  `UV2 = (y, 1)` – **`UV2.y` ist konstant `1.0` und komplett ungenutzt!**

## 2. Ziel

### Teil A – Fahrzeuge: 1 Surface pro Träger (`Hull`, `Turret`/`Mount`, `Gun`)
1. In `unit_camo.gdshader` einen Modus einführen (oder direkt umstellen), bei dem `slot`
   (0..7) aus **`UV2.x`** gelesen wird (gerundet: `int(round(UV2.x))`), statt über 8 getrennte
   `ShaderMaterial`-Ressourcen.
   - Rückwärtskompatibilität für Tools/Galerie/unmergte Modelle: z. B. `uniform int slot = -1;`
     (`-1` = aus `UV2.x` lesen, `>= 0` = wie bisher aus dem Uniform, sodass `gallery` und
     einzelne Slot-Materialien weiter funktionieren!).
2. In `unit_merge.gd`:
   - Alle Oberflächen eines Trägers (`Hull`, `Turret`/`Mount`, `Gun`) in **einen einzigen**
     `SurfaceTool` zusammenführen und dabei pro Vertex `UV2 = Vector2(float(slot_index), 0.0)`
     schreiben.
   - `st.index()` vor `st.commit_to_arrays()` aufrufen (verhindert de-indizierte Triangle-Lists
     und verbessert `generate_lods()`).
   - Nur **ein** gemeinsames `ShaderMaterial` (`unit_combined.tres` mit `slot = -1`) zuweisen.
   - Ergebnis: Ein Panzer sinkt von ~12–18 Surfaces auf **3 Surfaces** (je 1 für `Hull`, `Turret`, `Gun`).
3. **Prüfen:** Lässt sich der statische Merge (oder zumindest die Indizierung/LOD-Vorbereitung)
   sauber in `unit_post_import.gd` verlagern oder per Cache beim Start vorwärmen, ohne die
   `model_gallery` und `validate.py` zu brechen? Falls ja, umsetzen; sonst begründen.

### Teil B – Soldaten: 1 Surface pro Mesh (`Body`, `Gear`, `Weapon`) über `UV2.y`
1. **Ohne sofortigen 86-Modell-Neubau ODER beim Import/Laden:**
   - Option 1 (schnell, rein in Godot beim Laden/Import via `SoldierLook` / `unit_post_import.gd`):
     Die Oberflächen von `Body` (4→1), `Gear` (5→1) und `Weapon` (2→1) einmalig pro Modell-Szene
     zusammenführen und `slot` (0..9) in die ungenutzte `UV2.y`-Komponente schreiben (dabei
     `ARRAY_BONES`, `ARRAY_WEIGHTS` und Blend-Shapes / Shape-Keys `Stocky`/`Slim`/`FaceB`/`FaceC`
     unbedingt erhalten!).
   - Option 2 (sauber im Blender-Export `tools/blender/soldier/mesh.py`): `UV2.y = slot` direkt
     exportieren (falls zusammen mit Prompt 213 alle Soldaten ohnehin neu gebaut werden).
   - Die beste Option wählen und begründen (wichtig: Shape-Keys und Skinning dürfen nicht
     kaputtgehen!).
2. In `soldier.gdshader`: `slot` bei `uniform int slot = -1` aus `UV2.y` lesen; die
   Slot-Basisfarben, Rauheiten und Metallwerte (10 Einträge) als kleine Uniform-Arrays
   (`uniform vec4 slot_color[10]`, `uniform float slot_rough[10]`, `uniform float slot_metal[10]`)
   übergeben.
   - Ergebnis: `Body` 4→1, `Gear` 5→1, `Weapon` 2→1 (**11 → 3 Draw-Calls** nah, **4 → 1** für
     X-Ray und Fern-MultiMesh!).
   - Bonus prüfen: Kann für das Fern-MultiMesh (`infantry_view.gd`) ebenfalls ein 1-Surface-Mesh
     genutzt werden?

## 3. Prüfen und zeigen

- **Visuelle Gleichheit:**
  - Galerie- und Spiel-Screenshots (Panzer `de_pz4_h`, `us_m4`, `su_t34_76` in Wald/Wüste/Winter
    inkl. Ketten, Glas, Holz, Metall; Soldaten aller 4 Fraktionen inkl. Tarnanzug, Sanitäter-Armbinde
    `Insignia`, Shape-Keys `Stocky`/`Slim`, fliegendem Helm `helmet_off` und X-Ray-Silhouette).
  - Keine sichtbaren Abweichungen zu vorher!
- **Draw-Call- & FPS-Messung (vorher/nachher direkt hintereinander):**
  - `just bench-view` (40 / 150 / 600 m stehend & Schlacht): Draw-Calls und FPS vergleichen.
  - `just bench-battery` (40 / 150 m): Draw-Calls vergleichen.
  - Ergebnisse in `docs/PERFORMANCE.md` eintragen.
- `just check`, `just selftest`, `just gallery-check` grün; Golden Hash unverändert.

## 4. Nicht Teil dieses Prompts

- Auslagern der 61 Soldaten-Animationen aus den 86 GLBs (Prompt 213).
- Änderungen an der Simulation oder den Einheitenwerten.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/207-einheiten-surfaces-in-uv2-zusammenfassen.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis

- **Teil A – Fahrzeuge (`unit_camo.gdshader`, `combined.tres`, `unit_merge.gd`):**
  - `unit_camo.gdshader` liest bei `slot < 0` den Material-Slot aus `int(round(UV2.x))`; die bestehenden Einzelslot-`.tres` (`slot = 0..7`) bleiben für unmergte Modelle/Galerie rückwärtskompatibel.
  - Neues gemeinsames Material `godot/assets/materials/unit/combined.tres` (`slot = -1`).
  - `unit_merge.gd` stempelt für jede `unit_camo`-Surface `UV2.x = slot` (mit einheitlichem Attributformat ohne ungenutzte Tangenten), führt alle `unit_camo`-Surfaces pro `(carrier, shadow)`-Gruppe in einen einzigen `SurfaceTool` zusammen und ruft vor `commit_to_arrays()` `st.index()` auf → 1 Surface pro Träger (`Hull`, `Turret`/`Mount`, `Gun`, also 3 statt 12–18 Surfaces pro Panzer).
  - Begründung gegen Verlagerung nach `unit_post_import.gd` (vorläufig): `model_gallery`, `validate.py`, `UnitCamo.catalog_values` und Thumbnails erwarten intakte Einzelteile/Extras in den `.glb`-Szenen; der einmalige Typ-Cache in `unit_merge.gd` vermeidet einen 292-GLB-Reimport bei minimaler Einmalkostenzeit.
- **Teil B – Soldaten (`soldier.gdshader`, `soldier_look.gd`):**
  - Gewählt: **Option 1 (einmalige Laufzeit-Zusammenführung pro `Mesh`-Ressource in `SoldierLook`, vorläufig)** — vermeidet sofortigen 86-GLB-Neubau (Prompt 213 fasst die Soldaten-GLBs ohnehin gesondert an) und erhält `ARRAY_BONES`, `ARRAY_WEIGHTS` (inkl. 8-Bone-Flag) sowie alle Blend-Shapes (`Stocky`, `Slim`, `FaceB`, `FaceC`) bitgenau.
  - `soldier.gdshader` unterstützt bei `slot = -1` das Lesen aus `UV2.y` (in `vertex()` inkl. `helmet_off` und `fragment()`) mit `slot_color[10]`, `slot_rough[10]`, `slot_metal[10]` (und `base_color` = Cloth für bestehende Tests).
  - `Body` (4→1), `Gear` (5→1), `Weapon` (2→1) sinken von **11 auf 3 Surfaces**; Fern-Impostors (`infantry_view.gd::_make_impostor`) profitieren automatisch mit 1 Surface statt 4.
- **Messungen & Prüfung (`docs/PERFORMANCE.md`, M1 Max, 1280 × 720, `high`):**
  - `bench-view --perf-quick` 40 / 150 / 600 m: Draw-Calls **780 / 1556 / 2967 → 544 / 953 / 1938 (−30 % bis −39 %)**, Render-CPU bei 150 / 600 m **1,56 / 1,91 → 1,28 / 1,50 ms**.
  - `bench-battery` 40 / 150 m: Draw-Calls **2126 / 1226 → 983 / 732** (Bedienungs-Anteil **1437 → 373** bzw. **496 → 123**, ca. **4× weniger Draw-Calls**).
  - Visuell geprüft (`standing_40.png`, `battery_on_40.png`): keine Abweichungen an Tarnung, Ketten, Metall, Holz, Haut, Helmen oder Uniformen.
  - `infantry_preview --self-test` (`86 soldiers, 61 clips, 3 biomes`), `just selftest` und `just check` grün; alle Einheiten-/Soldaten-Checks in `model_gallery` grün (nur vorbestehende Boots-/UK-Geschütz-Hinweise). Simulation & Golden Hashes unverändert.
