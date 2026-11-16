# Missions and tutorial

Prompt 039. Single-player missions and the tutorial are scripts as data:
a TOML file next to its map that places the units, names objectives and
lists triggers with actions. The simulation runs the script every tick, so
a mission is as deterministic as any match: replays, saved games and the
state hash include it. Decisions with reasons: `prompts/done/039-tutorial-missionen.md`.

Items marked [provisional] are decisions the user should confirm.

## Playing

- **Main menu → Tutorial** starts the tutorial (`data/maps/test_small/missions/tutorial.toml`).
- **Main menu → Missions** lists the tutorial and every mission (title,
  side, map, "(won)" once won) with its briefing; **Start** plays it.
- In the match the mission panel (top right) shows the title (the tutorial:
  "Training – step n of m"), the latest message and the objectives (open,
  done, failed). Active objectives with an area get a yellow ring on the
  ground; the script can move the camera.
- A won mission is recorded in `user://missions.cfg` (section `done`, key
  = mission id). The flow self-test uses `user://missions-selftest.cfg`.
- The result screen says "Mission accomplished." or "Mission failed." when
  the script decided the match.

| Mission | Side | Map | What happens |
|---|---|---|---|
| Basic training (tutorial) | Germany | test_small | Camera, selection, moving, attacking two training targets, rifle squad into a wood, barracks, a rifle squad from it, the village supply point. |
| Meadow crossing | Soviet Union | twin_meadow | Take the western meadow's supply point from a light German screen, then destroy the counterattack from the north. |
| The village square | United States | desert_village | Clear the square of German infantry and a tank, then take its supply point. |
| Town in the snow | Germany | town_small (winter look) | Destroy the Soviet outpost on the square, then the Soviet relief from the north. |

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

| `do` | Keys | Effect |
|---|---|---|
| `text` | `text` | Shows the message (replaces the previous one). |
| `objective` | `objective`, `state` (`hidden`, `active`, `done`, `failed`) | Sets the objective's state. |
| `spawn` | `group`, `owner`, `unit`, `count`, `at`, `hold_fire` | Units appear around the place (not buildings) and join the group. |
| `order` | `group`, `to`, `order` (`advance`: halt to fight on the way; `move`: drive through) | The group's living units move there, as a command of their owner. |
| `fire` | `group`, `hold_fire` | Fire mode of the group. |
| `camera` | `at` | The camera glides there (presentation). |
| `victory` / `defeat` | – | The human player (and allies) win, or lose. |

### Signals

What only the presentation knows reaches the script as a command of the
human player, `Command::MissionSignal { name }`, through the lockstep
session and into the replay. The mission panel sends a signal only while
an armed trigger waits for it (`SimHost.mission_info()["wanted"]`):

| Signal | Sent when |
|---|---|
| `camera` | The view was panned 12 m, zoomed by 15 % or turned by 0.3 rad since the trigger was armed. |
| `select` | Own units are selected. |

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
  points, unit counts, errors, the points file, the hash.
- `crates/ai/tests/missions.rs`: every mission file is won by an autopilot
  that follows the objectives against the mission's computer players; the
  tutorial plays the same twice and survives a snapshot.
  `MISSION=<id> END=<ticks> cargo test -p ai --test missions debug_units -- --ignored --nocapture`
  prints the units while tuning a mission.
- `just selftest` plays the tutorial and every mission from the menu to
  victory (`menu/mission_self_test.gd`); `just editor-selftest` places,
  removes and saves trigger areas and spawn points.

## Open

- Multiplayer or cooperative missions (the script supports teams, the menu
  and lobby do not offer missions).
- A campaign (mission order, unlocking), mission ratings and timers in the
  panel.
- Objective markers on the minimap; voice-over; a script editor in the game.
- Translations of the mission texts (prompt 043).
