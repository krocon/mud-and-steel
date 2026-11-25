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
