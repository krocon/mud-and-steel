# Prompt 188 – Zielwahl nach Wirkung, Bedrohung und Rolle

> In einer frischen Session im Repo-Ordner ausführen. **Braucht 176**
> (Feuerdisziplin, dieselben Dateien). Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `docs/PERFORMANCE.md`, in `docs/ARCHITECTURE.md`
> „Combat“ (vor allem „Decisions“: Ziele, Schadensmodell), „Indirect fire and
> smoke“, dazu `docs/INFANTRY.md`, `docs/BALANCE.md`,
> `crates/sim/src/sim/combat.rs` (`choose_target`, Zielwechsel alle 10 Ticks),
> `crates/sim/src/combat.rs` (`can_damage`, Durchschlag), `data/combat.toml`.
> Fremde, nicht eingecheckte Änderungen an `crates/sim`: erst den Nutzer fragen.

## 1. Ausgangslage

Aufgeteilt aus dem früheren Prompt 176 (2026-11-21). Die Testauswertung zu einem
anderen WW2-Taktikspiel (nur Anregung) kritisiert Einheiten, die unpassend
angreifen, und Artillerie, die auf ferne unbedeutende Ziele feuert.

`choose_target` wählt heute nach festem Schlüssel: Hauptwaffe wirkt (ja/nein),
Ziel kann mir schaden, dann das nächste, dann die kleinste Id. Folgen:

- Ein Pak-Geschütz, das einen Lkw durchschlagen kann, schießt auf den nahen Lkw
  statt auf den Panzer daneben.
- Ein Panzer verschwendet Sprenggranaten auf einen einzelnen Mann statt auf das
  Pak.

Das berührt Balance und Laufzeit; deshalb ein eigener Prompt mit Messung.

## 2. Ziel

- Ersetze ja/nein durch eine **ganzzahlige Wertung** je Kandidat, aus Größen, die
  die Simulation schon kennt: Wirkung meiner besten Waffe auf das Ziel
  (Durchschlag gegen Panzerung aus meiner Richtung, Schaden je Treffer), Bedrohung
  des Ziels für mich, Entfernung, ob es schon beschossen wird (einfache Regel für
  Bündeln vs. Verteilen festlegen) und eine kurze Bindung an das aktuelle Ziel
  (kein Springen bei jeder Prüfung).
- Rollen aus der Kategorie: Panzerabwehr bevorzugt gepanzerte Ziele, MG und
  Infanterie bevorzugen weiche Ziele, Panzer wählen zwischen Panzer und
  Panzerabwehr nach Bedrohung.
- Gewichte in `data/combat.toml` mit Kommentar, deterministisch, total geordnet
  (Id als letzter Schlüssel). Die Stufen aus 176 (erwidern, nur Fahrzeuge,
  Eröffnungsentfernung) filtern vor der Wertung.
- Indirekte Waffen feuern ohne Befehl nicht auf Ziele jenseits einer Grenze und
  nicht auf Einzelschützen (Grenzen in Daten); erst prüfen und dokumentieren, was
  heute passiert.

## 3. Regeln

- Feste Kommazahlen, kein Zufall, keine `HashMap`-Iteration; Golden Hash ändert
  sich beabsichtigt (Commit-Nachricht).
- Laufzeit: `choose_target` läuft für viele Einheiten; `just bench` vorher und
  nachher, keine spürbare Verschlechterung (Zahl nennen).
- Balance: `just ai-tournament` und der Gefechtssimulator (Prompt 045) vorher und
  nachher; größere Verschiebungen nennen, nicht wegtunen ohne Rückfrage.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Prüfszenarien: Pak neben Lkw und Panzer; Panzer gegen Pak + Infanterie; MG
   gegen Trupp + Halbkette; Mörser ohne Befehl mit fernem Einzelschützen.
   Vorher-Ergebnis festhalten.
2. Zielwertung mit Tests (je Rolle ein Test, Determinismus, keine Zuckungen beim
   Zielwechsel).
3. Indirekte Waffen.
4. `just ai-tournament`, `just check`, `just selftest`, `just lan-selftest`,
   `just bench`.
5. Doku: `docs/ARCHITECTURE.md` (Combat → Decisions), `docs/BALANCE.md`,
   `docs/AI.md`.
6. Für den Nutzer: Ergebnis der Szenarien und Turnierzahlen vorher/nachher.
