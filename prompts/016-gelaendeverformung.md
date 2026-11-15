# Prompt 016 – Verformbares Gelände

## Ziel

Explosionen hinterlassen Krater, die das Höhenfeld der Sim ändern und Bewegung sowie
Sicht beeinflussen.

## Umfang

- Sim: Krater-Funktion (Radius, Tiefe, Randwall) auf dem Ganzzahl-Höhenfeld,
  deterministisch; betroffene Bereiche in Kostenraster und Sicht invalidieren.
- Geländeart im Krater wird zu „aufgewühlt/Schlamm“.
- Bridge meldet geänderte Rechtecke; Terrain aus 005 baut nur diese Chunks neu.
- Darstellung: Krater-Decal, Erdbrocken-Partikel, Deko-Vegetation im Krater entfernen.
- Grenzen: maximale Kraterzahl pro Tick, Speicher bleibt konstant.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Test: Krater ändert Höhe an erwarteten Punkten exakt; Hash identisch auf zwei Peers.
- Bildrate bei 100 Einschlägen in kurzer Zeit gemessen.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 016“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
