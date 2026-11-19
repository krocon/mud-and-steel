# Prompt 090 – Kein Bodenflackern beim Zoomen und Bewegen der Kamera

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `godot/scenes/camera/rts_camera.gd` ganz (vor allem `_process`, `zoom_level` und die
> Stelle, die `camera.near`/`camera.far` und `sun.directional_shadow_max_distance` setzt),
> `godot/scenes/terrain/terrain.gd` ganz (`update_chunks`, LOD-Wahl, Nahtschluss über die
> Nachbar-Strides), den Vertex-Teil von `godot/assets/materials/terrain.gdshader`,
> `godot/scenes/world/atmosphere.gd` (`SHADOW_DISTANCE_FACTOR`, `SPLITS_4`, `SPLIT_2`,
> SSAO-Einstellungen und wie der SSAO-Radius mit dem Abstand wächst),
> `godot/scenes/world/graphics_quality.gd`, `tools/ground-motion.sh` mit
> `tools/ground_flicker.py`, in `docs/ARCHITECTURE.md` die Abschnitte zum Flimmern
> („Flicker while the camera moves“, Prompt 068, und `just ground-motion`) und in
> `docs/PERFORMANCE.md` die Messregeln.

## 1. Ausgangslage

Beim Zoomen und beim Bewegen der Kamera flackert der Boden. Steht die Kamera still, ist
alles ruhig. Die Bodenfarben kommen pro Pixel aus Texturen mit anisotropen Mipmaps; an
der Texturfilterung liegt es nach erster Durchsicht also nicht. Drei Werte ändern sich
aber mit der Kamera und sind die Verdächtigen:

1. **Schattendistanz gleitet mit dem Zoom.** Die Kamera setzt jedes Bild
   `directional_shadow_max_distance = distance * shadow_distance_factor` (in
   `atmosphere.gd` auf 1,8 gesetzt). Damit ändert sich beim Zoomen ständig die Größe
   eines Schatten-Texels auf dem Boden, und die Kaskadengrenzen wandern. Godots
   Schattenstabilisierung hilft nur bei gleicher Distanz. Erwartet: Schattenkanten von
   Hügeln, Bäumen, Häusern und Einheiten laufen und flimmern beim Zoomen.
2. **Terrain-LOD ohne Hysterese.** `terrain.gd` wählt die Stufe hart aus der Entfernung
   (`lod0_distance` 140 m, je Verdopplung eine Stufe gröber, fünf Stufen). Ein Chunk an
   einer Grenze springt bei kleinen Bewegungen hin und her; Hügelsilhouetten und ihre
   Schatten „poppen“. Die Farbe ist davon nicht betroffen, nur Form und Schatten.
3. **SSAO-Radius wächst mit dem Abstand.** Beim Zoomen ändert sich die Abdunklung in
   Mulden fließend, das kann ein leichtes Wabern erzeugen (vermutlich schwächster
   Beitrag).

Die Frage, ob feste Zoomstufen helfen würden, ist geklärt: nein. Die Kamera gleitet
zwischen Stufen ohnehin durch alle Zwischenwerte. Der Zoom bleibt stufenlos.

## 2. Ziel

- Beim Zoomen und beim Schwenken flackert der Boden nicht mehr sichtbar; Schatten
  springen höchstens selten und klein (an einer Stufengrenze der Schattendistanz).
- Keine Chunks, die bei kleinen Bewegungen zwischen LOD-Stufen hin- und herspringen.
- Bildrate gleich oder besser (Richtwert: höchstens 2 % schlechter auf einer Stufe).
- Reine Darstellung: nichts in `crates/sim`, Golden Hash unverändert.

## 3. Regeln und Technik

- **Zuerst messen, welche Ursache wie viel ausmacht.** `just ground-motion` misst bisher
  Schwenks. Ergänze einen Zoom-Shot (z. B. `zoom_ground`: von etwa 40 m auf 300 m und
  zurück über offenem, hügeligem Gelände mit Bäumen) und miss ihn mit
  `--off=shadows`, `--off=ssao` und ohne Abschaltung. Für das LOD einen Schwenk auf
  etwa 140–300 m Abstand über Hügeln. Der Shader hat `show_lod` als Hilfe zum Ansehen.
