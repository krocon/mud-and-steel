# Prompt 020 – Zerstörbare Gebäude und Vegetation

## Ziel

Häuser, Mauern, Zäune und Bäume lassen sich beschädigen und zerstören. Das ändert
Deckung, Sicht und Wege.

## Umfang

- Sim: Lebenspunkte und Schadensstufen für statische Objekte; Zerstörung ändert
  Kostenraster (012) und Sichtblocker (018) bereichsweise.
- Bäume fallen um (Richtung aus der Einschlagsrichtung, deterministisch), Wald wird
  lichter, Zäune werden umgefahren.
- Gebäude: Stufen intakt / beschädigt / Ruine, Ruine bleibt als Deckung.
- Darstellung: Ruinen-Varianten aus 007, Wracks aus 006/008, Trümmer-Partikel, Staubwolke.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Test: zerstörte Mauer öffnet einen Weg (Pfad wird kürzer), Golden-Hash aktualisiert.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 020“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
