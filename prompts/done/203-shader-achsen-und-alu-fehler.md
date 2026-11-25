# Prompt 203 – Shader-Fehler beheben, Fragment-ALU sparen und Material-Relief schärfen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> dann `docs/ASSETS.md` (Abschnitte „Material system“, „Materials and shaders“)
> und `docs/PERFORMANCE.md`. Im Code:
> `godot/assets/shaders/env.gdshader`,
> `godot/assets/shaders/env_common.gdshaderinc`,
> `godot/assets/shaders/env_roof.gdshaderinc`,
> `godot/assets/shaders/env_surface.gdshaderinc`,
> `godot/assets/shaders/unit_camo.gdshader`,
> `godot/assets/shaders/soldier.gdshader`,
> `godot/assets/shaders/cloud_shadow.gdshaderinc`,
> `godot/assets/materials/terrain.gdshader`,
> `godot/assets/materials/terrain_fields.gdshaderinc`,
> `godot/assets/materials/water.gdshader`.
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hash bleibt unverändert.

## 1. Problem

Eine Code- und Render-Analyse hat mehrere konkrete Shader-Fehler, vermeidbare
ALU-Kosten pro Pixel sowie optische Schwächen (Flimmern, fehlendes Relief,
fehlerhafte PBR-Werte) offengelegt:

1. **Z-up- statt Y-up-Verwechslung in `env.gdshader`:**
   Godot-glTF-Importe wandeln Blender-Z-up in Godot-Y-up um (`VERTEX.y` ist Höhe).
   - Zeile 239 (`slot == 3`, Holz): `float u = abs(wn.y) < 0.7 ? (p.x + p.y) * 5.0 : p.x * 5.0;`
     addiert X + Höhe und erzeugt **45°-Diagonalstreifen** auf vertikalen Holzwänden
     (sichtbar am Giebel in `godot/assets/thumbnails/base_barracks.png`).
   - Zeilen 262–272 (`slot == 10`, Rinde): Birke (Zeile 262, 264), Palme (266),
     Kiefer (269, 270) und Eiche (272) verwenden `p.z` als vertikale Achse statt `p.y`.
     Folge: Die rötliche obere Kiefernrinde (`smoothstep(8.0, 20.0, p.z)`) greift nie,
     der dunkle Birkenfuß dunkelt die Nord-/Südseite statt den Stammfuß ab, Palmringe
     laufen vertikal statt horizontal.
   - Zeile 293 (`slot == 12, kind == 2`, Beton): Schalungsfugen (`fract(p.z * 1.6)`)
     laufen vertikal statt horizontal.
   - Zeile 247: Toter Ausdruck `albedo = mix(albedo, ..., env_dry * 0.0);`.
2. **Variable-Shadowing in `terrain_fields.gdshaderinc` (Zeilen 48 & 55):**
   Der Funktionsparameter `float fine` (Macro-Noise aus `terrain.gdshader:370`) wird
   in Zeile 48 lokal durch `float fine = field_band(u, 0.25, 0.06, px);` überschattet.
   Zeile 55 multipliziert daher die 25-cm-Drillreihen doppelt statt der Getreide-Mottling-Variation.
3. **Unbegrenzter `TIME`-Drift in `water.gdshader` (Zeile 131):**
   `(p - v_flow * TIME * 0.6)` verliert in langen Partien Float-Präzision und schert
   bei variierendem `v_flow`, statt die bereits berechneten Zwei-Phasen-Shifts
   `fshift0`/`fshift1` zu nutzen (Zeilen 125–126 berechnen diese außerdem doppelt zu
   `shift0`/`shift1` in Zeilen 87–88).
