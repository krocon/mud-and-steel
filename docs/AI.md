# Computer opponent

Prompt 024. A computer player (`crates/ai`) commands the enemy in single player.
It is a normal player (rule 5 of `CLAUDE.md`): it sees only what its player may
see and acts only through ordinary `sim::Command`s.

## Where the AI runs

**On one machine, outside the simulation, as a player of the lockstep session.**
In single player the local `SimHost` runs it; in a LAN or online match the
host runs the computer slots of the lobby (prompt 037, [TEAMS.md](TEAMS.md)).
Its commands travel through
`net::LockstepSession` in frames like a human's and are recorded in the replay.

| Criterion | AI in lockstep on every peer | AI on one machine (chosen) |
|---|---|---|
| Replays | Hold no AI commands: every AI change alters old replays and the golden hash. | Hold the AI's commands; they play back without AI code, also after AI changes. |
| Desync risk | Every line of AI code becomes protocol: no floats, fixed order, the same AI version everywhere. An AI bug is a desync. | None from the AI: peers apply only transmitted commands. |
| LAN | No traffic for the AI. | The host sends frames for each AI player; if the host leaves, the match ends anyway (star topology, no reconnect). |
| Computing time | On every peer, inside the tick budget. | On one machine; commands take effect `input_delay` ticks later anyway. |
| Fog of war | The AI would sit inside the full state. | The same, solved by the filtered `PlayerView`. |
| Effort | Small for single player, but golden hash, replay format and lockstep rules would be touched by every AI tweak. | A little lockstep code (hosted players), then the AI can change freely. |

The AI still follows the simulation's rules (no floats, no hash iteration, no
clocks, its own `sim::Rng`; same lints as `sim`), so tournaments give the same
results on every platform. That is for reproducible measurements, not for sync.

## Building blocks

```
SimHost (bridge/ai_funcs.rs) or ai::game::Game (CLI, tests)
   │ every tick, for every hosted player:
   │   PlayerView::new(&sim, player)     fog-of-war filter (Sim::snapshot_for)
   │   Ai::think(&view) -> Vec<Command>  usually empty; acts every N ticks
   ▼
LockstepSession::submit_hosted(player, commands)   same tick as the local frame
   ▼
frames → peers, replay, Sim::step
```

- **`PlayerView`** (`crates/ai/src/view.rs`) is the AI's only input: own units
  completely (position, move target, hit points, fire mode), enemy units only
  while the player sees them, and only what one sees from outside (position,
  type, hull and turret direction, hit points; not their orders or fire mode).
  Map and config are public (size, start positions, unit types). A test builds
  two matches that differ only in where a hidden enemy stands and checks that
  views and decisions are identical.
- **Hosted players** (`LockstepSession::host_player`, `submit_hosted`): a peer
  sends the frames of further players for the tick of its own last frame, and
  their hash reports (other peers cannot tell hosted players from peers).
  Frames from other peers for a hosted player are rejected. Single player with
  an AI therefore has two players but no remote one: no hash exchange, not
  "networked" (pausing works).
- **`ai::game::Game`**: a headless match in which every player is an AI, through a
  lockstep session (one AI local, the others hosted) with a replay recorder. The
  CLI tournament and the tests use it.

## Behavior

Every role of unit is commanded as one group: armor and infantry (prompt 022)
with the same rules, except that armor attacks enemy armor first, infantry
attacks only enemy infantry (small arms cannot hurt tanks) and stops when it
sees nothing but enemy armor; unarmed units stay where they are. Per group, every think:

1. **Memory:** enemies in sight are remembered with position and tick; wrecks are
   forgotten, and so are sightings an own unit comes close to without seeing
   the enemy there.
2. **Retreat** (Hard): units below 30 % hit points pull back 80 m behind the
   group while enemies are in sight, and keep firing at will; they rejoin when
   no enemy is in sight.
