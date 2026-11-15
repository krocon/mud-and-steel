# Prompt 012 – Kampf-Grundlagen

## Ziel

Einheiten können kämpfen: Lebenspunkte, Panzerung, Waffen, Reichweite, Geschosse,
Treffer. Alles deterministisch in der Sim.

## Umfang

- Werte in `data/units.toml` und `data/weapons.toml` (Ganzzahlen mit Einheiten).
- Befehle: Angriff auf Einheit, Angriff auf Boden, Halten/Feuer frei.
- Zielauswahl automatisch nach festen Regeln, Turm-Drehgeschwindigkeit, Nachladezeit.
- Geschosse als Sim-Objekte mit ballistischer Bahn in `Fixed`; Treffer gegen Höhenfeld
  und Objekte; Streuung über den geseedeten RNG.
- Panzerung frontal/seitlich/hinten, Schaden, Zerstörung (Wrack bleibt als Hindernis).
- Sim-Ereignisse (Schuss, Treffer, Zerstörung) im Snapshot für Effekte (014) und Sound.
- Godot: einfache Darstellung (Linie/Leuchtspur, Wrack-Modell), Lebensbalken.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Golden-Hash mit Kampf-Szenario; Loopback-Test mit Gefecht.
- Test: Panzer A zerstört Panzer B nach erwarteter Tickzahl.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 012“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
