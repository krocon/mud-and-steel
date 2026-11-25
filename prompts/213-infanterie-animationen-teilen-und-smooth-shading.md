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