3. **Engage** enemies within sight range of the group's center:
   - focus fire: the whole group gets one `Attack` order on the nearest enemy
     (Normal, Easy) or on the weakest enemy in range (Hard). The target is kept
     while it is in sight. Attack orders let the simulation drive the units into
     range (90 % of the gun's range) and fire from standing.
   - orders are repeated every 100 ticks (5 s): units drop attack orders on
     targets they lose sight of.
   - **stalled fight:** if the visible enemies lose no hit points for the
     patience time, the group drives to a point beside them (70° off the line
     of fire, at 60 % of its range), alternating sides, then attacks again.
     Wrecks stop rounds and units do not fire through comrades, so two lines
     can otherwise shoot into wrecks forever.
4. **Advance** with nothing in sight: gather stragglers first (Normal, Hard: all
   within 60 m + 8 m per unit of the center), then drive as one group (`MoveTo`
   with formation) to the freshest sighting, else to the enemy's start position
   (from the map), else to random points. A move is repeated only when its
   target moved by more than 40 m or the group stopped early.
5. At its first think the AI sets all own units to fire at will.

## Base building (prompt 023)

In matches with buildings ([GAME_DESIGN.md](GAME_DESIGN.md)) the AI also plays
the economy (`crates/ai/src/economy.rs`), with the same commands as a human:

- `PlayerView` adds the account, the own structures with their queues, the
  supply points (public) and, for every structure type the player can build
  now, the first free spot around its headquarters (`Sim::find_site`, the
  placement check of the simulation). The check sees units the player does
  not see; a construction site cannot go where one stands, which a human
  learns from the red preview as well.
- **Build order:** barracks, depot, vehicle works, depot, MG bunker of the
  faction (the AT gun position of Hard went with prompt 034). One step at a
  time; the price of the
  missing step is kept in reserve before anything is produced, otherwise
  infantry eats the manpower and the vehicle works is never built.
- **Production:** every think, completed barracks and vehicle works get an
  order while their queue is shorter than 1 (Hard 2), from a mix per
  difficulty (indices into the faction's list in `data/economy.toml`:
  Normal infantry rifle, MG, rifle, AT and vehicles medium, medium, tank
  destroyer, light (de/us: the halftrack), AT gun (prompt 034); Hard more
  AT and heavy tanks; Easy cheaper units). The AI
  saves up for the next unit of the mix instead of skipping it. Rally points
  are set once to a point 45 m in front of the headquarters.
- **Waves:** with a base, a group with nothing in sight goes to the nearest
  supply point it does not hold (if that is nearer to its base than to the
  enemy's), else to the rally point, until it has the wave size (Easy 8,
  Normal 6, Hard 5); then it attacks as before until it shrinks below half.
- **Targets:** buildings are attacked last: armor fights armor, then other
  units, then buildings; infantry groups with a weapon of 20 mm or more (AT
  squads) also take on armor and buildings once no enemy infantry is near.
- The enemy's home is the enemy's first building if the scenario places one.

`just ai-tournament --mode base` plays the base-building skirmish. Results
(10 games per pairing, at most 24 000 ticks = 20 min, test_small, seeds
1–10, Apple M1 Max, release, 16 s together; prompt 034: halftracks carry
infantry, Normal builds an AT gun, Hard no gun position; before in
brackets, measured with the code of prompt 038):

| Pairing (A vs B) | A wins | B wins | timeouts | mean decided |
|---|---|---|---|---|
| hard vs passive | 100 % (100) | 0 % | 0 | 218 s |
| normal vs passive | 100 % (100) | 0 % | 0 | 215 s |
| easy vs passive | 100 % (100) | 0 % | 0 | 216 s |
| hard vs normal | 30 % (10) | 10 % (10) | 6 (8) | 614 s |
| hard vs easy | 60 % (20) | 0 % (10) | 4 (7) | 591 s |
| normal vs easy | 0 % (50) | 20 % (20) | 8 (3) | 733 s |

Normal against Easy got worse: both lose their halftrack to transport duty
(Easy builds halftracks too) and Normal spends on an AT gun; with 10 games
and many timeouts the numbers are noisy. [provisional]

Against the passive enemy every level builds, produces and razes the base.
On `test_small` the bases stand only about 180 m apart (the free ground of
the start meadow; `start_distance_pct` finds no clear spot farther out), so
both bases are in sight and range of each other, armies meet in the middle
and replace their losses, and many games between two AIs end undecided:
after the first base falls the remaining units and fortifications are not
hunted down. Tuning the end game (attacking with everything at the unit
limit, hunting the last units) and measuring on `test_large` (bases 0.9 and
1.4 km from the center) are open. Thinking costs about 5–20 µs per tick on
average, at most a few milliseconds (the free-spot search of the view).

## Teams (prompt 037)

[TEAMS.md](TEAMS.md) has the rules. For the AI:

- `PlayerView::allies` holds the allied units (always visible),
  `PlayerView::enemies` only enemies; `enemy_players` lists the enemy
  players and whether they are defeated.
- Every enemy has a home (its first building, else its map start, else the
  centroid of its spawns). The AI heads for the nearest home of an
  undefeated enemy that it has not searched yet, then the next.
- Helping allies is kept simple: with shared vision the enemies an ally
  fights are sightings of the AI too, so its groups advance there.
  Barrages keep their safety distance from allied units as well.
- End game of base matches: with 85 % of the unit limit (68 units) the AI
  attacks without waiting for waves; with nothing left to search it goes
  for the supply points enemies hold before random points.

`just ai-tournament --teams 2v2` (map `twin_meadow`) or `--teams 3v3`
(`test_large`): A plays every slot of one team, B the other's; A plays
team 0 (players 0, 2, …) in even games, team 1 in odd ones. Results
(seeds 1–20 or 1–10, Apple M1 Max, release):

| Pairing (A vs B) | Mode, factions | Games | A wins | B wins | timeouts |
|---|---|---|---|---|---|
| hard vs passive | 2v2 battle, de vs su (scenario) | 20 | 40 % | 50 % | 2 |
| normal vs passive | 2v2 battle, de vs su | 20 | 30 % | 55 % | 3 |
| hard vs normal | 2v2 battle, de vs su | 20 | 30 % | 40 % | 6 |
| hard vs passive | 2v2 battle, su vs su | 20 | 5 % | 40 % | 11 |
| hard vs passive | 3v3 battle, de vs su, `test_large` | 10 | 50 % | 50 % | 0 |
| normal vs passive | 2v2 base, 24 000 ticks | 10 | 100 % | 0 % | 0 |
| hard vs normal | 2v2 base, 24 000 ticks | 10 | 40 % | 0 % | 6 |

**Reading the numbers.** In base matches the AIs play the teams as
intended (every one builds, the passive team falls every time). In the
battle mode the side decides, not the level: the attacking AI loses
against armies that stand and fire, and many games end with a few units
that are never found. The same happens without teams on `twin_meadow`
(1 vs 1, hard vs passive, su vs su: 50 % wins, 45 % timeouts, before and
after this prompt): the battle AI was tuned on `test_small`, where the
armies start inside each other's range. Tuning the approach (stopping at
the own range, moving as one front with the ally) and the hunt for the
last units is open.

