# Prompt 181 – Schwierigkeit je Mission und Warnung vor dem Unschaffbaren

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/MISSIONS.md` ganz (vor allem „Script reference“, `[[computer]]`,
> „Reinforcements drive in“, „Open“), `docs/AI.md` („Difficulties“), die vier
> Missionen unter `data/maps/*/missions/`, `crates/sim/src/mission.rs`,
> `crates/sim/src/sim/mission.rs`, `crates/bridge/src/mission_funcs.rs`,
> `godot/scenes/hud/mission_hud.gd` und das Missionsmenü unter `godot/scenes/`.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) kritisiert: zu steile Schwierigkeit schon auf mittlerer Stufe, und
frühe Fehler machen eine Mission unbemerkt unschaffbar.

Bei uns steckt die Schwierigkeit nur in der festen Stufe des Computerspielers im
Skript (`[[computer]] difficulty`). Der Spieler kann nichts wählen und erfährt
nicht, wenn die Mission praktisch verloren ist.

Kampagne, Bewertung und neue Missionen sind Prompt 189.

## 2. Ziel

### 2.1 Schwierigkeit je Mission

- Wahl **Leicht / Normal / Schwer** beim Start einer Mission (Menü, gemerkt in
  `user://missions.cfg`).
- Das Skript kann je Stufe mehrere Stellschrauben setzen (alles Daten, kein Code
  je Mission): Stufe des Computerspielers, Stärke und Zeitpunkt der gegnerischen
  Verstärkung, eigene Verstärkung, Zeitgrenzen, Startvorrat.
- Neuer Skript-Abschnitt (z. B. `[difficulty.easy]`, der Werte überschreibt),
  dokumentiert in `docs/MISSIONS.md`; die vier Missionen bekommen sinnvolle Werte.
  Ohne Abschnitt gilt das Skript wie heute (Normal).

### 2.2 Warnung vor dem Unschaffbaren

- Ein Skript kann eine Bedingung „Mission kaum noch zu gewinnen“ formulieren
  (z. B. weniger als N Panzer und Ziel noch offen; aus vorhandenen Auslösern
  zusammengesetzt, fehlende ergänzen).
- Dann sagt die Missionsmeldung das deutlich und bietet „Neu starten / Letzten
  Spielstand laden / Weiterspielen“ an, statt dass der Spieler 10 Minuten
  weiterkämpft. Die vier Missionen bekommen je eine solche Bedingung.

## 3. Regeln

- Missionen sind Daten; Texte Deutsch und Englisch (`just mission-po`).
- Simulation deterministisch; Golden Hash unverändert (nutzt keine Mission).
- Die Stufe ist Eingabe des Szenarios, kein versteckter Zustand.

## 4. Vorgehen

1. Bestand lesen, Entwurf der Stellschrauben und des Skriptformats dem Nutzer
   vorlegen.
2. Skript und Simulation mit Tests (Überschreiben je Stufe, Warnbedingung).
3. Menü, Missionsmeldung.
4. Werte für die vier Missionen; Selbsttest startet mindestens eine Mission auf
   Leicht und Schwer.
5. `just check`, `just selftest`.
6. Doku: `docs/MISSIONS.md`.
7. Für den Nutzer: Spieldauer und Ergebnis jeder Mission auf allen drei Stufen
   gegen eine einfache Teststeuerung, wo das automatisch geht.

## 5. Nicht Teil dieses Prompts

- Kampagne, Bewertung, neue Missionen (Prompt 189).
