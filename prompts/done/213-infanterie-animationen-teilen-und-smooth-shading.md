# Prompt 213 – Infanterie-Animationen in geteilte Bibliotheken auslagern und Smooth-Shading aktivieren

> In einer frischen Session im Repo-Ordner ausführen. Benötigt Blender (`docs/VERSIONS.md`).
> Lies zuerst `CLAUDE.md`, `docs/ASSETS.md` (Abschnitt „Infantry“) und `docs/INFANTRY.md`.
> Im Code:
> `tools/blender/build_soldier.py`,
> `tools/blender/soldier/mesh.py`,
> `tools/blender/soldier/body.py`,
> `tools/blender/soldier/gear.py`,
> `tools/blender/soldier/animation.py`,
> `tools/blender/validate_soldier.py`,
> `godot/scenes/tools/soldier_look.gd`,
> `godot/scenes/world/infantry_view.gd`,
> `godot/scenes/world/gun_crew_view.gd`,
> `godot/scenes/tools/model_gallery.gd`,
> `godot/scenes/tools/thumbnail_render.gd`,
> `web/units/viewer.js`.
> Reine Asset-/Präsentations-Änderung – **keine Änderungen in `crates/sim`**, Golden Hash unverändert.

## 1. Problem

Die Analyse aller 438 `.glb`-Modelle (237,4 MB) zeigt zwei massive Ineffizienzen bei den
86 Infanterie-Modellen (`godot/assets/models/*/infantry/*.glb`):

1. **~103 MB redundante Animations-Duplikate (54 % des gesamten 3D-Speichers):**
   - Alle 86 Soldaten-GLBs zusammen sind **128,8 MB** groß (~1,58 MB pro Datei!).
   - Beispiel `de/infantry/rifleman.glb`: nur 306 KB Mesh-Daten, aber **364 KB Animations-Binary
     + 827 KB Animations-JSON** für **61 Clips** (`export_force_sampling=True` in
     `build_soldier.py:140-142`).
   - Jede Rolle jeder Fraktion (86 Dateien!) exportiert alle 61 Clips komplett neu – obwohl
     viele Rollen dieselbe Waffe (z. B. Kar98k) nutzen und ein Großteil der Clips überhaupt
     nur vom Waffentyp (`kind`) abhängt (und alle 86 Soldaten exakt dasselbe
     `SkeletonProfileHumanoid`-Skelett teilen!).
   - Durch `just units-web-sync` werden diese ~129 MB zusätzlich noch nach `web/units/` kopiert.
2. **100 % Flat-Shading → 3,55× zu viele geskinnte GPU-Vertices:**
   - In `tools/blender/soldier/mesh.py:204` setzt `poly.use_smooth = False` ausnahmslos alle
     Flächen auf hartes Flat-Shading.
   - Messung an `de/infantry/rifleman.glb`: `Body[0]` hat nur **512** einzigartige
     Vertex-Positionen, exportiert aber **1.820 GPU-Vertices** (3,55× Bloat!). Dasselbe gilt
     für `Body[1]` (184→620), `Body[2]` (176→592) und `Gear` (121→432).
   - Optisch sind Helme, Köpfe, Arme und Hosen dadurch sichtbar facettiert
     (`godot/assets/thumbnails/de_inf_rifle_squad.png`), und bei 2.250 geskinnten Soldaten
     multipliziert sich die GPU-Skinning-Arbeit unnötig um den Faktor ~3!

## 2. Ziel

### Teil A – Smooth-Shading auf Soldaten-Meshes
1. In `tools/blender/soldier/mesh.py` (und ggf. `body.py` / `gear.py`):
   - Smooth-Shading (`poly.use_smooth = True`) für organische/runde Formen (Torso, Hals, Kopf,
     Ärmel, Hosenbeine, Stiefelschaft, Helm-Glocke, Feldflasche, Gasmaskendose) aktivieren.
   - Harte Kanten (bzw. `smooth=False`) gezielt für kantige Details beibehalten: Taschen,
     Koppel/Gürtel, Sohlen, Helmrand/Schirm, Gewehrschäfte/-kästen, Kisten.
   - Prüfen, dass Shape-Keys (`Stocky`, `Slim`, `FaceB`, `FaceC`), Skin-Weights und
     Rest-Pose-UVs (`UV`, `Rest`) vollständig intakt bleiben.
   - Ziel: GPU-Vertexzahl pro Soldat um **~55–65 %** senken (von ~4.460 auf ~1.500–2.000
     Vertices) bei sichtbar saubererer Oberfläche.

