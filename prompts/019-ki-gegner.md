# Prompt 019 – KI-Gegner

## Ziel

Ein Computerspieler, der nur über normale Befehle in die Sim eingreift (Regel 5).

## Umfang

- KI läuft deterministisch im Lockstep (auf allen Peers gleich) oder auf einem Host und
  sendet Befehle wie ein Spieler – Vor- und Nachteile abwägen und mit mir entscheiden.
- Sie liest nur, was ihr Spieler sehen darf (Nebel des Krieges).
- Ebenen: Strategie (Ziele, Bauen), Taktik (Gruppen, Angriff, Rückzug), Mikro.
- Schwierigkeitsstufen.
- Headless-Turnier in der CLI: KI gegen KI über viele Seeds, Statistik.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- KI gewinnt gegen eine passive KI zuverlässig; Turnier läuft reproduzierbar.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 019“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
