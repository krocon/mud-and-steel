# Prompt 050 – Erfahrung und Ränge

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/GAME_DESIGN.md`, `docs/INFANTRY.md`, `docs/ARCHITECTURE.md` (Kampf).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Einheiten, die überleben und kämpfen, werden besser: Erfahrung in Stufen mit moderaten
Boni und sichtbaren Zeichen, damit man erfahrene Männer schützen will.

## 2. Umfang

1. **Sim:** Erfahrungspunkte für Treffer, Abschüsse, überstandenes Feuer; Stufen
   (z. B. Rekrut, erfahren, Veteran, Elite) mit kleinen Boni (Zielgenauigkeit,
   Nachladen, Moral/Niederhalten). Regeln mit Begründung, Balance in 045.
2. **Darstellung** im Stil der Zeit: Rangabzeichen in der Einheitenanzeige, an Soldaten
   kleine Unterschiede (Ausrüstung, Tarnjacke, erbeutete Waffe), an Fahrzeugen
   Abschussringe am Rohr. Keine modernen Zusätze (Visiere, Laser, Schutzwesten).
3. **HUD:** Stufe und Fortschritt in der Einheitenanzeige (031).
4. **KI** zieht Veteranen bei Verlusten eher zurück.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Aufstieg und Boni; Golden-Hash begründet.
2. `just selftest`, `just ai-tournament` grün; `GAME_DESIGN.md` ergänzt.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
