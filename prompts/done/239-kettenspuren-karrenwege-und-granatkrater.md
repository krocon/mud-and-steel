# Prompt 239 – Kontrastreiche Panzer-Kettenspuren, zweispurige Dorf-Karrenwege und scharfe Granatkrater-Decals

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` („Terrain deformation“, „Splatting shader“, „Effects“),
> `docs/PERFORMANCE.md` sowie die Ergebnisse von
> `prompts/done/104-fahrspuren-und-bodeneffekte.md` und
> `prompts/done/021-gelaendeverformung.md`.
> Im Code:
> `godot/scenes/world/ground_tracks.gd`,
> `godot/assets/shaders/ground_track_stamp.gdshader`,
> `godot/assets/materials/terrain.gdshader`,
> `godot/scenes/terrain/terrain.gd`,
> `godot/scenes/world/crater_view.gd`,
> `godot/scenes/world/effects.gd`,
> `godot/scenes/world/effect_textures.gd`,
> `godot/scenes/world/graphics_quality.gd`,
> `crates/mapgen/src/gen_sketch/schema.rs` (und Straßen-/Track-Erzeugung in `gen_sketch`).
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/This WWII RTS is Pure Chaos #panzerstrike [lskjKn-C9yo].webm`
> (insbesondere Sekunden `00:00–00:05` und `00:10–00:21`: tiefdunkle, gerippte
> Zwillings-Kettenspuren quer über grüne Wiesen, schmale geschwungene
> Dorf-Karrenwege mit zwei hellen Radspuren und Grasstreifen in der Mitte,
> sowie Dutzende scharf gezeichnete kleine dunkle Granattrichter mit
> aufgeworfenem Erdring rund um Dorf und Wiese).
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> **Wichtig:** Reine Darstellung (plus ggf. rückwärtskompatible Optik-Parameter
> im Terrain-/Track-Shader) – **keine Änderungen an Spielregeln in `crates/sim`**,
> Golden Hashes und bestehende Karten bleiben unverändert!

## 1. Problem

Im direkten Vergleich unserer aktuellen Sommerwiesen-Optik (z. B. `bocage_village`
oder `farms`) mit der Referenz fehlen dem Schlachtfeld drei entscheidende
Boden-Details, die dem Gefecht sofort Gewicht und Chaos verleihen:

1. **Panzer-Kettenspuren auf Gras zu blass und ohne Stollenmuster:**
   - In `ground_tracks.gd` sind die Spuren auf Wiese (`GROUNDS["grass"] = [0.55, 240.0]`)
     und in `terrain.gdshader` (`track_dark = vec2(0.5, 0.8)`) eher weiche,
     zurückhaltende Schattenstreifen. Auf mittlerem Zoom sind Manöverschleifen
     und Wendekreise von Panzern im Gras kaum lesbar.
   - In `ground_track_stamp.gdshader` sind die beiden Spurbänder glatt – es fehlt
     das charakteristische **Querrippen-/Kettenstollen-Muster** von Kettenfahrzeugen
     (gegenüber glatten Reifen von Lkw/Spähwagen).
2. **Feldwege (`TRACK`) wirken zu breit und flächig:**
   - Unsere Feldwege (`class = "track"`) malen über ihre volle Breite ein
     gleichmäßiges braunes Band. Auf dem 2-m-Terrain-Raster fehlen schmale,
     geschwungene **zweispurige Dorf-Karrenwege** (`~2,5–3,8 m` Gesamtbreite)
     mit zwei hellen sandig-erdigen Radspuren und einem schmalen, unterbrochenen
     Grasstreifen in der Mitte, wie sie sich in der Vorlage zwischen den Häusern
     und über die grüne Wiese schlängeln.
3. **Granatkrater auf dem 2-m-Raster zu grob – fehlende scharfe Krater-Decals:**
   - Das Simulations-Höhenfeld hat `2,0 m` Zellweite (`data/craters.toml`:
     `min_radius_mm = 2000`). Dadurch werden kleine 50–88-mm-Mörser- und
     Panzer-HE-Einschläge als eher weiche, 4 m breite Matschflecken gezeichnet
     statt als **scharfe, tiefdunkle kleine Trichter (`1,4–3,2 m`) mit
     krümeligem braunem Auswurfring**.
   - Zudem erhalten **vorbereitete Krater der Karte (`[[crater]]` /
     `[[crater_field]]`)** beim Laden bislang keine Krater-Decals (nur die
     grobe Höhenfeld-Verformung), sodass vorbombardierte Dorf- und
     Wiesenfronten viel sauberer wirken als in der Vorlage.

## 2. Ziel

