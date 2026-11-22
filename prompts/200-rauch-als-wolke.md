# Prompt 200 – Rauch als zusammenhängende Wolke statt Partikel

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> in `docs/ARCHITECTURE.md` die Teile zu Effekten (Prompt 019), Billows
> (Prompt 072), „Indirect fire and smoke“ (Prompt 032) und die Reihenfolge der
> Durchgänge (Wasser, Rauch, Leuchtspur nach der Bildschirmkopie),
> `docs/PERFORMANCE.md`. Im Code `godot/scenes/world/effect_library.gd`
> (`smoke_column`, `explosion_smoke`, `salvo_smoke`, `dust_trail`, `_atlas`,
> Shader „billow“, weiche Partikel `fx_soft`), `effect_textures.gd` (`smoke`,
> `billow`), `effects.gd` (Pools), `artillery_view.gd` (Rauchwolken der
> Nebelgranaten, `_puff_mesh` als Billboard-Quads), `kill_sequence.gd`
> (Wrackrauch), `grass_fire.gd`, `graphics_quality.gd`, `atmosphere.gd`
> (Sonne, Wind). Renderer: Forward+ (`godot/project.godot`).
>
> **Vorlage** (nur Stil-Referenz, nichts daraus übernehmen): derselbe Trailer
> wie in Prompt 199, lokal unter
> `.temp/trailer/Defilade - Official Announcement Trailer (1074p_30fps_H264-128kbit_AAC).mp4`
> (nicht im Repo). Bilder mit `ffmpeg` in den Scratchpad ziehen und ansehen;
> **keine Bilder aus dem Video ins Repo**, keine Assets, Texturen oder Designs
> übernehmen.

## 1. Problem

Der Nutzer (2026-11-22): Der Rauch im Video sieht sehr gut aus, als Einheit,
wie eine Wolke, und nicht wie geometrische Partikel.

Bei uns besteht Rauch aus `GPUParticles3D`-Quads (z. B. `smoke_column`: 64
Quads, 3,2 m, Billow-Atlas mit Normale aus Prompt 072). Aus Spielhöhe sieht
man die einzelnen Scheiben: Ränder, Wiederholung des Atlas, Drehen der Quads,
harte Überlagerung, wo zwei Quads sich schneiden; die Säule zerfällt in Flecken
statt eine Masse zu bilden.

## 2. Was die Vorlage zeigt (Auswertung vom 2026-11-22)

- **20 s und 30 s** (Wüste, Wracks und Nebel): weiße und graue Wolken als
  **weiche, runde Ballen**, die ineinander übergehen; keine Kanten, kein
  Einzelteil erkennbar. Innen **sanft schattiert** (unten/Schattenseite etwas
  dunkler, sonnenseitig hell), außen **weich ausfransend und durchscheinend**.
  Sie wirken leicht unscharf, wie eine Masse mit Volumen.
- **16–17 s** (Großangriff im Schnee): hellgraue **Säulen** steigen auf,
  verbreitern sich nach oben und verwehen; dunkelgraue bis schwarze
  Rauchballen über Wracks, ebenfalls als zusammenhängende Pilze. Staubfahnen
  hinter Fahrzeugen als weiche Klumpenreihen.
- **26 s** (Häuserkampf): weißer Rauch zieht flach über ein Dach, eine
  durchgehende, durchscheinende Schwade mit weichem Rand, durch die man das
  Dach noch sieht.
- Wenige, große Formen statt vieler kleiner; ruhige Bewegung (langsames
  Quellen und Driften, kein Drehen).

## 3. Ziel

Rauch, Staub und Dampf lesen sich als zusammenhängende Wolke. Reine
Darstellung; was Sicht blockiert, entscheidet weiter die Simulation (Nebelwolken
aus `indirect.rs`).

**Zuerst Technik wählen** (kurz prototypen, Bilder und Messung je Variante,
dann eine nehmen und dem Nutzer mit Bildern begründen):

1. **Weichere Partikel:** weniger, größere Puffs, die stark überlappen;
   Textur ohne erkennbare Kanten, kein Drehen; Dichte- statt
   Alpha-Mischung (vormultipliziert); weiche Partikel immer; Beleuchtung pro
   Wolke statt pro Quad (Normale aus der Lage im Wolkenkörper, nicht aus der
   Quad-Textur), damit Schatten- und Sonnenseite über die ganze Säule laufen.
2. **Rauch-Puffer:** Rauchpartikel in einen eigenen Puffer in halber/viertel
   Auflösung zeichnen, dort weich zeichnen (Blur) und mit Tiefentest ins Bild
   mischen. Die einzelnen Quads verschmelzen zu einer Masse, der weiche Rand
   und die leichte Unschärfe der Vorlage entstehen von selbst.
3. **Volumen:** Raymarching in einem Proxy-Körper (Kugelhaufen bzw. Box je
   Wolke mit 3D-Rauschen) oder Godot-`FogVolume` (Forward+, volumetrischer
   Nebel nötig). Am echtesten, aber am teuersten; nur, wenn das Budget reicht,
   z. B. nur für große Nebelwände und Wrackrauch auf Hoch.

Dann für alle Rauchsorten umsetzen: Wrackrauch (`smoke_column`), Explosionsrauch,
Nebelgranaten-Wolken (`artillery_view.gd`), Salven-/Raketenrauch, Brand,
Staubfahnen, Mündungswolken. Farben und Lebensdauer wie heute, Wind wie heute.

- Licht: Sonnenseite hell, Unterseite dunkler, abends warm; nachts von Feuer
  angestrahlt (wo heute schon Licht von Bränden ist).
- Nebel des Krieges wie heute.
- Qualitätsstufen: Niedrig bleibt bei einfachen Partikeln; Mittel/Hoch die neue
  Technik; Kosten je Stufe in `ARCHITECTURE.md`.
- Wiederholbare Bilder (Trailer-Seeds aus Prompt 082) bleiben wiederholbar.

## 4. Nicht

- Keine Änderung an `crates/sim`, Golden Hash unverändert.
- Keine Godot-Zufälle für Spielrelevantes (Rauch ist nur Optik).
- Nichts aus der Vorlage übernehmen außer dem Eindruck.

## 5. Abnahme

- Vorher/Nachher-Bilder aus Spielhöhe und nah, Tag und Abend: brennendes
  Wrack, Nebelwand aus Nebelgranaten, Mörsereinschläge, Staubfahne einer
  Kolonne, Rauch über einem Dach; mit `just trailer-shots --stills` bzw. einem
  eigenen Schnappschuss-Werkzeug, nebeneinander gestellt.
- Bildrate mit 10 brennenden Wracks und einer Nebelwand im Bild
  (`just bench-view`), vorher/nachher direkt hintereinander, im Rahmen von
  `docs/PERFORMANCE.md`.
- `just selftest`, `just check` grün.
