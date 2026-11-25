# Prompt 211 – Wegfindungs-Budget abdichten und 20–31 ms Tick-Spitzen beseitigen

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach Prompt 210.
> Lies zuerst `CLAUDE.md`, `docs/DETERMINISM.md`, `docs/ARCHITECTURE.md` („Pathfinding“)
> und `docs/PERFORMANCE.md` (Abschnitt „Coastal heights“, Zeilen 522–537). Im Code:
> `crates/sim/src/path.rs`, `crates/sim/src/sim.rs` (`plan_paths`).
> **Achtung Regeländerung:** Da `NODE_BUDGET_PER_TICK` die Tick-Zuordnung fertiger Pfade
> bestimmt, kann das Anrechnen bisher unbudgetierter Arbeit in `path.rs` den Zeitpunkt
> beeinflussen, zu dem ein Pfad fertig wird – und damit Golden Hashes ändern.
> Nur wenn sich ein Golden Hash ändert: genau prüfen und belegen warum, und im Commit
> klar dokumentieren (`CLAUDE.md`).

## 1. Problem

In `docs/PERFORMANCE.md:522-537` verletzt die Wegfindung auf `coastal_heights` das
Tick-Maximum-Budget von 20 ms mit Spitzen von **20–31,3 ms** (`paths 30,1 ms max`), und
lange Suchen über offene/zerklüftete Karten expandieren im Schnitt **113k–163k Knoten**
pro Anfrage.

Vier konkrete Ursachen in `crates/sim/src/path.rs`:
1. **Unbudgetiertes `nearest_cell(..., u32::MAX, false)` in `Search::begin` (`path.rs:2389-2397`):**
   Liegt ein Ziel in einer anderen Komponente (Klippen, Wasser, zu schwache Brücke), scannt
   `nearest_cell` Ring für Ring potenziell bis zu 1,275 Mio. Zellen – `run_avoiding`
   (`path.rs:2636`) zieht dafür aber pauschal nur **`left -= 1`** (1 von 32.768!) ab.
   Mehrere unerreichbare Anfragen im selben Tick sprengen das Budget um ein Vielfaches.
2. **Unbudgetierte Stale-Heap-Pops & Glättung:**
   In `Search::run` (`path.rs:2440-2442`) verbraucht das Verwerfen bereits geschlossener
   Duplikat-Einträge (`ws.nodes[c].mark == closed`) kein Budget (`continue` ohne `*budget -= 1`).
   Auch `finish()` → `smooth()` (`path.rs:1963-2009`) läuft bis zu `SMOOTH_WINDOW = 32`
   `i128`-Linientests pro Wegpunkt unbudgetiert.
3. **Schleifen-Reibung & 5-MB-Allokation:**
   - `grid.decks_for(self.profile)` wird innerhalb der 8-Nachbarn-Schleife (`path.rs:2461`)
     aufgerufen statt einmal vor `while *budget > 0`.
   - `resolve_split` (`path.rs:2085`) allokiert bei jeder Komponenten-Split-Prüfung einen
     frischen 5-MB-Vektor (`vec![0u32; width * height]`).
   - `plan_paths` (`sim.rs:1193-1210`) durchsucht im `wanted`-Closure `self.units` linear
     (`O(units)`) pro Anfrage in der Warteschlange und klont `avoid` (Minenfelder) pro Anfrage.
4. **Hohe Expansionszahl bei Straßen-Heuristik (`HEURISTIC_WEIGHT_PCT = 0`, `path.rs:45`):**
   Da `MIN_CELL_COST = 100` (Straße) für Fahrzeuge mit Gelände-Kosten 250–400 sehr niedrig
   ist, degeneriert A* abseits von Straßen zu nahezu flächigem Dijkstra (163k Knoten).

## 2. Ziel

### Stufe 1 (Pflicht: harte Budget-Grenze & Zero-Overhead-Inner-Loop)
1. **`nearest_cell` budgetieren und beschleunigen:**
   - `nearest_cell` darf nie unbegrenzt Zellen scannen, ohne Budget zu zählen. Entweder:
     a) **Grob-Block-Beschleunigung** (z. B. 8×8-Block-Bitmaske/Komponenten-Präsenz, sodass
        leere Ringe/Blöcke in O(1) übersprungen werden) **plus** Abrechnung der geprüften
        Zellen/Blöcke gegen `*budget`, **oder**
     b) **Fortsetzbares `nearest_cell`** als eigener Zustand in `Search`, das pro Tick nur
        so viele Ringe/Zellen scannt wie `*budget` erlaubt.
   - Determinismus strikt wahren (feste Ring-/Zellenreihenfolge, Zustand in `SimSnapshot`
     serialisiert und gehasht!).
2. **Stale Heap-Pops & Glättung sauber einpreisen:**
   - Stale Pops in `Search::run` gegen `*budget` anrechnen (z. B. 1 Budget-Punkt je 4
     verworfener Einträge oder 1 je Pop – mit `just path-bench` kalibrieren).
   - `decks_for(self.profile)` vor die `while *budget > 0`-Schleife ziehen.
   - `resolve_split` (`path.rs:2085`) ohne 5-MB-Neuallokation (wiederverwendeter
     Generations-Puffer oder spärlicher `BTreeMap` für lokale Splits).
   - `plan_paths` (`sim.rs:1193-1210`): einmalig vor dem Durchlauf ein `BTreeSet`/Bitset der
     aktiven Request-IDs aufbauen statt `O(requests * units)`, und `avoid` als Slice borgen.

### Stufe 2 (Optional prüfen: deterministische ALT-/Landmark-Heuristik oder HPA*)
- Prüfen, ob eine **100 % zulässige, rein ganzzahlige Landmark-Heuristik (ALT)** mit
  z. B. 4–8 deterministisch gewählten Landmarks (vorberechnet beim Laden der Karte je
  dedupliziertem `CostGrid` aus Prompt 210) die 163k-Node-Suchen auf `coastal_heights` und
  `test_large` drastisch (5×–20×) senkt, ohne Straßenpräferenz oder Optimalität zu verlieren
  und ohne die Kartenladezeit unangemessen zu erhöhen.
- Falls das Kosten/Nutzen-Verhältnis (Ladezeit, RAM, Code-Einfachheit für Solo-Dev) klar
  positiv ist: umsetzen; sonst die Messung dokumentieren und bei Stufe 1 bleiben.

## 3. Prüfen und zeigen

- **Benchmarks vorher/nachher:**
  - `just bench --scenario data/scenarios/bench_coastal.toml --runs 2`: Ziel **max Tick ≤ 20 ms**
    (vorher 31,3 ms)!
  - `just path-bench coastal_heights`, `just path-bench test_large`, `just path-bench city_large`.
  - `just stuck-audit` (keine Regression bei steckengebliebenen Einheiten!).
  - Ergebnisse in `docs/PERFORMANCE.md` eintragen.
- **Determinismus & Tests:**
  - `just check`, `just selftest`, `crates/net/tests/snapshot.rs` grün.
  - Wenn sich Golden Hashes ändern: genau prüfen, ob es ausschließlich von der neuen
    Budget-Abrechnung kommt, in `docs/DETERMINISM.md` dokumentieren und im Commit erklären.

## 4. Nicht Teil dieses Prompts

- Änderungen an der Godot-Präsentation oder an 3D-Modellen.
- Lokale Ausweich-Algorithmen (RVO/ORCA).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/211-wegfindung-budget-und-tick-spitzen.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.
