# Prompt 219 – Normandie-/Bocage-Dorfhäuser und Dorfkirche verfeinern

> In einer frischen Session im Repo-Ordner ausführen. Benötigt Blender (`docs/VERSIONS.md`).
> Empfohlen nach Prompt 203 (Shader-Fixes) und idealerweise nach/mit Prompt 215 (`sash_window`).
> Lies zuerst `CLAUDE.md`, `docs/ASSETS.md` (Abschnitte „Environment objects“, „Town and
> winter buildings“, Prompt 186 `[surface]`) und `docs/SOURCES.md`. Im Code:
> `tools/blender/kit/buildings.py`,
> `tools/blender/kit/env.py`,
> `godot/assets/shaders/env.gdshader`,
> `godot/assets/shaders/env_surface.gdshaderinc`,
> `godot/assets/shaders/env_roof.gdshaderinc`,
> `assets-src/blueprints/neutral/env_house_*.toml`,
> `assets-src/blueprints/neutral/env_church*.toml`,
> `data/catalog/neutral.toml`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:00–00:06` (rechter oberer
> Bildbereich: zwei ländliche Steinhäuser + Dorfkirche mit Turm im Hintergrund).
> **Keine Bilder, Namen oder Assets aus dem Video ins Repo.**
> **Wichtig:** Wenn wir vorhandene Katalog-Einträge verfeinern oder reine Darstellungs-Details
> verbessern, bleiben Spielregeln & Golden Hashes unverändert! (Falls neue Katalog-IDs
> in `neutral.toml` nötig wären, vorher prüfen, ob bestehende Typen + neue Blueprint-Varianten
> oder `[surface]`-Parameter ausreichen – bestehende IDs vermeiden Golden-Hash-Änderungen!)

## 1. Problem

In der Referenz (Sek. 00:00–00:06, oben rechts & Mitte) geben die Gebäude der Dorfsilhouette
sofort Glaubwürdigkeit und Maßstab:
- **Ländliche Steinhäuser (Normandie-Stil):**
  - Warmer, cremefarbener bis sandgrauer verwitterter Putz mit erkennbarem dunklem
    Sockel, hell/dunkel abgesetzten Eckquadern/Rahmen und klarer Boden-Erdung (weicher
    Kontaktschatten rund um den Sockel).
  - Steile Satteldächer (ca. 45°) in kontrastierenden Eindeckungen nebeneinander:
    **dunkler Schiefer** und **warmes Ziegelrot/Rotbraun**, mit heller Ortgang-Kante am
    Giebel, gemauerten Schornsteinen am First und kleiner Dachgaube/Dachfenster.
  - Dunkle, klar gegliederte Sprossenfenster mit hölzernen Fensterläden und schmaler
    Sturzkante – auch aus 80–150 m scharf lesbar.
- **Dorfkirche als Landmarke:**
  - Schlanker, heller Naturstein-Kirchturm mit Strebepfeilern, Rundbogen-Schallöffnungen
    und steilem dunklem Schiefer-Spitzhelm über dem Langhaus – perfekt als vertikaler
    Blickfang über dem flachen Feld-Patchwork.

Bei unseren bestehenden Dorfhäusern (`env_house_farm`, `env_house_small`, `env_church`)
wirken Putzwände, Giebelabschlüsse, Dachdetails (Gauben, Schornsteine, Ortgang) und
Kontraste zwischen Schiefer- und Ziegeldächern aus mittlerer Entfernung noch etwas schlicht.

## 2. Ziel

1. **Baukasten `buildings.py` gezielt erweitern (rückwärtskompatibel!):**
   - Optionale architektonische Details für `house` (alle per Default aus, sodass bestehende
     Häuser unverändert bleiben, sofern nicht bewusst aktiviert):
     - **Ortgang-/Traufbretter** (`gable_trim = true`, heller Putz/Stein oder dunkles Holz).
     - **Eckquader / Werkstein-Sockel** (`quoins = true` oder `Stone`-Slot an Ecken/Stürzen,
       was auch eine offene Lücke aus Prompt 186 schließt!).
     - **Kleine Dachgaube / Dachfenster** (`dormer = true`) und sauber abgesetzter
       gemauerter **First-Schornstein** (`chimney` mit Ziegel-/Steinkrone).
     - Klare Sprossenfenster (`sash_window` aus `env.gdshader`, ggf. in Abstimmung mit
       Prompt 215) + Holzläden (`shutters = true`).
2. **Normandie-/Bocage-Haus- und Kirchen-Look zusammenstellen:**
   - Prüfen, ob sich der Look durch gezielte Aufwertung von `env_house_farm` / `env_house_small`
     / `env_church` (über `[surface]` + die neuen `buildings.py`-Parameter, inkl. `Intact`,
     `Damaged`, `Ruin` und Einhaltung des 3.000-/5.000-Dreiecke-Budgets!) erreichen lässt,
     sodass **keine neuen Katalog-IDs** nötig sind und der Golden Hash unverändert bleibt.
   - Besonders achten auf:
     - Guten Farbkontrast zwischen **dunklem Schieferdach** und **warmem Tonziegeldach**
       bei Nachbarhäusern (z. B. über `[surface]` oder `roof`-Varianz).
     - Kräftigen Sockel-AO im Bake, damit Häuser im Gras „verwurzelt“ stehen und nicht
       aufgesetzt wirken.
     - Glaubwürdige `Damaged`- und `Ruin`-Stufen (falls Artillerie das Dorf in Prompt 221
       trifft!).

## 3. Prüfen und zeigen

- **Vorschau-Blätter (`just building-preview`):**
  - Vorher/Nachher-Blätter der überarbeiteten Häuser und der Kirche (nah + Spielhöhe,
    Wald/Wüste/Winter + `Intact`/`Damaged`/`Ruin`).
- **Pflicht-Checks (`CLAUDE.md`):**
  - `just validate-assets` (alle Budgets, Maße ±2 %, Varianten grün).
  - `just import`, `just gallery-check`.
  - Thumbnails aller geänderten Gebäude neu rendern (`just thumbnails --only=...`) und
    `just units-web-sync` ausführen.
  - `just check`, `just selftest` grün; Golden Hash unverändert (bzw. falls neue Katalog-ID
    unvermeidbar, sauber dokumentieren).

## 4. Nicht Teil dieses Prompts

- Modernere Großstadt-Wohnblöcke (`blocks.py`) oder Wüstenhäuser (`desert.py`).
- Kamera (Prompt 217) und Effekte (Prompt 220).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/219-normandie-dorfhaeuser-und-kirche.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis

Umgesetzt am 2026-11-25 (keine neuen Katalog-IDs, `crates/sim` und Golden Hashes unverändert):

- **Baukasten `tools/blender/kit/buildings.py` (rückwärtskompatibel, alle neuen Optionen per Default aus):**
  - `_sash_quad()` / `sash_windows = true`: einzelne explizit gewickelte `Glass`-Quads (`UV.x = 102.001..102.999`) für scharfe Sprossenfenster (`sash_window` in `env.gdshader`) an Fassaden, Giebelfenstern und Dachgauben.
  - `quoins = true`: abwechselnde Werkstein-Eckquader (`Stone` Kind 1) an allen vier Hausecken sowie Werkstein-Fensterbänke und -Stürze über Fenstern und Türen (schließt offene Lücke aus Prompt 186).
  - `gable_trim = true | "wood"`: abgesetzter Ortgang an beiden Giebelseiten sowie Traufbrett an den Längsseiten.
  - `dormer = N`: kleine Satteldach-Dachgauben auf der vorderen Dachfläche (`Intact`) mit Sprossenfenster und passender Eindeckung.
  - `chimney_cap = true`: gemauerter First-Schornstein nahe dem Giebel mit abgesetzter Steinkrone und doppelten Tonrohren (streng ≤ Dachhöhe `H`, sodass `fit = false` bei `Damaged` nie über `Intact` hinausragt).
  - `church()`: fehlenden Chor-Giebel an `+Y` geschlossen; neue Optionen `buttresses = true` (abgestufte Eck- und Schiff-Strebepfeiler) und `cornice = true` (Gurtgesims unter den Schallöffnungen und Kranzgesims unter dem Spitzhelm).
- **Shader (`env.gdshader`, `env_surface.gdshaderinc`):**
  - Sanfte Boden-Kontaktabdunklung am Fuß von `Stone`-Wänden und Sockeln (`slot == 12`, `h < 0,85 m`) für geerdete Gebäude im Gras.
  - Schiefer-Ton (`surface_roof` Kind 5) etwas abgedunkelt, damit Schiefer- und warme Tonziegeldächer bei Nachbarhäusern klar kontrastieren (vorläufig).
- **Aufgewertete Dorfgebäude (bestehende IDs, alle Budgets eingehalten):**
  - `env_house_stone`: Normandie-Putz-/Steinhaus (`[surface] plaster = "cream", roof = "slate"`, dunkler Sockel, Eckquader, Ortgang, Läden, Sprossenfenster, Gaube, First-Schornstein) — 2628/3000 Tris.
  - `env_house_farm`: Fachwerk-Bauernhaus (`[surface] plaster = "lime", roof = "clay"`, Holz-Ortgang, Läden, Sprossenfenster, Gaube, First-Schornstein) — 2326/3000 Tris.
  - `env_house_farm_brick`: Backstein-Bauernhaus (`[surface] brick = "red", roof = "clay"`, Eckquader, Ortgang, Läden, Sprossenfenster, Gaube, First-Schornstein) — 1562/3000 Tris.
  - `env_church`: steinerne Dorfkirche (`[surface] roof = "slate"`, abgestufte Strebepfeiler, Gesimse, Ortgang, geschlossener Chorgiebel) — 1840/5000 Tris.
- **Prüfung:**
  - Vorher/Nachher-Vorschaublätter (`just building-sheet`, Wald & Winter, `Intact`/`Damaged`/`Ruin`) unter `build/preview_219/` visuell geprüft (inkl. behobenem Backface-Culling an `+X`-Fenstern/Gauben).
  - `just validate-env` für alle vier Gebäude grün; `just import`, `just thumbnails --only=...`, `just units-web-sync` ausgeführt.
  - `just check`, `just selftest` grün; Golden Hashes unverändert.
  - **Vorläufig:** Auswahl `env_house_stone` als cremefarbenes Putzhaus mit Werkstein-Eckquadern und Schieferdach statt rohem Feldstein, Gauben-/Schornstein-Proportionen und Schiefer-Helligkeit.
