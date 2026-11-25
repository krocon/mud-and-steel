# Prompt 217 – Perspektivische Weitwinkel-Kamera („vorne steiler als hinten“)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Abschnitt Kamera / `rts_camera.gd`, Schattenstufen aus Prompt 090,
> Kamera-Neigung aus Prompt 096, Unschärfe aus Prompt 183) und `docs/PERFORMANCE.md`
> („Camera tilt“, „Far camera distances“). Im Code:
> `godot/scenes/camera/rts_camera.gd`,
> `godot/scenes/world/atmosphere.gd`,
> `godot/scenes/world/graphics_quality.gd`,
> `godot/scenes/menu/settings.gd`,
> `godot/scenes/tools/trailer_shots.gd`.
>
> **Stil-Referenz (nicht im Repo, nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:00–00:06`.
> Einzelbilder bei Bedarf mit `ffmpeg -ss 1 -i "..." -frames:v 1 <scratch>/ref.png`
> ansehen; **keine Bilder oder Assets aus dem Video ins Repo**.
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hash unverändert.

## 1. Problem

In der Referenz (Sek. 00:00–00:06) entsteht eine sehr plastische, filmische RTS-Perspektive:
Die Panzer im unteren Bildbereich wirken fast wie aus steiler Draufsicht (~60°), während
der Blick nach oben flach über Felder, Dorf, Kirche und die entfernte Waldkante (~7–10°)
hinweggleitet („vorne steiler als hinten“).

Ursache ist reine Kamera-Optik – kein gekrümmter Boden:
- `rts_camera.gd:80` nutzt heute ein enges Tele-Sichtfeld (`fov_deg := 30.0`) bei
  `pitch = 45°–65°`. Dabei beträgt die Winkelspreizung vom unteren zum oberen Bildrand nur
  `±15°` (sehr flach/isometrisch).
- Mit einem **weiteren vertikalen FOV (`~50–55°`)** bei **flacherer Neigung (`~30–35°`)**
  entsteht genau dieser Effekt natürlich aus der Perspektivprojektion:
  - Unterer Rand: `33° + 26° = 59°` (steile Übersicht auf die nahen Einheiten)
  - Bildmitte: `33°`
  - Oberer Rand: `33° − 26° = 7°` (tiefer Weitblick bis zur Waldkante)

## 2. Ziel

1. **Weitwinkel-Perspektive in `rts_camera.gd` & Einstellungen:**
   - Ein umschaltbares Kamera-Profil in den Einstellungen (z. B. `camera_perspective`:
     `"classic"` = bisherige 30°-Tele-Optik als Standard/Fallback vs. `"wide"` =
     perspektivische Weitwinkel-Optik mit `fov_deg ≈ 50–54°`, `pitch_near_deg ≈ 30–34°`,
     `pitch_far_deg ≈ 52–58°`, passender `tilt_min_deg` / `far_factor` / `shadow_distance_factor`).
   - Alternativ oder zusätzlich prüfen, wie sich `ground_reach`, `far_plane`,
     `directional_shadow_max_distance` (Prompt 090), Tiefenunschärfe (`dof`, Prompt 183)
     und Nebel/Horizontdunst (`atmosphere.gd`) bei `fov = 52°` und `pitch = 32°` verhalten:
     - Der Horizont darf nicht im leeren Blau/Kartenrand abgeschnitten wirken; die
       `far_plane` und `shadow_distance` müssen bis zum oberen sichtbaren Drittel reichen,
       ohne die Schatten-Auflösung im Vordergrund zu ruinieren.
     - Ein weicher, distanzabhängiger **Tiefendunst / Luftperspektive** am fernen Horizont
       unterstützt das Raumgefühl der flachen Blickrichtung.
   - Maus-Picking (`ground_pick.gd`, `unit_pick.gd`), Auswahlrahmen, Minimap-Sichttrapez
     (`minimap.gd`) und Nebel des Krieges (`fog_of_war.gd`) müssen mit dem größeren FOV
     exakt funktionieren (das Minimap-Trapez wird vorne schmal und hinten breit – prüfen!).
2. **Saubere Unterstützung in `trailer_shots.gd`:**
   - `trailer_shots.gd` unterstützt bereits `fov`, `pitch`, `dist`, `haze` pro Kamera-Key.
   - Prüfen und sicherstellen, dass ein Shot mit `fov = 52`, `pitch = 33`, `dist = 65`
     konsistente Schattenstufen, Tiefendunst, `far_plane` und `dof` liefert (ohne harte
     Shadow-Cutoffs oder abgeschnittene Bäume im Hintergrund).
3. **Performance-Einfluss sauber messen:**
   - Ein weiter Winkel + flacher Pitch bringt deutlich mehr Gelände und Objekte ins Frustum.
   - Vorher/Nachher (`classic` 30° vs. `wide` 52°) mit `just bench-view --perf-quick` und
     auf einer Feld-/Dorfkarte messen und in `docs/PERFORMANCE.md` festhalten.

## 3. Prüfen und zeigen

- **Vorher/Nachher-Standbilder** derselben Feld-/Dorfszene (z. B. `farms` oder `valley_fields`):
  - `classic` (30°, 45°) direkt neben `wide` (52°, 32°), um den „vorne steiler als hinten“-Effekt
    zu belegen.
- **Funktionstests:**
  - Einheiten-Klick, Auswahlrahmen, Rechtsklick-Zielvorschau, Minimap-Sichttrapez und
    Nebel des Krieges im `wide`-Modus per Selbsttest (`just selftest`) prüfen.
- `just check`, `just selftest` grün; Golden Hash unverändert.
- Lokalisierung (`de`/`en`) für die neue Kamera-Einstellung inkl. `tools/l10n_check.py`.

## 4. Nicht Teil dieses Prompts

- Gekrümmte/nichtlineare Projektionsmatrizen („World-Curvature“-Shader) – reine echte
  Perspektiv-Kamera reicht völlig aus und hält Picking/Schatten/UI exakt.
- Änderungen in `crates/sim`.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/217-perspektiv-kamera-weiter-winkel.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (2026-11-25)

**Umgesetzt**

- **Kamera-Profil** `camera_perspective` (Einstellungen → Grafik, „Kameraperspektive“;
  `-- --camera-perspective=<id>` für Messungen und Bilder): `classic` bleibt Standard,
  `wide` ist neu. Die Werte stehen in `PERSPECTIVES` in `rts_camera.gd`:
  52° Sichtfeld, 33° Neigung bis zur Mitte des Zoombereichs, dann bis 58° am fernsten
  Zoom, Neigung ab 32° (oberer Bildrand 6° unter dem Horizont, kein Himmel über flachem
  Boden). Unterer Bildrand 59°, oberer 7° – der Selbsttest misst genau das.
- **Gleiche Zoomstufe = gleiche Größe:** Die Zoom-Entfernungen des Weitwinkels sind das
  0,6-Fache (9–360 m, 100 % = 48 m). Umschalten im laufenden Gefecht behält Zoomstufe,
  Blickpunkt, Drehung und Neigung. Gefechtsstart, Nahansicht, Übersicht und der
  Zoom-Reset der Minimap rechnen mit `zoom_scale`. Der Karteneditor bleibt klassisch.
- **Schatten:** Reichweite folgt wie bei der Neigung dem oberen Bildrand (nah 5,2
  Kameraentfernungen). Die Kaskaden des Weitwinkels kommen aus dem Blickwinkel
  (`shadow_splits`: logarithmisch zwischen unterem Bildrand und Schattenweite) und
  werden nur gesetzt, wenn die Schattenstufe wechselt – die erste Kaskade endet etwa
  eine Kameraentfernung tief, der Vordergrund bleibt scharf. Klassisch unverändert.
- **Dunst:** Dunstbereich des Bioms × 1,6: beginnt im oberen Bilddrittel, voll am
  oberen Rand.
- **Detailstufen:** Soldaten, Geschützbedienungen und Soldaten-Silhouetten wählen ihre
  Fernformen jetzt nach Bildgröße statt nach Metern (`detail_distance_factor`).
- **Trailer (`trailer_shots.gd`):** Shot-Schlüssel `perspective` (Standard `classic`,
  unabhängig von der Spielereinstellung). Freie Kamera: Schatten reichen bis zum Boden
  am oberen Bildrand von `pitch`/`fov` (im 500-m-Limit), Tiefenunschärfe richtet sich
  nach `dist`/`pitch` statt nach der Dunst-Entfernung. Neue Shots
  `survey_perspective_*` (`just perspective-shots`), darunter fov 52 / pitch 33 /
  dist 65 ohne und mit Tiefenunschärfe.
- **Tilt-Shift aus** (Zusatzauftrag vom 2026-11-25): `depth_of_field` steht standardmäßig
  auf `off`; die Modi bleiben im Menü und im Fotomodus wählbar.
- Lokalisierung de/en, `docs/ARCHITECTURE.md` („Camera rig“ → Perspectives),
  `docs/PERFORMANCE.md` („Camera perspectives“), `docs/CONTROLS.md`.

**Geprüft**

- `just check` und `just selftest` grün, Golden Hash unverändert (keine Änderung in
  `crates/sim`).
- Selbsttest `_wide_perspective_test`: Umschalten wie im Einstellungsmenü, gleiche
  Zoomstufe, 59°/7° an den Bildrändern, Bodenpick nah und fern, Einheiten-Klick,
  Auswahlrahmen, Zielvorschau bei gehaltener rechter Taste, Minimap-Trapez (vorn
  schmal, hinten mehr als dreimal so breit, Ecken unter den Bildecken), Schatten und
  Far-Plane bis zum oberen Rand, erste Kaskade im Vordergrund, Nebel des Krieges,
  zurück auf klassisch alles wie vorher. `flow_self_test.gd`: Option wird gespeichert
  und geladen.
- Vorher/Nachher: `docs/images/perspective/classic_wide.jpg` (Panzerkolonne auf
  `farms`, gleiche Zoomstufe); alle Standbilder in `build/perspective/after/`.
- Performance (`docs/PERFORMANCE.md`): Weitwinkel kostet auf `bench_large` 2–3,5 ms
  pro Bild (64/62/67 → 54/54/55 FPS), auf `valley_fields` 1,5–5 ms (48/46/35 →
  45/38/29 FPS), bei 1,7–3-fachen Draw Calls.

**Offen**

- **Kartenrand:** Die Karte ist ein Block ohne Umland. Nah am Rand zeigt der
  Weitwinkel-Blick nach außen in der oberen Bildhälfte den leeren Hintergrund
  (`survey_perspective_edge`); klassisch ist es nur ein Streifen. Nicht gelöst – das
  bräuchte Gelände um die Karte herum.
- Nicht gemessen: Gefechtsansichten, Qualität Mittel/Niedrig. Die Schattenauflösung im
  Vordergrund ist an Standbildern auf Qualität Hoch beurteilt, nicht gemessen.
- Sichtweiten der Kartenobjekte (`visibility_range`, 220 m) und der Bodenstreu (150 m)
  zählen weiter in Metern; im Weitwinkel enden sie deshalb weiter oben im Bild.