4. **Verschwendete 3D-Noise- & Licht-ALU:**
   - `env_common.gdshaderinc:127-131` (`snow_cover`) rechnet 4-Oktaven-`fbm3` (32 Hashes/Pixel)
     auch wenn `amount == 0.0` (Wald, Wüste, Mittelmeer).
   - `env.gdshader:195-196` rechnet `big = fbm3(p * 0.7)` und `grain = vnoise(p * 6.0)`
     pauschal für alle Slots, obwohl viele Slots `big` nicht nutzen; Zeilen 398–406
     berechnen `sin()`/`vnoise()` für Tonziegel-`bump_h` doppelt zu `roof_clay()`;
     Zeile 443 führt `surface_bump()` (4 Derivate + 2 Kreuzprodukte) auch bei `bump_h == 0.0` aus.
   - `unit_camo.gdshader:172, 181` rechnet `fbm()` für Schlamm und Schnee auch bei `mud == 0.0` / `snow == 0.0`.
   - `soldier.gdshader:137-138` rechnet hochfrequentes `noise(p * 40.0)` auch bei `detail == 0.0`.
   - `cloud_shadow.gdshaderinc:58-91` ruft `cloud_sun()` (2 Texturzugriffe) auch bei
     `ATTENUATION <= 0.0` oder `c_ndotl <= 0.0` auf, führt GGX-Specular auch auf
     verschatteten/abgewandten Pixeln aus und klemmt `alpha_ggx` nicht nach unten
     (NaN-Gefahr bei Pfützen/Glas). Zudem ruft `terrain.gdshader:350` `cloud_sun()` im
     Fragment-Shader und `cloud_shadow.gdshaderinc:61` ein zweites Mal im `light()`-Shader auf.
5. **PBR-Fehler, Flimmern & fehlendes Relief:**
   - Glas hat `metallic = 0.3–0.4` (`unit_camo.gdshader:146`, `env.gdshader:171, 256`)
     statt Dielektrikum (`metallic = 0.0`, höheres `SPECULAR`).
   - Vertex-AO wird sowohl in `ALBEDO` multipliziert als auch an `AO` + `AO_LIGHT_AFFECT`
     übergeben (verfälscht bei `Metal` den Fresnel-Wert `F0` und dunkelt doppelt ab).
   - Harte `step()`-Aufrufe ohne `fwidth`-Ausblendung in `unit_camo.gdshader:99, 164`
     und `env.gdshader:176-182, 219, 240, 262, 293, 104-107, 169` begünstigen Flimmern.
   - `unit_camo.gdshader` hat gar kein Normal-Relief (glatte Panzerplatten/Ketten), und
     `env.gdshader` nutzt sein vorhandenes `surface_bump()` nicht für `Rock`, `Bark`, `Wood`, Wellblech.

## 2. Ziel

1. **Bugs beheben:**
   - `env.gdshader`: Holzplanken-Koordinate auf `(p.x + p.z) * 5.0` korrigieren; Rinden-
     und Beton-Höhenachse auf `p.y` umstellen (und Noise-Anisotropie entsprechend auf Y
     strecken, z. B. `vec3(8.0, 1.5, 8.0)`); toten `env_dry * 0.0`-Aufruf bereinigen.
   - `terrain_fields.gdshaderinc`: lokale Variable in Zeile 48 umbenennen (z. B. `drill`),
     sodass Zeile 55 den echten `fine`-Parameter nutzt.
   - `water.gdshader`: Schaum-Sampling auf die periodischen Flow-Phasen umstellen und
     doppelte Shift-Berechnung zusammenlegen.
2. **ALU einsparen:**
   - Frühzeitige Guards (`if (amount <= 0.0) return albedo;`, `if (mud > 0.001)`,
     `if (snow > 0.001)`, `if (abs(bump_h) > 1e-6)` bzw. Bool-Flag `has_bump`).
   - `big`/`grain` in `env.gdshader` nur in Slots berechnen, die sie brauchen;
     `roof_clay()` gibt `bump_h` per `out`-Parameter zurück.
   - `cloud_shadow.gdshaderinc`: `cloud_sun()` und Specular-GGX hinter
     `ATTENUATION > 0.0` / `c_ndotl > 0.0` (unter Beachtung von `CLOUD_BACKLIGHT`) gaten;
     `alpha_ggx = max(roughness * roughness, 0.002)`.
   - `soldier.gdshader`: `grain` nur bei `detail > 0.001` berechnen.
