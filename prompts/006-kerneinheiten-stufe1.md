# Prompt 006 – Kerneinheiten Stufe 1 (parallel pro Fraktion)

> **Dieser Prompt läuft dreimal parallel**, je eine Session pro Fraktion, jede in einem
> eigenen Git-Worktree/Branch `assets/<fraktion>-tier1`:
> - „Bitte führe `prompts/006-kerneinheiten-stufe1.md` für Fraktion `de` aus.“
> - „… für Fraktion `su` aus.“
> - „… für Fraktion `us` aus.“
>
> Voraussetzungen: Prompts 003, 004 und 005 sind erledigt. Lies `CLAUDE.md`,
> `docs/ASSETS.md`, `docs/SCALE.md`, `docs/SOURCES.md` und den Katalog deiner Fraktion.

## 1. Ziel

Alle Stufe-1-Einheiten der Fraktion als fertige, geprüfte Modelle in allen drei
Tarnungen. Die Auswahl steht im Katalog (`tier = 1`), aktuell:

| Fraktion | Infanterie | Fahrzeuge | Geschütz |
|---|---|---|---|
| `de` | Schützengruppe, MG-42-Trupp | Panzer IV Ausf. H, Sd.Kfz. 251/1 | 7,5 cm PaK 40 |
| `su` | Schützentrupp, MG-Trupp | T-34/76 (1943), Lkw ZiS-5 | 76 mm ZiS-3 |
| `us` | Rifle Squad, MG-Team | M4A3 Sherman (75 mm), M3 Halftrack | 57 mm M1 |

Bereits in Prompt 004 gebaute Modelle werden geprüft und bei Bedarf verbessert, nicht
neu gebaut.

## 2. Umfang

1. **Referenz sammeln:** Für jede Einheit Maße und Ansichten aus den Quellen in
   `docs/SOURCES.md` beziehungsweise aus dem Katalog. Referenzbilder Dritter nur lokal im
   Scratchpad, nie ins Repo.
2. **Baupläne** in `assets-src/blueprints/<fraktion>/` schreiben und Modelle erzeugen.
   Typische Merkmale müssen in der Draufsicht erkennbar sein (Silhouette, Turmform,
   Laufwerk, Schürzen, Rohrlänge).
3. **Infanterie-Einheiten** aus der Basis von 005 zusammensetzen: Trupp-Zusammensetzung
   laut Katalog (z. B. Gruppenführer, Schützen, MG-Schütze, Hilfsschütze).
4. **Hoheitszeichen und Markierungen** als Decals gemäß `docs/SOURCES.md`.
5. **Katalog:** Feld `model` der Einträge setzen.
6. **Prüfung:** `just validate-assets` ohne Fehler, Drehteller und Aufstellung für die
   Fraktion im Scratchpad.

## 3. Regeln für die parallele Arbeit

- Nur in eigene Pfade schreiben: `assets-src/blueprints/<fraktion>/`,
  `assets-src/blender/<fraktion>/`, `godot/assets/models/<fraktion>/`,
  `tools/blender/kit/parts_<fraktion>.py`, eigene Einträge in `data/catalog/<fraktion>.toml`.
- Gemeinsame Baukasten-Dateien nur **additiv** ändern (neue Funktionen, keine geänderten
  Signaturen oder Standardwerte). Größere Änderungen am gemeinsamen Baukasten nicht
  selbst machen, sondern im Bericht vorschlagen.
- `prompts/README.md` und diese Prompt-Datei **nicht** ändern und nicht verschieben. Das
  erledigt die zusammenführende Session, wenn alle drei Fraktionen fertig sind.
- Commits nur auf dem eigenen Branch. Kein Merge, kein Push.

## 4. Akzeptanzkriterien

1. `just check`, `just validate-assets` und `just selftest` grün auf dem Branch.
2. Alle fünf Stufe-1-Einheiten der Fraktion stehen in der Galerie, in allen drei
   Tarnungen; Infanterie mit Animationen.
3. Aufstellung der Fraktion mit Referenzfigur im Scratchpad; im Bericht die
   Größenverhältnisse zu den Katalogmaßen.
4. Bericht: welche Einheiten fertig sind, Abweichungen von historischen Vorlagen,
   vorgeschlagene Baukasten-Erweiterungen, unsichere Daten.

## 5. Zusammenführen (eigene kurze Session, wenn alle drei fertig sind)

„Bitte führe die drei Branches `assets/*-tier1` zusammen.“ Diese Session merged
nacheinander, löst Konflikte, lässt alle Prüfungen laufen, rendert eine Gesamt-
Aufstellung aller drei Fraktionen, verschiebt diese Datei mit `git mv` nach
`prompts/done/` und aktualisiert `prompts/README.md`. Kein Push ohne meine Freigabe.

## 6. Arbeitsweise

Mit mir Deutsch, im Repo Englisch. Keine Software ohne mein Go installieren. Mein
laufendes Blender nicht anfassen. Kein Test wird abgeschwächt. Binärdateien über
Git LFS.
