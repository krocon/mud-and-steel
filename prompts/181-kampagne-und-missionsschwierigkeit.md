# Prompt 181 – Kampagne, Schwierigkeit je Mission und Missionen aus beiden Blickwinkeln

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach 180 und 179.
> Lies zuerst `CLAUDE.md`, `docs/MISSIONS.md` ganz (vor allem „Open“: Kampagne,
> Bewertungen, Zeitgeber), `docs/AI.md` („Difficulties“), `docs/GAME_DESIGN.md`,
> die vier Missionen unter `data/maps/*/missions/`, `crates/sim/src/mission.rs`,
> `crates/bridge/src/mission_funcs.rs` und das Missionsmenü unter `godot/scenes/`.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) lobt eine Kampagne mit vielen Einsätzen, unterschiedlichen
Aufgaben (Angriff, Verteidigung, Durchbruch, Rückzug, Nachschub sichern) und
derselben Schlacht aus beiden Blickwinkeln. Kritisiert wird: zu steile
Schwierigkeit schon auf mittlerer Stufe, und frühe Fehler machen eine Mission
unbemerkt unschaffbar.

Bei uns: vier Einzelmissionen (Tutorial, Wiesenübergang, Dorfplatz, Stadt im
Schnee), alle „einnehmen und Gegenangriff abwehren“. Die Schwierigkeit steckt
nur in der festen Stufe des Computerspielers im Skript. Keine Reihenfolge, keine
Bewertung, kein Zeitgeber im Panel.

## 2. Ziel

### 2.1 Schwierigkeit je Mission

- Wahl **Leicht / Normal / Schwer** beim Start einer Mission. Das Skript kann
  je Stufe mehrere Stellschrauben setzen (alles Daten, kein Code je Mission):
  Stufe des Computerspielers, Stärke und Zeitpunkt der gegnerischen Verstärkung,
  eigene Verstärkung, Zeitgrenzen, Startvorrat; Sichtweite nur, wenn das ohne
  Regeländerung geht.
- Neuer Skript-Abschnitt (z. B. `[difficulty.easy]`), dokumentiert in
  `docs/MISSIONS.md`; die vier Missionen bekommen sinnvolle Werte.
- **Warnung vor dem Unschaffbaren:** Ein Skript kann eine Bedingung „Mission kaum
  noch zu gewinnen“ formulieren (z. B. weniger als N Panzer und Ziel noch offen);
  dann sagt die Missionsmeldung das deutlich und bietet „Neu starten / Letzten
  Spielstand laden“ an, statt dass der Spieler 10 Minuten weiterkämpft.

### 2.2 Kampagne

- Eine Datei `data/campaigns/<id>.toml` mit Reihenfolge, Freischalten (nächste
  Mission nach Sieg), Zwischentexten. Menü „Kampagne“ mit Fortschritt; die
  Einzelmissionen bleiben im Menü „Missionen“.
- Bewertung nach Sieg (z. B. Zeit, eigene Verluste, Nebenziele) mit
  Stern/Abzeichen, gespeichert in `user://missions.cfg`. Kein Einfluss auf
  spätere Missionen (keine Kernstreitmacht) in diesem Prompt.

### 2.3 Neue Aufgabenprofile

- Mindestens **zwei neue Missionen** mit Aufgaben, die es noch nicht gibt, auf
  vorhandenen Karten: eine **Verteidigung** mit Zeitgeber (halten bis die
  Verstärkung kommt) und ein **Rückzug/Durchbruch** (Einheiten zum Kartenrand
  bringen). Eine davon als **Gegenstück** einer vorhandenen Mission aus der
  anderen Sicht (z. B. Wiesenübergang aus deutscher Sicht), mit eigenen
  Startpositionen und Zielen, nicht nur gespiegelt.
- Fehlende Skript-Bausteine (Zeitgeber im Panel, „Einheiten erreichen Gebiet“
  als Zählziel, Nebenziele) ergänzen.

## 3. Regeln

- Missionen und Kampagne sind Daten; Texte Deutsch und Englisch.
- Simulation deterministisch; Golden Hash unverändert (nutzt keine Mission).
- Keine historischen Einheitennamen oder Abzeichen; frei erfundene Lagen.
- Missionen werden mit dem Selbsttest durchgespielt (mindestens: Start, ein Ziel,
  Ergebnis).

## 4. Vorgehen

1. Bestand lesen, Entwurf (Stellschrauben, Kampagnenformat, zwei Missionsideen)
   dem Nutzer vorlegen.
2. Schwierigkeit je Mission und Warnung, mit Tests.
3. Kampagnenformat, Menü, Bewertung.
4. Zwei neue Missionen.
5. `just check`, `just selftest`.
6. Doku: `docs/MISSIONS.md`.
7. Für den Nutzer: Spieldauer und Ergebnis jeder Mission auf allen drei Stufen
   gegen eine einfache Teststeuerung, wo das automatisch geht.