3. **PBR, Anti-Aliasing und Relief verbessern:**
   - Glas als Dielektrikum (`metallic = 0.0`, `SPECULAR = 0.75`, `rough = 0.06`).
   - Doppelte AO-Anwendung auf `ALBEDO` prüfen und sauber auf `AO` + `AO_LIGHT_AFFECT`
     (oder zumindest auf Metall-Slots ohne `ALBEDO`-Abdunklung) umstellen, ohne dass
     Fahrzeuge/Häuser insgesamt zu hell oder kontrastarm wirken (Vorher/Nachher vergleichen!).
   - `fwidth`-basiertes Ausblenden hochfrequenter `step()`-Muster in `unit_camo.gdshader`
     und `env.gdshader` ergänzen.
   - Dezentes, `fwidth`-gefiltertes Normal-Relief (`surface_bump`-Muster) für
     `unit_camo.gdshader` (Kettenstege, Schlammkanten, Lackschäden) sowie in `env.gdshader`
     für `Rock` (Risse/Schichtung), `Bark` (Rindenfurchen), `Wood` (Bretterfugen) und Wellblech.

## 3. Prüfen und zeigen

- **Vorher/Nachher-Bilder:**
  - `base_barracks` (Giebelwand ohne Diagonalstreifen!), Birken-, Kiefern-, Palmen- und
    Eichenstämme (`env_tree_*`), Betonbunker (`fort_casemate_gun`), Getreidefelder,
    Panzer (`de_pz4_h`, `us_m4` in Wald/Wüste/Winter), Wasser nach längerer Laufzeit.
- **Flimmer- & Performance-Messung:**
  - `just ground-motion` ohne Verschlechterung.
  - `just bench-view --perf-quick` und `just objects-bench city_large --blocks-bench`
    vorher/nachher direkt hintereinander messen und Ergebnisse in `docs/PERFORMANCE.md` festhalten.
- **Thumbnails & Webseite:**
  - Betroffene Thumbnails (mindestens geänderte Holz-/Baum-/Beton-Objekte wie `base_barracks`,
    `env_tree_*`) mit `just thumbnails --only=...` neu rendern und `just units-web-sync` ausführen.
- `just check`, `just selftest` grün; Golden Hash unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an 3D-Meshes oder Blender-Generatoren (folgt in Prompts 213–215).
- Schatten-Pass-Umbau für Gras/Streuschicht (Prompt 205).
- Zusammenführen von Material-Slots über `UV2` (Prompt 207).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/203-shader-achsen-und-alu-fehler.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (2026-11-25)

**Fehler behoben**

- `env.gdshader`: Bretter stehen auf Wänden senkrecht (`p.x + p.z`), die Maserung läuft
  längs der Bretter; Rinde, Palmringe, Birkenmarken und Schalungsfugen nutzen `p.y`.
  Die Birke ist jetzt weiß mit dunklen Quermarken und dunklem Fuß (vorher überall die
  Fußfarbe), die Kiefer wird ab 8 m rötlich. Toter `env_dry * 0.0`-Ausdruck entfernt.
- `terrain_fields.gdshaderinc`: lokale Variable heißt `drill`; das Getreide ist jetzt
  im Meterbereich gefleckt (Bild `06_felder_110m.jpg`).
- `water.gdshader`: Schaumstreifen laufen auf den zwei Flow-Phasen der Normalen, die
  doppelte Shift-Berechnung ist weg.

**ALU**

- `snow_cover()` kehrt ohne Schnee sofort zurück; `big`/`grain` nur in Slots, die sie
  nutzen; `roof_clay()`/`roof_slate()` geben ihr Relief per `out` zurück;
  `surface_bump()` rechnet bei flacher Höhe nichts (Ableitungen vorher genommen).
- `unit_camo`: Schlamm, Staub, Schnee nur bei Werten > 0.001. `soldier`: Körnung nur
  bei `detail > 0`, Fleckung nur in Slots, die sie nutzen.