## Transport (prompt 034)

`crates/ai/src/transport.rs`. Trucks and halftracks are the role
`Transport`, towed guns the role `Gun` (both not commanded as groups;
squads aboard or on their way to a carrier are left out of the infantry
group):

- An empty carrier first fetches a towed gun that stands more than 150 m
  from its place (`Tow`, if the carrier is strong enough), else takes the
  idle squads within 120 m that are more than 200 m from the objective
  (`Embark`, as many as its seats hold).
- A loaded carrier leaves once nobody is still walking up to it (with a
  base: once the infantry or armor wave has left), drives to 180 m short of
  the objective (the visible enemies' center, else the freshest sighting,
  else the enemy start) and unloads there (`Disembark`), or at once when an
  enemy comes within 180 m.
- A tractor with a gun drives to the guns' place (the base's gathering
  point, else 60 m behind the army towards home) and unlimbers there; the
  gun then fires at will.
- Boarding and towing jobs are given up after 600 ticks.

Tank riders are not used by the AI (players only). Test:
`ai_rides_trucks_and_tows_guns` (`crates/ai/tests/ai.rs`).

## Supply and repair (prompt 036)

`crates/ai/src/supply.rs`, all difficulties. The view carries every own
unit's stocks (`OwnUnit::stock`) and the own and allied suppliers and
repairers (`PlayerView::sources`).

- Armor, infantry, artillery and transports with ammunition or fuel at or
  below the low marks of `data/logistics.toml` (25 % / 20 %), and mobile
  vehicles at or below half their hit points, drive to the nearest source
  that serves them (half way into its radius). Their group leaves them
  alone until they are back at 90 % (or the trip is given up after three
  minutes, or the unit is stranded); the order is repeated every 400 ticks
  while the unit stands outside the radius.
- Immobilized or dry units are not moved; trucks are not sent to them
  (open).
- Completed buildings below 70 % of their hit points are ordered repaired
  when every resource is at least 200 (production first). The AI never
  sells.

Test: `ai_sends_empty_units_to_resupply` (`crates/ai/tests/ai.rs`).

## Experience (prompt 050)

