# Prompt 190 – Koop-Missionen im LAN und online

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach 181, 189 und
> 182 (Lobby-Protokoll schon um Varianten erweitert). Lies zuerst `CLAUDE.md`,
> `docs/MISSIONS.md` (Script `[[computer]]`, Teams, „Open“: „the menu and lobby do
> not offer missions“), `docs/TEAMS.md`, `docs/LAN.md`, `docs/ONLINE.md`, in
> `docs/ARCHITECTURE.md` „Data flow (multiplayer)“ und „Missions and tutorial“,
> dazu `crates/net/` (Lobby-Protokoll), `crates/sim/src/scenario.rs`,
> `crates/sim/src/mission.rs` und die Lobby unter `godot/scenes/`.

## 1. Ausgangslage

Aufgeteilt aus dem früheren Prompt 182 (2026-11-21). Die Testauswertung zu einem
anderen WW2-Taktikspiel (nur Anregung) vermisst Koop-Szenarien.

Bei uns sind Missionen nur im Einzelspiel spielbar, obwohl das Skript Teams kennt.
Das ist der größte Prompt dieser Reihe (Skript, Lobby, Netz); deshalb am Ende.

## 2. Ziel

- Missionen können **mehrere menschliche Spieler auf einer Seite** haben: das
  Skript benennt Spielerplätze (z. B. `[[player]]` mit Name, Einheitengruppen,
  Startgebiet); übrige Plätze übernimmt der Computer oder sie werden
  zusammengelegt (Entscheidung dokumentieren).
- Lobby (LAN und Steam): Modus „Mission“ mit Auswahl der Missionen, die Koop
  unterstützen, Schwierigkeit aus 181; Briefing für alle.
- Mindestens **eine Koop-Mission** für zwei Spieler (z. B. Angriff auf zwei
  Achsen, die sich gegenseitig brauchen: einer nimmt die Brücke, der andere hält
  die Flanke), auf einer vorhandenen Karte.
- Ein Spieler, der eine Koop-Mission verlässt: seine Einheiten übernimmt der
  Verbündete oder der Computer (Regel festlegen).

## 3. Regeln

- Lockstep-Regeln bleiben: keine Pause im Netz, Spielstand-Laden nur, wenn das
  heute schon im Netz geht.
- Simulation deterministisch; Golden Hash unverändert, sonst begründen.
- Lobby-Protokoll mit Versionsnummer; ältere Clients werden sauber abgewiesen.

## 4. Vorgehen

1. Bestand lesen, Entwurf (Skriptfelder, Lobby-Ablauf, Verlassen-Regel) dem
   Nutzer vorlegen.
2. Skript und Simulation mit Tests, Lobby, Koop-Mission.
3. `just check`, `just selftest`, `just lan-selftest` (um eine Koop-Mission
   erweitert).
4. Doku: `docs/MISSIONS.md`, `docs/TEAMS.md`, `docs/LAN.md`, `docs/ONLINE.md`.
5. Für den Nutzer: Ablauf einer Koop-Partie über zwei lokale Instanzen.

## 5. Nicht Teil dieses Prompts

- Plattformübergreifendes Spiel und Matchmaking (Steam-Lobbys reichen vorerst).
- Teilen von Karten und Missionen über den Steam Workshop.
