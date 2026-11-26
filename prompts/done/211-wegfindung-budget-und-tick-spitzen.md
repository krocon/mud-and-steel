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

## Ergebnis (2026-11-26)

- **Stufe 1 vollständig umgesetzt (`crates/sim/src/path.rs`, `crates/sim/src/sim.rs`):**
  - **8×8-Block-Index & Komponenten-Bounding-Boxen in `ComponentGrid`:** `block_comp` (`0` = leer, `c` = einheitlich, `u32::MAX` = gemischt) + `comp_bounds` pro Label (`#[serde(skip)]`, bei `label_components`, inkrementell in `relabel_local` und beim Restore über `PathGrid::rebuild_derived()` gepflegt). `nearest_cell_in_component_blocks` liefert bit-identisch dasselbe Ergebnis wie der Volldurchlauf (belegt durch neuen Unit-Test `nearest_cell_component_matches_brute_force` inkl. dynamischem Hindernis) und rechnet `work / 8` gegen `NODE_BUDGET_PER_TICK` an.
  - **Stale-Heap-Pops budgetiert & `decks_for` vorgezogen (`Search::run`):** 1 Budget-Punkt je 8 verworfener Stale-Heap-Pops (kalibriert gegen `path-bench` und Missions-Autopilot `gulf_landing`: `1/4` + `smooth`-Abzug ließ `gulf_landing` im Selftest ins Timeout laufen, `1/8` hält `gulf_landing` bei Tick 55601 grün), `decks_for(self.profile)` und `rights` vor die `while *budget > 0`-Schleife gezogen.
  - **`resolve_split` ohne 5-MB-Neuallokation:** Wiederverwendeter `#[serde(skip)] split_owner: Vec<u32>` auf `PathGrid`, der am Ende nur die tatsächlich berührten Zellen in O(touched) zurücksetzt (eine zwischenzeitliche `BTreeMap`-Variante verlangsamte `Sim::new` auf `coastal_heights` um +238 ms, der wiederverwendete Flat-Puffer hält `Sim::new` bei 243 ms).
  - **`Sim::plan_paths` (`sim.rs`):** Aktive `Route::Waiting`-IDs einmalig sortiert + dedupliziert (`binary_search` statt `O(requests × units)`), Early-Return bei leerer Queue und geborgter `&[u32]`-Slice für `avoid` in `run_avoiding`.
- **Stufe 2 (ALT-Landmarks) geprüft & bewusst nicht umgesetzt:**
  - 4 Landmarks × 20–86 deduplizierte `CostGrid`s (Prompt 210) würden auf `coastal_heights` **408 MB–1,75 GB** zusätzlichen RAM und **12–20 s** Ladezeit kosten (plus Invalidierung bei jeder Kosten-Verringerung durch zerstörte Mauern/Bäume, gelegte Brücken oder abtrocknendes Wetter). Da Stufe 1 allein bereits alle Budgets mit großem Puffer einhält (Max-Tick `6,53 ms` statt `39,73 ms`), bleibt es sauber bei Stufe 1.
- **Messwerte vorher → nachher (`docs/PERFORMANCE.md`):**
  - `bench_coastal` (`coastal_heights`, 2 Läufe): Sim-Step Mean **5,103 → 4,818 ms** (unter 5-ms-Budget), p99 **6,094 → 5,736 ms**, Max **39,726 → 6,527 ms (−83,6 %)**; `paths` Max **38,588 → 5,312 ms (−86,2 %)**.
  - `path-bench coastal_heights`: Planning-Ticks Mean **4,714 → 4,327 ms**, p99 **5,985 → 5,248 ms**, Max **9,520 → 5,739 ms** (21,0 ms in Prompt 113).
  - `path-bench test_large`: Mean **4,664 → 4,341 ms**, p99 **6,028 → 5,644 ms**, Max **6,294 → 6,004 ms**.
  - `path-bench city_large`: Mean **5,064 → 4,806 ms**, p99 **8,263 → 7,792 ms**, Max **15,350 → 15,067 ms**.
  - `just stuck-audit`: 11 145 / 11 184 angekommen (0,4 % über 37 Karten, 0 % auf allen Stammkarten, keine Regression).
- **Determinismus & Tests:**
  - `determinism.rs` und `economy.rs` Golden Hashes **unverändert**; End-Hash langer Suchen (`bench_coastal`) ändert sich beabsichtigt (`0x16723eab64b2dc45` → `0xa116473879561de6`) durch die neue Budget-Abrechnung (`docs/DETERMINISM.md` ergänzt).
  - Clippy (alle 3 Profile), `nextest` (`sim`, `net` inkl. aller Snapshot-Tests, `ai`, `editor`, `bridge`, `steam`) und `just selftest` (inkl. `gulf_landing`, `riders`, `gun crews`, `boats`) grün.