### Teil B – Geteilte Animations-Bibliotheken statt 86× Einbettung
1. Analysieren, welche der 61 Clips in `tools/blender/soldier/animation.py` von konkreten
   `weapon.points` abhängen (z. B. Hand-IK an `grip_l`/`grip_r`) vs. nur vom `kind` oder
   komplett waffenunabhängig sind:
   - **Prüfen:** Wie viele distinkte Animationssätze ergeben sich nach Waffe (`weapon.id`,
     ca. 20–25) oder nach Waffenklasse (`kind`, ca. 6–8, falls Griffpunkte je `kind` nahe
     beieinander liegen oder per `Weapon`-Bone-Offset passen)?
   - Die einfachste, wartbare Lösung wählen, die **keine sichtbaren Hand-zu-Waffe-Fehlgriffe**
     erzeugt (lieber ~15–20 geteilte Bibliotheken mit exakten Griffen oder 1 geteilte
     Basis-Bibliothek für alle waffenfreien Clips + kleine Waffen-Overlays als schwebende Hände!).
2. **Export-Pipeline umstellen:**
   - Die 86 Soldaten-`.glb`s werden **ohne** die 61 NLA-Clips exportiert (`export_animations=False`)
     und tragen im Root-Extra den Schlüssel ihrer Animationsbibliothek (z. B. `anim_library`).
   - Ein neuer Builder-Schritt (Teil von `just soldiers` / `build_soldier.py`) exportiert die
     geteilte(n) Animations-`.glb`-Datei(en) unter `godot/assets/models/infantry/anims/` (ohne
     redundant gesampelte konstante Bone-Kanäle, wo glTF-Export das zulässt).
   - In Godot (`soldier_look.gd` / `infantry_view.gd` / `gun_crew_view.gd` / `model_gallery.gd` /
     `thumbnail_render.gd` / `squad`-Szenen): die `AnimationLibrary` einmalig pro Bibliothek
     laden und dem `AnimationPlayer` jedes Soldaten zuweisen (shared `AnimationLibrary`-Ressource!).
   - **`web/units/viewer.js` mitnehmen:** Prüfen, wie die Einheiten-Webseite Soldaten-Posen/Clips
     anzeigt, und die geteilte Animations-GLB auch dort sauber nachladen (sonst stehen die
     Soldaten im Web-Viewer in T-Pose!).
3. `tools/blender/validate_soldier.py` entsprechend anpassen (prüft Soldaten-GLBs +
   geteilte Animations-Bibliotheken + Referenzintegrität).

## 3. Prüfen und zeigen

- **Speicher- & Vertex-Vergleich vorher/nachher:**
  - Gesamtgröße `godot/assets/models/*/infantry/*.glb` + `infantry/anims/*.glb`
    (Ziel: von **128,8 MB** auf **< 30 MB**, ideal ~15–20 MB!).
  - Vertices/Dreieck-Verhältnis bei `de/infantry/rifleman.glb` (Ziel: `Body` von `1.98 v/t`
    auf `< 1.0 v/t`).
- **Visuelle Prüfung:**
  - Nahaufnahmen (Galerie / Thumbnails) aller 4 Fraktionen in mehreren Clips (Stehen, Zielen,
    Laufen, Nachladen, Liegen, Mörser/Geschütz) → Hände sitzen exakt an den Waffengriffen,
    Helme & Körper ohne Facettierung oder Shading-Artefakte!
  - Alle Infanterie- & Geschütz-Thumbnails neu rendern (`CLAUDE.md`: ein geänderter Soldat
    betrifft alle Trupp- und Geschützbilder!), `just units-web-sync` ausführen und den
    Web-Viewer (`just units-web`) prüfen.
- **Tests & Performance:**
  - `just validate-assets`, `just import`, `just check`, `just selftest`, `just gallery-check` grün.
  - `just bench-view` (stehend 40/150 m) vorher/nachher in `docs/PERFORMANCE.md` eintragen.
  - Doku in `docs/ASSETS.md` und `docs/INFANTRY.md` aktualisieren.

## 4. Nicht Teil dieses Prompts

- Zusammenführen der Soldaten-Material-Slots über `UV2` (Prompt 207; wenn Prompt 207
  bereits erledigt ist, dessen `UV2.y`-Kodierung beibehalten!).
- Änderungen an Spielregeln oder Kampfanimationen in `crates/sim`.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/213-infanterie-animationen-teilen-und-smooth-shading.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (2026-11-26)