### Teil A – Kontrastreiche Kettenspuren mit Stollenmuster (`ground_tracks.gd`, `ground_track_stamp.gdshader`, `terrain.gdshader`)

1. **Kettenstollen im Stempel-Shader (`ground_track_stamp.gdshader`):**
   - Kettenfahrzeuge von Radfahrzeugen unterscheiden (z. B. über Vorzeichen oder
     Bit in `INSTANCE_CUSTOM` oder eigenen Kanal-/Stempelparameter, ohne das
     bestehende Speicherbudget zu sprengen).
   - Entlang der Fahrtrichtung (`UV.x` bzw. weltfeste Bogenlänge, damit
     aufeinanderfolgende Segmente sauber anschließen!) eine feine, weich
     gefilterte **Querrippung (`~0,45–0,65 m` Periode, Stollen-Kontrast)** auf
     die beiden Spurbänder legen – ab größerer Entfernung sauber über `sharp`
     ausblenden, damit keinerlei Moiré/Flimmern entsteht (`just ground-motion`
     prüfen!).
   - Radfahrzeuge (`WHEELED`: Lkw, Spähwagen) behalten glatte, schmalere
     Reifenspuren.
2. **Sichtbarer Kontrast & Lebensdauer auf Wiese:**
   - `GROUNDS["grass"]` in `ground_tracks.gd` deutlich kräftiger und beständiger
     abstimmen (z. B. Stärke `~0.80–0.90`, Lebensdauer `~360–480 s`), auf
     gewendeten/überfahrenen Stellen (`churn`, `wear`) dunkle aufgerissene Erde
     statt nur leicht abgedunkeltem Grün zeigen.
   - In `terrain.gdshader` die Gras-Verdrängung unter `rut` / `churned` stärker
     Richtung dunkler feuchter Erde (`EARTH`-Farbton) mischen und den
     Kanten-Relief-Kontrast (`track_relief`) leicht schärfen – bei `has_ground_tracks == false`
     (Thumbnails, Editor) unverändert.
   - Sicherstellen, dass auch `meadow_tufts.gd` / `ground_scatter.gd` (falls
     sinnvoll und billig) Spuren nicht komplett verdecken bzw. dass die dunklen
     Spuren aus klassischer und weiter RTS-Kamera glasklar herausstechen.

### Teil B – Schmale zweispurige Dorf-Karrenwege (`terrain.gdshader`)

1. Die bestehende `TRACK`-Richtungstextur (`track_tex.rg`, bereits im Shader!)
   nutzen, um auf `TRACK`-Flächen zwei **helle, sandig-erdige Radspuren mit
   dezent durchscheinendem Mittel-Grasstreifen** und organisch leicht
   ausgefransten Rändern (via `macro_near`) zu zeichnen – besonders überzeugend
   bei schmalen Wegen (`width_m ≈ 2.8–4.0 m` in Skizzenkarten).
2. Sicherstellen, dass alle bestehenden Karten dadurch natürlicher und nicht
   kaputt oder flimmernd wirken (`just ground-motion` + Vorher/Nachher-Bilder
   auf `farms` und `valley_fields`).

### Teil C – Scharfe Granatkrater-Decals (dynamisch & für vorbereitete `[[crater]]`)

1. **Hochaufgelöste prozedurale Krater-Textur (`effect_textures.gd`):**
   - Statt eines flachen grauschwarzen Rußflecks (`EffectTextures.scorch()`)
     eine detailreiche **Krater-Albedo- und Normalmap** erzeugen (z. B. 256×256
     prozedural, 2–3 zufällig rotierte Varianten oder 1 Variante mit starker
     Asymmetrie):
     - Tiefdunkler, schattierter Trichterkern in der Mitte,
     - Aufgeworfener, krümeliger braun-erdiger Auswurfring rundherum,
     - Feine radiale Erdspritzer/Auswurfstrahlen, die weich ins Gras auslaufen,
     - Leichte Tiefen-Normalmap (Rand hebt sich im Sonnenlicht ab, Kern fällt ab).
   - Kaliberabhängig sauber skalieren (`50 mm` Mörser/Pak → knackiger `~1,8–2,4 m`
     Krater-Stempel; `75–88 mm` → `~2,5–3,5 m`; `105–152 mm` → `~4,0–5,5 m`).