`OwnUnit::rank` carries the rank of every own unit. Normal and Hard pull a
damaged unit of rank veteran or better (index 2 of `data/experience.toml`)
back out of the fight earlier than others (Normal below 40 %, Hard below
50 % of its hit points; for squads the hit points are the living
soldiers), once per fight, while healthier units of the group stay;
recruits are pulled back as before (Hard below 30 %, Normal never). Test:
`normal_pulls_back_hurt_veterans` (`crates/ai/tests/ai.rs`). [provisional]

## Engineers (prompt 035)

`crates/ai/src/engineers.rs`, Normal and Hard only. Engineer squads have
their own role (not part of the infantry group). With a base the AI
produces one engineer squad once its vehicle works stand (earlier it
starved the build order of Hard). Chokepoints come from the public map when
the AI starts: the own end of every intact bridge on the own half (16 m
beyond the deck, nearest to the line between both homes first), then the
points at 30 % and 40 % of the way to the enemy. The engineers lay one
anti-tank section at the next chokepoint whenever they have no site under
way, at most two per match. Hostile mines the player knows
(`PlayerView::mines`) within 150 m of the army or the home are cleared
first. Units avoid known hostile fields by themselves (path search). Test:
`crates/ai/tests/ai.rs::ai_lays_mines_that_the_enemy_does_not_see`.
[provisional]

## Difficulties

| | Passive | Easy | Normal | Hard |
|---|---|---|---|---|
| Orders | none (units only fire at will) | yes | yes | yes |
| Reaction (thinks every) | – | 80 ticks (4 s) | 20 ticks (1 s) | 10 ticks (0.5 s) |
| Common target | – | nearest | nearest | weakest in range |
| Gathers stragglers | – | no | yes | yes |
| Pulls back damaged units | – | no | no | below 30 % |
| Pulls back veterans (prompt 050) | – | no | below 40 % | below 50 % |
| Patience before flanking | – | 1600 ticks | 800 ticks | 800 ticks |
| Artillery (prompt 032) | – | with the group | with the group | own role: barrages, smoke |

`Passive` is the baseline of the tournament and the enemy of the matches
before prompt 024; the self-test's first two matches use it.

## Artillery (prompt 032)

Units with an indirect weapon (mortar teams, self-propelled guns) are their
own role for Hard (`crates/ai/src/artillery.rs`); the other difficulties
command them with their infantry or armor group, and the simulation turns
their attack orders into indirect fire.

- With no enemy in sight the artillery stays 80 m behind the army (towards
  home), or at the gathering point of the base.
- When enemies come into sight after a quiet spell, smoke-capable units fire
  two salvos of smoke 40 m in front of the enemy towards the army (at most
  every 800 ticks).
- Every 200 ticks the units in range fire a barrage (3 salvos, 15 m) on the
  largest visible enemy group (most enemies within 25 m of one).
- Hard builds a mortar team in its infantry mix. Not the self-propelled gun:
  with it in the vehicle mix, saving up for it starved the tank production
  and Hard lost 4 of 8 base matches against Normal (before: 8 timeouts).

## Air support (prompt 033)

Normal and Hard call aircraft with `Command::CallAir`
(`crates/ai/src/air.rs`); the view carries the player's air options
(`Sim::air_options`: price, cooldown, why blocked) and the number of enemy
aircraft in sight.

- At most one call every 400 ticks. Target: the largest visible enemy group
  (as for the artillery), with no own unit within 150 m (the aircraft need
  10–30 s to arrive, and bombs and strafing hurt friends too). Five or more
  units: bombers, else ground attack, else fighters, whichever is ready.
  Fewer than three units: no attack.
