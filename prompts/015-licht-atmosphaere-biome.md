# Prompt 015 – Licht, Atmosphäre und Biome

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Terrain-Darstellung, Kamera), `docs/ASSETS.md`
> (Material-System, Umwelt-Objekte) und `docs/MAPS.md`. Deren Regeln gelten unverändert.

## 1. Ziel

Die Szene wirkt bisher flach: ein Standard-`DirectionalLight3D` ohne Einstellungen,
graue Hintergrundfarbe, konstantes Umgebungslicht. Ziel ist der Look der
Stil-Referenzen: kräftige Sonne mit harten Schatten, weiche Verschattung in Senken
und an Objektkanten, leichter Dunst in der Ferne, stimmige, nicht übersättigte Farben.

Dazu drei **Biome**: gemäßigt (`forest`: Wiese, Wald, grüne Felder), Wüste (`desert`)
und Winter (`winter`). Die Karte wählt ihr Biom in `map.toml`; Licht, Himmel, Dunst,
Bodenfarben, Tarn-Paletten der Einheiten (`UnitCamo`, Prompt 004) und `EnvLook` der
Umwelt-Objekte (Prompt 007) folgen daraus.

Das Biom ist **reine Darstellung**. Die Simulation liest das Feld nur und gibt es an
die Bridge weiter; es geht nicht in den Karten-Hash oder den State-Hash ein. Der
Golden-Hash bleibt unverändert.

## 2. Ausgangslage

- `godot/scenes/main.tscn`: `WorldEnvironment` mit Hintergrundfarbe und festem
  Umgebungslicht, `Sun` mit `shadow_enabled`, in `main.gd` per `look_at` gerichtet.
  Die RTS-Kamera (`rts_camera.gd`, 15–600 m, Neigung 45°–65°) setzt
  `directional_shadow_max_distance = 2,5 × Abstand` und `far = 4 × Abstand`.
- Terrain: `assets/materials/terrain.gdshader`, zehn Schichten aus zwei
  Textur-Arrays (Gras, Feldweg, Schotter, Sand, Schlamm, Schnee, Kiesel, Schlick,
  Fels, Erde), Geländeart-Gewichte aus der Sim. Die Geländearten der Testkarten
  sind überwiegend Gras; eine Wüste oder ein Winter müssen daher über eine
  biomabhängige **Umdeutung der Schichten** entstehen, nicht über andere Kartendaten.
- Einheiten: `UnitCamo.apply(model, UnitCamo.palette(faction, biome))`, Paletten
  `assets/materials/camo/<faction>_<biome>.tres`; `main.gd` nutzt fest `forest`.
- Umwelt-Objekte: `EnvLook.apply(model, biome)`; Wind ist als Material-Parameter
  `wind_strength`, `wind_direction`, `wind_speed` vorbereitet. Platzierung kommt mit
  Prompt 013 (parallel).
- Werkzeuge: `just terrain-bench` (FPS je Ansicht), `just terrain-shots`
  (`--write-movie`), `just gallery` / `just gallery-check` mit eigener Beleuchtung.

## 3. Umfang

1. **Biom in der Karte.** `map.toml` bekommt das optionale Feld
   `biome = "forest" | "desert" | "winter"` (Standard `forest`, unbekannte Werte
   sind ein Ladefehler). `sim::Biome` (Enum), `Map::biome()`; nicht im Karten-Hash.
   `cli gen-map --biome` schreibt das Feld, die beiden Testkarten erhalten
   `biome = "forest"`. Bridge: `SimHost.map_biome()`. In Godot überschreibt
   `-- --biome=<id>` das Kartenbiom (zum Vergleichen und für Aufnahmen).
2. **Biom-Presets als Ressourcen:** `BiomePreset` (`godot/assets/biomes/biome_preset.gd`)
   mit `godot/assets/biomes/{forest,desert,winter}.tres`: Sonne (Höhe, Richtung,
   Farbe, Stärke), Himmel (prozedural: Zenit, Horizont, Boden), Umgebungslicht,
   Dunst (Farbe, Stärke), Belichtung, Weißpunkt, Kontrast, Sättigung,
   Farbkorrektur-Verlauf, Bodenfarben (Schicht-Umdeutung und Tönung je Terrain-Schicht,
   Schneedecke), `EnvLook`-Schlüssel, Tarn-Biom der Einheiten, Wind, Schneefall.
3. **Atmosphäre-Knoten** (`godot/scenes/world/atmosphere.gd`): wendet ein Preset und
   eine Qualitätsstufe auf `WorldEnvironment`, `Sun`, Terrain-Material und
   Umwelt-Materialien an und führt zoomabhängige Werte jedes Bild nach:
   - Sonne mit **PSSM-4-Kaskaden** (niedrig: 2), Splits und Bias passend zur
     Kameradistanz, harte Schatten (kleiner Lichtwinkel, wenig Weichzeichnung).
   - **Tonemapping AgX** (Filmic als Rückfall), Belichtung je Biom, Farbkorrektur
     über einen 1D-Verlauf (`GradientTexture1D`, kühle Schatten / warme Lichter) plus
     Kontrast und Sättigung.
   - **SSAO** mit Radius und Stärke nach Zoomstufe (nah: Kontaktschatten an Ketten und
     Rädern, fern: Verschattung von Senken); SSIL nur in „hoch“, falls messbar
     sinnvoll; SDFGI nicht (große Karte, Kosten, Lichtlecks).
   - **Dunst:** Tiefennebel, Anfang und Ende proportional zur Kameradistanz, so dass
     der Vordergrund klar bleibt und der obere Bildrand leicht im Dunst liegt; Farbe
     aus dem Himmel (Luftperspektive).
   - **Vignette** als schlichter Vollbild-Shader unter der HUD.
   - **Tiefenunschärfe** nur in der Nahansicht (unter ~50 m) und nur in „hoch“.
