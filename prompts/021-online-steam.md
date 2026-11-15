# Prompt 021 – Online über Steam

## Ziel

Online-Mehrspieler über Steam: Lobbys, Freunde einladen, Matchmaking, NAT-Traversal.

## Umfang

- Voraussetzung: Steam App-ID (von mir). Bis dahin mit der Test-App-ID 480 arbeiten.
- GodotSteam (GDExtension) in passender Version einbinden; `Transport` über Steam
  Networking Sockets.
- Lobbys erstellen und beitreten, Einladungen, Rich Presence.
- Plattformübergreifend (macOS, Linux, Windows) mit derselben Lobby.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Zwei Steam-Konten spielen eine Partie über das Internet ohne Desync (mein Test,
  Ablauf beschrieben).
- Betroffene Doku in `docs/` ist aktualisiert.

## Arbeitsweise

> **Entwurf.** Vor der Ausführung mit dem dann aktuellen Stand abgleichen und bei Bedarf
> konkretisieren (am besten in einer kurzen Session: „Bitte konkretisiere Prompt 021“).

- In einer frischen Session ausführen. Zuerst `CLAUDE.md` und `docs/` lesen, deren
  Regeln gelten (Simulation ist die Wahrheit, keine Floats in `sim`, Godot zeigt nur an).
- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe. Keine Software
  ohne mein Go installieren. Kein Test wird abgeschwächt.
- Erst prüfen und einen Plan zeigen, bei schwer umkehrbaren Entscheidungen auf mein Go
  warten. Pro Schritt ein Commit, `just check` und `just selftest` grün.
- Am Ende diese Datei mit `git mv` nach `prompts/done/` verschieben, Status in
  `prompts/README.md` auf „erledigt“ setzen, committen. Abschlussbericht: was gebaut
  wurde, wie ich es teste, Messwerte, was offen ist.
