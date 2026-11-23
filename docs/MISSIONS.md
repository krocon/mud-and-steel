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
- **Main menu → Campaign** (prompt 189) shows a campaign of
  `data/campaigns/` (a choice box if there are several) with its missions
  in play order and "n of m missions won": the first is open, every other
  one opens once the one before it was won (on any level); locked ones are
  grey. Below: the campaign's briefing (first mission), the mission's
  intro and briefing, and its outro once won. Difficulty and Start as in
  the missions screen. Nothing carries over between missions.
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
  `settings`, key `level`: the level chosen last; section `stars`, key
  `<id>.<level>`: the most stars won with at that level, prompt 189).
- **Rating** (prompt 189): a victory earns up to three stars, one each for
  winning in time (`[rating] par_s`; none set: given), losing at most
  `losses_pct` percent of the own units (default 25; spawned units count),
  and doing every optional objective (none: given). The result screen
  shows them ("★★☆ – in time, 2 of 9 units lost, side objectives 1/2"),
  the campaign and missions screens the best per level.
- The panel marks side objectives "Optional: …", shows an objective's
  count "(2/6)" (`progress`) and the time left "– 3:20" (`timer`). The flow self-test uses
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
| Holding the meadow (`meadow_hold`, prompt 189) | Germany | twin_meadow | The counterpart of Meadow crossing: hold the western meadow's supply point for five minutes (timer) against two waves of T-34s and riflemen from the south until the relief drives in from the north; lost if the Soviets take the point. Optional: keep the anti-tank gun in action. |
| Two bridges (`two_bridges`, prompt 190, cooperative) | United States | river_land | Two seats: "Bridge" (player 0) takes the far end of the stone bridge from infantry with a machine gun, "Flank" (player 2) destroys the half-tracks of the counterattack over the southern timber bridge before they get behind the lines (else the mission is lost). Alone, one player commands both forces. |
| Breaking out (`village_breakout`, prompt 189) | United States | desert_village | Cut off on the village square: break through a screen of riflemen and a machine gun and bring six of eight units to the road out in the south (count) before German tanks from the north arrive (timer). Optional: both trucks out. |
| The great bridge (`bridge_assault`, prompt 116) | United States | river_crossing | The first mission with a historical model (below): clear the bridgehead on the south bank (riflemen, a machine gun and, from Normal, an anti-tank team on the road between the castle hill and the park); paratroopers who crossed in boats appear on the north bank (`spawn`); the tanks drive over the long steel bridge while everyone takes its north end and destroys the demolition party before the charges go off (timer: 10 minutes, `demolish`); a bridge piece that falls loses the mission (`bridge_down`). |
| Holding the bridge (`bridge_hold`, prompt 118) | Great Britain | rhine_bridge | The second mission with a historical model (below): paratroopers at the north end of a road bridge occupy the houses beside the ramp (`garrison`) before armoured cars and half-tracks come over the bridge from the south, then hold for 16 minutes against infantry from the east, a tank and infantry from the old town, an assault gun from the north and mortar fire; ammunition comes only from three drops (`resupply`), one of them into the enemy's lines; lost if the enemy takes the supply point at the north end. The ending, marked as not historical, shows the armoured relief driving over the bridge. |
| Landing on the beach (`beach_landing`, prompt 119) | United States | dune_coast | Not historical: an LCVP with a rifle platoon and an LCM with a Sherman come in from the sea (`spawn` with `ride`), run onto the sand (`land`) and unload (`unload`); the men wade ashore, then clear the dunes behind the beach of riflemen and a machine gun (`destroyed`). |
| Landing in the gulf (`gulf_landing`, prompt 121) | United States | landing_gulf | The third mission with a historical model (below): commandos land from an LCVP in a rocky cove and take a battery on the western slope from behind; the first wave (four LCVPs) runs onto a mined and wired beach while the batteries on both slopes shell it (`barrage` while their objective is open); the second wave brings an anti-tank gun and a half-track (LCVP, LCM), the third two Shermans (LCMs); the village at the harbour (`captured`) and the eastern battery (`area`: the crews give up, `fire` hold) are open at the same time; then a counterattack with an assault gun and a half-track drives in from the plain and the road north is opened. |

