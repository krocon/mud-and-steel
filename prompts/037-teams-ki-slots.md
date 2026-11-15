# Prompt 037 – Teams, KI-Plätze in der Lobby und Spielmodi

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Spielablauf, Basisbau), `docs/AI.md`, `docs/LAN.md`,
> `docs/ONLINE.md`, `docs/GAME_DESIGN.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Bisher spielt jeder gegen jeden, die Lobby hat keine KI-Plätze und im LAN gibt es keine
Moduswahl („There are no teams yet“, „The lobby offers no AI slots yet“). Nach diesem
Prompt sind 2-gegen-2 und 3-gegen-3 mit Menschen und KI gemischt möglich, lokal, im LAN
und über Steam.

## 2. Umfang

1. **Sim:** Teams in der Spielkonfiguration (im Hash), Sieg pro Team, Verbündete greifen
   sich nicht an, teilen Sicht (Option), können sich keine Einheiten steuern.
   Kapitulation eines Spielers übergibt nichts; Team verliert, wenn alle besiegt sind.
2. **Lobby (Einzelspieler, LAN, Steam):** Plätze mit Mensch/KI (Stufe)/geschlossen,
   Team- und Fraktionswahl pro Platz, Kartengröße passend zur Spielerzahl, Modus
   (Gefecht ohne Basis, Basisbau). KI-Spieler hostet der Host wie in 024 vorbereitet.
3. **Karten:** Startpositionen und Versorgungspunkte für 4 und 6 Spieler; Editor (028)
   kann Teams/Startplätze setzen; mindestens eine 2v2-Karte.
4. **KI:** kennt Verbündete, greift deren Gegner an, hilft bei Angriffen (einfach halten).
   Endspiel der KI in Basisbau-Partien (aus `GAME_DESIGN.md` „Open“).
5. **HUD:** Spielerfarben pro Team, Verbündete auf der Minikarte, Ping an Verbündete.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Teamsieg, geteilte Sicht, kein Eigenbeschuss
   unter Verbündeten; Golden-Hash bewusst aktualisiert.
2. `just lan-selftest` mit zwei Menschen + zwei KIs im 2v2 grün; `just selftest` grün.
3. `just ai-tournament` unterstützt Teams.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
