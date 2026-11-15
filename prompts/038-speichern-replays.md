# Prompt 038 – Speichern, Laden und Replay-Ansicht

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Replays, Spielablauf), `docs/DETERMINISM.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Replays werden geschrieben, aber man kann sie im Spiel nicht ansehen; Partien lassen
sich nicht speichern. Nach diesem Prompt kann man Einzelspieler-Partien speichern und
laden und jedes Replay im Spiel abspielen, vorspulen und aus jeder Perspektive ansehen.

## 2. Umfang

1. **Speichern über Replay:** Speicherstand = Replay bis zum aktuellen Tick + Hash;
   Laden spielt schnell nach (headless, ohne Darstellung) und prüft den Hash.
   Messen, ob das für lange Partien schnell genug ist; sonst periodische
   Sim-Schnappschüsse (Serialisierung des gesamten Zustands, deterministisch, mit
   Hash-Prüfung) als Beschleunigung. Entscheidung begründen.
2. **Replay-Ansicht:** Menüpunkt „Replays“ mit Liste (Datum, Karte, Spieler, Dauer,
   Ergebnis), Abspielen mit Pause, 1×/2×/4×/8×, Springen (über Schnappschüsse),
   Perspektive wählen (ein Spieler mit dessen Nebel oder alles sichtbar).
3. **Versionierung:** Replays und Spielstände tragen Spielversion und Regel-Hash;
   inkompatible werden klar abgelehnt statt still falsch abzuspielen.
4. **Mehrspieler:** kein Speichern, aber das Replay wird wie bisher geschrieben und ist
   ansehbar.

## 3. Akzeptanzkriterien

1. `just check` grün; Tests: Speichern/Laden ergibt denselben Hash wie Durchspielen,
   Schnappschuss-Rundreise, alte Replays lesbar oder sauber abgelehnt.
2. `just selftest` mit Speichern, Laden und Replay-Abspielen grün.
3. Doku: `docs/ARCHITECTURE.md` und `docs/DETERMINISM.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