The example missions use the passive computer player (holds its ground and
fires) and scripted `order` actions for counterattacks [provisional]. They
use no historical unit names or insignia.

### The great bridge (prompt 116)

The first mission with a historical model: the **Waal bridge at
Nijmegen**, taken intact on 20 September 1944 by American paratroopers of
the 82nd Airborne Division and British tanks of the Guards Armoured
Division, after paratroopers had crossed the Waal in boats west of the
railway bridge under fire. The game has no British units yet (prompt
117), so US Shermans stand in for the Guards; the German side uses the
generic `de` catalog (no SS insignia, docs/SOURCES.md). The mission file,
its texts and the map name no real place, unit or date [provisional:
whether the briefing names them is the user's decision, nobody could be
asked during the run].

- **Phase 1:** four Shermans, three rifle squads and a machine gun team
  on the road 300 m south of the bridge clear its south end (`destroyed`
  of the group `south_guard`).
- **Phase 2:** a message tells the boat crossing; three rifle squads and
  a machine gun team appear on the floodplain 200 m west of the north
  end (`spawn`, appear: the crossing itself is not played, prompt 116).
- **Phase 3:** three objectives at once: the demolition party (engineers
  behind the dam) must die before the fuse (`timer`), the north guard
  (two squads, a machine gun) must be destroyed, two Shermans must stand
  at the north end (`area` with `unit`). Victory when all three are done.
- **Demolition rule** [provisional]: the defenders fire the prepared
  charges when the fuse runs out (10 minutes after the landing on
  Normal, `time_pct` on the other levels) while the demolition party
  lives (`open = ["demolition"]`): `demolish` brings down the arch and
  the approach span next to it, and the trigger `bridge_down` on the
  whole bridge loses the mission, the same as when the player's own fire
  brings a piece down (prompt 115 shelling). Killing the party stops it.
- Measured (`crates/ai/tests/missions.rs` autopilot, seed 42, Easy /
  Normal / Hard): won after 278 / 659 / 363 s, 4 / 4 / 5 own units lost
  (on Normal the anti-tank team on the road holds phase 1 longest; on
  Hard the computer player comes out to fight the column); `just selftest`
  plays it from the menu on Normal: won after 1 088 s game time. First
  versions with the guard on the castle hill's plateau and the player in
  the old town were not won within 20 minutes: the autopilot's units
  stood below the hill's steep sides or could not see squads in the
  park's trees.

### Holding the bridge (prompt 118)

The second mission with a historical model: the **road bridge at
Arnhem**, 17–20 September 1944. A battalion of the British 1st Airborne
Division reached the north end in the evening of 17 September and held
the houses beside the ramp; on the next morning a German reconnaissance
unit tried to rush the bridge from the south with armoured cars and
half-tracks and was shot up on the ramp; infantry, tanks, assault guns
and mortars attacked from the town; supplies came by air and mostly fell
to the Germans; the armoured relief from the south never arrived and the
last defenders were overrun on 21 September. The German side uses the
generic `de` catalog (no SS insignia, docs/SOURCES.md). The mission file,
its texts and the map name no real place, unit or date (as in prompt 116
[provisional]).

- **Start:** four airborne sections, a Vickers team, three PIAT teams,
  two 6-pounders and two jeeps at the foot of the ramp; the enemy holds
  the south end of the bridge.
- **Objective 1** (shown, may fail): two airborne sections inside houses
  in the ring of the ramp houses (`garrison`) before the first wave
  (150 s); failed when the wave comes first.
- **Hold** (timer, 16 minutes): waves drive in as columns (`spawn`
  with `arrive = "drive"`): 150 s two armoured cars and two half-tracks
  from the south edge over the bridge; 360 s three rifle squads and a
  machine gun from the east; 420 s two medium mortars behind the town
  fire on the houses (`barrage`, again at 600 and 780 s); 600 s a
  Panzer IV and two squads through the old town from the west; 780 s an
  assault gun and two squads from the north.
