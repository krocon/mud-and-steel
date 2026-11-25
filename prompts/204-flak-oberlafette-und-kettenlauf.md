# Prompt 204 – Drehbare Flak-Oberlafetten (`Mount`) und animierter Kettenlauf (`track_offset`)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md` und
> `docs/ASSETS.md` (Abschnitte „Hierarchy and names“, „Material system“). Im Code:
> `godot/scenes/world/unit_merge.gd`,
> `godot/scenes/world/combat_view.gd`,
> `godot/scenes/world/blast_effects.gd`,
> `godot/scenes/main.gd`,
> `godot/assets/shaders/unit_camo.gdshader`.
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hash bleibt unverändert.

## 1. Problem

Zwei bereits vollständig in den 3D-Modellen und Shadern vorbereitete Fahrzeug-Animationen
sind in der Godot-Präsentation nicht oder fehlerhaft verdrahtet:

1. **8 Flak-Geschütze drehen ihre Oberlafette (`Mount`) nicht:**
   Laut `docs/ASSETS.md` und `tools/blender/validate.py:51` besitzen alle 8 Flak-Modelle
   (`de_flak36`, `de_flak38`, `su_52k`, `su_61k`, `uk_37in_aa`, `uk_bofors_40mm`,
   `us_90mm_m2`, `us_bofors_m1`) die Hierarchie `Carriage -> Mount -> Gun`, wobei `Mount`
   die 360°-drehbare Oberlafette (mit Sitzen und Schutzschild) ist.
   - `unit_merge.gd:45` sucht jedoch **nur** nach `"Turret"` und `"Gun"`, nicht nach `"Mount"`.
     Dadurch wird `Mount` fest in das Wurzel-Mesh der Kreuzlafette verschmolzen!
   - `combat_view.gd:353` sucht in `_turret_of()` ebenfalls nur nach `"Turret"`.
     Die Oberlafette folgt also nie `aim_yaw`.
   - Auch Mehrturm-Panzer (`GunTurret_*`, z. B. `su_t35_1935`, `us_m3_lee` etc.) sollten
     in `unit_merge.gd` geprüft werden, damit ihre Struktur konsistent bleibt.
2. **Panzerketten stehen beim Fahren still (`track_offset` ungenutzt):**
   - `tools/blender/kit/running_gear.py` versieht alle Kettenflächen mit `FLAG_TRACK`
     und nahtlosen Glieder-UVs (`v = links` entlang des Bandes, `link_pitch` m/Glied).
   - `unit_camo.gdshader:41, 126-134` deklariert `instance uniform float track_offset = 0.0;`
     und scrollt damit das Kettenglied-Muster (`fract(UV.y + track_offset)`).
   - Ein Grep über `godot/scenes/` zeigt jedoch: `track_offset` wird **nirgends im Code gesetzt**.

## 2. Ziel

1. **`Mount` als drehbaren Turm-Ersatz unterstützen:**
   - In `unit_merge.gd`: Wenn kein `"Turret"` existiert, nach `"Mount"` suchen und diesen
     als eigenen Träger behandeln (analog zu `Turret`, sodass `Gun` weiter Kind von `Mount` bleibt).
     Prüfen, ob `blast_effects.gd:213` (Turmabsprengung) bewusst nur `"Turret"` betrifft
     (eine Flak-Oberlafette fliegt i. d. R. nicht wie ein Panzerturm weg – Entscheidung dokumentieren).
   - In `combat_view.gd:_turret_of()`: zuerst `"Turret"`, sonst `"Mount"` suchen, sodass
     Flak-Oberlafetten samt Geschützrohr sauber auf das Ziel schwenken, während die
     Kreuzlafette (`Carriage`) am Boden stehen bleibt.
   - Prüfen, wie sich Flaks beim Abprotzen / Drehen verhalten (Drehachse, Vorzeichen von
     `aims[i] - facings[i]`, kein Versatz des Rohrs).
2. **`track_offset` beim Fahren antreiben:**
   - Für fahrende Kettenfahrzeuge (bzw. Halbketten) die tatsächlich zurückgelegte Strecke
     pro Frame (aus Positionsdifferenz entlang der Fahrtrichtung, inkl. Vorzeichen für
     Rückwärtsfahrt; optional bei Drehung auf der Stelle kleine Bewegung) auf einen
     Akkumulator pro Einheit aufaddieren, durch die Kettenglied-Teilung (z. B. `0.15 m`
     bzw. aus glTF-Extras, falls vorhanden) teilen und via
     `set_instance_shader_parameter(&"track_offset", offset)` auf dem Hull-Mesh setzen.
   - Nur aktualisieren, wenn sich das Fahrzeug bewegt hat (`!still`), damit stehende
     Einheiten keine unnötigen Uniform-Writes auslösen.
   - Keine Auswirkung auf Nicht-Kettenfahrzeuge (`is_track` ist dort `0` im Shader;
     trotzdem nur für Ketten-/Halbketten-Meshes oder bewegte Fahrzeuge setzen).

## 3. Prüfen und zeigen

- **Selbsttest erweitern:**
  - Prüfen, dass eine `de_flak36` oder `us_bofors_m1` nach `UnitMerge.merge()` noch einen
    eigenen `Mount`-Knoten mit gemergtem Mesh hat und dessen `rotation.y` bei Zielwechsel
    dem `aim_yaw` folgt, während `Carriage` ruht.
  - Prüfen, dass sich `track_offset` bei einem fahrenden Panzer ändert und im Stand konstant bleibt.
- **Visuelle Kontrolle:**
  - Kurze Bildfolge einer schwenkenden Flak (`de_flak36` / `us_bofors_m1`) und eines
    fahrenden Panzers (`de_pz4_h`) aus Nahaufnahme (`~15–20 m`), um Drehrichtung und
    Laufrichtung der Kette (oberes Trum läuft vorwärts!) zu verifizieren.
- `just check`, `just selftest` grün; Golden Hash unverändert.

## 4. Nicht Teil dieses Prompts

- Einzelne drehende Laufrollen-Nodes (werden weiterhin gemergt, um Draw-Calls niedrig zu halten).
- Umbau auf 1-Surface-Meshes über `UV2` (Prompt 207).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/204-flak-oberlafette-und-kettenlauf.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.
