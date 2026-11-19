# Prompt 099 – Zielvorschau der Formation und Reichweitenkreise

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `prompts/done/046-*.md` (Formationen, Rechtsziehen),
> `crates/sim/src/formation.rs` (`formation_points`, `LINE_WIDTH`, `WEDGE_SIZE`,
> `TWO_FILES_FROM`), `crates/sim/src/sim.rs` (`formation_slots`),
> `crates/bridge/src/orders_funcs.rs`, `crates/bridge/src/hud_funcs.rs` (`unit_info`,
> `weapons[].range_m`), `crates/bridge/src/air_funcs.rs` (`unit_aa_range`),
> `godot/scenes/selection/selection.gd` (`_on_right_drag`, `_draw`),
> `godot/scenes/selection/formation_orders.gd`, `godot/scenes/hud/waypoints.gd`,
> `godot/scenes/hud/supply_rings.gd` (Muster für Kreise am Boden) und
> `godot/scenes/world/artillery_view.gd` (Einschlagmarken, Prompt 084).

## 1. Ausgangslage

Aus der Auswertung des Referenzvideos `docs/.vorlagen/vorlage.mp4` (nicht im Repo,
nur Vorbild für die **Wirkung**, nichts daraus nachbauen oder übernehmen):

- Beim Bewegungsbefehl für mehrere Einheiten erscheint am Ziel **je Einheit eine
  Marke am Boden**. Man sieht vor dem Loslassen, wie die Formation stehen wird.
- Für die ausgewählte Einheit liegt ein **gestrichelter Kreis** für die
  Schussweite am Boden.
- Der Tester bemängelt, dass man die Reichweite eines **angehängten Geschützes**
  nicht sieht: abkoppeln, nachsehen, ankoppeln.

Bei uns heute:

- Die Plätze rechnet nur die Simulation (`formation_points`, `formation_slots`);
  über die Bridge kommen sie nicht heraus. Beim Rechtsziehen zeigt `selection.gd`
  nur einen gelben Pfeil (`FACING_LINE`), `waypoints.gd` zeichnet nach dem Befehl
  Route und Wegpunkte, aber keinen Platz je Einheit.
- Reichweite und Sicht gibt es nur als Text (Einheitenfeld, Einheitenkarte). Es
  gibt keinen Kreis auf der Karte. `unit_aa_range` wird von keinem Skript benutzt.
  Für die Sichtweite einer Einheit gibt es keine Bridge-Funktion.
- Prompt 097 (Abschnitt Darstellung) geht davon aus, dass Reichweiten- und
  Sichtkreise im HUD existieren. Sie entstehen erst hier; Prompt 095 schließt sie
  ausdrücklich aus.

## 2. Ziel

1. **Zielvorschau:** Solange die rechte Maustaste für einen Bewegungs-, Angriffs-
   bewegungs- oder Sichern-Befehl gedrückt ist, liegt am Boden je ausgewählter
   Einheit eine Marke am späteren Platz, ausgerichtet nach der gezogenen
   Blickrichtung. Nach dem Loslassen bleiben die Marken kurz stehen (Richtwert 1 s)
   und blenden aus. Bei einem einfachen Rechtsklick ohne Ziehen erscheinen sie
   ebenfalls kurz.
2. **Reichweitenkreis:** Für die ausgewählten Einheiten liegt die Schussweite der
   Hauptwaffe als dünner Kreis am Boden. Ein zweiter, anders gezeichneter Kreis
   zeigt auf Wunsch die Sichtweite.
3. **Angehängt und aufgesessen:** Ein angehängtes Geschütz zeigt seinen Kreis um
   die Position der Zugmaschine, sichtbar anders gezeichnet (z. B. gestrichelt und
   blasser), weil es so nicht feuern kann. Dasselbe für die Vorschau am Ziel: beim
   Bewegungsbefehl für ein Gespann liegt der Kreis des Geschützes am Zielpunkt.

## 3. Umsetzung

Reine Darstellung. `crates/sim` rechnet nichts Neues, der Golden Hash bleibt.

### 3.1 Plätze aus der Simulation holen

- Eine lesende Bridge-Funktion, die für Einheiten-IDs, Formation, Zielpunkt und
  Blickrichtung die Plätze liefert, **mit derselben Funktion**, die der Befehl
  benutzt. Keine zweite Rechnung in GDScript, sonst stimmen Vorschau und Ergebnis
  nicht überein.
