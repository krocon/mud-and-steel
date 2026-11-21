# Prompt 182 – Startvarianten für Gefechtskarten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/MAPS.md`, `docs/TEAMS.md`, `docs/LAN.md`, `docs/ONLINE.md`, in
> `docs/ARCHITECTURE.md` „Data flow (multiplayer)“ und den Abschnitt zum
> Karteneditor, dazu `crates/sim/src/scenario.rs`, `crates/editor/`,
> `crates/net/` (Lobby-Protokoll) und die Lobby unter `godot/scenes/`.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) nennt zu wenig Abwechslung im Mehrspieler-Umfang. Langzeitmotivation
hängt an Inhalten, nicht an der Grundmechanik.

Bei uns hat jede Karte feste Startplätze; wer eine Karte oft spielt, kennt jede
Eröffnung. Das gilt auch für Gefechte gegen die KI.

Koop-Missionen sind Prompt 190.

## 2. Ziel

- Eine Karte kann **mehrere Startaufstellungen** haben (Startplätze,
  Versorgungspunkte als Ziel, ggf. gesperrte Brücken): Datenformat in
  `data/maps/<name>/` erweitern; Karten ohne Varianten bleiben gültig. Karteneditor
  und `just map-check` lernen es.
- Lobby (Einzelspiel, LAN, Steam): Variante wählen oder „zufällig“ (Zufall aus dem
  Spiel-Seed, deterministisch für alle Teilnehmer).
- Für fünf vorhandene Gefechtskarten je eine zweite Variante, mit
  `just map-reach` geprüft (alle Starts erreichen Versorgungspunkte);
  KI-Turnier auf beiden Varianten, Ausgewogenheit nennen.

## 3. Regeln

- Simulation deterministisch; Varianten sind Eingabe des Szenarios, kein
  versteckter Zustand. Golden Hash unverändert (Demo-Szenario bleibt), sonst
  begründen.
- Lobby-Protokoll mit Versionsnummer; ältere Clients werden sauber abgewiesen.

## 4. Vorgehen

1. Bestand lesen, Entwurf des Variantenformats und der Lobby-Wahl dem Nutzer
   vorlegen; die fünf Karten mit ihm abstimmen.
2. Format, Szenario, Prüfung mit Tests.
3. Editor, Lobby.
4. Fünf Varianten, `just map-reach`, Turnier.
5. `just check`, `just selftest`, `just lan-selftest`, `just editor-selftest`.
6. Doku: `docs/MAPS.md`, `docs/LAN.md`, `docs/ONLINE.md`.
7. Für den Nutzer: Bilder der fünf Varianten und die Turnierzahlen.

## 5. Nicht Teil dieses Prompts

- Koop-Missionen (Prompt 190).
