# Prompt 045 – Balancing und Playtest-Werkzeuge

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/GAME_DESIGN.md`, `docs/AI.md`, `docs/SCALE.md`, `docs/UNIT_CATALOG.md`.
>
> **Konkretisiert** (Abschnitt „Concretization“ unten, Englisch wie das Repo).

## 1. Ziel

Die Einheitenwerte sind echt (Katalog), die Spielregeln (Kosten, Reichweitenfaktor,
Einkommen) vorläufig. Nach diesem Prompt gibt es Werkzeuge, mit denen sich das Balancing
messen lässt, und eine erste bestätigte Balance aller drei Fraktionen.

## 2. Umfang

1. **Statistik nach der Partie:** Verluste, Abschüsse pro Einheitentyp, Ressourcen-
   verlauf, eingenommene Punkte; im Ergebnisbildschirm und als JSON.
2. **Headless-Auswertung:** KI-Turniere über alle Fraktionspaare, Karten und Modi;
   Gefechtssimulator „Einheit A gegen Einheit B“ über Distanzen und Winkel; Bericht als
   Tabelle in `docs/BALANCE.md`.
3. **Stellschrauben:** nur Spielregeln (Kosten, Bauzeiten, Faktoren in `data/*.toml`),
   nie Katalogwerte; jede Änderung mit Messung begründet.
4. **Playtest-Ablauf:** Checkliste für Partien mit dem Nutzer, Rückmeldeformular in
   `docs/PLAYTEST.md`; `GAME_DESIGN.md` vom Status „vorläufig“ auf „bestätigt“
   bringen, wo der Nutzer zustimmt.

## 3. Akzeptanzkriterien

1. `just check` grün; Turnier über alle Paarungen ohne Fraktion mit mehr als 60 %
   Siegquote bei gleicher KI-Stufe (oder begründete Ausnahme).
2. `docs/BALANCE.md` mit Messwerten; Golden-Hash-Änderungen begründet.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Concretization (2026-11-16)

### Starting point in the code

- Systems of prompts 023–060 are in: economy (023), supply and resupply
  (036), transport (034), engineers and positions (035/052), air (033),
  artillery (032), heavy strike (054), experience (050), infantry tactics
  (047), wounds (049), weather (040), teams (037), ten maps, the AI
  commander (051, `Difficulty::HardOld` for comparison).
- `cli ai-tournament` existed (one pairing of difficulties, one faction
  setting, one map); no statistics beyond wins and units left; the result
  screen showed match time and units left per player.
- Known findings from prompt 051: (1) base matches between two AIs on
  1–2 km maps end undecided in 20 minutes, ammunition the bottleneck; (2)
  Hard against Passive stays below 90 % for the Soviet side on small maps;
  (3) faction win rates unknown.

### Decisions

| # | Decision | Reason |
|---|---|---|
| 1 | Statistics as an observer (`sim::stats::MatchStats`) outside the simulation state, fed after every tick by the game and the headless matches. | Same numbers in the game and the tournaments, no change of the state hash, snapshots or saves. A loaded game counts from loading on [provisional]. |
| 2 | Kills from the `Destroyed` events (last hit); deaths without an event count as losses without a killer. | The simulation already reports the shooter of the last hit; no new state. |
| 3 | Resource curves sampled every 30 s of game time (account, income, points held, living units). | Enough for a chart over a 20–30 minute match, small JSON. |
| 4 | Result screen: one line per player (losses, kills, points taken), the three best unit types of the local player, a chart of the own account; JSON in `user://stats/<date-time>.json`. | The result screen should stay one panel; details are in the JSON. |
| 5 | `cli balance`: every faction pairing in both orders on every given map and mode, same difficulty on both sides; Markdown tables. | Faction balance needs equal AIs and both start positions per faction. |
| 6 | `cli duel`: two units standing on flat empty ground, firing at will, over distances and angles. | Isolates the catalog values and combat rules from AI decisions. |
| 7 | Tuning only in rule and scenario data (`data/economy.toml`, `data/logistics.toml`, `data/scenarios/*.toml`), never the catalog; each change with a measurement in `docs/BALANCE.md`. | CLAUDE.md: real values only in the catalog. |
| 8 | Finding 2 is a rule/scenario problem, not an AI problem: battle mode gives equal numbers of tanks that are not of equal worth (Panzer IV 375 m and 10 rounds/min against T-34 250 m and 5 rounds/min). Battle armies get equal strength when factions are chosen: new `[faction_count_pct]` in the scenario (su 125 %, us 112 %). The plain demo (golden hash, tests) keeps 8 against 8. | Measured: 71 / 25 / 41 % (de/su/us) before, 47 / 41 / 49 % after; su 150 % overshoots (70 % against de). The 90 % target against a passive enemy cannot be reached by the outranged side on small open maps by any AI (see BALANCE.md) [provisional, user decision]. |
| 9 | Economy: more ammunition and manpower income, less fuel, start 500/600/500, resupply 25 % instead of 50 %. | Finding 1: ammunition was the bottleneck, fuel piled up; manpower starved the US. Variants E1–E3 measured. [provisional] |
| 10 | Skirmish start vehicle: an unarmed truck for every faction instead of the very differently armed halftracks. | Finding 3: the Soviet scout car (.50 + .30) killed 15 units per loss and decided small-map base matches (su 95 %, us 0 %). [provisional] |
| 11 | Unit price: the ammunition term of direct-fire weapons scales with the game range (reference 250 m, 50–150 %). | Balancing principle 1 ("a stronger unit is never cheaper") was broken: a T-34 cost more than a Panzer IV, the Panzerschreck squad ten times the PTRD squad. [provisional] |
| 12 | Rejected: a range cap for small arms (no measurable effect), E2 (ammunition only) and E3 (more manpower). | See BALANCE.md. |
| 13 | Nothing in GAME_DESIGN.md is marked confirmed; the decisions for the user are listed in `docs/PLAYTEST.md`. | Only the user confirms. |

### Scope

1. Post-match statistics in the result screen and as JSON (done).
2. `cli balance`, `cli duel`, `just balance`, `just duel`; report in
   `docs/BALANCE.md` (done).
3. Rule tuning with measurements (decisions 8–11).
4. `docs/PLAYTEST.md`: session flow, checklist, feedback form, decisions
   waiting for the user.

Acceptance criterion 1 (no faction above 60 % at equal AI level): see the
results in `docs/BALANCE.md`; exceptions are named there.

RESULTS_PROMPT
