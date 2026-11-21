# Prompt 174 – Verbände fahren gemeinsam: Gleichschritt, Plätze in Fahrtrichtung, Engstellen nacheinander

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach 173 (dessen
> Prüfstand misst auch hier). Lies zuerst `CLAUDE.md`, `docs/DETERMINISM.md`, in
> `docs/ARCHITECTURE.md` „Players, commands and formation“, „Formations,
> attack-move and securing“, „Pathfinding“ (vor allem „Groups and formation“,
> „Movement and separation“, „Open“), `prompts/done/046-*.md`,
> `prompts/done/099-*.md`, `crates/sim/src/formation.rs`, `crates/sim/src/path.rs`
> und die Bewegung in `crates/sim/src/sim.rs`. Fremde, nicht eingecheckte
> Änderungen an `crates/sim`: erst den Nutzer fragen.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) lobt Gruppensteuerung und Formationen, kritisiert aber: Verbände
lösen sich an kleinen Hindernissen auf, Einheiten blockieren sich gegenseitig,
große Schlachten werden zur Folge von Einzelbefehlen.

Bei uns stehen in `docs/ARCHITECTURE.md` („Pathfinding → Open“) genau diese
Lücken:

- Plätze werden nach y, dann x vergeben, nicht entlang der Fahrtrichtung: bei
  schrägen Befehlen kreuzen sich die Mitglieder beim Neuordnen.
- Kein Gleichschritt: jedes Mitglied fährt mit seiner eigenen
  Geländegeschwindigkeit; Kettenfahrzeuge, Lkw und Infanterie kommen weit
  auseinandergezogen an, die schnellsten zuerst und allein.
- An Brücken und Engstellen drängen alle gleichzeitig hinein; das Auflösen
  übernimmt die Trennung (Schieben und Ausweichen) und die Festfahr-Regeln.

## 2. Ziel

### 2.1 Plätze in Fahrtrichtung

- Plätze der Formation so vergeben, dass sich die Wege der Mitglieder möglichst
  nicht kreuzen (z. B. beide Mengen entlang der Fahrtrichtung und quer dazu
  sortieren und der Reihe nach zuordnen). Deterministisch, ohne teure
  Optimierung; Kosten messen.
- Die Zielvorschau aus Prompt 099 zeigt dieselben Plätze.

### 2.2 Gleichschritt

- Ein Gruppenbefehl (Bewegen, Angriffsbewegung, Sichern) fährt im Verband: das
  Tempo richtet sich nach dem **langsamsten Mitglied im jeweiligen Gelände**, wer
  vorne zu weit vorauskommt, wartet bzw. fährt langsamer; Nachzügler holen auf.
- Abschaltbar je Befehl: Befehl „Bewegen“ ohne Verband (heute M) bleibt, oder eine
  Taste/Einstellung, die der Nutzer festlegt (Vorschlag in `docs/CONTROLS.md`,
  vorläufig).
- Ausnahme: Einheiten unter Beschuss oder im Gefecht verlassen den Gleichschritt.

### 2.3 Engstellen nacheinander

- Liegt auf dem Gruppenweg eine Engstelle (Brückendeck, Furt, Durchlass schmaler
  als die Formation), reihen sich die Mitglieder **in Kolonne** davor ein und
  fahren in der Reihenfolge ihres Abstands hindurch; danach nehmen sie die
  Formation wieder ein.
- Kein Stau vor dem Deck: wer warten muss, wartet außerhalb des Weges der anderen.

## 3. Regeln

- Simulation: feste Kommazahlen, kein Zufall, keine `HashMap`-Iteration. Golden
  Hash ändert sich beabsichtigt: in der Commit-Nachricht sagen.
- Werte (Vorauslauf, Warteabstand, Engstellen-Breite) in einer Datei unter
  `data/`, nicht im Code.
- `just path-bench` und `just bench` vorher und nachher; Budgets halten.
- KI nutzt dieselben Befehle; prüfen, ob `just ai-tournament` sich verschiebt, und
  Zahlen nennen.

## 4. Vorgehen

1. Prüfszenarien: acht gemischte Einheiten schräg über offenes Feld, über eine
   Brücke, durch einen Waldweg; messen: Ankunftsspanne (erste bis letzte),
   Kreuzungen, Zeit bis alle angekommen sind, Festfahrer (Prüfstand aus 173, falls
   da).
2. 2.1, 2.2, 2.3 nacheinander, je mit Tests und neuen Zahlen.
3. `just check`, `just selftest`, `just lan-selftest`, `just ai-tournament`.
4. Doku: `docs/ARCHITECTURE.md` („Open“ aktualisieren), `docs/CONTROLS.md`.
5. Für den Nutzer: Vorher/Nachher-Zahlen und ein kurzer Clip der Brückenüberfahrt.

## 5. Nicht Teil dieses Prompts

- Lokales Ausweichen nach RVO (bewusst nicht, siehe „Decisions“).
- Flussfelder für Massenbefehle.