- Nicht befahrbare Plätze (Wasser, Haus, Fels) so anzeigen, wie die Simulation sie
  behandelt. Wenn sie Plätze erst beim Ausführen verschiebt, in der Vorschau die
  unverschobenen zeigen und sie als „gesperrt“ markieren (andere Farbe), nicht
  raten.
- Die Vorschau wird höchstens einmal pro Bild und nur bei geänderter Mausposition
  neu geholt. Obergrenze für die Zahl der Marken prüfen (Auswahl mit 100 Einheiten).

### 3.2 Marken zeichnen

- Schlichte, flache Marken am Boden in der Farbe der eigenen Seite
  (`relation_colors.gd`), der Größe der Einheit grob angepasst (Fahrzeug größer als
  Trupp). Sie folgen dem Gelände und liegen **über** der Nebelschicht (Prompt 098).
- Der gelbe Richtungspfeil bleibt.
- Die Zuglänge beim Rechtsziehen ändert die Breite **nicht** (das wäre ein neuer
  Sim-Parameter). Als Befund nennen, ob sich das lohnt.

### 3.3 Kreise

- Bridge: Schussweite der Hauptwaffe je Einheit gibt es schon (`unit_info`). Für
  die Sichtweite eine kleine Lesefunktion ergänzen, die denselben Wert liefert, den
  `vision.rs` benutzt (mit Wetter und Tageszeit, falls die dort einfließen).
- Für indirektes Feuer Mindest- und Höchstweite als zwei Kreise. Für Flak den Wert
  aus `unit_aa_range` als eigener Kreis, nur bei Flak.
- Viele ausgewählte Einheiten: nicht hundert Kreise übereinander. Regel: Kreise nur
  bis zu einer kleinen Zahl ausgewählter Einheiten (Richtwert 8), darüber nur für
  die aktive Einheit (Prompt 094). Gleiche Kreise an gleicher Stelle
  zusammenfassen.
- Schalter in den Einstellungen (Reiter Interface): Reichweitenkreis
  „immer bei Auswahl / nur mit gedrückter Taste / nie“, Sichtkreis ebenso, Standard
  für Sicht „nur mit Taste“. Eine belegbare Taste dafür in `project.godot` und
  `docs/CONTROLS.md`.
- Der Kreis ist ein Kreis, kein Sichtfeld: verdeckte Bereiche (Wald, Haus, Hang)
  werden hier nicht ausgespart. Das im Ergebnis sagen.

## 4. Regeln

- Kein Godot-Zufall, keine Godot-Physik für Spielrelevantes. Keinen Test
  abschwächen.
- Farben und Strichstärken als Konstanten oben im Skript, lesbar auf Wald, Wüste,
  Schnee, Stadt und bei Nacht.
- Batch-Sessions teilen sich das Arbeitsverzeichnis: vor schweren Builds nachsehen,
  ob andere Prompt-Sessions laufen.

## 5. Vorgehen

1. Bestand lesen, die Plätze-Funktion in der Bridge bauen und mit einem Test
   absichern: Vorschau-Plätze = Zielplätze des ausgeführten Befehls (mehrere
   Formationen, Blickrichtungen, Einheitenzahlen).
2. Marken beim Rechtsdrücken, Rechtsziehen und nach dem Klick.
3. Kreise für Schussweite, Sicht, indirektes Feuer, Flak; Sonderfall Gespann.
4. Einstellungen und Taste.
5. Selftest erweitern: Vorschau erscheint und verschwindet; Zahl der Marken =
   Zahl der Einheiten; Kreisradius = Wert aus `unit_info`; Gespann zeigt den Kreis
   des Geschützes.
6. `just check`, `just selftest`, `just lan-selftest`; `just bench-view` mit großer
   Auswahl vorher/nachher.
7. Doku: `docs/CONTROLS.md`, `docs/ARCHITECTURE.md`. In `prompts/097-*.md` bzw.
   seinem Ergebnis den Verweis auf die Kreise richtigstellen, falls 097 noch offen
   ist.
8. Für den Nutzer: Bilder der Vorschau (Linie, Keil, Kolonne) und der Kreise auf
   zwei Kartenlooks.

## 6. Nicht Teil dieses Prompts

- Breite der Formation über die Zuglänge (neuer Sim-Parameter).
- Echtes Sichtfeld mit Verdeckung als Fläche am Boden.
- Änderungen an Formationen, Reichweiten oder Sichtweiten selbst.
