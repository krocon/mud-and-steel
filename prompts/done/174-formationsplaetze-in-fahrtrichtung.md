# Prompt 174 – Formationsplätze in Fahrtrichtung: Mitglieder kreuzen sich nicht mehr

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach 173 (dessen
> Prüfstand misst auch hier). Lies zuerst `CLAUDE.md`, `docs/DETERMINISM.md`, in
> `docs/ARCHITECTURE.md` „Players, commands and formation“, „Formations,
> attack-move and securing“, „Pathfinding“ (vor allem „Groups and formation“,
> „Open“), `prompts/done/046-*.md`, `prompts/done/099-*.md`,
> `crates/sim/src/formation.rs` und `godot/scenes/hud/order_preview.gd`. Fremde,
> nicht eingecheckte Änderungen an `crates/sim`: erst den Nutzer fragen.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) kritisiert: Verbände lösen sich auf, Einheiten blockieren sich
gegenseitig. Bei uns steht in `docs/ARCHITECTURE.md` („Pathfinding → Open“):
Plätze werden nach y, dann x vergeben, nicht entlang der Fahrtrichtung. Bei
schrägen Befehlen kreuzen sich die Mitglieder beim Neuordnen, schieben sich
gegenseitig und kommen ungeordnet an.

Dieser Prompt ist bewusst klein. Gleichschritt und Kolonne an Engstellen sind
Prompt 187.

## 2. Ziel

- Plätze der Formation so vergeben, dass sich die Wege der Mitglieder möglichst
  nicht kreuzen: z. B. Mitglieder und Plätze beide entlang der Fahrtrichtung und
  quer dazu sortieren und der Reihe nach zuordnen. Deterministisch, total
  geordnet (Id als letzter Schlüssel), ohne teure Optimierung; Kosten messen.
- Die Zielvorschau aus Prompt 099 zeigt dieselben Plätze (dieselbe Funktion,
  nicht nachgebaut).
- Prüfszenario: acht gemischte Einheiten, Befehl schräg (45°, 135°) und quer zur
  Aufstellung über offenes Feld. Messen: Kreuzungen der Wege, Zeit bis alle
  stehen, Ankunftsspanne. Vorher und nachher.

## 3. Regeln

- Simulation: feste Kommazahlen, kein Zufall, keine `HashMap`-Iteration. Golden
  Hash ändert sich beabsichtigt: in der Commit-Nachricht sagen.
- `just path-bench` und `just bench` vorher und nachher; Budgets halten.
- `just ai-tournament`: prüfen, ob es sich verschiebt, Zahlen nennen.

## 4. Vorgehen

1. Prüfszenario, Vorher-Zahlen.
2. Zuordnung mit Tests (Determinismus, keine Kreuzung in den Prüffällen).
3. `just check`, `just selftest`, `just lan-selftest`, `just ai-tournament`.
4. Doku: `docs/ARCHITECTURE.md` („Open“ aktualisieren).
5. Für den Nutzer: Vorher/Nachher-Zahlen und ein Bild der Zielvorschau bei einem
   schrägen Befehl.

## 5. Nicht Teil dieses Prompts

- Gleichschritt, Kolonne an Engstellen (Prompt 187).
- Lokales Ausweichen nach RVO (bewusst nicht, siehe „Decisions“).