- After 1200 ticks (60 s) without any enemy in sight: a reconnaissance
  flight on the newest remembered enemy position (else the enemy's start).
- In base matches a call needs twice its price in the account, so the
  economy keeps its resources.
- Once enemy aircraft appeared, the AI builds the faction's light flak in its
  base (one, needs barracks).
- Easy and Passive never call aircraft.

## Game and menu

- Single-player setup: **Opponent** Passive / Easy / Normal (default) / Hard
  (`menu/match_setup.gd` `OPPONENTS`). `main.gd` sets
  `SimHost.ai_opponent` before `start_match`; direct starts take
  `--ai=<level>` (`just run --skip-menu --ai=hard`), default none.
- `SimHost.ai_status()`: difficulty, number of AI players, decisions so far.
- The AI uses the match seed for its random choices, so a single-player match
  with the same seed and the same own orders plays the same.

## Tournament

```bash
just ai-tournament                                  # every difficulty against every lower one, 40 games each
just ai-tournament --a hard --b normal --games 100  # one pairing
just ai-tournament --factions su,su --verbose       # mirror match, every game printed
```

`cli ai-tournament` plays the demo scenario (eight tanks per side, 120 m apart)
on `test_small` (`--map` for others). Game *i* uses seed *seed + i*; A plays
player 0 in even games and player 1 in odd ones, so with the scenario's
factions each AI plays both Germany and the Soviet Union. A game not decided
after 6000 ticks (5 min) is a timeout. Games run on all cores; every game is its
own deterministic simulation, so the **result hash** over all games is the same
for any number of threads and on every run (checked: `--jobs 1` and parallel
give `0x4f59adf0e6662daf` for the default matrix).

Results (40 games per pairing, seeds 1–40, Apple M1 Max, release; all
pairings of all four faction settings take about 3 s together):

| Pairing (A vs B) | de vs su (scenario) | de vs de | su vs su | us vs us |
|---|---|---|---|---|
| hard vs passive | 100 % | 100 % | 100 % | 100 % |
| normal vs passive | 100 % | 100 % | 100 % | 100 % |
| easy vs passive | 100 % | 100 % | 100 % | 100 % |
| hard vs normal | 50 % | 60 % | 57 % | 62 % |
| hard vs easy | 90 % | 95 % | 72 % | 62 % |
| normal vs easy | 50 % | 97 % | 50 % | 67 % |

(Win rate of A; no draws, no timeouts.) Matches against the passive enemy last
14–16 s (Easy 38 s) with 6–8 of 8 tanks left. The AI needs about 1 µs per
tick on average; a think costs at most a few milliseconds (path-free, but
cloning the snapshot).

**Reading the numbers.** With the scenario's factions, pairings of near-equal
AIs end 50 %: the Panzer IV side wins (range 375 m against 250 m, 130 mm
penetration against 109 mm), whoever plays it. The mirror matches show the
order Hard ≥ Normal ≥ Easy; on a 120 m start distance the fight starts at
once, so reaction time and target choice decide little. Larger maps with
starts far apart (meeting engagements) should separate the levels more.

**Measured and rejected:**

- *Flanking early* (patience 200–300 ticks): lost up to 18 % of the games
  against the passive enemy (driving sideways under fire, flanks exposed).
- *Backing off to the own range* when it exceeds the enemy's (Panzer IV against
  T-34): the tanks turn round and show their rear armor; Hard won 4 of 60
  instead of 30 of 60 against Normal.
- *Splitting fire* (two or three attackers per target to avoid overkill):
  clearly worse than one common target (5–9 of 60 against Normal).
- *Every unit at its own nearest enemy* (old Easy): lost every game as the
  Soviet side against the passive Germans.

## Tests

- `crates/ai/tests/ai.rs`: Normal and Hard beat Passive in 6 of 6 games (both
  sides), Easy in 4 of 4; Passive gives no command; tournaments are
  reproducible; only own living units are commanded; a replay of an AI match
  plays back without the AI to the same end hash; the view leaks no hidden
  units.
- `crates/net/tests/hosted.rs`: a peer hosting a second player plays in sync
  with another peer (the result equals the scripted demo), frames from
  another peer for a hosted player are rejected, and single player with a
  hosted AI needs no peer.
- `just selftest` (flow test, match 3): the hard computer opponent beats tanks
  that hold fire; matches 1 and 2 play against the passive opponent; match 4
  (prompt 023) checks that the normal opponent builds in a base match.
- `crates/ai/tests/air.rs` (prompt 033): Normal calls bombers on a large
  group far from its own units, Easy does not; Hard builds light flak once
  it saw enemy aircraft.
- `crates/ai/tests/economy.rs` (prompt 023): every level builds barracks
  and depot and produces units against the passive enemy; base matches
  between two AIs are reproducible and both sides build.

## Open

- Infantry tactics: stances, cover spots and buildings (prompt 022) are not
  used by the AI yet; attacking squads walk in standing and lose against
  kneeling defenders. The end game of base matches (see above),
  using terrain (hull-down positions, woods, crests). Artillery (prompt
  032) is Hard only and simple: no observers sent forward, no
  counter-battery, smoke only when the enemy first appears.
- Air support (prompt 033) is simple: no recon before attacks, no flak
  placed by threat direction, no heavy flak, no call on structures.
- Meeting engagements on large maps for a better spread of the levels.
- Formation facing and speed matching (pathfinding, see ARCHITECTURE.md).
