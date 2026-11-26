# Prompt 223 – Herbstlandschaft: mehrfarbige Herbstbäume, Herbstfelder, Bodennebel und trübe Nieselregen-Stimmung

> In einer frischen Session im Repo-Ordner ausführen. Baut auf Prompt 218
> (Bocage-Felder & Atmosphäre) und Prompt 222 (Umland um die Karte) auf.
> Benötigt Blender nur, falls optional eine spärlich belaubte Herbst-Baumvariante
> ergänzt wird – das Kernziel läuft vollständig über Shader & Look-Preset.
> Lies zuerst `CLAUDE.md`, `docs/ASSETS.md`, `docs/MAPS.md`, `docs/ARCHITECTURE.md`
> („Lighting, atmosphere and biomes“, „Meadow“, „Splatting shader“, „Map surround“)
> sowie die Ergebnisse von `prompts/done/218-bocage-landschaft-felder-hecken-und-atmosphaere.md`
> und `prompts/done/222-umland-um-die-karte.md`. Im Code:
> `godot/project.godot` (`[shader_globals]`),
> `godot/assets/shaders/env_common.gdshaderinc`,
> `godot/assets/shaders/env_foliage.gdshader`,
> `godot/assets/shaders/env.gdshader` (`slot == 10` Rinde),
> `godot/assets/shaders/env_look.gd`,
> `godot/assets/biomes/biome_preset.gd`,
> `godot/assets/biomes/forest_bocage.tres`,
> `godot/assets/biomes/forest_mud.tres`,
> `godot/scenes/world/atmosphere.gd`,
> `godot/scenes/world/weather_look.gd`,
> `godot/scenes/world/puddles.gd` (`MIST_SHADER`),
> `godot/scenes/world/meadow_tufts.gd`,
> `godot/assets/shaders/meadow_palette.gdshaderinc`,
> `godot/assets/materials/terrain_meadow.gdshaderinc`,
> `godot/assets/materials/terrain_fields.gdshaderinc`,
> `godot/scenes/terrain/map_surround.gd`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:20–00:30`.
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> **Wichtig:** Reine Darstellung – **keine Änderungen in `crates/sim`**,
> Spielregeln & Golden Hashes bleiben unverändert!

## 1. Problem

Die Referenz (Sek. 00:20–00:30) lebt von einer eindringlichen, nasskalten
Spätherbst-Stimmung in Nordwesteuropa, die unsere bisherigen Biome/Looks
(`forest`, `forest_bocage`, `forest_mud`, `winter`) so noch nicht abbilden:

1. **Mehrfarbiger Herbstwald statt einheitlichem Grün:**
   - Aktuell färbt `EnvLook.apply_global()` alle Laubbäume über einen einzigen
     globalen Farbwert `env_foliage` einheitlich grün.
   - In der Referenz bildet jeder Wald und jede Baumgruppe ein **kontrastreiches
     Mosaik aus mehreren Herbstfarben von Baum zu Baum**: leuchtendes Goldgelb /
     Blassgelb, Senf-/Oliv-Ocker, warmes Kupfer-Orange, Rostrot und tiefes
     Karmesin-/Weinrot, dazwischen einzelne **kahle oder spärlich belaubte Bäume**
     mit dunklen, nassen Stämmen und fein verästelten schwarzen Zweigen.
   - Die Baumkronen zeigen ein klares **Hell-Dunkel-Gefälle** (sonnige Oberseite
     leuchtet gelb/orange, Unterseite und Schattenkern fallen tief rostbraun/dunkelrot
     ab), und die dunklen Stämme heben sich markant vom hellen Laub ab.
2. **Herbstliche Wiesen & Felder:**
   - Statt frischem Sommergrün zeigt der Boden **abgestorbenes, feuchtes
     Oliv-Braun-Ocker** (ohne Frühlingsblumen) mit abgefahrenen, helleren
     Spuren und dunklen feuchten Senken.
   - Die scharf geschnittenen Feldparzellen (Prompt 218) stehen im harten
     Spätherbst-Kontrast: **tiefe, feucht glänzende, dunkelbraune gepflügte
     Äcker** (`field_ploughed`) neben **fahlen, abgeernteten sandig-beigen
     Stoppel-/Brachfeldern** (`field_grain`) mit spärlichen dunklen Stoppeln
     und matten, späten Grünstreifen (`field_green`).
3. **Feucht-trübe Herbst-Stimmung, Bodennebel & feiner Nieselregen:**
   - Über dem Waldboden links vorne und um das Dorf liegt ein **flacher,
     kühl-bläulichgrauer Bodendunst / Nebelschleier**, der die dunklen Stämme
     vom Untergrund löst und dem Hintergrund Tiefe gibt.
   - Durch das gesamte Bild fallen **feine, schräge Nieselregen-Striche**;
     der Boden schimmert leicht feucht (`wet_ground`). Bisher ist Regen in
     `weather_look.gd` starr an die regelwirksame Wetterstufe `weather = "rain"`
     gebunden (die Sichtweite, Bodensoftness und Fahrgeschwindigkeit in `sim`
     ändert), während es für Schnee bereits das rein optische `BiomePreset.snowfall`
     gibt.

## 2. Ziel

1. **Instanzabhängige Herbstfärbung im Laub-Shader (0 extra Draw-Calls, 0 neue Meshes):**
   - Neuen globalen Shader-Uniform `env_autumn` (`float`, Default `0.0`) in
     `godot/project.godot`, `env_common.gdshaderinc`, `env_look.gd` und
     `BiomePreset` (`autumn_foliage: float = 0.0`) einführen – bei `0.0` bleiben
     alle bestehenden Biome und Looks **Bit-für-Bit unverändert**.
   - In `env_foliage.gdshader` bei `env_autumn > 0.0`:
     - Für Laub-Muster (`pattern == 0`, `5`, `7`, `8` – also Kronen, Büsche,
       Hecken und `_Far`-Laubkronen) pro Objekt-Instanz **deterministisch aus
       `MODEL_MATRIX[3].xyz`** (funktioniert automatisch in `MultiMesh` und im
       Umland `map_surround.gd`!) eine Farbe aus einer abgestimmten 5-stufigen
       Herbst-Palette wählen:
       1. Blass-/Goldgelb,
       2. Senfgelb / Oliv-Ocker,
       3. Warmes Kupfer-Orange,
       4. Rostrot,
       5. Tiefes Herbst-/Weinrot.
     - Innerhalb einer Krone über `COLOR.b` / `v_tint` leicht modulieren, sodass
       einzelne Äste minimal wärmer/heller oder dunkler ausfallen, ohne den
       Baum unruhig scheckig zu machen.
     - Das volumetrische Kronen-Shading (Prompt 218) für Herbstlaub harmonisch
       abstimmen: warme leuchtende Kronenoberseite, tief schattige Unterseite.
     - **Immergrüne Pflanzen bleiben grün:** Nadelbäume (`pattern == 1`, `9`, `12`),
       Palmen (`2`), Sukkulenten (`11`) und Weinreben (`13`) werden von `env_autumn`
       nicht umgefärbt (allenfalls minimal abgedämpft).
     - Optional prüfen: bei `env_autumn > 0.5` eine sanfte Ausdünnung einzelner
       Blattcluster oder das Mischen von `env_tree_deciduous:Bare` (existiert
       bereits für Winter!) als kahle Herbstbäume, damit die dunklen Astgabeln
       zwischen den bunten Kronen sichtbar werden.
   - **Dunkle, feuchte Herbst-Rinde (`env.gdshader`, `slot == 10`):**
     - Wenn `env_autumn > 0.0`, Rinde und Äste etwas dunkler und leicht feuchter
       zeichnen, damit Stämme und Zweige sich wie in der Referenz kräftig vom
       goldgelben/roten Laub abheben.
   - **Umland-Konsistenz (`map_surround.gd`):**
     - Sicherstellen, dass `_Far`-Bäume im Umland über das globale `env_autumn`
       ebenfalls bunt herbstlich erscheinen (und bei Herbst-Look die Laubbaum-
       und `Bare`-Gewichte im Umland stimmig sind, ohne Schatten-Mehrkosten).

2. **Neues Herbst-Look-Preset `godot/assets/biomes/forest_autumn.tres` (`--look=autumn` / `look = "autumn"`):**
   - **Wiese & Streu (`meadow_tint`, `meadow_tufts.gd`, `tall_grass.gd`, `ground_scatter.gd`):**
     - Entsättigtes, feuchtes Herbstgras in Oliv-Braun-Ocker, `flower_density = 0.0`,
       leicht glänzender feuchter Boden (`wet_ground ≈ 0.30–0.42`), abgestimmte
       Farben für Wiesenbüschel, hohes Gras und Bodenstreu (keine grellgrünen
       Grasbüschel auf brauner Herbstwiese!).
   - **Herbst-Felder (`field_grain`, `field_leaf`, `field_soil`, `field_earth`):**
     - `field_ploughed`: sattes, tiefes feuchtes Herbst-Ackerbraun (`field_earth`),
       ggf. in `terrain.gdshader` / `terrain_fields.gdshaderinc` prüfen, ob
       `wetness` auf gepflügtem Acker einen dezenten Feuchteglanz erzeugt.
     - `field_grain`: fahles, abgeerntetes beige-sandfarbenes Stoppel-/Brachfeld
       mit spärlichen dunklen Stoppel-/Unkraut-Tupfern.
     - `field_green`: gedämpftes, mattes Spätherbst-Grün.
   - **Licht, Grading & Tiefendunst:**
     - Bedeckt-kühles, leicht flaches Spätherbstlicht (weiche Schatten, gedämpfte
       Sättigung `saturation ≈ 0.78–0.86`, kühlgrauer Horizontdunst
       `haze_amount ≈ 0.45–0.55`, dezente Vignette `~0.28`), sodass die warmen
       gelb-roten Herbstkronen und orangefarbenen Explosionsblitze (Prompt 220)
       stark herausstechen.

3. **Rein optischer Nieselregen & Waldboden-Nebel (`biome_preset.gd`, `atmosphere.gd`, `weather_look.gd`, `puddles.gd`):**
   - **Feiner Nieselregen (`BiomePreset.rainfall: float = 0.0`, analog zu `snowfall`):**
     - Bei `rainfall > 0.0` (und nicht bereits `weather == "rain"`, um doppelte
       Regen-Emitter zu vermeiden) erzeugt `atmosphere.gd` bzw. `weather_look.gd`
       feine, schräg fallende, dezente Nieselregen-Striche rund um den Kamerafokus
       (schmaler und transparenter als Starkregen, respektiert `GraphicsQuality`
       inkl. Aus auf `Low`).
     - Spielregeln für `weather = "rain"` (`data/weather.toml`, Bodensoftness,
       Sichtweite) bleiben unangetastet!
   - **Herbst-Bodennebel (`puddles.gd` `MIST_SHADER`):**
     - Prüfen, ob `mist_<n>`-Marker in `puddles.gd` ihren Farbton/Dichte aus dem
       Biom-/Look-Preset übernehmen können (`BiomePreset.mist_tint` / Default wie
       bisher), sodass auf einer Herbstkarte ein flacher, kühl-bläulichgrauer
       Dunstschleier zwischen den Stämmen im Wald und rund um das Dorf schwebt,
       ohne harte Schnittkanten am Gelände.

## 3. Prüfen und zeigen

- **Vorher/Nachher-Standbilder** (klassische und Weitwinkel-Perspektive aus
  Prompt 217, z. B. auf `bocage_village` oder `farms` mit `--look=autumn` sowie
  ein Survey-Shot in `trailer_shots.gd`):
  - Dichter Laubwald aus der Nähe und aus Spielhöhe (klare Mehrfarbigkeit:
    Goldgelb, Ocker, Orange, Rostrot, Tiefrot + dunkle Stämme/Äste + einzelne
    kahle Bäume + bläulicher Waldboden-Dunst).
  - Feldparzellen-Patchwork mit dunklem gepflügtem Herbst-Acker, hellem
    Stoppelfeld, bräunlicher Herbstwiese, Nieselregen-Strichen und Herbst-Umland
    am Kartenrand.
  - Vergleichsbild auf einer Standard-`forest`- und `forest_bocage`-Karte:
    unverändert (kein Farbshift bei `env_autumn == 0.0`).
- **Flimmer- & Performance-Check:**
  - `just ground-motion` (keine flimmernden Feldreihen/Wiesen).
  - `just terrain-bench`, `just objects-bench` und `just bench-view --perf-quick --perf-edge --perf-dof=off`
    vorher/nachher (`docs/PERFORMANCE.md`, Ziel: keine messbare Regression auf
    Standard-Karten, Herbst-Look im Budget).
- `just check`, `just selftest` grün; Golden Hashes unverändert.
- `docs/ARCHITECTURE.md` (Abschnitt „Lighting, atmosphere and biomes“ / `look`)
  knapp nachziehen.

## 4. Nicht Teil dieses Prompts

- Neue Geländearten oder Jahreszeiten-Regeln in `crates/sim`.
- Treppengiebel-Häuser (Prompt 224) und aufsteigende Artillerie-Rauchbögen
  (Prompt 225).
- Bau der finalen Herbst-Karte und 1080p-Videoaufnahme (Prompt 226).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/223-herbstlandschaft-baeume-felder-und-stimmung.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Globaler Herbst-Parameter & Look-Preset:** `env_autumn` (`godot/project.godot`, `env_common.gdshaderinc`, `env_look.gd`), `Look::Autumn` (`crates/sim/src/map.rs`), `BiomePreset`-Felder (`autumn_foliage`, `rainfall`, `mist_tint`, `mist_density`) und neues Preset `godot/assets/biomes/forest_autumn.tres` (`--look=autumn` / `look = "autumn"`).
- **Weiche, flauschige Herbstkronen & Vegetation:** Instanzabhängige 5-Stop-Herbstpalette (Buttergelb, Ocker, Terrakotta-Orange, Rostrot, warmes Karmesin) mit geschlossenem, weichem Laubballen-Kern, feinem Punkt-Blattrand und geglätteten Kronen-Dome-Normalen in `env_foliage.gdshader` (nur bei `env_autumn > 0`, übrige Biome unverändert), abgedunkelte feuchte Rinde in `env.gdshader`, herbstliches Boden-Scatter in `ground_scatter.gdshader` und herbstlicher Umland-Baummix in `map_surround.gd`.
- **Nieselregen & Waldboden-Nebel:** Rein optischer Nieselregen (`_build_drizzle()` in `atmosphere.gd`, ohne Gameplay-Einfluss) und abgetönte Bodennebel-Flächen über `puddles.apply_biome()` (`puddles.gd`, `main.gd`).
- **Verifikation:** `just trailer-shots --only=survey_autumn_fields_wide,survey_autumn_wood_wide,survey_bocage_fields_wide --stills` (Bocage unverändert, Herbst & weiche Kronen geprüft), `just check` und `just selftest` grün. Werte vorläufig.
