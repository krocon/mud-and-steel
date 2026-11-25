# Prompt 218 – Bocage-Landschaft: scharfe Feldparzellen, füllige Hecken/Bäume und bedeckte Atmosphäre

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach Prompt 203
> (Shader-Fixes) und Prompt 205 (Schatten-Passes). Benötigt Blender nur, wenn neue
> Hecken-/Baumvarianten gebaut werden.
> Lies zuerst `CLAUDE.md`, `docs/ASSETS.md`, `docs/MAPS.md` und `docs/ARCHITECTURE.md`
> („Meadow“, „Splatting shader“, Prompt 071, Prompt 088 Felder). Im Code:
> `godot/assets/materials/terrain.gdshader`,
> `godot/assets/materials/terrain_fields.gdshaderinc`,
> `godot/assets/materials/terrain_meadow.gdshaderinc`,
> `godot/assets/shaders/env_foliage.gdshader`,
> `godot/assets/biomes/`,
> `godot/scenes/world/atmosphere.gd`,
> `tools/blender/kit/vegetation.py`,
> `tools/blender/kit/parts_neutral.py` (Hecken),
> `assets-src/blueprints/neutral/env_hedge.toml`,
> `assets-src/blueprints/neutral/env_tree_*.toml`,
> `crates/cli/src/gen_sketch.rs` (oder `crates/mapgen/` falls Prompt 216 erledigt).
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:00–00:06`.
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> **Wichtig:** Wenn nur Optik/Shader/Assets/Skizzen-Parameter geändert werden, bleiben
> Spielregeln & Golden Hashes unverändert!

## 1. Problem

Im Vergleich zur Referenz (Sek. 00:00–00:06) wirken unsere Feld- und Wiesenlandschaften
oft zu weich, zu einheitlich oder zu hell-gesättigt:
1. **Feldparzellen & Wege:**
   - Die Referenz lebt von **messerscharf geschnittenen rechteckigen Parzellen** in klaren,
     ruhigen Naturtönen: helles beige-gelbes Stoppel-/Strohfeld mit feinen, geraden Reihen
     und vereinzelten Halmen, sattes dunkelbraunes gepflügtes Ackerland, zartgrüne
     gestreifte Saatfelder und dunkle, leicht fleckige olivgrüne Wiesen – sauber eingerahmt
     von Hecken.
   - Bei uns verwaschen die 2-m-Splat-Maske (`3×3`-Filter + `warp`) und einheitliche
     Reihenrichtungen die Feldgrenzen; Feldwege neigen bei schrägem Verlauf zu sichtbarer
     Treppenbildung.
2. **Bocage-Hecken & Baumkronen:**
   - Die Referenz nutzt **dicke, voluminöse, knubbelige dunkelgrüne Heckenwälle** (~1,5–2,5 m
     breit, ~1,8–2,2 m hoch) mit kräftigen weichen Kontaktschatten auf dem Gras und klaren
     Durchfahrtslücken für Wege.
   - Die einzelnen Laubbäume wirken durch **dichte, rundliche, tiefgrüne Kronenballen**
     (mit deutlichem Hell-Dunkel-Gefälle von sonniger Oberseite zu dunkler Unterseite) sehr
     körperhaft; im Hintergrund schließt eine **dichte Nadel-/Mischwald-Kulisse** den
     Horizont ab.
3. **Licht & Tiefendunst:**
   - Die Referenz nutzt eine **weiche, leicht bedeckte, entsättigte Westeuropa-Lichtstimmung**
     mit leichtem hellem Bodendunst am Waldrand statt harter knallgrüner Sommerfarben.

## 2. Ziel

1. **Scharfe, kontrastreiche Feldparzellen & weiche Feldwege (`terrain.gdshader`, `terrain_fields.gdshaderinc`):**
   - Für die drei Feldarten (`field_grain`, `field_green`, `field_ploughed`) den `warp` am
     Übergang zu Wiese/Hecken deutlich reduzieren (oder unterdrücken), sodass rechteckige
     Feldparzellen gerade, saubere Kanten behalten – während Naturgrenzen (Wiese/Erde/Weg)
     weich bleiben!
   - **Reihen-Ausrichtung & Farbpalette schärfen:**
     - `field_grain`: warmes, helles Stroh-/Stoppel-Beige mit klaren, regelmäßigen Reihen
       (ohne Moiré beim Bewegen, Regel aus Prompt 071 beachten!) und spärlichen dunklen
       Stoppel-/Kraut-Akzenten.
     - `field_ploughed`: sattes, dunkles Ackerbraun mit gut erkennbaren Furchen.
     - `field_green`: helleres, frisches Grün mit deutlichen Saat-Reihen im Kontrast zur
       dunkleren, fleckig-olivgrünen Wiese (`meadow`).
     - Prüfen, ob die Furchenrichtung (z. B. 0° vs. 90° oder über die vorhandene
       `field_tex` / Parzellen-Hash) zwischen benachbarten Feldern variiert werden kann,
       ohne die Terrain-Textur-Pipeline zu verkomplizieren.
   - Feldwege (`TRACK`): weichere, organisch abgenutzte Ränder mit dunklen Fahrspuren,
     ohne sichtbare 2-m-Treppenstufen.
2. **Voluminöse Bocage-Hecken & vollere Baumkronen:**
   - Prüfen, wie `env_hedge` aktuell aussieht, und entweder die bestehenden Varianten oder
     eine neue Variante (z. B. `Bocage` / `BocageEnd` in `assets-src/blueprints/neutral/env_hedge.toml`,
     Budget einhalten) kräftiger, knubbeliger und dichter gestalten – mit dunklerem,
     voluminöserem Laub und sattem Boden-AO/Kontaktschatten.
   - In `env_foliage.gdshader` / Baum-Varianten prüfen, ob die Kronen-Beleuchtung ein
     kräftigeres Hell-Dunkel-Gefälle (Oberseite warm-olivgrün, Unterseite tief schattig)
     und dichtere Silhouette verträgt, sodass Einzelbäume aus 40–120 m körperhaft und nicht
     „durchlöchert“ wirken.
3. **Licht- & Dunst-Stimmung („Bedeckter Westeuropa-Tag“ / Bocage):**
   - Ein neues (oder verfeinertes) Tageszeit-/Wetter-/Biom-Profil bzw. `atmosphere.gd`-Tuning
     für leicht bedecktes, leicht diesiges Tageslicht (gedämpfte Sättigung, weicher
     Horizont-/Bodendunst vor der Waldkante, volle Wolkenschatten optional).

## 3. Prüfen und zeigen

- **Vorher/Nachher-Bilder** (aus klassischer und Weitwinkel-Perspektive nach Prompt 217):
  - Feldparzellen-Patchwork mit allen 3 Feldarten, Wiese, Feldweg, Bocage-Hecken und
    Waldrand im Hintergrund.
- **Flimmer- & Performance-Check:**
  - `just ground-motion` (Feldreihen & Wiese dürfen beim Schwenken nicht flimmern!).
  - `just objects-bench` & `just terrain-bench` vorher/nachher (`docs/PERFORMANCE.md`).
- Falls Modelle geändert wurden: `just validate-assets`, `just import`, `just thumbnails --only=...`,
  `just units-web-sync`.
- `just check`, `just selftest` grün; Golden Hash unverändert.

## 4. Nicht Teil dieses Prompts

- Neue Geländearten in `crates/sim` (wir nutzen die vorhandenen `field_grain`,
  `field_green`, `field_ploughed`, `grass`, `track`).
- Gebäude (Prompt 219) und Explosionen/Rauch (Prompt 220).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/218-bocage-landschaft-felder-hecken-und-atmosphaere.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis

Umgesetzt am 2026-11-25 (nur Darstellung, `crates/sim` und Golden Hashes unverändert):

- **Scharfe Feldparzellen & weiche Feldwege (`terrain.gdshader`, `terrain_fields.gdshaderinc`, `crates/bridge/src/terrain_layers.rs`):**
  - `field_tex` wird nur noch mit minimalem Warp (`warp * 0.08`) abgetastet und mit enger Rampe (`smoothstep(0.42, 0.58, total)`) eingeblendet → gerade, messerscharfe Rechteckkanten an Parzellen, während Naturübergänge weich bleiben.
  - `field_grain`: warmes Stroh-/Stoppel-Beige mit aus Spielhöhe lesbaren Erntebahnen (`period = 1.40 m`, gefiltert über `field_band`), feinen Drillreihen im Nahbereich, Fahrgassen und spärlichen länglichen Stoppeltupfern (vorläufig).
  - `field_green`: frischere hell- bis mittelgrüne Saat-Reihen (`period = 1.10 m`, 62 % Laubanteil) im Kontrast zur dunkleren Wiese (vorläufig).
  - `field_ploughed`: sattes dunkles Ackerbraun mit mittleren Furchen (`period = 1.25 m`), feinen Nah-Kämmen und breiten Pflugbahnen (vorläufig).
  - Fast quadratische Felder alternieren in `terrain_layers.rs` deterministisch per Schwerpunkt-Hash zwischen 0° und 90°.
  - `TRACK` (Feldweg): globale UV-Rotation um den Kartenursprung (Ursache von Treppen-/Knicklinien auf diagonalen Wegen) und gekreuzte Texturstreifen durch die isotrope `EARTH`-Textur mit weicherem Blend-Fenster (`0.32`) und aus `track.b` abgeleiteten Doppel-Fahrspuren ersetzt.
- **Organische Bocage-Hecken & vollere Baumkronen (`parts_neutral.py`, `env_hedge.toml`, `env_foliage.gdshader`):**
  - `hedge()` baut statt eines Quaders überlappende, abgerundete organische Laub-Ballen (`_hedge_lobe`, Triplanar-UVs in `CARD_SOLID`) mit dichten äußeren `CARD_SHRUB`-Karten; neue Varianten `Bocage` (`3.2 × 2.2 × 2.2 m`), `BocageEnd` und `BocageCorner` mit `kind = 3` (dunkles Bocage-Laub), insgesamt 586/2000 Tris.
  - `env_foliage.gdshader`: etwas vollere Blattfüllung (`leaves`/`leaves_cheap`), Unterstützung für `kind == 3` (`dark`), sowie volumetrisches Kronen-/Hecken-Shading über den gebackenen Höhenanteil `COLOR.g` und `v_rest_normal.y` (sonnige olivgrüne Oberseite, tief schattige Unterseite und starker Boden-AO am Heckenfuß).
- **Bedeckte Westeuropa-Lichtstimmung (`forest_bocage.tres`, `biome_preset.gd`, `meadow_tufts.gd`, `terrain.gd`, `map_objects.gd`):**
  - Neues Look-Preset `godot/assets/biomes/forest_bocage.tres` (`--look=bocage` bzw. `look = "bocage"` in `map.toml`, vorläufig): weiches, leicht bedecktes Licht, entsättigte Palette (`saturation = 0.85`), heller Bodendunst am Waldrand (`haze_amount = 0.42`), dunklere olivgrüne Wiese über das neue optionale `BiomePreset.meadow_tint` (Default weiß, bestehende Biome unverändert) und abgestimmte Feldtöne.
  - Survey-Shots `survey_bocage_fields_classic`, `survey_bocage_fields_wide` und `survey_bocage_farms_wide` in `trailer_shots.gd` ergänzt; Vorher/Nachher-Bilder lokal unter `build/bocage_218/`.
- **Prüfung:**
  - `just validate-env env_hedge` (586/2000 Tris, 6 Varianten), `just import`, `just thumbnails --only=env_hedge`, `just units-web-sync` grün.
  - `just ground-motion`: `column_high grass` Slide 0,14 px/Frame (ncc 4–8 px 0,964), `survey_ground grass` Slide 0,05 px/Frame (ncc 4–8 px 0,959).
  - `just terrain-bench` & `just objects-bench` (high, `test_large`) ohne Regression in `docs/PERFORMANCE.md` dokumentiert.
  - `just check`, `just selftest`, `just validate-assets` grün; Golden Hashes unverändert.
  - **Vorläufig:** exakte Feldreihen-Abstände/-Kontraste, Stoppel-Dichte, `Bocage`-Abmessungen (`2.2 m` Breite) und Farb-/Dunstwerte in `forest_bocage.tres`.
