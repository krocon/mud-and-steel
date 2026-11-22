# Prompt 191 – Boden mit Relief: Büschel, Kontaktschatten, Streuschicht

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> dann in `docs/ARCHITECTURE.md` den Abschnitt „Meadow (prompt 057)“ samt
> „Relief beyond the blades (prompt 191)“, „Splatting shader“ (Regel aus
> Prompt 071: Texturkoordinaten nur aus der Weltposition) und den
> Performance-Teil (`docs/PERFORMANCE.md`, Budget aus Prompt 016). Im Code
> `godot/assets/materials/terrain.gdshader`,
> `terrain_meadow.gdshaderinc` (`meadow_relief`, `m_relief_layer`),
> `godot/scenes/world/meadow_tufts.gd`, `godot/scenes/world/map_objects.gd`
> (Dekorationsschichten), `godot/scenes/world/graphics_quality.gd`,
> `godot/scenes/world/atmosphere.gd` (`_apply_sun`, `ground_sun`,
> `ground_relief`), `crates/cli/src/gen_textures.rs` (`clumps`).
>
> **Vorlage** (nicht im Repo, nur Stil-Referenz): Bildschirmfoto des Nutzers
> vom 2026-11-22, Schrägansicht eines Bahndamms mit Wiese aus RTS-Höhe. Es wird
> nichts daraus übernommen, nur der Eindruck: ein „krisseliger“, körniger
> Boden mit vielen kleinen Hell-Dunkel-Wechseln und Schatten auf wenigen
> Pixeln, statt einer glatten grünen Fläche.

## 1. Problem

Der Nutzer (2026-11-22): „Bei uns sieht der Boden (Gras) so platt aus.“

Ursachen (im Code geprüft):

- Halme im Shader gibt es nur bis 2,5 cm Pixelgröße, die 3D-Büschel
  (`meadow_tufts.gd`, `RANGE := 55.0`) nur bis 55 m vor der Kamera. Aus der
  üblichen Spielhöhe ist der größte Teil des Bildes weiter weg und zeigt nur
  die flache Textur.
- Fast nichts am Boden wirft Schatten (nur die hohen Büschel). Der körnige
  Eindruck der Vorlage entsteht aus tausenden kleinen Schatten und
  Kontaktschatten.
- Wenig Kontrast im Detail: generierte Platzhalter-Texturen (`CREDITS.md`).

## 2. Erledigt in der Session vom 2026-11-22 (Teil 1 und 2)

1. **Kontrast, Lücken, AO auf Gras:** `meadow_relief` legt Grasbüschel in drei
   Größen (0,45 / 1,1 / 2,6 m) über die Wiese, jeweils dort eingeblendet, wo ein
   Büschel etwa 6–15 Pixel breit ist. Dunkle Lücken, leicht hellere Kuppen
   (mittlere Helligkeit ausgeglichen), Normale aus der Kuppenneigung, etwas AO
   in den Lücken, Stärke schwankt über einige zehn Meter. Nicht unter Hochgras.
2. **Fern-Kontaktschatten:** ein zusätzlicher Texturzugriff zur Sonne hin
   (`ground_sun`, globaler Shader-Parameter aus `atmosphere.gd`, schwächer bei
   schwacher Sonne und unter Wolkenschatten).
3. Büschel als Textur `terrain_clumps.png` (`just terrain-textures`), nicht pro
   Pixel aus Hashes (das kostete etwa das Dreifache).
4. Qualitätsstufen: Niedrig aus, Mittel ohne Schatten, Hoch beides.

Messung (M1 Max, 1728 × 1084, `just terrain-bench`, Hoch): etwa +3–4 ms pro
Bild bei 120 m und 600 m (Schatten davon etwa 1 ms), auf Niedrig nichts.
Vergleichsbilder: `just terrain-shots <dir>` mit `SHOT_SET=meadow`
(`meadow_110`, `meadow_350`) und `bash tools/trailer-shots.sh --stills
--only=survey_clouds_meadow`.

Offen aus Teil 1/2 (zuerst prüfen):

