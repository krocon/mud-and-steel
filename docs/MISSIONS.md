# Missions and tutorial

Prompt 039. Single-player missions and the tutorial are scripts as data:
a TOML file next to its map that places the units, names objectives and
lists triggers with actions. The simulation runs the script every tick, so
a mission is as deterministic as any match: replays, saved games and the
state hash include it. Decisions with reasons: `prompts/done/039-tutorial-missionen.md`.

Items marked [provisional] are decisions the user should confirm.

## Playing

- **Main menu → Tutorial** starts the first chapter of basic training not
  won yet (prompt 180; all won: chapter 1). The missions screen lists every
  chapter (in `order`) and starts any of them.
- **Main menu → Missions** lists the tutorial and every mission (title,
  side, map, "(won)" once won, with the hardest level it was won on) with
  its briefing; **Difficulty** (Easy, Normal, Hard; prompt 181) picks the
  level, **Start** plays it. The tutorial button uses the level chosen last.
- In the match the mission panel (top right) shows the title (the tutorial:
  "Training – step n of m"), the latest message and the objectives (open,
  done, failed). Active objectives with an area get a yellow ring on the
  ground; the script can move the camera.
- A won mission is recorded in `user://missions.cfg` (section `done`, key
  = mission id; section `level`: the hardest level won; section
  `settings`, key `level`: the level chosen last). The flow self-test uses
  `user://missions-selftest.cfg`.
- When the script says the mission can hardly be won any more (action
  `hopeless`, prompt 181), a panel in the middle says so and offers
  **Restart** (same mission and level), **Load last save** (the newest
  saved game of this map; greyed without one) and **Play on**.
- The result screen says "Mission accomplished." or "Mission failed." when
  the script decided the match.

| Mission | Side | Map | What happens |
|---|---|---|---|
| Basic training (tutorial, chapter 1, `tutorial`) | Germany | test_small | Camera, selection, moving, attacking two training targets, rifle squad into a wood, barracks, a rifle squad from it, the village supply point. |
| Basic training 2: Seeing and being seen (`tutorial_sight`) | Germany | test_small | Scouts up the hill find a lookout only seen from there (`spotted`); a squad lies prone in a wood (`stance`) and ambushes a patrol; a tank is camouflaged while a scout car passes (`unseen`). |
| Basic training 3: Infantry (`tutorial_infantry`) | Germany | test_small | Prone, kneel (`stance`), behind hedges, a house occupied (`garrison`), an enemy squad pinned down (`pinned`) and destroyed. |
| Basic training 4: Tanks (`tutorial_tanks`) | Germany | test_small | Scouts find an anti-tank gun; it fires only at a tank in front of it (area `front`), a lost tank comes back twice (`destroyed` → `spawn`); the gun from the flank, a T-34 from the side. |
| Basic training 5: Support (`tutorial_support`) | Germany | test_small | Scouts spot, a mortar barrage pins (`pinned`), smoke (`smoke`), the tactical pause (signal `pause`), an air call (`air`). |
| Meadow crossing | Soviet Union | twin_meadow | Take the western meadow's supply point from a light German screen, then destroy the counterattack that drives in from the north edge. |
| The village square | United States | desert_village | Clear the square of German infantry and a tank, then take its supply point. |
| Town in the snow | Germany | town_small (winter look) | Destroy the Soviet outpost on the square, then the Soviet relief that drives in from the north edge. |

The example missions use the passive computer player (holds its ground and
fires) and scripted `order` actions for counterattacks [provisional]. They
use no historical unit names or insignia.

## Files

```
data/maps/<map>/missions/<id>.toml   mission script (written by hand)
data/maps/<map>/mission_points.toml  trigger areas and spawn points (map editor)
data/maps/<map>/missions/<id>.de.po  German texts (prompt 043, `just mission-po`)
```

**Translations.** Mission files are written in English. `<id>.<lang>.po`
next to the file holds the translation of every text the player reads
(title, briefing, objectives, `text` actions); `msgid` is the English text.
After adding or changing a text, run `just mission-po`: it adds the new
texts with an empty `msgstr` and keeps the others; fill them in (by hand or
with a PO editor). `just l10n-check` fails while a text has no translation.
A changed English text needs its translation again, as its `msgid` changed.

The id is the file name without `.toml`; progress is stored under it. A map
can host several missions. Areas and spawn points of the mission file win
over the map's of the same name; `test_small` keeps its places in the
tutorial file because the editor does not save that protected test map.
Maps the player builds in the editor (`user://maps` in an exported game)
work the same way.

