# Prompt 189 – Kampagne mit Bewertung, neue Aufgaben und Missionen aus beiden Blickwinkeln

> In einer frischen Session im Repo-Ordner ausführen. **Braucht 181**
> (Schwierigkeit je Mission), empfohlen nach 179 und 180. Lies zuerst
> `CLAUDE.md`, `docs/MISSIONS.md` ganz (vor allem „Open“: Kampagne, Bewertungen,
> Zeitgeber), `docs/GAME_DESIGN.md`, die Missionen unter `data/maps/*/missions/`,
> `crates/sim/src/mission.rs`, `crates/bridge/src/mission_funcs.rs` und das
> Missionsmenü unter `godot/scenes/`.

## 1. Ausgangslage

Aufgeteilt aus dem früheren Prompt 181 (2026-11-21). Die Testauswertung zu einem
anderen WW2-Taktikspiel (nur Anregung) lobt eine Kampagne mit vielen Einsätzen,
unterschiedlichen Aufgaben (Angriff, Verteidigung, Durchbruch, Rückzug, Nachschub
sichern) und derselben Schlacht aus beiden Blickwinkeln.

Bei uns: vier Einzelmissionen, alle „einnehmen und Gegenangriff abwehren“. Keine
Reihenfolge, keine Bewertung, kein Zeitgeber im Panel.

## 2. Ziel

### 2.1 Kampagne

- Eine Datei `data/campaigns/<id>.toml` mit Reihenfolge, Freischalten (nächste
  Mission nach Sieg), Zwischentexten. Menü „Kampagne“ mit Fortschritt; die
  Einzelmissionen bleiben im Menü „Missionen“.
- Bewertung nach Sieg (z. B. Zeit, eigene Verluste, Nebenziele) mit
  Stern/Abzeichen je Stufe aus 181, gespeichert in `user://missions.cfg`. Kein
  Einfluss auf spätere Missionen (keine Kernstreitmacht).

### 2.2 Neue Aufgabenprofile

- Mindestens **zwei neue Missionen** mit Aufgaben, die es noch nicht gibt, auf
  vorhandenen Karten: eine **Verteidigung** mit Zeitgeber (halten bis die
  Verstärkung kommt) und ein **Rückzug/Durchbruch** (Einheiten zum Kartenrand
  bringen). Eine davon als **Gegenstück** einer vorhandenen Mission aus der
  anderen Sicht (z. B. Wiesenübergang von der Gegenseite), mit eigenen
  Startpositionen und Zielen, nicht nur gespiegelt.
- Fehlende Skript-Bausteine (Zeitgeber im Panel, „Einheiten erreichen Gebiet“ als
  Zählziel, Nebenziele) ergänzen.

## 3. Regeln

- Missionen und Kampagne sind Daten; Texte Deutsch und Englisch.
- Simulation deterministisch; Golden Hash unverändert (nutzt keine Mission).
- Keine historischen Einheitennamen oder Abzeichen; frei erfundene Lagen.
- Missionen werden mit dem Selbsttest durchgespielt (mindestens: Start, ein Ziel,
  Ergebnis).

## 4. Vorgehen

1. Bestand lesen, Entwurf (Kampagnenformat, Bewertung, zwei Missionsideen) dem
   Nutzer vorlegen.
2. Fehlende Skript-Bausteine mit Tests.
3. Kampagnenformat, Menü, Bewertung.
4. Zwei neue Missionen mit Werten für alle drei Stufen.
5. `just check`, `just selftest`.
6. Doku: `docs/MISSIONS.md`.
7. Für den Nutzer: Spieldauer und Ergebnis der neuen Missionen auf allen drei
   Stufen, wo das automatisch geht.