- **Schattendistanz in Stufen:** z. B. 50 / 100 / 200 / 400 / 800 m (oder Faktor √2,
  wenn die Sprünge sonst zu sehen sind), immer die nächste Stufe über
  `distance * SHADOW_DISTANCE_FACTOR`; Wechsel erst mit etwa 10–15 % Hysterese, damit
  die Stufe beim Zoomen an der Grenze nicht pendelt. Die Kaskaden-Splits bleiben
  Bruchteile der Distanz; prüfen, ob sie mit der Stufe weiterhin den sichtbaren Boden
  treffen (sie sind auf 0,8–1,5 Kameraabstände ausgelegt). Den Rest des Abstands nicht
  durch weichere Schatten kaschieren.
- **LOD-Hysterese im Terrain:** eine gröbere Stufe erst, wenn die Entfernung etwa 10 %
  über der Grenze liegt, die feinere erst etwa 10 % darunter. Dafür die letzte Stufe je
  Chunk merken (`_lods` gibt es schon). Der Nahtschluss über die Nachbar-Strides muss
  weiter stimmen; Selftest bzw. Bild prüfen, dass keine Risse entstehen.
- **SSAO:** nur anfassen, wenn die Messung einen spürbaren Anteil zeigt; dann den Radius
  ebenfalls in Stufen oder deutlich träger nachführen.
- Vorhandene Werte wie `rts_camera.gd`'s Standard `shadow_distance_factor = 2.5`
  (wird von `atmosphere.gd` auf 1,8 überschrieben) nicht still ändern; wenn eine
  Stelle tot ist, aufräumen und im Commit sagen.
- Nur Godot-Seite: `godot/scenes/camera/`, `godot/scenes/terrain/`,
  `godot/scenes/world/atmosphere.gd`, `godot/scenes/tools/trailer_shots.gd` (neue Shots),
  `tools/ground-motion.sh`.

## 4. Vorgehen

1. Vorher messen: `just ground-motion` mit dem neuen Zoom-Shot und dem LOD-Schwenk
   (`--png` für das feine Band), je mit und ohne Schatten/SSAO. Kurze Tabelle, welche
   Ursache wie viel ausmacht. Liegt die Ursache woanders (z. B. Tall Grass, Wiese,
   Wasser), das melden und dort ansetzen statt blind die Liste abzuarbeiten.
2. Schattendistanz in Stufen mit Hysterese umsetzen, erneut messen.
3. LOD-Hysterese umsetzen, erneut messen; auf Risse an LOD-Nähten achten.
4. SSAO nur, wenn Schritt 1 es verlangt.
5. Bildrate vorher/nachher nach `docs/PERFORMANCE.md` (`just bench-view`, alle
   Qualitätsstufen, mindestens die größte Karte). Laufen andere Sessions im selben
   Checkout, vor dem Messen pausieren lassen.
6. Self-Test ergänzen: Schattendistanz nimmt nur Stufenwerte an und pendelt bei kleinem
   Hin- und Herzoomen an einer Grenze nicht; ein Chunk an einer LOD-Grenze wechselt bei
   kleinen Bewegungen nicht hin und her. Vorhandene Tests nicht abschwächen.
7. Doku: `docs/ARCHITECTURE.md` (Abschnitt Flimmern: Messung des Zooms, Stufen,
   Hysterese, Messwerte), `docs/PERFORMANCE.md` (Bildrate).
8. Der Nutzer schaut es sich danach selbst mit `just run` an; nenne ihm Karte und
   Zoombereich, an denen der Unterschied am deutlichsten war.

## 5. Nicht Teil dieses Prompts

- Feste Zoomstufen oder eine andere Zoom-Bedienung.
- Neue Schattentechnik (z. B. eigene Schattenkarten), andere Sonnenwinkel, weichere
  Schatten als Ersatz für stabile.
- Änderungen an Texturen, Splatting oder Tall Grass, außer die Messung zeigt dort die
  Ursache (dann melden und eng begrenzt beheben).
- Flimmern bei stehender Kamera.