## Script reference

Places are whole meters, `x` east and `y` south, like start positions in
`map.toml`. Times are game seconds (the simulation runs at 20 ticks per
second). Unknown keys are errors.

### `[mission]`

| Key | Default | Meaning |
|---|---|---|
| `title` | – | Name in the menu and the panel. |
| `kind` | `"mission"` | `"tutorial"` puts it first in the list, starts it from the Tutorial button and shows step numbers. |
| `order` | 0 | Position in the mission list. |
| `faction` | `""` | Side of the player for the menu (`de`, `su`, `us`). |
| `briefing` | `""` | Text of the mission screen. |
| `biome` | `""` | Look of the map (`forest`, `desert`, `winter`); empty: the map's. |
| `player` | 0 | The human player. |
| `elimination` | `true` | Enemies lose when their last unit is destroyed, as in a skirmish. `false`: only the script decides; the player still loses with the last unit. |
| `economy` | `false` | Base building: adds the base buildings and the production lists of the factions in the mission. |

### `[[computer]]`

`player`, `difficulty` (`passive`, `easy`, `normal`, `hard`): a computer
player of `crates/ai`, like in a skirmish. Players without one give no
orders at all (training targets).

### `[[group]]`

Units at the start, laid out like a `MoveTo` formation around the place.

| Key | Default | Meaning |
|---|---|---|
| `name` | `group<k>` | Group name for triggers and actions (groups of the same name are joined). |
| `owner` | – | Player. Every player of the mission needs a unit at the start. |
| `unit` | – | Catalog id; base buildings (`base_*`) need `economy = true`. |
| `count` | 1 | Number of units. |
| `at` / `position_m` | – | A place (spawn point, area or map marker) or `[x, y]`. |
| `spacing_m` | 1.5 × footprint | Distance in the formation grid. |
| `hold_fire` | `false` | Start with hold fire. |
| `levels` | every level | Difficulty levels the group exists at, e.g. `["hard"]` (prompt 181). |

### `[[area]]`, `[[spawn]]`

`[[area]]`: `name`, `center_m = [x, y]`, `radius_m`. `[[spawn]]`: `name`,
`position_m = [x, y]`. Usually in `mission_points.toml` (map editor).

### `[[objective]]`

