# Prompt 176 – Zielwahl nach Rolle und Bedrohung, Feuerdisziplin für Hinterhalte

> In einer frischen Session im Repo-Ordner ausführen. **Braucht 110** (Reaktion
> auf Beschuss von der Seite; arbeitet in denselben Dateien). Lies zuerst
> `CLAUDE.md`, `docs/DETERMINISM.md`, in `docs/ARCHITECTURE.md` „Combat“ (vor
> allem „Decisions“: Ziele, Schadensmodell), „Indirect fire and smoke“, „Vision
> and fog of war → Ambush“, `docs/GAME_DESIGN.md` („Ambush“, „Fire from the
> flank“), `docs/INFANTRY.md`, `crates/sim/src/sim/combat.rs` (`choose_target`,
> Zielwechsel alle 10 Ticks), `crates/sim/src/combat.rs`, `data/combat.toml`,
> `data/tactics.toml`. Fremde, nicht eingecheckte Änderungen an `crates/sim`:
> erst den Nutzer fragen.

## 1. Ausgangslage

Die Testauswertung zu einem anderen WW2-Taktikspiel (Nutzer-Datei, 2026-11-21,
nur Anregung) kritisiert Einheiten, die unpassend angreifen, Artillerie, die auf
ferne unbedeutende Ziele feuert, und Besatzungen, die auf Beschuss nicht
reagieren. Folge: zu viel Mikromanagement.

Bei uns wählt `choose_target` nach festem Schlüssel: Hauptwaffe wirkt, dann Ziel
kann mir schaden, dann das nächste, dann die kleinste Id. Das ist einfach und
deterministisch, aber:

- „Hauptwaffe wirkt“ ist ja/nein. Ein Pak-Geschütz, das einen Lkw durchschlagen
  kann, schießt auf den nahen Lkw statt auf den Panzer daneben; ein Panzer
  verschwendet Sprenggranaten auf einen einzelnen Mann statt auf das Pak.
- Es gibt nur „Feuer frei“ und „Feuerverbot“ (Taste F). Ein Hinterhalt nach
  `docs/GAME_DESIGN.md` braucht aber: **erst feuern, wenn das Ziel nah genug ist
  oder ein Panzer kommt**, sonst verrät sich die Stellung an der ersten Spitze.
- Unbewaffnete Fahrzeuge (Lkw, Zugmaschinen) bleiben unter Beschuss stehen, wenn
  sie keinen Befehl haben.

## 2. Ziel

### 2.1 Zielwahl (Simulation)

- Ersetze ja/nein durch eine **ganzzahlige Wertung** je Kandidat, aus Größen, die
  die Simulation schon kennt: Wirkung meiner besten Waffe auf das Ziel
  (Durchschlag gegen Panzerung aus meiner Richtung, Schaden je Treffer), Bedrohung
  des Ziels für mich und für Verbündete in der Nähe, Entfernung, ob es schon
  beschossen wird (Feuer verteilen vs. bündeln: einfache Regel festlegen), und
  eine kurze Bindung an das aktuelle Ziel (kein Springen bei jeder Prüfung).
- Rollen aus der Kategorie: Panzerabwehr bevorzugt gepanzerte Ziele,
  MG und Infanterie bevorzugen weiche Ziele, Flak bevorzugt Luftziele (wo es die
  gibt), Panzer wählen zwischen Panzer und Panzerabwehr nach Bedrohung.
- Gewichte in `data/combat.toml` mit Kommentar, deterministisch, total geordnet
  (Id als letzter Schlüssel).
- Indirekte Waffen feuern ohne Befehl nicht auf Ziele jenseits einer Grenze und
  nicht auf Einzelschützen (Grenzen in Daten); prüfen, was heute passiert, und
  dokumentieren.

### 2.2 Feuerdisziplin (neuer Befehl)

- Drei Stufen statt zwei: **Feuer frei**, **Feuer erwidern** (schießt nur auf
  Einheiten, die sie oder ihren Trupp beschossen haben oder die sie entdeckt
  haben), **Feuerverbot**. Dazu für „Feuer frei“ eine optionale
  **Eröffnungsentfernung** (Befehl mit Klick oder fest: halbe Reichweite) und
  „nur auf Fahrzeuge“ für Panzerabwehr.
- Neuer `Command`-Inhalt, gilt für Spieler und KI gleich; Taste F schaltet weiter
  um (Belegung in `docs/CONTROLS.md`), Zustandsmarke und Kachel zeigen die Stufe.
- Zusammen mit der Tarnung und dem Hinterhalt aus Prompt 056 prüfen: Wer nicht
  feuert, bleibt verborgen.

### 2.3 Unbewaffnete unter Feuer

- Unbewaffnete Fahrzeuge ohne Befehl, die getroffen werden, fahren aus der
  Schusslinie (kurzer Rückzug weg von der Beschussrichtung aus Prompt 110), statt
  stehenzubleiben. Einstellbar abschaltbar, Werte in `data/tactics.toml`.

## 3. Regeln

- Feste Kommazahlen, kein Zufall, keine `HashMap`-Iteration; Golden Hash ändert sich
  beabsichtigt (Commit-Nachricht).
- Laufzeit: `choose_target` läuft für viele Einheiten; `just bench` vorher/nachher,
  keine spürbare Verschlechterung.
- Die KI nutzt die neuen Stufen über normale `Command`s (Hinterhalte der
  Verteidiger, Panzerabwehr „nur auf Fahrzeuge“), mindestens in einem einfachen
  Fall; mehr als Befund.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Prüfszenarien: Pak neben Lkw und Panzer; Panzer gegen Pak + Infanterie; MG gegen
   Trupp + Halbkette; Hinterhalt im Wald gegen eine Kolonne mit Spähwagen vorne.
   Vorher-Ergebnis festhalten.
2. Zielwertung mit Tests (je Rolle ein Test, Determinismus, keine Zuckungen).
3. Feuerdisziplin: Befehl, Simulation, Bridge, HUD, Taste.
4. Unbewaffnete Fahrzeuge.
5. `just ai-tournament`, `just check`, `just selftest`, `just lan-selftest`,
   `just bench`.
6. Doku: `docs/ARCHITECTURE.md` (Combat), `docs/GAME_DESIGN.md` (Entscheidung,
   vorläufig, mit Datum), `docs/CONTROLS.md`, `docs/AI.md`, `docs/BALANCE.md`.
7. Für den Nutzer: Ergebnis der vier Szenarien vorher/nachher.