4. **Terrain je Biom:** `terrain.gdshader` bekommt pro Schicht eine Quelle
   (`layer_source`) und eine Tönung (`layer_tint`) sowie eine Schneedecke
   (flache Flächen, Rauschen) – Wüste: Gras wird trockene Steppe aus Sand-/Erde-Textur,
   Winter: Gras wird Schnee, Wege und Straßen bleiben teilweise frei. Der
   Kartenrand (`terrain_edge.gdshader`) bekommt die passende Oberkante.
5. **Einheiten und Objekte je Biom:** `main.gd` nimmt Palette und `EnvLook` aus dem
   Preset statt fest `forest`; Wind global aus dem Preset.
6. **Qualitätsstufen** `low`, `medium`, `high` (`godot/scenes/world/graphics_quality.gd`):
   Schattenauflösung und Kaskaden, Schatten-Filter, SSAO an/aus und Qualität, SSIL,
   Glow, Antialiasing (FXAA / TAA / MSAA), Tiefenunschärfe, Schneefall-Partikel.
   Auswahl über `-- --quality=<stufe>` (Standard `high`) und im Debug-Menü (F3).
   Ein Einstellungsmenü folgt in Prompt 027.
7. **Wetter vorbereiten:** Schneefall als `GPUParticles3D` um den Kamera-Fokus im
   Winter-Biom (ab „mittel“), Dichte und Volumen mit der Zoomstufe skaliert.
8. **Werkzeuge:** `just terrain-bench` und `just terrain-shots` reichen `--biome` und
   `--quality` durch; die Modell-Galerie nutzt dieselben Presets für Licht und Himmel,
   so dass Modelle dort so aussehen wie im Spiel.

## 4. Akzeptanzkriterien

1. `just check`, `just selftest` und `just gallery-check` grün; der Golden-Hash
   ändert sich nicht. Der Demo-Ablauf `-- --demo` läuft weiter.
2. Tests: `map.toml` mit jedem Biom lädt, ohne Feld ergibt `forest`, ein unbekannter
   Wert ist ein Fehler, der Karten-Hash hängt nicht vom Biom ab. Der Selbsttest prüft,
   dass Preset, Palette und Terrain-Uniforms zum Biom passen.
3. Die drei Biome per `--write-movie` gerendert (Scratchpad, nah / mittel / fern auf
   `test_large`), im Bericht beschrieben und verglichen. Nichts davon ins Repo.
4. Bildrate pro Qualitätsstufe gemessen (`test_large`, 1080p, nah / mittel / fern),
   Tabelle in `docs/ARCHITECTURE.md` und im Bericht.
5. `docs/ARCHITECTURE.md` (Abschnitt „Lighting, atmosphere and biomes“) und
   `docs/MAPS.md` (Feld `biome`) sind aktualisiert.

## 5. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| HDRI-Himmel oder prozedural | `ProceduralSkyMaterial` je Biom | Keine Downloads, keine Lizenzpflege, Farben pro Biom frei einstellbar. Bei 45°–65° Neigung ist der Himmel fast nie im Bild; er dient vor allem als Umgebungslicht und Dunstfarbe. |
| Biom-Namen | `forest`, `desert`, `winter` wie bei Paletten und `EnvLook` | Ein Vokabular in Karte, Paletten und Objekten, keine Übersetzungstabelle. `forest` steht für das gemäßigte Biom (Wiese und Wald). |
| Biom in der Sim | Enum im `Map`, außerhalb des Hashes | Die Sim prüft den Wert beim Laden (Tippfehler fallen früh auf) und reicht ihn weiter; Spielregeln hängen nicht davon ab. Spätere Regeln (Schnee bremst stärker) würden es bewusst in den Hash holen. |
| Bodenfarben je Biom | Schicht-Umdeutung und Tönung im Shader | Keine neuen Texturen und keine anderen Kartendaten; dieselbe Karte ist in jedem Biom spielbar. Eigene Biom-Texturen können später eine Schicht-Quelle werden. |
| Farbkorrektur | `GradientTexture1D` statt 3D-LUT-Datei | Im `.tres` editierbar, ohne Bilddateien; genügt für Split-Toning. |
| Dunst | Tiefennebel relativ zur Kameradistanz | Exponentieller Nebel wäre bei 600 m Zoom überall gleich dicht und würde den Vordergrund vergrauen. |
| Global Illumination | Kein SDFGI/VoxelGI | 4-km-Karte, Kosten und Lichtlecks; SSAO plus Himmelslicht reichen für den Stil. |

## 6. Arbeitsweise

- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine
  Software ohne mein Go installieren. Kein Test wird abgeschwächt.
- Parallel laufen Prompt 013 (Vegetation) und 025 (LAN): Änderungen an `main.gd`,
  `main.tscn`, `justfile` und der Doku klein und additiv halten.
- Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, Bildvergleich, was offen ist.