| Key | Default | Meaning |
|---|---|---|
| `id` | – | Name for `objective` actions. |
| `text` | – | Shown in the panel. |
| `area` | – | Its area: drawn as a ring (the self-test's autopilot goes there). |
| `unit` | – | The catalog id it is about (a building to build, a unit to produce). |
| `shown` | `false` | Active from the start; otherwise hidden until an action activates it. |

### `[[trigger]]`

A trigger is **waiting** until every trigger in `after` has fired, then
**armed**: its condition is checked once per tick at the end of the tick
(file order). When it holds, the actions run after `delay_s`. Every
trigger fires once.

| Key | Meaning |
|---|---|
| `id` | Name for `after`. |
| `after` | Triggers that must have fired first (default: armed at the start). |
| `when` | The condition, see below. |
| `delay_s` | Seconds between the condition and the actions. |
| `open` | Objectives that must still be open (hidden or active, not done or failed) for the condition to count (prompt 181). |
| `actions` | Array of inline tables with `do = "…"`. |

| `when` | Keys | Holds when |
|---|---|---|
| `time` | – | always: a timer of `delay_s` after arming |
| `area` | `area`, `player` (default: the mission's), `count` (1), `unit` (catalog id prefix, optional) | at least `count` living matching units of `player` stand in the area |
| `destroyed` | `group` | every unit of the group is destroyed (and it had units) |
| `eliminated` | `player` | the player has no living unit |
| `captured` | `point` (supply marker of the map), `player` | the player holds the supply point |
| `signal` | `signal` | the human player's presentation reported it this tick (below) |
| `units` | `unit` (catalog id prefix), `count` (1), `player` | the player owns that many living matching units; buildings count once completed |
| `fewer` | `unit` (catalog id prefix, optional), `count` (1), `player` | the player owns fewer than `count` living matching units (prompt 181) |
| `stance` | `stance` (`stand`, `kneel`, `prone`), `unit`, `area` (optional), `count` (1), `player` | that many living matching squads have the stance (not while changing it; prompt 180) |
| `garrison` | `unit`, `area` (optional), `count` (1), `player` | that many living matching squads are inside a building (prompt 180) |
| `spotted` | `group`, `player` | the player sees a living unit of the group (prompt 180) |
| `unseen` | `by`, `unit`, `area` (optional), `count` (1), `player` | that many living matching units of the player are not seen by player `by` (ambush, camouflage; prompt 180) |
| `pinned` | `group`, `pct` (50) | a living squad of the group is suppressed by at least `pct` % (prompt 180) |
| `smoke` | `area` | smoke lies at the center of the area (prompt 180) |
| `air` | `player` | the player called air support this tick (prompt 180) |

| `do` | Keys | Effect |
|---|---|---|
| `text` | `text` | Shows the message (replaces the previous one). |
| `objective` | `objective`, `state` (`hidden`, `active`, `done`, `failed`) | Sets the objective's state. |
| `spawn` | `group`, `owner`, `unit`, `count`, `at`, `hold_fire`, `arrive` (`appear`), `from`, `ride` | Units arrive at the place (not buildings) and join the group: they appear there in formation, or with `arrive = "drive"` drive in from the map edge in a column ([below](#reinforcements-drive-in)). `ride`: squads mount carriers of that group at once. |
| `order` | `group`, `to`, `order` (`advance`: halt to fight on the way; `move`: drive through) | The group's living units move there, as a command of their owner. |
| `fire` | `group`, `hold_fire` | Fire mode of the group. |
| `barrage` | `group`, `at`, `radius_m` (0), `salvos` (1) | The group's units with an indirect weapon (mortars, howitzers, rocket launchers) fire at the place, as a `Barrage` command of their owner (prompt 082). |
| `air` | `mission` (`recon`, `strafe`, `attack`, `bomb`, `airborne`), `at`, `to` (optional), `player` (default: the mission's) | The player calls air support on the place, with `to` along the line `at` → `to`; price and cooldown apply as for any call (prompt 082). |
| `camouflage` | `group`, `on` (default `true`) | Vehicles and guns of the group put up or take off their camouflage (prompt 082). |
| `camera` | `at` | The camera glides there (presentation). |
| `victory` / `defeat` | – | The human player (and allies) win, or lose. |
| `hopeless` | `text` | The mission can hardly be won any more: shows the text as the message and opens the warning panel (once, only while the match runs; prompt 181). |

### `[difficulty.easy]`, `[difficulty.normal]`, `[difficulty.hard]`

Prompt 181. The player picks the level in the missions screen; it is an
input of the scenario: `sim::mission::parse_mission_at` applies the values
of that level when the script is parsed, so the simulation, replays and
saved games only see the result (and `Mission::level`). A missing section
(or a script without any) is the script as written; Normal usually has
none. Every key is optional; percentages round to the nearest unit.

| Key | Changes |
|---|---|
| `computer` | `difficulty` of every `[[computer]]` (`passive`, `easy`, `normal`, `hard`). |
| `enemy_start_pct` | `count` of the start groups of other players (at least 1 per group). |
| `enemy_count_pct` | `count` of `spawn` actions for other players (reinforcements; 1 to 50). |
| `enemy_delay_pct` | `delay_s` of triggers that spawn units for other players. |
| `own_count_pct` | `count` of `spawn` actions for the mission's player. |
| `time_pct` | `delay_s` of time limits: triggers that fail an objective or `defeat` (they win over `enemy_delay_pct`). |
| `resources_pct` | Start resources of the mission's player (economy missions). |

Groups with `levels` leave out the other levels. A `destroyed` trigger on
a group that has no units at a level is an error (it could never fire).

| Mission | Easy | Hard | Warning (`hopeless`) |
|---|---|---|---|
| Basic training | 150 % start resources | as Normal | both tanks lost while the targets stand |
| Meadow crossing | counterattack 50 % | computer `normal`, counterattack 150 % | one T-34 left before the counterattack is beaten |
| The village square | garrison 50 % (one squad, the tank) | computer `normal`, garrison 150 % | one Sherman left before the square is cleared |
| Town in the snow | relief 50 % (one T-34) | computer `normal`, outpost 200 %, relief 150 % | one Panzer IV left before the relief is beaten |

Measured against the test autopilot (`every_level_of_every_mission`,
seed 42; game time to victory, Easy / Normal / Hard; since prompt 180 the
autopilot attacks the group a `destroyed` objective names): tutorial 153 s
on every level; meadow crossing 390 / 391 / 84 s; village square 80 / 80 /
202 s; town in the snow 82 / 83 / 72 s. Chapters 2–5 (no levels): 368,
201, 167 (one tank lost on purpose by the route) and 50 s. Hard is not always longer: with `computer = "normal"`
the enemy comes out to fight the autopilot's column instead of waiting.
On Hard with two Panzer IV at the crossing (`enemy_start_pct = 200`) the
autopilot lost meadow crossing after 98 s, so that level only grows the
counterattack.

### Reinforcements drive in

Prompt 108. `arrive = "drive"` turns a `spawn` into a column that drives
in; without it (`arrive = "appear"`, the default) the units appear at the
place as before, so old files play unchanged.

```toml
actions = [
  { do = "text", text = "Soviet tanks approach from the north!" },
  { do = "spawn", group = "relief", owner = 1, unit = "su_t34_76_1943", count = 2, at = "north_entry", arrive = "drive" },
  { do = "spawn", group = "relief", owner = 1, unit = "su_zis5", count = 1, at = "north_entry", arrive = "drive" },
  { do = "spawn", group = "relief", owner = 1, unit = "su_inf_rifle_squad", count = 1, at = "north_entry", arrive = "drive", ride = "relief" },
  { do = "order", group = "relief", to = "square", order = "move" },
]
```

- **Entry.** `from` (a place: spawn point, area or marker) if given;
  otherwise the point on the map edge nearest to `at`, 4 m inside, moved
  along that edge to the nearest road (or track) within 150 m, else to the
  nearest passable ground there. A mission whose edge is blocked or held by
  the enemy names another entry with `from` [provisional]: the column
  drives in anyway, there is no check for enemies at the edge.
- **Column.** The units stand one behind the other along the path from the
  entry to `at` (the path their type would drive; a straight line if there
  is none), twice their footprint apart (a vehicle length of gap; squads
  march in file the same way), facing along the path. The first unit of
  the first spawn leads; spawns of one trigger with the same entry and
  place form one column in file order, the last one at the entry.
- **Order.** From the first tick they are ordinary units with a move order
  of their owner to `at` (right-click default: squads halt for enemies in
  range, vehicles drive on), where they take the formation of the move.
  No protection, no special rule: they can be shot on the way and fire
  back. An `order` action after the spawns in the same trigger replaces
  that move (the columns get their move before the next action that is not
  a driving spawn).
- **Mounted.** `ride = "<group>"`: each squad mounts the first carrier of
  that group (spawned before, same owner) with free seats, trucks and
  halftracks before tanks (riders on the hull), and rides with it; squads
  without a seat march. They stay aboard until the carrier is hit or the
  owner orders them off (no automatic dismount at the place).
- **Message.** The event feed says "Reinforcements arriving" (own and
  allied) or "Enemy reinforcements" at the entry (a click jumps there) and
  the minimap pings it.
- Enemy reinforcements use the same action (`owner` of the enemy).

Which spawns drive in (prompt 108):

| Mission | Spawn | Arrival | Entry | Won (autopilot, game time) |
|---|---|---|---|---|
| Basic training | two training targets at `targets` | appear (they must stand at the range) | – | 188 s / 191 s (unchanged) |
| Town in the snow | relief, 2 × T-34 at `north_entry` | drive | north edge (0, -296), on the road | 86 s → 87 s / 84 s |
| Meadow crossing | counterattack, Pz IV + 2 halftracks (new) with the 2 squads aboard at `north_west` | drive | north edge (-110, -496) | 435 s → 585 s / 485 s |
| The village square | – (no spawn) | – | – | 81 s / 82 s (unchanged) |

Times: `crates/ai/tests/missions.rs` autopilot / `just selftest`. On foot
the two squads of the meadow counterattack took too long for the 500 m from
the edge (the self-test's autopilot waited 100 minutes for the last one),
so they now ride in two halftracks; a squad mounts vehicles with seats
before a tank's hull.

### Signals

What only the presentation knows reaches the script as a command of the
human player, `Command::MissionSignal { name }`, through the lockstep
session and into the replay. The mission panel sends a signal only while
an armed trigger waits for it (`SimHost.mission_info()["wanted"]`):

| Signal | Sent when |
|---|---|
| `camera` | The view was panned 12 m, zoomed by 15 % or turned by 0.3 rad since the trigger was armed. |
| `select` | Own units are selected. |
| `pause` | The tactical pause is on (prompt 180; the trigger fires with the first tick after it). |

Texts (messages, objectives, the `hopeless` text) may contain
`{key:<action>}` (prompt 180): the mission panel shows the key the action
is bound to now (Settings → Controls, `Settings.action_key_name`), "–"
without one. Translations keep the placeholder. Each objective's `goal`
in `SimHost.mission_info()` (the condition of the trigger that completes
it, `Sim::mission_goal`) tells the self-test's autopilot what to do.

Signals from other players, unknown names and names over 32 bytes are
ignored.

## How it works

- `sim::mission` parses the files into `SimConfig::mission` (resolved:
  names become indices, places positions, seconds ticks, catalog ids unit
  types); `parse_mission` builds the whole `SimConfig`.
- `Sim` keeps the script and its progress (`sim/mission.rs`: trigger
  states, objective states, unit ids per group, last message, last camera
  request). `Sim::step` visits the triggers after vision, right after the
  tick counter advances and before the standings, so a victory lands in the
  tick its condition was met. `victory`/`defeat` decide the standings with
  `DefeatReason::Mission`.
- The state hash includes the mission's progress only when there is a
  mission, so skirmishes and the golden hash are unchanged. Replay format 9
  and lobby protocol 5 carry the new config field and command.
- Bridge (`crates/bridge/src/mission_funcs.rs`): `SimHost.start_mission`,
  `mission_info`, `mission_signal`, `SimHost.mission_header` (static, for
  the menu); the map editor's `add_mission_area`, `add_mission_spawn`,
  `remove_mission_points`, `mission_points`.
- Godot: `hud/mission_hud.gd` (panel, rings, camera, signals, progress),
  `menu/missions.gd` (files, start, progress), `menu/missions_screen.gd`,
  main menu buttons, `menu/mission_self_test.gd`.

## Map editor

Tools **Trigger area** (radius slider) and **Spawn point**, with an optional
**Name** (empty: `area_<n>` / `spawn_<n>`). A click inside an area or near
a spawn point removes it. Areas are drawn as yellow rings, spawn points as
green ones, each with its name. They are saved with the map as
`mission_points.toml` (no undo for them [provisional]); the script is a text
file written by hand next to it [provisional: no script editor in the game].

## Tests

- `crates/sim/tests/missions.rs`: timers, areas, signals (only from the
  player, only while armed), spawns and destroyed groups, orders, captured
  points, unit counts, errors, the points file, the hash; columns that
  drive in (start at the edge in file order, drive to the place, an
  `order` replaces their move, passengers mount, the entry finds the road,
  deterministic).
- `crates/sim/tests/mission_levels.rs` (prompt 181): the values of each
  level, `levels` of groups, level errors, start resources, the `fewer`
  condition with `open` and the `hopeless` action.
- `crates/sim/tests/mission_triggers.rs` (prompt 180): `stance`,
  `garrison`, `spotted`, `unseen`, `air`.
- `crates/ai/tests/missions.rs`: every mission file is won by an autopilot
  that follows the objectives against the mission's computer players (it
  reads what completes each objective: stance, house, scouts to the area,
  camouflage, attack or barrage of a group, smoke, air call;
  `a_tank_lost_in_training_comes_back`: a tank driven at the gun of
  chapter 4 is lost and replaced)
  (`every_level_of_every_mission`: on Easy and Normal, Hard reported); the
  tutorial plays the same twice and survives a snapshot; every column that
  drives in has a path from its entry to its place.
  `MISSION=<id> END=<ticks> cargo test -p ai --test missions debug_units -- --ignored --nocapture`
  prints the units while tuning a mission.
- `just selftest` plays every tutorial chapter and every mission from the
  menu to victory (`menu/mission_self_test.gd`; the first mission on Easy, the
  second on Hard, chosen in the screen; the warning panel opens and
  closes); `just editor-selftest` places,
  removes and saves trigger areas and spawn points.

## Open

- Multiplayer or cooperative missions (the script supports teams, the menu
  and lobby do not offer missions).
- A campaign (mission order, unlocking), mission ratings and timers in the
  panel (prompt 189).
- Prompt 180: chapters on moving in formation and transport, engineers,
  supply (later prompt); help cards; a picture per chapter for the menu.
  Chapter 2's camouflage step does not fail when the scout car sees the
  tank (only a hint). Camera keys in chapter 1 are fixed text (not
  rebindable).
- Prompt 181: the warning panel does not pause the match; "Load last save"
  takes the newest save of the map, not only of this mission; a level per
  computer player.
- Voice-over; a script editor in the game. (Objective markers on the
  minimap: done in prompt 107, a gold ring with four ticks.)
- Translations of the mission texts (prompt 043).
- Reinforcements by train at a station or by landing craft on a coast
  (prompt 108 proposal); a check for enemies at the entry; waypoints
  (`via`) for a column.
