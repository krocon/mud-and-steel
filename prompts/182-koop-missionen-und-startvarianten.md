# Prompt 182 – Koop-Missionen im LAN und online, Startvarianten für Gefechtskarten

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach 181. Lies
> zuerst `CLAUDE.md`, `docs/MISSIONS.md` (Script `[[computer]]`, Teams, „Open“:
> „the menu and lobby do not offer missions“), `docs/TEAMS.md`, `docs/LAN.md`,
> `docs/ONLINE.md`, `docs/MAPS.md`, in `docs/ARCHITECTURE.md` „Data flow
> (multiplayer)“ und „Missions and tutorial“, dazu `crates/net/` (Lobby-Protokoll),
> `crates/sim/src/scenario.rs` und die Lobby unter `godot/scenes/`.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) nennt zu wenig Mehrspieler-Umfang: wenige Karten je Modus, kaum
Abwechslung, keine Koop-Szenarien. Langzeitmotivation hängt an Inhalten, nicht an
der Grundmechanik.

Bei uns: LAN und Steam mit Lobby, Teams 2v2/3v3 und KI-Plätzen, viele Karten
(und viele weitere in offenen Prompts). Missionen sind nur im Einzelspiel
spielbar, obwohl das Skript Teams kennt. Jede Karte hat feste Startplätze; wer
eine Karte oft spielt, kennt jede Eröffnung.

## 2. Ziel

### 2.1 Koop-Missionen

- Missionen können **mehrere menschliche Spieler auf einer Seite** haben: das
  Skript benennt Spielerplätze (z. B. `[[player]]` mit Name, Einheitengruppen,
  Startgebiet); übrige Plätze übernimmt der Computer oder sie werden
  zusammengelegt (Entscheidung dokumentieren).
- Lobby (LAN und Steam): Modus „Mission“ mit Auswahl der Missionen, die Koop
  unterstützen; Briefing für alle.
- Mindestens **eine Koop-Mission** für zwei Spieler (z. B. Angriff auf zwei
  Achsen, die sich gegenseitig brauchen: einer nimmt die Brücke, der andere hält
  die Flanke), auf einer vorhandenen Karte.
- Lockstep-Regeln bleiben: keine Pause im Netz, Spielstand-Laden nur, wenn das
  heute schon im Netz geht.

### 2.2 Startvarianten

- Eine Karte kann **mehrere Startaufstellungen** haben (Startplätze,
  Versorgungspunkte als Ziel, ggf. gesperrte Brücken): Datenformat in
  `data/maps/<name>/` erweitern, Karteneditor und `just map-check` lernen es.
- Lobby: Variante wählen oder „zufällig“ (Zufall aus dem Spiel-Seed,
  deterministisch für alle Teilnehmer).
- Für fünf vorhandene Gefechtskarten je eine zweite Variante, mit
  `just map-reach` geprüft (alle Starts erreichen Versorgungspunkte);
  KI-Turnier auf beiden Varianten, Ausgewogenheit nennen.

## 3. Regeln

- Simulation deterministisch; Szenario-Varianten sind Eingabe, kein versteckter
  Zustand. Golden Hash unverändert (Demo-Szenario bleibt), sonst begründen.
- Lobby-Protokoll mit Versionsnummer; ältere Clients werden sauber abgewiesen.
- Ein Spieler, der eine Koop-Mission verlässt: seine Einheiten übernimmt der
  Verbündete oder der Computer (Regel festlegen).

## 4. Vorgehen

1. Bestand lesen, Entwurf (Skriptfelder, Lobby-Ablauf, Variantenformat) dem
   Nutzer vorlegen.
2. Skript und Simulation mit Tests, Lobby, Koop-Mission.
3. Variantenformat, Editor, Prüfung, fünf Varianten, Turnier.
4. `just check`, `just selftest`, `just lan-selftest` (um eine Koop-Mission
   erweitert), `just editor-selftest`.
5. Doku: `docs/MISSIONS.md`, `docs/TEAMS.md`, `docs/LAN.md`, `docs/ONLINE.md`,
   `docs/MAPS.md`.
6. Für den Nutzer: Ablauf einer Koop-Partie über zwei lokale Instanzen und die
   Turnierzahlen der Varianten.

## 5. Nicht Teil dieses Prompts

- Plattformübergreifendes Spiel und Matchmaking (Steam-Lobbys reichen vorerst).
- Teilen von Karten und Missionen über den Steam Workshop.
