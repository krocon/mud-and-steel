# Prompt 022 – Infanterie-Verhalten

## Ziel

Soldaten in Trupps: kleiner als Fahrzeuge, nutzen Deckung, laufen, kriechen, gehen in
Gebäude.

## Umfang

- Sim: Trupp als Einheit mit mehreren Soldaten (eigene Positionen in Formation),
  Haltungen (stehend, kniend, liegend) mit Einfluss auf Geschwindigkeit und Treffbarkeit,
  Deckung durch Objekte und Krater.
- Gebäude besetzen (Kapazität, Schutz).
- Darstellung: Modelle und Animationen aus Prompt 005 (Infanterie-Basis) und 006/008;
  bei vielen Soldaten Vertex-Animation oder Animations-Texturen für Performance.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Test: Trupp in Deckung erleidet weniger Treffer (statistisch über festen Seed).
- Performance mit 500 Soldaten gemessen.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 022“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