- `cloud_shadow.gdshaderinc`: `light()` tut für Pixel im Schatten oder von der Lampe
  abgewandt nichts (mit `CLOUD_BACKLIGHT` nur der Schatten-Fall), `alpha_ggx` hat die
  Untergrenze 0.002, `cloud_sun()` liest Mip 0. Terrain: `CLOUD_SUN_IN_FRAGMENT`, ein
  Wolken-Lookup statt zwei.

**PBR, Filter, Relief**

- Glas: `metallic 0`, `SPECULAR 0.75`, Rauheit 0.06 (auch Sprossenfenster; in
  `web/units/viewer.js` nachgezogen).
- AO: `ALBEDO` bleibt unverdunkelt, dafür `AO = ao * mix(0.45, 1, ao)` mit
  `AO_LIGHT_AFFECT 0.6` (Einheiten 0.5 / 0.55). Die Faktoren sind so gewählt, dass die
  Helligkeit der alten Formel herauskommt; in den Vergleichsbildern sind Panzer und
  Häuser gleich hell (`03_panzer.jpg`, `05_wueste_winter.jpg`).
- Neue Datei `relief.gdshaderinc` (`aa_step`, `aa_line`, `pattern_fine`,
  `surface_bump`). Gefiltert: Ambush-Punkte, Lackflecken, Kettenglieder, Fugen
  (`bond()` box-gefiltert), Brettfugen, Wellblechrippen, Birkenmarken, Schalungsfugen,
  Felsrisse, Vorhangfalten.
- Relief: Kettenglieder, Schlamm, Lackschäden; Fels (Risse, Schichtung, Körnung),
  Rinde, Brettfugen, Quaderfugen, Schalungsfugen, Wellblech.

**Entscheidungen über den Prompt hinaus**

- Die Brettfugen waren invertiert (Bretter auf 65 % abgedunkelt, Fugen hell). Jetzt
  dunkle Fugen; die Holzfarben sind mit 0.68 skaliert, damit das Holz gleich hell bleibt.
- Wellblechrippen laufen jetzt wie die Blechnähte (`rf.uv.x`), vorher quer dazu.
- Ziegel ohne Bauplan-Oberfläche laufen über `surface_brickwork()` (gleiches Bild,
  aber mit Ausblendung); Quader-Töne folgen dem Verband.
- `just thumbnails` startet mit `--always-on-top` (ein verdecktes Fenster ließ den
  Lauf hängen); neu `--biome=` und `--raw-scale=` für große Vergleichsbilder.
  `SHOT_TIME=<s> just terrain-shots` spult die Shader-Uhr vor.
- `tools/ground_flicker.py`, `tools/shadow_flicker.py`: `-vsync 0` durch
  `-fps_mode passthrough` ersetzt (ffmpeg 9 kennt die Option nicht mehr).

**Geprüft**

- `just check` und `just selftest` grün, Golden Hash unverändert (keine Änderung in
  `crates/sim`).
- `just bench-view --perf-quick` und `just objects-bench city_large --blocks-bench`,
  alt/neu abwechselnd je zweimal: kein Unterschied außerhalb der Streuung (±0.3 ms),
  Tabelle in `docs/PERFORMANCE.md`. Die Ansichten sind CPU-begrenzt; ein GPU-begrenzter
  Fall wurde nicht gemessen.
- `just ground-motion` und `--roofs`, alt/neu: Boden identisch, Dächer gleich bzw. bei
  110 m minimal besser. Wände, Stämme und Fahrzeuge misst das Werkzeug nicht.
- Alle 409 Thumbnails neu gerendert, `just units-web-sync` ausgeführt.
- Vergleichsbilder: `build/prompt-203/` (nicht im Repo).

**Offen**

- Wasser: In den Standbildern (Furt und Fluss auf `test_small`, Uhr 30 min vorgespult)
  ist zwischen alt und neu kein Unterschied zu sehen; die Streifen liegen dort im
  Baumschatten und sind schwach. Die Korrektur ist also nur im Code belegt.
- Flimmern an Wänden und Fahrzeugen in Bewegung ist nicht gemessen, nur an Standbildern
  beurteilt.
