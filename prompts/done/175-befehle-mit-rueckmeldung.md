# Prompt 175 – Befehle mit Rückmeldung: was die Einheit gerade tut und warum nicht

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, in `docs/ARCHITECTURE.md` „Players, commands and
> formation“, „Pathfinding → Decisions“ (unerreichbare Ziele), „HUD“ mit „Unit
> values in the interface“ und „Selection“, dazu `prompts/done/099-*.md`,
> `prompts/done/101-*.md`, `prompts/done/109-*.md`, `godot/scenes/hud/unit_panel.gd`,
> `godot/scenes/hud/state_marks.gd`, `godot/scenes/hud/order_preview.gd`,
> `crates/sim/src/snapshot.rs` und `crates/sim/src/command.rs`.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) fordert: „Befehle müssen nachvollziehbar ausgeführt werden;
Probleme und ungültige Befehle müssen sichtbar sein.“ Spieler dort wussten oft
nicht, warum eine Einheit nicht fährt, nicht schießt oder nicht einsteigt.

Bei uns:

- Unerreichbare Ziele werden still durch den nächsten Punkt der eigenen
  Zusammenhangskomponente ersetzt. Der Spieler sieht das erst, wenn die Einheit
  am anderen Flussufer stehen bleibt.
- Befehle, die die Simulation verwirft (Ziel außer Reichweite, zu wenig Plätze
  zum Aufsitzen, Zugmaschine zu schwach, Feuerverbot aktiv, keine Munition,
  Geschütz nicht abgeprotzt …), verschwinden ohne Hinweis.
- Das Infofeld zeigt Werte und Zustand, aber nicht die **aktuelle Tätigkeit**
  („fährt zu …“, „wartet auf Brücke“, „feuert auf Panzer IV“, „sucht Deckung“,
  „niedergehalten“, „kein Ziel in Sicht“).

## 2. Ziel

### 2.1 Beim Befehl

- Zielvorschau (099): Ist das Ziel für eine Einheit nicht erreichbar, zeigt die
  Vorschau den **Ersatzpunkt** in eigener Farbe und eine kurze Linie zum
  angeklickten Ziel; Hinweistext „Ziel nicht erreichbar – fährt bis hierher“.
  Dazu braucht die Bridge eine Leseabfrage für den Ersatzpunkt (reine Abfrage,
  ändert nichts).
- Verworfene Befehle: die Simulation meldet den Grund als Ereignis mit Code
  (keine Texte in `sim`); die Darstellung zeigt einen kurzen Hinweis am Mauszeiger
  bzw. im Ereignis-Feed und einen dezenten Ton. Gründe vollständig auflisten und
  in `docs/CONTROLS.md` dokumentieren.
- Vor dem Befehl erkennbar: Befehlskacheln, die für die ganze Auswahl nichts
  bewirken würden, sind gedimmt, ihr Tooltip nennt den Grund.

### 2.2 Während der Ausführung

- Neue Zeile „Tätigkeit“ im Infofeld der aktiven Einheit, bei Mehrfachauswahl
  zusammengefasst („5 fahren, 2 im Gefecht, 1 kommt nicht weiter“).
- Grund, warum eine Einheit **nicht feuert**, wenn ein Gegner bekannt ist: keine
  Sicht, außer Reichweite, Waffe wirkungslos gegen das Ziel, Feuerverbot,
  Nachladen, außerhalb des Schwenkbereichs, niedergehalten. Das braucht die
  Snapshot-Seite der Simulation; was die Simulation schon weiß, wird gelesen,
  nicht nachgerechnet.
- Tooltip auf feindlichen Einheiten bei ausgewählten eigenen: „kann
  durchschlagen / nur von der Seite / nicht“ als einfache Ampel (Grundlage:
  dieselbe Formel wie die Trefferrechnung, nur gelesen). Das war in Prompt 066
  als „Vorhersage beim Zielen“ offen geblieben.

## 3. Regeln

- Regeln in `crates/sim` ändern sich nicht. Ereignisse und Snapshot-Felder dürfen
  dazukommen, sind aber nicht Teil des Zustands-Hashs; falls doch, begründen.
- Godot zeigt nur an; keine eigene Logik, die etwas entscheidet.
- Texte über die vorhandene Übersetzung (Deutsch/Englisch, Prompt 043).
- Lesbar auf großen Bildschirmen (Prompt 092), mit Farbschwäche-Paletten.

## 4. Vorgehen

1. Bestand lesen; Liste aller Stellen, an denen die Simulation Befehle verwirft
   oder umdeutet.
2. Ereignis-Codes und Snapshot-Felder mit Tests.
3. Darstellung: Vorschau, Hinweise, Tätigkeitszeile, Ampel.
4. Selbsttest um je einen verworfenen Befehl und ein unerreichbares Ziel
   erweitern.
5. `just check`, `just selftest`, `just lan-selftest`.
6. Doku: `docs/CONTROLS.md`, `docs/ARCHITECTURE.md` (HUD).
7. Für den Nutzer: Bilder vorher/nachher (unerreichbares Ziel, Infofeld im
   Gefecht, Ampel).

## 5. Nicht Teil dieses Prompts

- Festfahr-Meldung (Prompt 173).
- Gefechtsbericht nach dem Spiel (Prompt 179).