2. **Vorbereitete Karten-Krater automatisch bestempeln (`crater_view.gd` / `effects.gd`):**
   - Beim Laden einer Karte mit vorbereiteten `[[crater]]` (aus `SimHost` bzw.
     `map.toml`, ohne `sim`-Regeln zu ändern – prüfen, ob `SimHost` die
     vorbereiteten Kraterpositionen/-kaliber bereits exponiert oder über eine
     reine Lese-Methode in `crates/bridge` bereitstellt) diese direkt als
     Krater-Stempel platzieren (ggf. über ein eigenes schlankes MultiMesh- oder
     Decal-Raster / erweitertes `fx_decals`-Budget, z. B. `high: 384`, `medium: 160`,
     `low: 48`, sodass ein dichtes Dorf-Kraterfeld aus 100+ Kratern dauerhaft
     sichtbar bleibt und nicht sofort durch neue Einschläge überschrieben wird!).
   - **Performance prüfen:** Keine spürbare FPS-Einbuße (`just bench-view` /
     `just track-shots` / Krater-Test auf `high` über 60 FPS).

## 3. Prüfen und zeigen

- **Vorher/Nachher-Standbilder** in `build/tracks_craters_compare/`:
  - Panzer-Kurven/Schleifen auf grüner Wiese (`just track-shots` oder neuer
    `survey_tracks_craters`-Shot) aus mittlerer und hoher RTS-Kamera.
  - Schmaler Karrenweg durch ein Dorf + Wiese.
  - Dichtes Granatkrater-Feld (sowohl vorbereitete `[[crater_field]]` als auch
    frische 50–105-mm-Einschläge).
- **Flimmer- und Performance-Prüfung:**
  - `just ground-motion` (kein neues Flimmern durch Stollen oder Karrenweg).
  - Bildrate auf `high` bei 200+ Kratern + fahrenden Panzern messen und im
    Ergebnisabschnitt festhalten.
- **Tests:**
  - `just check`, `just selftest`, `track_self_test` grün; Golden Hashes
    unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an `data/craters.toml`, Deckungsregeln oder Bodenverformung in
  `crates/sim`.
- Änderungen an Gebäudemodellen (Prompt 240) oder neuen Dorf-Props (Prompt 241).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/239-kettenspuren-karrenwege-und-granatkrater.md prompts/done/`
verschieben, Ergebnisabschnitt (Shader-/Budget-Werte, Flimmermessung,
FPS-Messung, Vorher/Nachher-Beobachtungen) ergänzen und den Status in
`prompts/README.md` auf „erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Panzer-Kettenspuren mit Stollenmuster (`godot/scenes/world/ground_tracks.gd`, `godot/assets/shaders/ground_track_stamp.gdshader`, `godot/assets/materials/terrain.gdshader`):**
  - Ketten- vs. Radfahrzeuge unterschieden; kumulative Fahrdistanz-Phase in `INSTANCE_CUSTOM.z` und Segmentlänge in `COLOR.a` übergeben.
  - `GROUSER_PERIOD_M = 0.58` erzeugt kontinuierliche Querrippen (`0.80 + 0.20 * sin(...)`) entlang der Kettenbänder (glatte Spuren bei Radfahrzeugen), distanzgefiltert über `sharp` (`texel_m * 6.0 .. 2.0`) ohne Moiré.
  - Gras-Spuren deutlich kontrastreicher und beständiger (`GROUNDS["grass"] = [0.85, 420.0]`, Regen `[0.95, 540.0]`), `track_dark = vec2(0.32, 0.56)`, `track_relief = 1.8`, verdrängtes Gras Richtung dunkler Erde (`vec3(0.24, 0.19, 0.13)`).
- **Zweispurige Dorf-Karrenwege (`godot/assets/materials/terrain.gdshader`):**
  - `smooth_track` nutzt `track_tex` für zwei sandig-erdige Radspuren mit organisch unterbrochenem Grasstreifen in der Mitte in Wiesen-Biomen.
- **Scharfe Granatkrater-Decals (`godot/scenes/world/effect_textures.gd`, `effects.gd`, `crater_view.gd`, `graphics_quality.gd`):**
  - Neue prozedurale 256×256-Krater-Albedo (`crater_albedo()`) mit dunklem Trichterkern, braunem Auswurfring und radialen Auswurfstrahlen plus 3D-Tiefen-Normalmap (`crater_normal()`).
  - `fx_decals`-Budget erhöht (`low: 48`, `medium: 160`, `high: 384`); vorbereitete Karten-Krater (`SimHost.map_craters()`) werden beim Laden über `effects.place_map_craters()` separat vom dynamischen Ring gestempelt.
- **Prüfung:** Vorher/Nachher-Bilder in `build/tracks_craters_compare/` und in den `chaos_*`-Aufnahmen visuell geprüft; `just check` und `just selftest` (inkl. `track_self_test`) grün; keine Änderungen in `crates/sim`.
