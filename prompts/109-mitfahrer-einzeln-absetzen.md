# Prompt 109 – Mitfahrer im Infofeld zeigen und einzeln absetzen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `docs/INFANTRY.md` (Häuser besetzen und verlassen),
> `prompts/done/034-*.md` (Transport), `crates/sim/src/command.rs`
> (`Command::Disembark { units }`), `crates/sim/src/transport.rs` (um Zeile
> 307–325: einzelne Mitfahrer-IDs werden schon angenommen),
> `crates/bridge/src/transport_funcs.rs` (`disembark_units`),
> `godot/scenes/selection/transport_orders.gd` (`disembark`, Taste J),
> `godot/scenes/selection/selection.gd` (`_selectable_index`, `_own_indices`),
> `godot/scenes/hud/unit_panel.gd` (Zeile „Passengers 9 / 10“, Kacheln für mehrere
> Trupps in einem Haus) und `prompts/095-hud-neu-anordnen.md`.

## 1. Ausgangslage

Im Test zum Referenzvideo (`docs/.vorlagen/vorlage.mp4`, nur als Anregung) ärgert
sich der Tester über kleinteilige Bedienung: Will er **einen** Sanitäter aus einem
voll besetzten Graben holen, müssen alle aussteigen. Das kostet Zeit und bringt die
anderen in Gefahr.

Bei uns heute:

- Die Simulation kann es schon: `Disembark` nimmt einzelne Mitfahrer-IDs an.
- Die Bedienung kann es nicht: Taste J schickt nur die Fahrzeug-ID, also steigen
  alle aus. Mitfahrer sind nicht auswählbar; das Infofeld zeigt nur
  „Passengers 9 / 10“.
- Häuser: Trupps bleiben auswählbar und verlassen das Haus einzeln per
  Bewegungsbefehl. Gräben sind kein Behälter. Dort besteht das Problem nicht.

## 2. Ziel

- Ist ein Transporter (oder ein anderes Fahrzeug mit Mitfahrern) ausgewählt, zeigt
  das Infofeld **je Mitfahrer bzw. Trupp eine Kachel** mit Bild, Art und Zustand
  (Stärke, verwundet).
- **Klick auf eine Kachel** setzt genau diesen Trupp ab (er steigt an der üblichen
  Stelle aus und ist danach ausgewählt, das Fahrzeug bleibt in der Auswahl oder
  nicht: die einfachere, vorhersehbare Variante nehmen und nennen).
  Umschalt+Klick sammelt mehrere.
- Taste J setzt weiter **alle** ab.
- Gezogene Geschütze erscheinen ebenfalls als Kachel (abkoppeln per Klick), wenn
  das mit demselben Befehl geht; sonst weglassen und nennen.
- Bei mehreren ausgewählten Transportern: Kacheln nach Fahrzeug gruppiert oder nur
  für die aktive Einheit (Prompt 094). Einfachste lesbare Lösung.
- Tooltip der Kachel nennt die Aktion. Gestaltung passend zu Prompt 095 (prüfen,
  ob umgesetzt).

## 3. Regeln

- Nur Bedienung und Darstellung. Kein neuer Befehl, `crates/sim` und der Golden
  Hash bleiben. Mehrspieler: Der Befehl läuft wie jeder andere über Lockstep.
- Gegnerische Transporter zeigen keine Mitfahrer.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Bestand lesen; prüfen, was die Bridge über Mitfahrer liefert (IDs, Art,
   Stärke), fehlende Lesefunktion ergänzen.
2. Kacheln, Klick, Umschalt+Klick.
3. Selftest: Transporter mit drei Trupps, Klick auf Kachel 2 → genau dieser
   Trupp steht draußen, zwei sitzen noch; J → alle draußen.
4. `just check`, `just selftest`, `just lan-selftest`.
5. Doku: `docs/CONTROLS.md`.
6. Für den Nutzer: ein Bild des Infofelds mit Kacheln.

## 5. Nicht Teil dieses Prompts

- Trupps in einzelne Soldaten teilen (gibt es nicht; eigener Entwurf, falls
  gewünscht: Sanitäter oder Scharfschütze aus dem Trupp lösen).
- Mitfahrer aus dem Fahrzeug heraus feuern lassen.
