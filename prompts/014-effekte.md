# Prompt 014 – Effekte

## Ziel

Rauch, Explosionen, Mündungsfeuer, Leuchtspur, Staubfahnen hinter Fahrzeugen, brennende
Wracks – wie in den Vorlagen. Reine Darstellung, ausgelöst durch Sim-Ereignisse.

## Umfang

- Effekt-Bibliothek mit `GPUParticles3D` (Rauchwolken, Feuerball, Funken, Trümmer),
  Licht-Blitz bei Explosionen, Kamera-Wackeln abhängig von Entfernung.
- Leuchtspur und Geschossbahnen, Mündungsfeuer an den Anbaupunkten aus 010.
- Staub hinter fahrenden Fahrzeugen abhängig von Geländeart.
- Decals für Einschläge und Brandflecken (bis 016 nur optisch).
- Budget: maximale Partikelzahl, Pooling, LOD nach Entfernung und Zoom.
- Effekte dürfen nie zurück in die Sim wirken.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Effekt-Testszene mit allen Effekten, Bildrate bei 50 gleichzeitigen Explosionen.
- Bildvergleich im Scratchpad.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 014“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
