# Prompt 023 – Basisbau und Ressourcen

## Ziel

Bauen, Ressourcen abbauen und Einheiten produzieren, wie geplant für das Spielkonzept.

## Umfang

- Vorher mit mir klären: welche Ressourcen (z. B. Treibstoff, Munition, Personal),
  wie abgebaut wird, welche Gebäude es gibt. Ergebnis in `docs/GAME_DESIGN.md`.
- Sim: Bauplätze auf dem Raster (Steigung und Geländeart prüfen), Bauzeit,
  Produktionswarteschlangen, Ressourcen-Konto pro Spieler.
- Darstellung: Bau-Vorschau mit Gültigkeit, Baustelle, Produktions-UI.
- Gebäude sind zerstörbar (020).

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Test: Produktionskette läuft auf zwei Peers identisch; Golden-Hash aktualisiert.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 023“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