- `just ground-motion` (und `--near`) mit Relief laufen lassen: Flimmern beim
  Schwenken und Zoomen messen, mit den Tabellen in `ARCHITECTURE.md`
  vergleichen. Übergänge zwischen den Büschelgrößen beim Zoomen ansehen
  (Pumpen?).
- Wiese im Wüsten- und Winter-Biom (`meadow_strength` 0: kein Relief; ob Sand
  und Schnee ein eigenes, schwächeres Relief brauchen, entscheiden).
- Die Bildzeit schwankte zwischen Läufen stark (28 bis 97 FPS bei gleicher
  Ansicht, wohl Fensterzustand). Vorher/Nachher immer direkt hintereinander
  messen.

## 3. Ziel dieses Prompts (Teil 3–5)

### 3.1 Streuschicht mit Kleinkram (Hauptteil)

Die Vorlage wirkt vor allem durch echte Geometrie überall im Bild: niedrige
Sträucher, Farn, trockene Stängel, kleine Steine. Neue Dekorationsschicht
(Muster: `meadow_tufts.gd` + Dekorationsgitter von Prompt 013 in
`map_objects.gd`):

- Varianten: niedriger Strauch (0,4–0,9 m, rund, dunkel), Farn/Kraut-Büschel,
  trockene Stängel (Distel/Ampfer, hell), Feldsteine 0,1–0,4 m. Alles
  prozedural oder als kleines Blender-Modell mit Quelle (`docs/ASSETS.md`,
  Budgets einhalten), Farben aus der Wiesenpalette.
- Dichte nach Geländetyp, Feuchte, Waldnähe und Makro-Rauschen, nicht
  gleichmäßig: Gruppen, Ränder an Hecken, Wegen und Waldrand dichter.
- **Sichtweite größer als die Büschel:** Ziel 120–150 m, mit Ausblenden
  (Verkleinern oder Dithern, kein Aufploppen). Sträucher und große Steine
  werfen Schatten, kleine Teile nicht.
- Fahrzeuge drücken Sträucher platt bzw. lassen sie verschwinden, wie die
  Bäume aus Prompt 106 (nur Darstellung, keine Spielregel), Krater und
  Fahrspuren räumen die Streuschicht.
- Qualitätsstufen: Dichte und Sichtweite je Stufe in `graphics_quality.gd`.
- Thumbnails: neue Modelle mit Katalogeintrag brauchen ein Bild (`CLAUDE.md`),
  reine Dekoration ohne Katalogeintrag nicht.

### 3.2 SSAO/Kontaktschatten prüfen

In `graphics_quality.gd` ist SSAO auf Mittel/Hoch an. Prüfen, ob stärkere
Werte (Radius, Intensität, `environment_set_ssao_quality`) den Boden zwischen
Büscheln und Streuschicht sichtbar verbessern und was es kostet. Nur
übernehmen, wenn es Bilder und Messung rechtfertigen.

### 3.3 Optional: Fotoscans

CC0-Fotoscans (ambientCG, Poly Haven, 1K/2K mit Albedo, Normale, Rauheit) für
die Lagen Gras und Erde testen. In das Streifenformat packen
(`gen_textures.rs`, `CREDITS.md` je Quelle eine Zeile). Nur übernehmen, wenn
sie mit Palette und Relief zusammenpassen und keine Kachelung zeigen.

## 4. Nicht

- Keine Godot-Physik, keine Spielregel: Streuschicht und Relief sind reine
  Darstellung, nichts davon gelangt in `crates/sim`.
- Keine Texturkoordinaten, die vom Kameraabstand abhängen (Prompt 071).
- Keine Assets, Namen oder Designs aus der Vorlage.

## 5. Abnahme

- Vorher/Nachher-Bilder aus 45, 110, 300 m auf `test_large` (Wiese am Dorf),
  `ambush_meadow` und einer Karte mit Hecken; dazu Wüste und Winter.
- `just terrain-bench` und `just bench-view` vorher/nachher direkt
  hintereinander, Kosten je Qualitätsstufe in `ARCHITECTURE.md`.
- `just ground-motion` ohne Verschlechterung gegenüber den Tabellen.
- `just check` grün.
