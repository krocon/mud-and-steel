# Prompt 036 – Nachschub, Verbrauch und Reparatur

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/GAME_DESIGN.md`, `docs/ARCHITECTURE.md` (Basisbau, Kampf, Transport aus 034).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

`GAME_DESIGN.md` hat Treibstoff, Munition und Material als Konten, aber noch keinen
Verbrauch an der Einheit, keine Versorgungs-LKW und keine Reparatur (alles unter
„Open“). Nach diesem Prompt muss eine Offensive versorgt werden: Einheiten verbrauchen
Munition und Treibstoff, Versorgungsfahrzeuge füllen auf, Werkstätten und
Bergefahrzeuge reparieren, bewegungsunfähige Panzer können geborgen werden.

## 2. Umfang

1. **Design zuerst:** mit mir klären, wie viel Logistik Spaß macht (Vorschlag: Vorrat
   pro Einheit, Nachschub im Radius von Versorgungs-LKW/Depots, kein Mikromanagement
   einzelner Granaten). Ergebnis in `GAME_DESIGN.md` als bestätigt markieren.
2. **Sim:** Vorräte pro Einheit, Verbrauch pro Schuss/Strecke, Versorgung im Radius,
   Reparatur über Zeit (Werkstatt, Bergepanzer, Pioniere), Gebäude reparieren und
   verkaufen, Baustellen abbrechen (alles aus „Open“).
3. **Bedienung und HUD:** Vorratsbalken in der Einheitenanzeige (031), Warnung bei
   knapper Munition, Versorgungsradius sichtbar.
4. **KI:** versorgt und repariert ihre Einheiten, zieht leere Einheiten zurück.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Verbrauch, Nachschub, Reparatur, Bergung;
   Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest`, `just ai-tournament` grün; die KI gewinnt
   gegen `Passive` weiterhin zuverlässig.
3. `GAME_DESIGN.md` aktualisiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
