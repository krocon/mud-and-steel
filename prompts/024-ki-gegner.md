# Prompt 024 – KI-Gegner

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md`, `docs/DETERMINISM.md` und `docs/LAN.md`.

## 1. Ziel

Ein Computerspieler steuert im Einzelspieler die bisher passiven Gegner-Panzer. Er
greift nur über normale `Command`s ein (Regel 5), sieht nur, was sein Spieler sehen
darf (Nebel des Krieges), hat Schwierigkeitsstufen und lässt sich headless in einem
Turnier KI gegen KI messen. Er funktioniert mit Panzern und ist so gebaut, dass
Infanterie (Prompt 022, läuft parallel) und später Basisbau (Prompt 023) dazukommen
können.

## 2. Kernentscheidung: wo die KI läuft

**Die KI läuft auf einem Rechner (im Einzelspieler lokal, im LAN beim Host) außerhalb
der Simulation und schickt ihre Befehle als eigener Spieler durch die
Lockstep-Session.** Sie ist kein Teil von `crates/sim` und läuft nicht auf allen
Peers.

| Kriterium | KI deterministisch im Lockstep auf allen Peers | KI auf einem Rechner, Befehle als Spieler (gewählt) |
|---|---|---|
| Replays | Replays enthalten keine KI-Befehle; jede Änderung an der KI ändert den Ablauf alter Replays und den Golden-Hash. | Die Befehle stehen im Replay wie die eines Menschen; Replays laufen auch nach KI-Änderungen, ohne KI-Code. |
| Desync-Risiko | Jede Zeile KI-Code wird Teil des Protokolls: keine Floats, keine Hash-Iteration, feste Reihenfolge, jede Version auf allen Peers gleich. Ein KI-Fehler ist ein Desync. | Keins durch die KI: die Peers wenden nur übertragene Befehle an. |
| LAN | Funktioniert ohne Netzwerkverkehr für die KI. | Der Host sendet für jeden KI-Spieler Frames wie für sich selbst; geht der Host, endet das Spiel ohnehin (Sterntopologie, kein Reconnect). |
| Rechenzeit | Läuft auf jedem Peer, muss in den Tick-Takt passen. | Nur auf einem Rechner; darf sich Zeit lassen, weil Befehle ohnehin `input_delay` Ticks später greifen. |
| Fairness (Nebel) | Die KI hätte als Teil der Sim Zugriff auf den vollen Zustand; Nebel wäre nur Disziplin. | Dasselbe Problem, gelöst durch eine gefilterte Sicht (`PlayerView`), die nur sichtbare Information enthält. |
| Aufwand | Klein für den Einzelspieler, aber Golden-Hash, Replay-Format und Lockstep-Regeln wären bei jeder KI-Feinjustierung betroffen. | Etwas Lockstep-Code: die Session muss Frames für „gehostete“ Spieler senden. Danach ist die KI frei änderbar. |

Damit die Turnierergebnisse auf allen Plattformen gleich sind, rechnet die KI trotzdem
nur mit Ganzzahlen und `Fixed` und hat einen eigenen `sim::Rng`. Das ist eine
Eigenschaft für reproduzierbare Messungen, keine Bedingung für Synchronität.

## 3. Umfang

1. **Crate `crates/ai`** (hängt nur von `sim` ab, gleiche Lints wie `sim`: keine
   Floats, keine `HashMap`/`HashSet`, keine Uhr). API:
   - `Difficulty`: `Passive` (gibt keine Befehle: Baseline und Testgegner), `Easy`,
     `Normal`, `Hard`; als Text `passive`/`easy`/`normal`/`hard`.
   - `PlayerView::new(&Sim, player)`: der einzige Weg der KI an den Zustand. Enthält
     eigene Einheiten vollständig, gegnerische nur, wenn der Spieler sie sieht
     (`Sim::snapshot_for`), und davon nur, was man sieht (Position, Typ, Ausrichtung,
     Turm, Lebenspunkte; nicht Ziel und Feuermodus). Dazu Tick, Kartendaten
     (öffentlich: Größe, Startpositionen) und das Spielergebnis.
   - `Ai::new(player, difficulty, &SimConfig, &Map, seed)` und
     `Ai::think(&mut self, &PlayerView) -> Vec<Command>`, einmal pro Tick aufgerufen;
     die KI entscheidet selbst, in welchen Ticks sie handelt (Reaktionszeit je Stufe).
2. **Verhalten (Panzer)**, in Ebenen:
   - *Strategie:* Ziel wählen: sichtbare Gegner, sonst zuletzt gesehene Position
     (Gedächtnis pro Gegner-Einheit), sonst die gegnerische Startposition der Karte
     (öffentlich) bzw. ihr Spiegelbild durch die Kartenmitte. Basisbau gibt es noch
     nicht (Prompt 023).
   - *Taktik:* die Einheiten als Gruppe (`MoveTo` mit Formation); Nachzügler
     sammeln, bevor angegriffen wird; vor dem Gefecht an der eigenen Reichweite
     anhalten statt in den Gegner zu fahren; beschädigte Einheiten ziehen sich zurück
     (`Hard`).
   - *Mikro:* Feuer bündeln (`Attack` auf ein gemeinsames Ziel: das
     angeschlagenste bzw. nächste), Feuermodus zu Beginn auf `FireAtWill`.
   - Schwierigkeitsstufen unterscheiden sich in Reaktionszeit, Feuerbündelung,
     Sammeln und Rückzug (Tabelle in `docs/AI.md`).
   - Erweiterbar: Einheiten werden nach Rolle (Panzer, später Infanterie) in Gruppen
     eingeteilt; neue Rollen bekommen eigene Regeln, ohne die Panzer-Logik zu ändern.
3. **Lockstep:** `LockstepSession::host_player(player)` meldet einen Spieler an,
   dessen Frames dieser Peer sendet; `submit_hosted(player, commands, transport)`
   plant sie für denselben Tick wie den letzten eigenen Frame. Frames von Remote-Peers
   für gehostete Spieler werden abgelehnt. Hash-Vergleich und „Einzelspieler“ zählen
   nur echte Remote-Spieler. Rein additiv, bestehende Aufrufe ändern sich nicht.
4. **Bridge und Spiel:** `SimHost.start_match(seed, map_dir, factions, ai)` (neuer
   optionaler Wert, Default ohne KI wie bisher): im Einzelspieler wird Spieler 1 als
   KI-Spieler in der Session gehostet und bekommt seine Befehle vor jedem Tick aus
   `Ai::think`. Das Replay enthält sie. Im Einzelspieler-Setup des Menüs wählt man
   den Gegner: `Passive`, `Easy`, `Normal` (Default), `Hard`. `--skip-menu`
   akzeptiert `--ai=<stufe>` (Default: ohne KI, wie bisher). LAN-Spiele mit KI-Slots
   im Lobby-Menü sind nicht Teil dieses Prompts (der Lockstep-Teil ist vorbereitet).
5. **CLI-Turnier:** `cli ai-tournament --games N --seed S --map DIR --p0 hard
   --p1 passive [--factions de,su] [--max-ticks T]`. Jedes Spiel hat einen eigenen
   Seed (S, S+1, …), die Seiten wechseln jedes Spiel (jede KI spielt beide Seiten und
   Fraktionen). Die KI-Befehle laufen wie im Spiel mit `input_delay` 1 durch eine
   Lockstep-Session. Ausgabe pro Spiel (Sieger, Tick, Einheiten übrig) und eine
   Statistik: Siege/Remis/Zeitlimit je KI, mittlere Spieldauer, Einheiten übrig,
   Rechenzeit der KI pro Tick, und ein Ergebnis-Hash über alle Spiele.
   `just ai-tournament` ruft es auf.
6. **Doku:** `docs/AI.md` (Entscheidung, Aufbau, Verhalten, Stufen, Turnier mit
   Messwerten), Verweise in `docs/ARCHITECTURE.md` und `docs/LAN.md`,
   `CLAUDE.md`-Layout.

## 4. Akzeptanzkriterien

1. `just check` grün. Golden-Hash unverändert (die Sim ändert sich nicht).
2. Tests in `crates/ai`:
   - Die KI sieht nur, was ihr Spieler sieht: zwei Sims, die sich nur in Positionen
     nicht sichtbarer Gegner unterscheiden, ergeben dieselbe `PlayerView` und
     dieselben Befehle.
   - `Normal` und `Hard` gewinnen gegen `Passive` auf `test_small` in jedem von
     mindestens 6 Spielen (beide Seiten); `Passive` gibt keinen Befehl.
   - Zwei Läufe desselben Turniers ergeben dieselben Ergebnisse (gleicher
     Ergebnis-Hash).
   - Die KI gibt nur Befehle für eigene, lebende Einheiten.
3. `crates/net`: Test mit gehostetem Spieler (Frames kommen beim Remote-Peer an,
   Ergebnis gleich wie ohne Netz); ein Replay eines Spiels gegen die KI spielt ohne
   KI-Code zum selben End-Hash ab.
4. Turnier (`just ai-tournament`, 40 Spiele pro Paarung auf `test_small`):
   `Normal` und `Hard` gewinnen ≥ 95 % gegen `Passive`; `Easy` gewinnt mehr als die
   Hälfte gegen `Passive`; die Paarungen der Stufen untereinander sind gemessen und
   in `docs/AI.md` dokumentiert. Zwei Läufe ergeben denselben Ergebnis-Hash.
5. Im Spiel: Einzelspieler-Setup mit Gegnerwahl; die Gegner-Panzer bewegen und
   kämpfen. `just selftest` (Flow-Test mit neuem Teil: ein Spiel gegen die KI, deren
   Panzer sich in einer Frist bewegen und Befehle geben), `just lan-selftest`,
   `just editor-selftest`, `just gallery-check` und der Demo-Lauf bleiben grün.
6. Doku aktualisiert (siehe Umfang 6).

## 5. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Wo läuft die KI | Auf einem Rechner, Befehle als Spieler über die Session | Siehe Abschnitt 2: Replays und Golden-Hash unabhängig von der KI, kein Desync-Risiko, LAN-tauglich. |
| Eigener Crate | `crates/ai`, nicht in `sim` oder `bridge` | Bridge und CLI brauchen sie beide; die Sim bleibt frei von KI-Code. |
| Nebel | `PlayerView` als einziger Eingang | Die Regel ist strukturell erzwungen und testbar statt Disziplin. |
| Mehrere KI-Spieler / LAN | Session kennt „gehostete“ Spieler | Gleicher Mechanismus für Einzelspieler, LAN-Host und Turnier. |
| Bestehende Selbsttests | Starten ohne KI (`Passive`) | Ihr Ablauf (Halten des Feuers, Angriff nacheinander) bleibt gleich; ein neuer Teil prüft die KI. |

## 6. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, Commits pro Schritt. Parallel laufen 020
(Zerstörung) und 022 (Infanterie): Änderungen an gemeinsamen Dateien (`sim.rs`,
`bridge/src/lib.rs`, `main.gd`, Menüs, `self_test.gd`, `justfile`, Doku) klein und
additiv halten. Am Ende nach `prompts/done/` verschieben und `prompts/README.md`
aktualisieren.
