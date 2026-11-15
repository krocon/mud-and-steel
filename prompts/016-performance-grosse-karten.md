# Prompt 016 – Performance für große Karten

## Ziel

Sicherstellen, dass Sim und Darstellung auf 4-km-Karten mit vielen Einheiten
schnell bleiben, und Messwerte dauerhaft im Blick behalten.

## Umfang

- Räumliches Raster in der Sim (Uniform Grid) für Nachbarschafts- und Bereichsabfragen;
  deterministische Iteration.
- Benchmarks: `cli bench` (z. B. 2000 Einheiten, 10 000 Ticks) mit Zeit pro Tick;
  Ergebnis als Tabelle in `docs/PERFORMANCE.md`.
- Budgets festlegen: Sim-Tick < 5 ms bei 20 Hz für das Zielszenario; Darstellung 60 FPS.
- Bridge: Snapshots ohne unnötige Kopien (nur geänderte Daten, Packed-Arrays wiederverwenden).
- Godot: Sichtbarkeits-Culling pro Chunk, MultiMesh für Einheiten bei hoher Anzahl,
  Profiler-Auswertung.
- CI: Benchmark als eigener, nicht blockierender Job mit Ausgabe der Werte.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Messwerte vor und nach den Optimierungen im Bericht.
- Golden-Hash bleibt unverändert (Optimierungen ändern das Verhalten nicht).
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
