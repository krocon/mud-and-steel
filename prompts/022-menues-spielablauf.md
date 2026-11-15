# Prompt 022 – Menüs und Spielablauf

## Ziel

Aus dem Prototyp wird ein Spiel: Hauptmenü, Spielvorbereitung, Einstellungen,
Sieg und Niederlage, Pause.

## Umfang

- Hauptmenü, Einzelspieler-Setup (Karte, Gegner, Fraktion), LAN/Online-Lobby-UI.
- Einstellungen: Grafikstufen (009), Tastenbelegung, Lautstärke, Kamera-Empfindlichkeit.
- Siegbedingungen in der Sim (z. B. alle Einheiten zerstört, Kontrollpunkte).
- Pause im Einzelspieler, Abstimmung im Mehrspieler.
- Ladebildschirm für große Karten.

## Akzeptanzkriterien

- `just check` und `just selftest` sind grün.
- Kompletter Ablauf Menü → Partie → Ergebnis → Menü im Selbsttest abgedeckt.
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
