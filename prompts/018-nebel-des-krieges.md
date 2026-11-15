# Prompt 018 – Nebel des Krieges

## Ziel

Jeder Spieler sieht nur, was seine Einheiten sehen. Sichtlinie berücksichtigt
Höhen (Hügel verdecken), Wald und Gebäude.

## Umfang

- Sichtraster pro Spieler in der Sim (Auflösung z. B. 4 m), aktualisiert in festen
  Intervallen; Sichtweite und Sichthöhe pro Einheitentyp.
- Sichtlinie gegen Höhenfeld und Objekte, effizient (z. B. inkrementell, nur bewegte
  Einheiten).
- Zustände: nie gesehen, erkundet (zuletzt gesehener Stand), sichtbar.
- Snapshot liefert pro Spieler nur sichtbare Feindeinheiten (Vorbereitung gegen
  Map-Hacks ist bei Lockstep begrenzt; im Bericht kurz einordnen).
- Godot: Nebel als Shader-Overlay auf Terrain und Vegetation, weiche Ränder.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Tests: Einheit hinter Hügel unsichtbar, auf dem Hügel sichtbar.
- Performance-Messung mit 500 Einheiten.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 018“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