- **Teil A – Smooth-Shading auf Soldaten-Meshes:**
  - `tools/blender/soldier/mesh.py`: Per-Face-`smooth`-Flag (`box`/`_fix_cap` = `False`, `tube`/`revolve`/`ellipsoid` = `True`, `cylinder` = `sides >= 6`), in `to_object()` auf `poly.use_smooth` angewendet. Shape-Keys (`Stocky`/`Slim`/`FaceB`/`FaceC`), Skin-Weights, `UVMap`, `Rest` und `Col` bleiben unverändert intakt.
  - `tools/blender/soldier/gear.py`: Helmglocke außen glatt (`smooth=True`), Helmrand/-unterseite und Y-Riemen kantig (`smooth=False`).
  - `de/infantry/rifleman.glb`: `Body` **3 160 → 1 398 Vertices** (`1,98 → 0,88 v/t`, −55,8 %), `Gear` **1 124 → 809 Vertices** (`1,95 → 1,40 v/t`), `Weapon_kar98k` **180 → 168 Vertices**, gesamt **4 464 → 2 375 Vertices** (−46,8 % geskinnte GPU-Vertices) bei 2 262 Dreiecken.
- **Teil B – Geteilte Animations-Bibliotheken:**
  - `tools/blender/soldier/loadouts.py`: 16 geteilte Bibliotheken (`ANIM_LIBRARIES` / `WEAPON_ANIM_LIBRARY`, ≤ 2 cm Griffabstand innerhalb jeder Gruppe) für alle 45 Waffen + unbewaffnet.
  - `tools/blender/build_soldier.py`: Soldaten-GLBs ohne Animationen (`export_animations=False`, `export_morph_normal=False`, Root-Extra `anim_library`); `build_anim_library()` exportiert 16 Bibliotheken (`godot/assets/models/infantry/anims/<lib>.glb`) mit `Armature` an der Wurzel und `_prune_constant_anim_channels()` (entfernt nur global konstante Rest-Pose-Kanäle über alle 61 Clips: **4 392 → 1 159 Kanäle/Bibliothek**, −73,6 %); `animations.json` ergänzt um `libraries` und `roles`.
  - `godot/scenes/tools/soldier_look.gd`: `shared_library()` lädt und cached jede `AnimationLibrary` einmalig (inkl. Loop-Modi), `find_player()` hängt sie bei Bedarf an `arm.get_parent()` ein (Track-Pfad `Armature/Skeleton3D:<Bone>` passt identisch für alle 86 Soldaten).
  - `web/units/viewer.js` & `tools/units_web.py`: Web-Viewer lädt `animations.json` und `anims/<lib>.glb` bei animationslosen Soldaten-GLBs nach; `units-web-sync` spiegelt alle 16 Bibliotheken und `animations.json`.
  - `tools/blender/validate_soldier.py` erweitert (prüft `anim_library`, 0 Clips in Soldaten-GLBs, alle 16 `anims/*.glb` mit allen 61 Clips und `animations.json`).
  - `godot/scenes/tools/infantry_preview.gd`: Helm-Landetest auf feste `physics_frame`s umgestellt, weil Headless-`process_frame`s nach der Optimierung schneller als 120 FPS laufen.
- **Speicher-Vergleich:**
  - Vorher: 86 Soldaten-GLBs = **128,82 MB** (`de/infantry/rifleman.glb`: 1 578,6 KB).
  - Nachher: 86 Soldaten-GLBs (**17,89 MB**) + 16 Animations-Bibliotheken (**8,70 MB**) = **26,59 MB** (**−102,23 MB / −79,4 %**, `rifleman.glb`: 239,4 KB).
- **Prüfung & Benchmark:**
  - `just validate-soldiers` (86 Soldaten + 16 Bibliotheken), `just validate-squads` (53 Trupps), `just infantry-selftest` (86 Soldaten, 61 Clips, 3 Biome, Ragdolls, Helme, Tarnung), `just selftest`, `just gallery-check` (alle 53 Trupps grün; nur die 10 vorbestehenden Boot-/Geschütz-Meldungen aus Prompt 202), 75 Infanterie-/Geschütz-Thumbnails neu gerendert & visuell geprüft, `just units-web-sync` grün.
  - `just bench-view --perf-quick --quality=high` (1280×720, M1 Max): 40 / 150 / 600 m **75,8 / 75,5 / 76,1 FPS** (vorher 75,0 / 75,6 / 74,7 FPS).
  - `docs/ASSETS.md`, `docs/INFANTRY.md` und `docs/PERFORMANCE.md` aktualisiert; `crates/sim` und Golden Hashes unverändert.
