# Prompt 025 – LAN-Mehrspieler

## Ziel

Echter Netzwerk-Transport im lokalen Netz: ENet über UDP, Spielsuche per Broadcast.

## Umfang

- `Transport`-Implementierung mit ENet (Rust-Crate oder Godot-ENet über die Bridge –
  vorher abwägen und mit mir entscheiden).
- Spielsuche: UDP-Broadcast im LAN, Liste offener Spiele.
- Verbindungsaufbau, Bereitschaft, gemeinsamer Start mit Seed und Konfiguration.
- Umgang mit Latenz (Eingabeverzögerung dynamisch), Verbindungsabbruch, Desync-Meldung
  mit Replay-Export zur Fehlersuche.
- Plattformübergreifend testen: mindestens macOS ↔ Windows oder Linux.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Zwei Prozesse auf einem Rechner spielen über localhost ein Szenario ohne Desync.
- Mein Test mit zwei Rechnern ist beschrieben.
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 025“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