- **Ammunition:** nothing on the player's side supplies (no trucks, no
  account); three drops (`resupply`): 300 s 50 % of a load for every
  unit within 110 m of the ramp houses, 540 s into the enemy's lines in
  the east (the enemy's units there get it), 720 s 40 % at the ramp.
- **Lost** when the enemy holds the supply point east of the ramp
  (`captured`) or the last unit falls. **Won** when the time runs out;
  then the ending, marked "not historical" in its texts: a Firefly and
  two Cromwells drive over the bridge from the south and the mission is
  won 45 s later. Optional: keep both 6-pounders.
- Player-called supply drops from the air bar (prompt 055) are not
  built: a new air mission would change the air rules of every match
  (and the golden hash); the drops are scripted [provisional].
- Measured (`crates/ai/tests/missions.rs` autopilot, seed 42, Easy /
  Normal / Hard): won after 1 005 s on every level, 6 / 9 / 10 own units
  lost of 15 (12 at the start, 3 of the relief); objective 1 done on
  every level. A first version (garrison area centred on a small house
  for 12 men, three armoured cars, two assault guns, two PIAT teams) was
  lost on Normal: the autopilot's four sections stood beside the small
  house and fought the armoured cars on the open ramp road.

### Landing on the beach (prompt 119)

`data/maps/dune_coast/missions/beach_landing.toml`, the example mission for
boats; not historical (no landing was made on this island). The start
groups are the two landing craft 250 m off the beach; the first trigger
spawns the platoon into the LCVP and the Sherman into the LCM (`ride`), the
next lands both and unloads them (`land`, `unload`). Objectives: get
ashore (`reach`: four units in the dunes' foot, out of reach of the boats
themselves, which lie 75 m below it), then clear the dunes (`destroyed`).
Easy halves the defenders, Hard adds a squad and a normal computer.
Measured against the test autopilot (seed 42): won on Easy and Normal
after 751 s (one squad lost), not on Hard; the boats ground about 50 s
after the start. The Sherman stays at the foot of the dunes (too steep
for it there); the riflemen take them.

### Landing in the gulf (prompt 121)

The third mission with a historical model: the **landing in the Golfo di
Campo on Elba** on 17 June 1944 (Operation Brassard): French commandos
landed before the main landing to silence the coast batteries (the
battery of Poro surrendered to them, other detachments took Fonza),
colonial infantry landed on the mined and wired beach under fire from the
slopes, the village at the harbour was cleared after several hours
([SOURCES.md](SOURCES.md#maps-from-real-geodata-the-landing-gulf-prompt-121)).
There is no French nation: the French, equipped almost entirely with
American gear, are US units; the Germans the generic `de` catalog. The
mission file, its texts and the map name no real place, unit or date (as
in prompt 116 [provisional]); the briefing says the distances are
shorter than they were: ranges in the game are an eighth of the real
ones (data/scale.toml), so the batteries stand on the slopes 300–900 m
from the beach, not on the far heights where they were, and light field
howitzers stand in for the 90 mm and 102 mm guns (no heavy coast guns in
the catalog).

- **Start:** an LCVP with two rifle squads and a flamethrower team
  (the commandos) off the rocky cove south of the village; four LCVPs
  with eight rifle squads, two machine guns and a bazooka team 300–400 m
  off the beach.
- **Commandos** (shown): they land at once (`land`, `unload`) and take
  the battery on the western slope (two howitzers, a machine gun) from
  behind; the objective is done when two units stand in the battery
  (`area`), its crews give up (`fire` with hold fire).
- **H-hour** (60 s): the first wave lands at both ends of the beach, one
  half at the village, the other at the north-east end below the
  eastern battery; mines (two sections) and wire with gaps lie on the
  beach (map objects). Ashore when six infantry units stood on the beach
  (`reach`). The batteries fire two salvos at the beach every two
  minutes while their objective is open (`barrage` with `open`).
- **Then at the same time** (a player sends each landing group to its
  own): take the village at the harbour (`captured` of its supply point;
  two rifle squads and a machine gun on the square, on Hard an anti-tank
  gun) and silence the eastern battery (a howitzer, a mortar, a machine
  gun and from Normal a rifle squad; two units in it).
- **Waves:** 30 s after the landing the second wave (an LCVP with a
  57 mm gun, an LCM with a half-track), two minutes later the third (two
  LCMs with Shermans), both onto the beach at the village.
- **Counterattack:** once village and eastern battery are taken, an
  assault gun and a half-track with a rifle squad drive in from the road
  in the plain and on into the village square (`arrive = "drive"`, then
  `order … move`) [provisional: not taken from a source]. Destroyed:
  two units on the road north of the village open it (`area`); victory
  once the western battery is taken too.
- **Warning** (`hopeless`): fewer than four infantry units left.
- Levels: Easy 60 % of the defenders; Hard the
  anti-tank gun in the village, 140 % defenders, computer `normal`.
- **Autopilot:** the test autopilot of `crates/ai/tests/missions.rs`
  leaves boats afloat out of its orders since this mission (a move to an
  objective on land would land a boat there; the script steers them); a
  beached boat gets orders again (it stays and fires). `beach_landing`
  is still won. The game's self-test (`mission_self_test.gd`) is
  unchanged: leaving boats out there made it lose `beach_landing`; it
  wins this mission on Normal after 1 746 s game time (its time limit is
  far longer than the test's 20 minutes).
- Measured (`crates/ai/tests/missions.rs` autopilot, seed 42, Easy /
  Normal / Hard): won after 1 085 / 1 110 s, 5 / 6 own units lost of 27;
  not won on Hard within 20 minutes (19 units lost: the computer comes
  out to fight on the beach). First versions were not won in time: the
  objectives one after the other (beach, battery, village, battery, the
  crossroads in the plain) made the autopilot's infantry walk 3.4 km;
  the garrison stood in a house and was never seen; a mine section lay
  on the landing route; the assault gun stopped among the houses, out of
  sight. Now several objectives are open at once (the autopilot sends
  each unit to the nearest), the garrison stands on the square, the
  counterattack drives into it, and the objective on the road lies
  500 m from the village.

## Battles

`data/battles/<map>/<name>.toml` are mission scripts that are no menu
missions: shows of fighting for testing performance, sound and effects.
No test plays them and the missions screen does not list them; they need
no translation (the texts stay English). `just battle <map>/<name>`
starts one (`-- --battle=<map>/<name>`, fire at will), `--battle-report`
prints shots, hits, losses and living units every 30 game seconds (to check
a battle headless).

| Battle | What happens |
|---|---|
| `valley_fields/all_units` | Every German and Soviet unit type three times (no aircraft, fortifications or anti-aircraft positions), each type once in each of three mixed battle groups (north, center, south). The Soviets hold the east edge of the town, their tanks right behind the houses (covered from the east), the tank reserve behind houses deeper in, the artillery in the west; the Germans attack from the fields and the village in the east, about 450 m away, in two waves after an artillery preparation (the Sturmtiger in wave 2); the Soviets answer with their artillery and a tank counterattack, both artilleries fire every 40 s; a German recon plane circles over the town (`recon_loiter`, 60 s on a 200 m circle) every 60 s, so its impacts stay visible; ten Sturmtiger (3 / 4 / 3) shell the houses of the east edge, every 45 s the next ones, deeper into the town (27 buildings hit after 3.5 minutes). Measured (seed 42, headless): 1996 shots, 39 units destroyed in 6.5 minutes, most of it after the waves reach the town (from about 4 minutes on). Generated from the catalog. |

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
| `faction` | `""` | Side of the player for the menu (`de`, `su`, `us`, `uk`). |
| `briefing` | `""` | Text of the mission screen. |
| `biome` | `""` | Look of the map (`forest`, `desert`, `winter`); empty: the map's. |
| `player` | 0 | The human player. |
| `elimination` | `true` | Enemies lose when their last unit is destroyed, as in a skirmish. `false`: only the script decides; the player still loses with the last unit. |
| `economy` | `false` | Base building: adds the base buildings and the production lists of the factions in the mission. |

### `[[seat]]` (cooperative missions)

Prompt 190. Further human players on the side of `[mission] player` (the
leader):

| Key | Default | Meaning |
|---|---|---|
| `player` | – | The seat's player id: it needs start groups of its own, may not be a computer player or the leader. |
| `name` | `Player <n>` | Shown in the lobby ("Flank"). |
| `briefing` | `""` | The seat's own part, shown in the lobby. |

- **Free seats are merged** [provisional]: a seat nobody takes (always in
  single player, or an open slot in the lobby) belongs to the leader:
  its units, the units spawned for it and every condition about it become
  the leader's (`sim::mission::take_seats`, applied when the match is set
  up, so replays and saves see the result). The leader and the taken seats
  are one team with shared vision against all other players of the mission.
- **Leaving** [provisional]: a human of the side who gives up (the game
  menu's surrender) hands every unit, with its orders, to the first other
  human of the side still playing (the leader first, then the seats);
  the match goes on. The last one to give up loses the mission for the
  side. A lost connection still ends the match for everyone, as in every
  network match (no pause and no loading over the network).
- Objectives, messages and the panel are the same for everybody; victory
  and defeat are the whole side's.
- **Lobby** (LAN, Steam): **Cooperative mission** lists the missions with
  seats, with **Difficulty**; the host plays the leader, the seats are open
  slots on team 0, the other slots up to the highest seat are the
  mission's computer players on team 1 (their difficulty comes from the
  script and the level when it starts). Everybody sees the mission's title,
  difficulty, briefing and the seats' parts. Lobby protocol 9 (`mission`
  and `level` in `LobbySetup`).

### `[[computer]]`

`player`, `difficulty` (`passive`, `easy`, `normal`, `hard`): a computer
player of `crates/ai`, like in a skirmish. Players without one give no
orders at all (training targets). A computer player never surrenders in a
mission (prompt 187): the script decides.

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
| `optional` | `false` | A side objective (prompt 189): "Optional: …" in the panel, not needed for the victory (the script decides), counts for the rating. |
| `timer` | – | A trigger id: while that trigger counts down (its condition held, its `delay_s` runs; a `time` trigger from its arming) the panel shows the time left (prompt 189). |
| `progress` | – | A trigger id of `when = "area"`, `"units"` or `"reach"`: the panel shows its count, e.g. "(2/6)"; a fired trigger shows it full (prompt 189). |

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
| `reach` | `area`, `player`, `count` (1), `unit` (catalog id prefix, optional) | at least `count` different living matching units of the player stood in the area since the trigger was armed; they need not stay (prompt 189: a breakout; the remembered ids are state and hashed once one is there) |
| `bridge_down` | `area` | a map bridge (a piece of a long bridge) whose center lies in the area is down: blown by the script or engineers, or shot to pieces (prompt 116); loading fails if no bridge has its center in the area |

| `do` | Keys | Effect |
|---|---|---|
| `text` | `text` | Shows the message (replaces the previous one). |
| `objective` | `objective`, `state` (`hidden`, `active`, `done`, `failed`) | Sets the objective's state. |
| `spawn` | `group`, `owner`, `unit`, `count`, `at`, `hold_fire`, `arrive` (`appear`), `from`, `ride` | Units arrive at the place (not buildings) and join the group: they appear there in formation, or with `arrive = "drive"` drive in from the map edge in a column ([below](#reinforcements-drive-in)). `ride`: squads mount carriers of that group at once; vehicles and guns mount boats of that group (prompt 119). Boats `appear` on the water; `drive` is for land units. |
| `order` | `group`, `to`, `order` (`advance`: halt to fight on the way; `move`: drive through) | The group's living units move there, as a command of their owner. |
| `fire` | `group`, `hold_fire` | Fire mode of the group. |
| `barrage` | `group`, `at`, `radius_m` (0), `salvos` (1) | The group's units with an indirect weapon (mortars, howitzers, rocket launchers) fire at the place, as a `Barrage` command of their owner (prompt 082). |
| `air` | `mission` (`recon`, `recon_loiter`, `strafe`, `attack`, `bomb`, `airborne`), `at`, `to` (optional), `player` (default: the mission's) | The player calls air support on the place, with `to` along the line `at` → `to`; price and cooldown apply as for any call (prompt 082). |
| `camouflage` | `group`, `on` (default `true`) | Vehicles and guns of the group put up or take off their camouflage (prompt 082). |
| `shell` | `group`, `at`, `radius_m` (60) | The group's units with a weapon that damages map objects shell the buildings within `radius_m` of the place, nearest first, one building per unit (in id order, round again when there are fewer buildings), as `AttackObject` orders of their owner (shelling, prompt 093): they drive into range and fire until the building falls. Used by the battle `valley_fields/all_units` (Sturmtiger against the town's east edge). |
| `camera` | `at` | The camera glides there (presentation). |
| `victory` / `defeat` | – | The human player (and allies) win, or lose. |
| `hopeless` | `text` | The mission can hardly be won any more: shows the text as the message and opens the warning panel (once, only while the match runs; prompt 181). |
| `resupply` | `at`, `radius_m` (40), `pct` (50), `player` (default: the mission's) | Supply containers land (prompt 118): every living unit of the player within `radius_m` of the place that has an ammunition stock and is not riding regains `pct` percent of a full load, free (`Sim::drop_supplies`). A drop in the wrong place names another player or a place where nobody is. |
| `demolish` | `at`, `radius_m` (10) | The prepared charges go off (prompt 116): every intact map bridge piece whose center lies within `radius_m` of the place collapses at the end of the tick, like a piece shot to pieces (what stands on it falls, `sim::Sim::demolish_bridges`); loading fails if no bridge is that close. |
| `land` | `group`, `at` | The group's boats run onto the beach at the place (prompt 119, `Land` of their owner): they drive to the open water nearest to it, run on until they ground, lower the ramp. A boat aground backs off first. A plain `order` onto land lands boats too. |
| `unload` | `group` | Everybody aboard the group's boats and vehicles gets off (`Disembark` of their owner): from a boat only once it lies on the beach, so `land` and `unload` can go out together. |

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
| Holding the meadow | first wave 50 % (one T-34, one squad), second wave 50 % | first wave 150 %, second wave 150 % and 30 % earlier | every Panzer IV lost while the point is held |
| Two bridges | garrison and counterattack 50 %, the counterattack 50 % later | a Panzer IV in the garrison, computer `normal`, everything 150 % | one Sherman left on the side while both objectives are open |
| Breaking out | screen 50 %, the tanks 50 % later | an anti-tank gun in the screen, screen 150 %, the tanks 150 % and 40 % earlier | fewer than six units left before six are out |
| The great bridge | north riflemen 50 % (one squad), no anti-tank team, fuse 150 % (15 minutes) | computer `normal`, guards 150 %, fuse 75 % (7.5 minutes) | one Sherman left before it is over the river |
| Holding the bridge | waves 60 %, 30 % later | computer `normal`, waves 140 % and 15 % earlier | one airborne section left while the north end is held |
| Landing in the gulf | defenders 60 % | an anti-tank gun in the village, computer `normal`, defenders 140 % | fewer than four infantry units left |

Measured against the test autopilot (`every_level_of_every_mission`,
seed 42; game time to victory, Easy / Normal / Hard; since prompt 180 the
autopilot attacks the group a `destroyed` objective names): tutorial 153 s
on every level; meadow crossing 390 / 391 / 84 s; village square 80 / 80 /
202 s; town in the snow 82 / 83 / 72 s. Chapters 2–5 (no levels): 368,
201, 167 (one tank lost on purpose by the route) and 50 s. Hard is not always longer: with `computer = "normal"`
the enemy comes out to fight the autopilot's column instead of waiting.
Prompt 190 (same test, both seats on the one autopilot): two bridges 265 /
925 s, not won on Hard. The autopilot now splits its units by the nearest
active objective (a front per seat) and never attacks units riding in a
vehicle; the game's self-test plays cooperative missions on Easy (on
Normal its autopilot let the half-tracks break through). Prompt 189 (same test, Easy / Normal / Hard): holding the meadow 303 /
303 / 303 s (the relief decides it; 1 / 0 / 2 own units lost), breaking out
69 / 65 s and not won on Hard (the anti-tank gun); first versions were not
winnable: an anti-tank gun in the breakout's screen on Normal shot every
Sherman of the autopilot from the front, and the defense with three T-34s
in the first wave and two Panzer IV was lost on Easy (the autopilot went
out to meet them; it now holds an objective with a timer and attacks only
enemies within 120 m of its area). On Hard with two Panzer IV at the crossing (`enemy_start_pct = 200`) the
autopilot lost meadow crossing after 98 s, so that level only grows the
counterattack.

### `[rating]`

Prompt 189. `par_s`: win within this many game seconds for the time star
(0 or missing: the star comes with the victory, e.g. for a defense whose
length the script sets); `losses_pct` (default 25): the most own units lost,
in percent of all own units of the match. `Sim::mission_rating` gives the
stars once the player won (`SimHost.mission_info()["rating"]`).

### Campaigns

Prompt 189. `data/campaigns/<id>.toml` orders missions (by id and map) and
holds the texts between them; translations in `<id>.de.po` next to it
(`just mission-po` writes them with the missions'). The simulation never
reads it (`sim::campaign`, bridge `SimHost.campaign_header`).

```toml
[campaign]
title = "Summer front"
order = 0              # position among campaigns
briefing = "…"         # shown with the first mission

[[mission]]
id = "meadow_crossing" # mission file name without .toml
map = "twin_meadow"
intro = "…"            # before the mission
outro = "…"            # once it was won
```

The campaign "Summer front" plays meadow crossing, holding the meadow (the
same meadow from the other side), the village square, breaking out and the
town in the snow.

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
- `crates/sim/tests/missions.rs` (prompt 118): a supply drop refills the
  units near it, not those farther away or of the enemy.
- `crates/sim/tests/mission_levels.rs` (prompt 181): the values of each
  level, `levels` of groups, level errors, start resources, the `fewer`
  condition with `open` and the `hopeless` action.
- `crates/sim/tests/mission_coop.rs` (prompt 190): free seats merged,
  taken seats on the leader's team, units passed on when a human gives up,
  errors; `crates/net/tests/lobby.rs` carries mission and level in the
  setup; `just lan-selftest` plays `two_bridges` with the client in the
  seat (it gives up, the host gets its units, then gives up).
- `crates/sim/tests/mission_rating.rs` (prompt 189): `reach` counts units
  that left the area again, timers and counts of objectives, optional
  objectives and the rating, errors; `sim::campaign` tests the campaign
  format.
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
  prints the units while tuning a mission;
  `MISSION=<id> cargo test --release -p ai --test missions one_mission_every_level -- --ignored --nocapture`
  plays one mission on every level (prompt 118).
- `just selftest` plays every tutorial chapter and every mission from the
  menu to victory (`menu/mission_self_test.gd`; the first mission on Easy, the
  second on Hard, chosen in the screen; the warning panel opens and
  closes; prompt 189: timers and counts shown in the panel, a rating and
  stars on the result screen, the campaign screen locked before and open
  with stars after); `just editor-selftest` places,
  removes and saves trigger areas and spawn points.

## Open

- Prompt 190: the leader has no seat name of its own in the lobby; a
  seat cannot be re-taken after its player gave up; competitive missions
  (humans on both sides) are not offered; only one cooperative mission.
- Prompt 189: units that reach a `reach` area stay on the map (they do
  not leave it and can still be shot); a campaign mission cannot be
  restarted from its outro; one campaign only; the time star of a defense
  comes with the victory; the breakout is not won by the test autopilot on
  Hard and is short (about a minute) for it on Normal.
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
