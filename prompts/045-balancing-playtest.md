# Prompt 045 – Balancing und Playtest-Werkzeuge

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/GAME_DESIGN.md`, `docs/AI.md`, `docs/SCALE.md`, `docs/UNIT_CATALOG.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Die Einheitenwerte sind echt (Katalog), die Spielregeln (Kosten, Reichweitenfaktor,
Einkommen) vorläufig. Nach diesem Prompt gibt es Werkzeuge, mit denen sich das Balancing
messen lässt, und eine erste bestätigte Balance aller drei Fraktionen.

## 2. Umfang

1. **Statistik nach der Partie:** Verluste, Abschüsse pro Einheitentyp, Ressourcen-
   verlauf, eingenommene Punkte; im Ergebnisbildschirm und als JSON.
2. **Headless-Auswertung:** KI-Turniere über alle Fraktionspaare, Karten und Modi;
   Gefechtssimulator „Einheit A gegen Einheit B“ über Distanzen und Winkel; Bericht als
   Tabelle in `docs/BALANCE.md`.
3. **Stellschrauben:** nur Spielregeln (Kosten, Bauzeiten, Faktoren in `data/*.toml`),
   nie Katalogwerte; jede Änderung mit Messung begründet.
4. **Playtest-Ablauf:** Checkliste für Partien mit dem Nutzer, Rückmeldeformular in
   `docs/PLAYTEST.md`; `GAME_DESIGN.md` vom Status „vorläufig“ auf „bestätigt“
   bringen, wo der Nutzer zustimmt.

## 3. Akzeptanzkriterien

1. `just check` grün; Turnier über alle Paarungen ohne Fraktion mit mehr als 60 %
   Siegquote bei gleicher KI-Stufe (oder begründete Ausnahme).
2. `docs/BALANCE.md` mit Messwerten; Golden-Hash-Änderungen begründet.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
