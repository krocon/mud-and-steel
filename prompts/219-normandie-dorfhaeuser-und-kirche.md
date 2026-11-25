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
