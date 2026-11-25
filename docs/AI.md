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
   target moved by more than 40 m or the group stopped early. Normal and
   Hard advance with attack-move (prompt 046: `FormationMove` with
   `engage = All`, formation Auto): every unit halts while an enemy is in
   its range and drives on afterwards; at the rally point of a base match
   the group faces the enemy's home. Easy keeps plain `MoveTo`
   [provisional].
5. At its first think the AI sets all own units to fire at will.

## Commander (prompt 051)

Normal and Hard lead their army as a commander (`crates/ai/src/commander.rs`,
`intel.rs`, `scout.rs`, `surrender.rs`); Easy keeps the group logic above.
`Difficulty::HardOld` (`--a hard-old`, not in the menus) is Hard without
the commander, the behavior of prompts 024–055, kept for tournaments. The
commander still reads only the `PlayerView`: sightings of its own (and
allied) units and its own losses.

**Situation picture** (`intel.rs`):

- Sightings carry type, owner, hit points and hull direction. Knowledge
  ages: full weight for 30 s, then fading to nothing at 4 minutes; mobile
  units older than that are forgotten (buildings do not move and are kept).
  A separate list of every enemy ever seen and not seen destroyed gives the
  enemy's composition (counter-production) and known strength (surrender).
- **Danger** at a point: the value of own units lost within 120 m (full for
  2 minutes, fading over 2 more) plus the value of freshly seen enemies whose
  weapons reach the point. Staging points and scouting goals avoid it.
- **Fronts:** fresh sightings within 160 m of each other form one front with
  a center, strength (price × hit point share × freshness), anti-tank and
  overall weapon reach, and the mean hull direction of its vehicles.
- Unit value is the price of `data/economy.toml` (sum of the three
  resources).

**Army plan** (the lead group is armor while any armor can still fight,
else infantry; infantry follows 50 m behind the armor while enemy armor is
known, else it hunts on its own; prompt 198: in the assault, with an enemy
in sight within 300 m and no support vehicle in the army, the squads within
90 m of the armor go along in its order and advance behind the tanks,
docs/INFANTRY.md "Infantry behind tanks"):

1. **Stage:** against the nearest front (before anything is seen: the
   enemy's home, assumed to face the own home) the army gathers on a
   staging point 70 m outside the front's reach (before any anti-tank weapon
   was seen: 1.5 × the own range). The axis is chosen among 0, ±30, ±60, ±90°
   (Normal 0, ±45°): flank or rear of the hulls seen, little danger, short
   way round, not an axis that failed; the way leads round the front in
   legs of 45° at that distance. Armor that clearly outranges the front's
   anti-tank weapons stages and fights at 85 % of its own range instead.
2. **Assault:** gathered (or after 30 s there, 3 minutes at most) the army
   drives in line with attack-move to 60 % of its range; while nothing comes
   into sight it creeps 60 m closer every 10 s; after a minute without
   contact the group logic leads for two minutes (attack-move to the
   sightings, buildings included).
3. **Fight:** the focus fire of prompt 024. Armor first closes in to 45 % of
   its range when it stands beyond 60 % of it (dispersion grows with the
   distance), except when it outranges the enemy; with no anti-tank weapon
   at the front it drives to 60 m (machine guns). Every 15 s the fight is
   reviewed: losing more than twice what it inflicts, the army moves round
   to another axis at 85 % of its range; clearly outmatched (30 % of its
   strength lost and below 60 % (Normal 40 %) of the front's) it
   **withdraws** 150 m beyond the front's reach and attacks again over
   another axis.
4. **Hold:** when the front comes 12 m closer within 3 s (the enemy
   advances), the army stops in line facing it and lets it come.
5. **Maneuvering enemy:** once a known enemy unit was seen 8 m away from
   where it was seen before (within the last 2 minutes), the army stops
   going round fronts: it holds against an advance and otherwise advances
   with the group logic (attack-move), and in the fight it neither closes
   in nor moves round a flank (driving under fire lost the exchange
   against the old Hard). Staged flank attacks are for static defenses.

[provisional] all distances and shares above; they were tuned against the
passive enemy and the old Hard on the ten maps of the tournament.

**Typical combat distances (prompt 067).** The AI's fighting distances are
shares of the weapon ranges it reads from the unit types (90 %, 85 %, 60 %
of the range, 1.5 × the own range before anything was seen) and of the
sight ranges, so they followed the shorter ranges without a change in
`crates/ai`: armor that fought from 225–340 m now fights from 105–160 m
(75 mm guns reach 175 m instead of 250–375 m). The fixed distances (staging
70 m outside the front's reach, fronts of 160 m, danger within 120 m,
scouts turning back 200 m) are map distances, not weapon ranges, and were
kept; the battle tournament of [BALANCE.md](BALANCE.md) was measured with
them. The numbers quoted below from prompts 024–051 are from before
prompt 067 (Panzer IV 375 m, T-34 250 m).

**Vehicle speed and ranges after a reference video (prompt 097).** Ground
vehicles drive their real speed (half of before), direct fire and sight
are compressed 1/8 instead of 1/4 ([SCALE.md](SCALE.md)). The shares of
the ranges follow by themselves; what did not:

- **Closing in keeps the group's places.** `close_in` sent the armor into
  a line formation; a line of eight tanks is about 85 m wide at full size,
  and at 87.5 m gun range the outer tanks drove across the front for
  twice as long (in real distance) as before, under fire. Normal and Hard
  then lost 40 % of their attacks on a passive army of Panzer IV with
  T-34s (60 games on `test_small`; before 4 % and 15 %). Now every tank
  drives the same way forward (`MoveTo` per tank): Hard and Normal win
  100 % against Passive again.
- **Stalled fights.** Wrecks are full size and block the line of fire;
  at the short ranges they often stand between the last tanks of both
  sides, which then fire into the wrecks for minutes. A fight in which
  neither the enemies in sight nor the own army lose hit points for
  `STALL_TICKS` (160 ticks, longer than the slowest reload) makes the
  commander's armor step 20 m to the side (`sidestep`, alternating). The
  group logic's patience move (Easy) goes round the same way every time
  instead of alternating sides (which swung back behind the same wrecks),
  and its flank point lies at least 30 m (was 60 m) from the enemy.
- **Halved with the ranges:** `OVERRUN_M` 60 → 30 m (inside the 50 m of the
  vehicles' machine guns: tanks stopped at 60 m beside the last enemy squad
  and could not reach it), `STAGE_MARGIN_M` 70 → 35, `WITHDRAW_M` 150 → 75,
  `CREEP_M` 60 → 30, `FRONT_LINK_M` 160 → 80, `LOSS_RADIUS_M` 120 → 60,
  scouts turn back at 175 m (half their sight; was 350 m), passengers get
  off 90 m short of the enemy (was 180 m, beyond every gun then and now).
- **Strike evasion:** supply and transport no longer send a unit back into
  the area of an enemy heavy strike while it leaves it (the slower trucks
  made this visible).

**Scouting** (`scout.rs`): reconnaissance vehicles (catalog category
`recon_vehicle`) are their own role. They drive to the 200 m cell the
player saw longest ago (nearer to the enemy's home and to the scout first,
no cell with known danger) and turn back 200 m once an enemy is within
350 m. Base matches build one whenever none is left (every sixth vehicle
at most). Air reconnaissance (prompt 033) stays as it was.

**Counter-production** (`economy.rs`): from the enemies seen, mostly armor
(at least half the value): rifle squads become AT squads and every other
medium tank a tank destroyer; mostly infantry (at most 30 % armor): AT
squads become MG teams, tank destroyers medium tanks, every third rifle
squad a mortar team. Flak after enemy aircraft as before. When the next
unit of the mix waits 30 s for one resource (ammunition ran dry while
manpower piled up), the commander builds the first affordable unit of
rifle squad / MG / AT squad (vehicles: medium tank / tank destroyer /
reconnaissance car) meanwhile.

**Supply:** waves take the free supply points near the own base first (as
before); the second depot and the bunker are built from 100 m in front of
the headquarters towards the map center (`PlayerView::forward_sites`, the
placement check of the simulation), i.e. towards the front. From the fourth
vehicle on, a supply truck is built whenever none is left; the transport
logic carries infantry with it and it refills units within 40 m near the
front [provisional: a forward depot on wheels; depots far forward are not
possible, buildings need an own building within 60–120 m]. With no enemy
armor near, the commander's armor shoots enemy trucks and halftracks first
(raids on the enemy's supply). Protecting own trucks beyond keeping them
behind the army is open.

**All-in:** in base matches the army stops waiting for waves once its
armed units are worth twice every enemy unit known.

**Other systems:** formations and attack-move (046) for every move, the
artillery's smoke screen also when an assault begins (032), air support
(033), transport (034), engineers (035), teams through shared vision (037),
the heavy strike (054) and bridges (055) as before; fire and movement of
squads (047) and wounds (049) are the simulation's.

**Surrender** (`surrender.rs`, Easy, Normal, Hard; not `HardOld`): after a
minute of a hopeless situation the AI sends `Command::Surrender`. Hopeless
[provisional]: no completed barracks or vehicle works (or production but
no headquarters), and either none of its armed units with ammunition can
hurt any enemy unit it knows to be alive (rifle squads against tanks), or
its armed units are worth less than 15 % (without headquarters 10 %) of
the known enemies. Reason: a lost match should end instead of running out
the clock, but never because of units the AI could not see; 15 % is about
one tank against seven.

Tests: `crates/ai/tests/commander.rs` (Hard surrenders when its rifles see
only tanks, the old Hard does not; Hard attacks a passive tank platoon from
the flank, not along the direct line).

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
  With only buildings near, the headquarters, barracks and vehicle works
  come first (decision 6 of prompt 045: a player without them is
  defeated; `[defeat] base_buildings`).
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

Prompt 110: the guns of one place stand side by side, 20 m apart across the
direction of the enemy home (by unit id), and once set up each turns to a
facing whose aim point lies 100 m ahead and twice its side offset off the
center line (`FormationMove` on the spot with a facing): the outer guns look
outwards, so the arcs cover each other's flanks. Not for `HardOld`.

## Fire from the flank (prompt 110)

`crates/ai/src/flank.rs`, Easy, Normal and Hard (not `HardOld`). The
simulation turns every standing vehicle and gun towards hits on its own
(`[flank]` in `data/tactics.toml`, GAME_DESIGN.md "Fire from the flank");
the view gives the AI the hull direction and the remembered direction of
fire of its own units (`OwnUnit::facing`, `OwnUnit::alert`). The AI only
decides when a position is lost:

- Who: own guns and armored vehicles (roles `Gun`, `Armor`) that stand
  (no move target), tow nothing and lost hit points since the last think,
  with fire remembered from more than 60° off the hull front.
- When: they cannot answer (no enemy seen within their range within 45° of
  that direction) or are below 40 % of their hit points.
- What: a vehicle with smoke rounds lays two smoke salvos 40 m towards the
  fire (within its ranges); other armor backs off 40 m away from the fire
  (`MoveTo`) and its group leaves it alone for 20 s; a gun is towed 40 m
  back by the nearest free tractor within 250 m that can pull it, and set
  up there facing the fire (the transport keeps the new place until the gun
  is unlimbered). A gun without a tractor stays and keeps turning (the
  simulation has no crew that leaves its gun for cover).
- At most once per minute per unit, so the AI does not regroup at every
  shot.

Tests: `crates/ai/src/flank.rs` (a tank hit from the side by an unseen
enemy backs off, not again within the cooldown; fire from the front or from
a seen enemy changes nothing; a flanked gun is towed away by a nearby
truck, without one it stays). Test scenario with numbers: BALANCE.md
"Fire from the flank (prompt 110)".

## Stances and positions (prompt 178)

`crates/ai/src/posture.rs`. The simulation's automatic stance (prompt 047)
already kneels squads that stand, fight or are fired at while moving, lays
them down when pinned or in a close fight and lets their fire teams bound;
the AI keeps attacking squads on it. A first rule that ordered a fixed
kneel whenever an enemy came within twice the range switched the bounds off
and stalled attacks (village scenario below, Hard: 30 of 108 defenders
killed before, none with the rule), so it was dropped.

The AI adds, on Normal and Hard: a squad that stands without an order within
80 m of an own supply point or building (it defends) and sees an enemy
within twice its range occupies the nearest enterable building within 30 m
(`Garrison`, at most once a minute); without one it kneels, or lies down
when its suppression is 30 or more; it returns to the automatic stance 10 s
after the last enemy. A stance holds at least 5 s. The view gives the
squads' stance, building and suppression (`OwnUnit::stance`, `garrison`,
`suppression`); the buildings come from the map (public).

Checked, already in place: carriers unload 90 m short of seen enemies
(`transport.rs`), unarmed units stay at home, and since prompt 176 unarmed
vehicles drive out of fire by themselves. Defenders do not leave a point
for a single enemy out of range only as far as the group logic keeps them
(no separate rule yet).

Scenarios (`crates/ai/tests/posture.rs`): two AI rifle squads attack the
village of `test_small` held by three passive squads (4 seeds each):
defenders killed Easy 0, Normal 9, Hard 30 of 108 before and after (the
attack is unchanged). In a base match against Hard, the Normal AI gives
21 house and stance orders, Easy none. Tournament: battle mode unchanged;
base mode between two AIs now all 10-game series end in timeouts (before:
1 of 10 and 2 of 10 decided): defenders in houses hold longer. The AI's
think time per tick is unchanged (about 10 µs).

## Fire discipline (prompt 176)

Every AI (also `HardOld`) sets its towed anti-tank guns (category `gun_at`)
to "vehicles only" (`Command::SetFireLimits`) when it sees one without
that limit, also guns produced later: they do not give their position away
to infantry. Direct-fire support vehicles return fire only (prompt 195,
"Support from the second line"). The other units keep firing at will;
close range is not used by the AI yet (finding for prompt 178: defenders in
woods could wait with "close range only").

## Target choice (prompt 188)

The AI does not pick its units' targets: the simulation's score
(ARCHITECTURE.md "Combat") does it for every player alike. With it the
tournament shifted little (BALANCE.md "Target choice by score").

## Support from the second line (prompt 195)

`crates/ai/src/support.rs`, every level but `HardOld`. Support vehicles
are a property of the unit's kind, not of one id (`TypeInfo::support`): an
assault gun whose main gun has at least 100 mm (Sturmtiger, Sturmpanzer IV,
StuIG 33B, SU-122, M4A3 (105)) and an indirect-fire vehicle that the level
does not lead as artillery (Wespe, Priest, Calliope on Easy and Normal).
They leave the armor group (the commander's lead role and strength ignore
them) and get, per vehicle and think, ordinary `MoveTo`, `Attack` and
`SetFireMode` commands:

- **Escort:** at least 2 effective own armor or infantry units within
  80 m (a squad and a tank, or two squads). Without them it drives back to
  the gathering point (base) or the own start and waits there.
- **Second line:** its distance to the objective (nearest enemy in sight,
  else the nearest remembered one, else for 60 s the last of these, else
  the escorts' move goals, else the enemy home) stays at least 12 m more
  than the foremost escort's; farther forward it backs off, more than 40 m
  behind it moves up. For a shot it may come up to 5 m behind the lead.
- **Targets:** direct-fire support vehicles fire only in return
  (`ReturnFire`; the simulation's score of prompt 188 picks among those
  that fire at it) and get `Attack` orders only on targets within 95 % of
  their range from where they stand: buildings and fortifications, towed
  guns, squads within 15 m of a house, groups of 3 enemies within 25 m;
  never lone squads, trucks, halftracks or scout cars. A worthy target out
  of range is approached to 90 % of the range if that stays behind the
  lead. Indirect support keeps firing at will (`indirect_min_soldiers`).
- **Reload:** after a shot (ammunition went down) it backs off 15 m more
  until the main gun is loaded (Sturmtiger 600 ticks).

[provisional] all distances. They are game meters: the Sturmtiger's 38 cm
reaches 62.5 m in the game (`range_typical_m` 500 m × 1/8), shorter than a
Panzer IV's 87.5 m, so the prompt's 80–150 m (real) would put it out of
range. From the second line it therefore fires mostly when the escorts
close in (infantry in the village); against an anti-tank gun that the
tanks fight from 65 m it does not reach.

Measured (`crates/ai/tests/support.rs`, two Panzer IV, two rifle squads
and a Sturmtiger starting in front of them against two rifle squads beside
houses, an MG team and a ZiS-3 in the `test_small` village, Normal and
Hard, two seeds each): after its first decisions the Sturmtiger is never
nearer the defenders than the foremost escort while they are in contact;
it fires one to three rounds per game (ammunition 100 → 78–92 %) and
backs off to reload. With only the two squads as escorts they fall in 3 of
4 games and the Sturmtiger drives back 80–90 m within 20–25 s, to its
start (escorts in fog of war are compared with every defender, also
unseen ones, so the table can show it a few meters nearer a hidden MG
team than its escort). Tournament: the battle and base matrices
(6 000 ticks) are unchanged to the hash (no support vehicle is built that
early); base matches de vs us over 24 000 ticks, 20 games: hard vs normal
55/35 % (2 timeouts) → 60/35 % (1), hard and normal vs passive 100 % as
before; think time unchanged (51–52 µs per tick).

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

## Bailed-out crews (prompt 049)

Crews that bailed out of destroyed vehicles (catalog `inf_crew`) are no
target and no objective for the AI (`TypeInfo::crew`: left out of the
enemies of a group and of the memory of sightings), and its own crews have
the role `Unarmed` and walk back home. Without that, tanks spent minutes
firing at prone crews in the AI tests while the real enemy waited. The
simulation itself lets crews withdraw 150 m from the fight: crews left
between the lines caught the rounds meant for the tanks behind them
(Easy against Passive won only 65 % of 40 games instead of 92 %; with the
withdrawal 97 %).
Crews do not count for elimination either. [provisional]

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

Positions (prompt 052): once its mines are laid (or while it has no
ammunition for them) the AI defends its chokepoints: at the next one it
digs a 16 m zigzag trench across the way to the enemy 20 m behind the
point, then 24 m of wire 12 m in front of the trench, an MG nest and a
camouflage net, one step whenever no site is under way and it can pay six
pieces; at most two positions per match. Test:
`ai_digs_a_position_after_its_mines`. [provisional]

## Bridge-laying tanks (prompt 055)

`crates/ai/src/bridgelayer.rs`, Normal and Hard only. When the AI starts it
looks for deep water on the line from its home to the enemy (public map):
the first river there and points along it (every 40 m, up to 200 m to both
sides). While no bridge of the map stands within 150 m of the crossing
(`PlayerView::bridges_intact`), it builds one bridge-laying tank of its
faction in the vehicle works (at most two per match; the US has none) and
sends it `LayBridge` at these points in turn until one leads to a laid
bridge (`PlayerView::layers`). The army uses the bridge through the path
search. The tank has no weapon (role `Unarmed`) and stays out of the attack
waves. The AI does not call paratroopers. Test:
`crates/ai/tests/bridgelayer.rs`. [provisional]

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
| Artillery (prompt 032) | – | with the group | rocket launchers only (prompt 062) | own role: barrages, smoke |
| Commander (prompt 051) | – | no | yes, axes 0/±45° | yes, axes 0/±30/±60/±90° |
| Withdraws below (share of the front) | – | – | 40 % | 60 % |
| Surrenders when hopeless | no | yes | yes | yes (`HardOld`: no) |
| Squads defending own points take houses, kneel, lie down (prompt 178) | – | no | yes | yes |

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
- Rocket launchers (prompt 062) are artillery on Normal too. Their barrage
  is one salvo of all tubes, and its target keeps 120 m (instead of 60 m)
  from own and allied troops: the rockets scatter ±55 m at the longest
  range. Normal and Hard build a rocket launcher and an assault howitzer
  (Sturmpanzer IV, M4A3 (105); the Soviet list has none, so the IS-2) once
  per two rounds of their vehicle mix: the mix entries `ROCKETS` and
  `ASSAULT_HOWITZER` find them by catalog category wherever they stand in
  the list (`crates/ai/src/economy.rs`). [provisional]

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

## Heavy strike (prompt 054)

Normal and Hard use the rocket strike of `data/strike.toml`
(`crates/ai/src/strike.rs`); the view carries the player's strike option
(`Sim::strike_option`) and every called strike (`Sim::strikes`, public).

- In base matches the battery position is placed once the vehicle works
  stand and the strike unlocks within 120 s.
- A call needs twice its price in the account. Target: the largest visible
  enemy group of four or more (as for the artillery), else a visible enemy
  building, else the enemy's base; never within the strike radius + 60 m of
  own or allied units.
- Against an enemy strike, own units within its radius + 40 m move out of
  the marked area (busy for the warning time + 30 s: the groups leave them
  alone); the Hard AI also sends its four nearest tanks against the
  revealed battery position.
- Easy and Passive never use it. On `test_small` AI matches are usually
  decided before the unlock (about 8 minutes), so the tournament rarely sees
  a strike. [provisional]

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

Prompt 051: `--a hard-old` plays the Hard before the commander;
`--verbose` also lists the living units of every timeout (why a game did
not end); `--trace N` (with `--games 1`) prints every N ticks the living
units, the commander's plan and every player's account and buildings.

`cli ai-tournament` plays the demo scenario (eight tanks per side, 60 m apart since prompt 097)
on `test_small` (`--map` for others). Game *i* uses seed *seed + i*; A plays
player 0 in even games and player 1 in odd ones, so with the scenario's
factions each AI plays both Germany and the Soviet Union. A game not decided
after 6000 ticks (5 min) is a timeout. Games run on all cores; every game is its
own deterministic simulation, so the **result hash** over all games is the same
for any number of threads and on every run (checked: `--jobs 1` and parallel
give `0xc9539c0a6296c902` for the default matrix, measured with prompt 110).

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

### Results of the commander (prompt 051)

Measured on the merged code of prompts 043/049 (Apple M1 Max, release, all
cores; the machine was shared with other builds, so the times are upper
bounds). Battle mode: the demo armies (eight tanks and two rifle squads,
de vs su) on the map's starts, 20 games, a game not decided after 12 000
ticks (10 min) is a timeout (`--max-ticks 12000`; on 1–2 km maps the
approach alone takes minutes). "Old" is `hard-old`, the Hard of prompts
024–055 on the same code. Win rate of A / of B / timeouts:

| Map | hard vs passive | old hard vs passive |
|---|---|---|
| test_small | 100 / 0 / 0 | 100 / 0 / 0 |
| skirmish | 75 / 0 / 5 | 55 / 0 / 9 |
| twin_meadow | 45 / 55 / 0 | 60 / 30 / 2 |
| test_large | 100 / 0 / 0 | 100 / 0 / 0 |
| mountain_pass | 80 / 0 / 4 | 40 / 0 / 12 |
| winter_village | 85 / 5 / 2 | 25 / 0 / 15 |
| farms | 75 / 0 / 5 | 15 / 0 / 17 |
| river_land | 45 / 25 / 6 | 50 / 0 / 10 |
| town_small | 35 / 40 / 5 | 35 / 0 / 13 |
| desert_village | 40 / 10 / 10 | 20 / 0 / 16 |

Normal / Easy vs passive (battle): test_small 100/0/0 and 100/0/0,
skirmish 50/50/0 and 25/60/3, farms 0/40/12 and 15/60/5, river_land
60/10/6 and 45/50/1. "B wins" against the passive enemy are surrenders:
the AI's armor is gone and its rifle squads face tanks they cannot hurt.

Hard against the old Hard (battle, 20 games; scenario factions de vs su,
A plays both sides): test_small 40 / 60 / 0, skirmish 40 / 50 / 2,
twin_meadow 50 / 50 / 0, test_large 50 / 50 / 0, mountain_pass
20 / 45 / 7, winter_village 55 / 30 / 3, farms 55 / 40 / 1, river_land
35 / 50 / 3, town_small 35 / 25 / 8, desert_village 10 / 65 / 5. Mirror
matches: skirmish su,su 25 / 65 / 2 and de,de 10 / 50 / 8, farms su,su
40 / 55 / 1 and de,de 15 / 75 / 2, twin_meadow su,su 30 / 70 / 0 and
de,de 15 / 80 / 1.

Base mode (`--mode base`, 10 games, 24 000 ticks = 20 min; "before" =
the code of prompt 056 before this prompt):

| Pairing | test_small | skirmish | test_large |
|---|---|---|---|
| hard vs passive | 90 / 0 / 1 (before 100 / 0 / 0) | 100 / 0 / 0 (80 / 0 / 2) | 90 / 0 / 1 (0 / 0 / 10) |
| normal vs passive | 100 / 0 / 0 (100 / 0 / 0) | 100 / 0 / 0 (40 / 0 / 6) | 100 / 0 / 0 (0 / 0 / 10) |
| easy vs passive | 100 / 0 / 0 (100 / 0 / 0) | 100 / 0 / 0 (100 / 0 / 0) | 100 / 0 / 0 (100 / 0 / 0) |
| hard vs normal | 50 / 50 / 0 (60 / 10 / 3) | 0 / 0 / 10 (0 / 0 / 10) | – (0 / 0 / 10) |
| hard vs old hard | 40 / 60 / 0 | 0 / 0 / 10 | – |
| old hard vs normal | 50 / 40 / 1 | 0 / 0 / 10 | – |

(test_large normal and easy vs passive were measured before the merge of
prompts 043/049; "–": not measured again, every game of these pairings
ran out of time before.)

AI time per tick (both AIs, `--jobs 1`): battle (skirmish, hard vs passive, 4 games) 9 µs on average, at most 0.7 ms; base match (skirmish, hard vs normal, 20 minutes) 104 µs on average, at most 2.3 ms (the free-spot searches of the view and the scouting grid).

**Reading the numbers.**

- Against static defenses the commander wins clearly more often than the
  old Hard (farms 15 → 75 %, winter_village 25 → 85 %, mountain_pass
  40 → 80 %, skirmish 55 → 75 %) and base matches against the passive
  enemy on the 2.8 km map `test_large` are decided now (0 → 90–100 %).
  The target of 90 % against the passive enemy on every map is **not**
  reached: as the Soviet side (T-34, 250 m against the Panzer IV's 375 m)
  on small maps where no flank lies outside the enemy's reach
  (`twin_meadow`, starts 170 m apart; `town_small`, 600 m square;
  `river_land`) the passive Germans win the slugfest, and the AI then
  surrenders instead of running out the clock.
- Against the old Hard (an enemy that attacks) the commander is even at
  best, not better: the staged flank approach is caught on the move by an
  enemy that drives straight at it, most clearly in mirror matches.
  Measured and rejected: advancing straight until the first contact
  (mirror matches 45–55 % against the old Hard, but against the passive
  enemy farms 30 %, winter_village 15 %), a probe tank 300 m ahead of the
  army (worse against both), holding against any moving front (no
  effect). [provisional] The trade-off is the user's call: the current
  setting favors beating defenses.
- Base matches between two AIs on 1–2 km maps stay undecided within 20
  minutes (skirmish: all pairings 10 timeouts of 10, before and after):
  ammunition is the bottleneck of both economies (resupply costs it),
  armies rebuild faster than they die, and the old Hard never gives up.
  The commander builds substitutes when one resource is short, attacks with
  everything when twice as strong and surrenders when crushed, which
  decides some games on `test_small`, but the economy balance (prompt 045)
  decides the rest.

### The large city (prompt 079)

`city_large` (3 km, starts 2.26 km apart, a block quarter between them)
needed no AI change. Battle mode, 20 games each, `--max-ticks 18000`
(15 min), win rate A / B / timeouts: hard vs normal 60 / 20 / 4, hard vs
easy 50 / 50 / 0, hard vs passive 5 / 50 / 9, normal vs passive
5 / 35 / 12. Base mode (6 games, 30 000 ticks = 25 min): hard vs passive
100 / 0 / 0 (about 11 minutes each), hard vs normal 0 / 0 / 6 (as on
`skirmish`). The armies take the main streets through the city (the three
routes and the square) and meet in the block quarter and the suburb; the
traces show tanks fighting on the square, on the avenues and at the
suburb rings. Every timeout ends with tanks out of fuel or immobile
against rifle squads that cannot hurt them (both armies drive more than
2 km); no unit stood stuck. Before the suburb streets were widened from
8 to 12 m, three tanks stayed 12 minutes between wrecks in one of them
(prompt 079, decision 4). Against the passive enemy the attacker loses
the slugfest at the enemy start, as on `town_small` and `river_land`.

### The harbor city (prompt 081)

`harbor_city` (3.6 km × 2.4 km from real geodata, a third of the land
too steep for anyone, starts about 3 km apart at the ends of the coastal
strip) needed no AI change. Battle mode, `--max-ticks 18000` (15 min),
win rate A / B / timeouts (scenario factions, de west / su east): hard vs
normal 20 / 20 / 6 of 10, hard vs passive 40 / 0 / 6 of 10, normal vs
easy 50 / 10 / 4 of 10; 2v2 hard vs normal 50 / 0 / 3 of 6. Mirror
matches over 20 games (who wins by side): Germans on both sides west 5,
east 8, 7 timeouts; Soviets on both sides west 14, east 1, 5 timeouts.
Base mode (30 000 ticks): hard vs passive 4 of 4 (about 13 minutes each), hard vs normal 0 / 0 / 4 (as on `skirmish` and `city_large`). The armies
take the coast road and the streets of the town and meet around the
harbor and in the east town; the traces show no column waiting in front
of a slope.

How unreachable goals are handled today: a move order's goal in another
part of the path grid (on a slope too steep, on a plateau no road leads
to) is moved by the simulation to the nearest cell of the start's part
(`PathGrid`, components), so the units drive there and stop; the
commander's staging points only avoid water (`intel::Ground`). A ground
model from the path grid (grade and objects, largest connected part) was
tried: it did not change the stalemates on this map and moved results
on `skirmish`, so it was dropped [provisional]. What did hang on an
early version of the map was a tank at a house corner next to a street
(the house box closed the street's edge cell); buildings now keep a path
cell (4 m) off every road. Every timeout ends with tanks out of
ammunition or out of sight of each other (buildings, terrain) and rifle
squads that cannot hurt tanks; the infantry needs more than the 15
minutes to walk the 3 km between the starts.

### The brick quarter (prompt 112)

`brick_quarter` (3.2 km × 2.2 km from real geodata, closed street walls,
starts about 2.8 km apart at the north and south ends) needed no AI
change. Battle mode, `--max-ticks 18000` (15 min), 10 games each, sides
alternating, win rate A / B / timeouts: hard vs normal 10 / 50 / 4, hard
vs passive 50 / 10 / 4, normal vs easy 20 / 70 / 1. Base mode
(30 000 ticks): hard vs passive 4 of 4 (20–22 minutes each).
`just stuck-audit --map brick_quarter`: 55 group orders, 176 units, none
stuck.

The battle results are upside down: the side that attacks into the
streets loses. Easy and normal (and passive) hold their start while the
more active AI drives its small battle army through 2 km of street
walls, where it meets the defender one street at a time (games end
after 4–6 minutes with 2–9 units left). As on `town_small` and
`river_land` the attacker loses the slugfest at the enemy start; here it
happens against every lower level. Units do not hang in the narrow
streets (the stuck audit shows none); the timeouts were not traced one by
one.
Open: the AI should not attack into a dense town without infantry ahead
and enough superiority (prompt 198's escort may help); not changed
within prompt 112 [provisional].

### The coastal heights (prompt 113)

`coastal_heights` (6.0 km × 3.4 km from real geodata, grass ridges up to
334 m, the starts 5.3 km apart at the forts in the west and east; the
routes between them go over the ridge roads since the tunnels are
missing) needed no AI change. Battle mode, `--max-ticks 18000` (15 min),
10 games each, sides alternating, win rate A / B / timeouts: hard vs
normal 10 / 0 / 9, hard vs passive 0 / 0 / 10, normal vs easy 50 / 40 / 1.
Two hard vs passive games with 36 000 ticks (30 min) also timed out. Base
mode (30 000 ticks): hard vs passive 0 of 4, all timeouts with the
passive side down to 3 units. `just stuck-audit --map coastal_heights`:
50 group orders, 160 units, none stuck (no column hangs at the hairpins
of the ridge roads).

What the AI does on the heights (traces of the long games):

- It reaches the heights and crosses them: the armies get to the far
  side over the ridges (there is no other way); reconnaissance cars
  spread over ridges and valleys.
- The hard attacker stops short or burns out: in one game its eight
  tanks halted on the east ridge about 900 m before the passive start
  and stayed there; in the other its tanks drove into the defended start
  one group at a time and were lost, the crews left on foot. Passive
  defenders behind the fort wait; the games run into the timeout.
- In base mode the hard AI wins the field but does not finish: a few
  enemy units (squads at low strength, a tank crew on the beach below the
  west fort) stay alive next to its armored cars.
- Guns and mortars are not set up on the heights: mortars and rocket
  launchers stay at or near the own base (no towed gun showed up in the
  traces); the AI has no notion of gun positions or of observers
  (`height_*` markers are for the tools and tests only).

Open (not changed within prompt 113) [provisional]: the attacker should
push on from a held ridge or pull back instead of halting in sight of the
enemy; finishing off stragglers in awkward places (beach, cliff foot);
artillery positions on the heights with observers in front (prompt 113
section 8: observers and forward artillery spotting as a rule, if guns do
little on this map); longer `--max-ticks` for maps of this size.

### The dune coast (prompt 114)

`dune_coast` (4.6 km × 3.2 km from real geodata, dunes of 10–50 m along
the cliff and in the north, heath and the village in the middle, mud
flats along the east shore; starts 4.2 km apart at the south and north
ends) needed no AI change. Battle mode, `--max-ticks 18000` (15 min), 10
games each, sides alternating, win rate A / B / timeouts: hard vs normal
40 / 40 / 2, hard vs passive 0 / 30 / 7, normal vs easy 80 / 20 / 0. Base
mode (30 000 ticks): hard vs passive 0 of 4, all timeouts.
`just stuck-audit --map dune_coast`: 60 group orders, 192 units, all
arrived, none stuck (in the dunes, on the sand of the beach or in the mud
of the flats).

What the AI does here (one traced game hard vs passive):

- No sand traps and no endless detours: the attacking armor takes the
  main road along the island through the village (half way after about
  5 minutes, at the enemy start after about 7.5 minutes); the dunes are
  open enough that paths cross them where the road bends, and nothing
  waited in front of a steep lee side.
- The attacker loses the tank fight at the defended start, as on
  `brick_quarter` and `town_small`: its eight tanks arrive one after the
  other in sight of eight waiting tanks, the crews stay on foot near the
  start, the game runs into the timeout. That is why passive "wins"
  three of ten games and hard none.
- The short sight (dunes, village) does not change the AI's behaviour:
  it has no notion of ambush positions in dune valleys; infantry walks
  the 4 km too slowly to matter within 15 minutes.

Open (not changed within prompt 114) [provisional]: the attacker should
not drive into a defended start without superiority (the same point as
on `brick_quarter`); ambushes from dune valleys and anti-tank guns behind
crests as an AI tactic; longer `--max-ticks` for maps of this size.

### The archipelago (prompt 122)

`archipelago` (3.6 km × 2.4 km sketch map: eight islands around a ring
island with a lagoon, six long stone bridges and a short one, fords
between the islands of a side; team 0 on the west and south-west island,
team 1 on the north-east and south-east island) needed no AI change.
Battle mode, `--max-ticks 18000` (15 min), sides alternating, `--jobs 4`,
win rate A / B / timeouts. 1v1 (start 0 west island against start 1
north-east island, 2.8 km apart): hard vs normal 50 / 20 / 3, hard vs
passive 10 / 30 / 6, normal vs easy 60 / 30 / 1, normal vs normal
50 / 20 / 3 (10 games each). 2v2 (6 games): hard vs normal 33 / 33 / 2,
normal vs normal 50 / 33 / 1. `just stuck-audit --map archipelago`: 125
group orders, 400 units, 395 arrived; two trucks with towed guns gave up
at the east end of the north-west bridge (slope beside the ramp, one with
407 re-plans) [open, like the deck end of `river_crossing`].

What the AI does here: the armies meet on the ring island and at the
bridgeheads; the columns cross the long bridges (no unit stuck on a
deck in the tournament). Many games run out of time: the fights at the
bridges wear both armies down. Against the passive AI the attacker loses
more often than it wins: it has to cross two bridges and the ring into a
start that holds its whole army, and the defender fires first
[provisional: small series; a decision whether attacking AIs should wait
for more strength on bridge maps is left open].

### The omega bend (prompt 130)

`omega_bend` (2.0 km × 2.0 km sketch map, mirror-symmetric west against
east, a river with an omega loop and a peninsula in the north, four long
stone bridges, wood strips and a pond in the meadow of the south; team 0
west, team 1 east; 1v1 start 0 west against start 1 east) needed no AI
change; measured on 24.11.2026. Battle mode, `--max-ticks 18000` (15 min),
sides alternating, `--jobs 4`, `--verbose` (winning side). Equal AIs and
equal factions on both sides, wins west / east / timeouts:

| Pairing | de | su | us | uk | west share of decided games |
|---|---|---|---|---|---|
| normal vs normal, 40 games each (seed 101) | 7 / 13 / 20 | 18 / 22 / 0 | 21 / 17 / 2 | 24 / 16 / 0 | 51 % (138 decided) |
| hard vs hard, 2 × 20 each (seeds 201, 401) | 23 / 14 / 3 | 27 / 11 / 2 | 20 / 20 / 0 | 23 / 17 / 0 | 60 % (155 decided) |

Over all 293 decided games 56 % west (the 1v1 starts: west is player 0).
With normal AIs the sides are even (de leans east, 7 of 20, but half the
de games time out); with hard AIs su (71 %) and de (62 %) lean west, us is
even [open: as on `kessel_lake` and `pit_hill` hard AIs win more often
from the west start; not a property of this map's layout, which is a
mirror image].

**Wood strips, neck, bridges.** In battle mode the armies meet in the
meadow of the south: of the positions of four traced games (normal su and
uk, hard de and su, `--trace 300`) 10–20 % lie in the wood strips (the
armies pass through them and fight at their edges) and up to 14 % round
the pond; none at the neck, on the peninsula, at a bridge or in the north
quarters. In base mode (normal vs normal, `--trace 600`, 36 000 ticks)
units go everywhere: wood strips 3 %, neck 2 %, south bridges 1.6 %,
peninsula, loop bridges and north quarters below 1 % of the positions
(the AI takes the supply points there); the game timed out with 86 units
a side. [open: in battle mode the AI does not fight for the neck and the
bridges; no AI changes in prompt 130]
`just stuck-audit --map omega_bend`: 115 group orders, 368 units, 365
arrived, two Panzer IV gave up beside the north ramp of the west south
bridge (embankment slope, as on `archipelago`).

### The pit hill (prompt 129)

`pit_hill` (2.0 km × 2.0 km sketch map, point-symmetric, a terraced hill in
the middle reached by vehicles only over two winding ramps, side ridges,
mountain ranges north and south; team 0 west, team 1 east; 1v1 start 0
west against start 1 east) needed no AI change; measured on 24.11.2026.
Battle mode, `--max-ticks 18000` (15 min), sides alternating, `--jobs 4`,
`--verbose` (winning side). Equal AIs and equal factions on both sides,
wins west / east / timeouts:

| Pairing | de | su | us | uk | west share of decided games |
|---|---|---|---|---|---|
| normal vs normal, 40 games each (seed 101) | 0 / 0 / 40 | 20 / 19 / 1 | 20 / 19 / 1 | 15 / 18 / 7 | 50 % (111 decided) |
| hard vs hard, 2 × 20 each (seeds 201, 401) | 0 / 6 / 34 | 23 / 10 / 7 | 16 / 8 / 16 | 19 / 19 / 2 | 57 % (101 decided) |

Over all 212 decided games 53 % west. su, us and uk are even with normal
AIs; with hard AIs su (70 %) and us (67 %) lean west, uk is even, de east
(6 of 6). **de against de rarely decides**: the two tank armies take
images of the same route round the hill on the ring track (west army
north, east army south, traced), so they circle in the same sense and
pass each other on the far side of the hill; each reaches the enemy start
after about 9 000 ticks and finds little there. Faster or mixed armies
(su, us, uk) meet. [open: a symmetric map with identical deterministic AIs
lets the armies chase each other; not changed (no AI changes in prompt
129)]

**Ramps, top, rock bands.** In battle mode the AI does not climb the hill:
six traced games (normal su, us, uk; hard uk, su, de; `--trace 300`) had
no unit on the top or the upland ring at any traced tick; the armies stay
on the ring track and in the valley. In base mode (two traced games,
normal vs normal, `--trace 600`) engineers and infantry go up onto the top
over the open slopes (12 of about 2 000 positions on the top); no vehicle
used a ramp. Nobody stood stuck at a rock band or boulder row;
`just stuck-audit --map pit_hill` sends 65 group orders (208 units) to
starts, supply points and bridges: all arrived (one of them is the top,
reached only over the ramps). [open: the AI neither takes the contested top in
battle mode nor uses heights for observers; as on `kessel_lake` and
`mesa_desert`]

### The Kessel lake (prompt 128)

`kessel_lake` (2.0 km × 2.0 km sketch map, point-symmetric, a rock ring
with two passages in the middle, lakes with fords north-west and
south-east; team 0 west and south, team 1 east and north; 1v1 start 0
west against start 1 east) needed no AI change; measured on 24.11.2026.
Battle mode, `--max-ticks 18000` (15 min), sides alternating, `--jobs 4`,
`--verbose` (winning side). Equal AIs and equal factions on both sides,
wins west / east / timeouts:

| Pairing | de | su | us | uk | west share of decided games |
|---|---|---|---|---|---|
| normal vs normal, 40 games each (seed 101; uk also 501, 701) | 19 / 19 / 2 | 18 / 22 / 0 | 25 / 15 / 0 | 26 / 14 / 0, 16 / 24 / 0, 22 / 18 / 0 | 53 % (238 decided) |
| hard vs hard, 2 × 20 each (seeds 201, 401; us and uk also 601) | 13 / 16 / 11 | 21 / 19 / 0 | 24 / 11 / 5, 11 / 6 / 3 | 32 / 8 / 0, 13 / 7 / 0 | 63 % (181 decided) |

Over all 419 decided games 57 % west (the 1v1 starts: west is player 0). de and su are even; us and uk lean
west, with normal AIs only on some seeds (uk 65 % on seed 101, 40 % on
501, 55 % on 701), with hard AIs on every seed (us and uk together 71 %
west over 112 decided games). The map is point-symmetric (heights,
terrain, objects, path cells and path costs checked by
`kessel_lake_is_point_symmetric`); two things are not: the AI plans
around the map center (0, 0), not the symmetry point (1, 1) (traced: the
staging points of the two sides differ by about 1.3 m from each other's
image), and the 8 m vision cells start at the map edge, so sight is
symmetric only to within a cell (as on `river_front`). Whether the lean
of hard us / uk comes from these or from the deterministic growth of a
small difference is open [open: a series of the same pairing on a mirrored
copy of the map, and AI planning about the symmetry point].

**Kessel, fords, passages.** In battle mode the AI does not fight for the
Kessel: ten traced games (hard uk and de, normal us and su over four
seeds, `--trace 600`) sent every army from the start along the ring track
around the ring to the enemy start; no unit stood inside the ring or on a
ford at any traced tick, one once in a passage (about half of all unit
positions of the normal games lay on or beside the ring track). In base mode (two traced games, normal
vs normal) units enter the Kessel late (from tick 9 600 and 12 000),
through the passages and partly over the climbs, without a jam; the
fords stay unused there too. `just stuck-audit --map kessel_lake` sends
45 group orders (144 units) to every start and supply point: all arrived,
none stuck in a passage. [open: the AI does not take the contested
centre in battle mode and never chooses the fords (they are slower than
the lake way); not part of prompt 128]

### The mesa desert (prompt 126)

`mesa_desert` (2.4 km × 2.4 km sketch map, point-symmetric, mesas with
cliffs and ramps rising from open basins; team 0 in the north-west and
north-east corner, team 1 in the south-east and south-west; 1v1 start 0
north-west against start 1 south-east) needed no AI change; measured on
24.11.2026. Battle mode, `--max-ticks 18000` (15 min), sides alternating,
`--jobs 4`, `--verbose` (winning side). Equal AIs and equal factions on
both sides, wins north / south / timeouts:

| Pairing | de | su | us | uk | north share of decided games |
|---|---|---|---|---|---|
| normal vs normal, 40 games each (seed 101) | 7 / 19 / 14 | 19 / 17 / 4 | 23 / 7 / 10 | 13 / 10 / 17 | 54 % (115 decided) |
| hard vs hard, 2 × 20 each (seeds 201, 401) | 11 / 22 / 7 | 18 / 18 / 4 | 21 / 12 / 7 | 7 / 22 / 11 | 44 % (131 decided) |

Over all 246 decided games 48 % north: the map is even, single factions
lean one way or the other between seeds (de 7 / 19 with normal, us 23 /
7), as on `river_fork_front`: the games are deterministic and a slightly
different first contact grows. Other pairings (default armies, de north
and su south, which favour the north as on the other maps): hard vs
normal 40 / 60 / 0 (win rate A / B / timeouts; 16 of 20 won by the
north), normal vs normal 16 / 3 / 1 (north / south / timeouts), normal vs
easy 65 / 35 / 0, hard vs passive 50 / 0 / 5 (five of ten run out of
time); 2v2 hard vs normal 20 / 50 / 3 (all seven decided games won by the
north), 2v2 normal vs normal de 50 / 40 / 1.

**Ramps, heights, cliffs.** The path search finds the ramps: the AI takes
the supply points on the plateaus (the big butte, the hollows) up their
ramps, and `just stuck-audit --map mesa_desert` sends 55 group orders (176
units) to every start and supply point, all arrived, none stuck at the
foot of a cliff. The AI does not occupy heights on purpose: two traced
games (hard vs hard, `--trace 1200`) kept every unit in the basins and
lanes between the supply points, and of 1 657 survivors at the end of
timeout games 59 stood on a plateau (nearly all on the big butte and its
ramp: supply points). Many timeouts end with tank crews and stragglers
spread over the basins. [open: using heights for guns and observers,
attacking up a ramp; a prompt of its own (not part of prompt 126)]


### The battery heights (prompt 131)

`battery_heights` (4 km × 4 km sketch map, point-symmetric, west against
east, a river with three bridges between two pairs of wooded heights with
prepared gun positions) needed no AI change; measured on 25.11.2026,
`--jobs 4`, sides alternating, wins A / B / timeouts:

| Pairing | Games | Result | Side |
|---|---|---|---|
| battle, hard vs normal, default armies (`--max-ticks 18000`) | 10 | 5 / 5 / 0 | side 0 (west, de) wins all ten in 6–13 min |
| battle, normal vs normal, us against us | 10 | 5 / 3 / 2 | west 4, east 4 |
| battle, normal vs normal, de against de | 10 | 0 / 1 / 9 | nine timeouts with 4–10 units a side left |
| battle 2v2, normal vs normal | 6 | 2 / 4 / 0 | west 5, east 1 |
| base, normal vs normal (`--max-ticks 36000`, 30 min) | 6 | 0 / 0 / 6 | all timeouts, 85–88 units a side left |

So the default armies (de west, su east) decide the battle by faction,
not by the map; with equal factions the sides are even (us) or no side
gets across (de). In base games both AIs build up to about 87 units and
never attack within 30 minutes. They build some artillery (a game traced
at the end: the west 6 medium mortars, a Panzerwerfer, a Sturmpanzer, 3
PaK 40; the east 8 medium mortars, a Katyusha, two ZiS-3) but no field
howitzers, and they do not move guns to the prepared positions on the
heights (the AI knows no gun positions) [open: AI for artillery positions,
observers and river crossings is not part of prompt 131].

### The river fork front (prompt 125)

(Measured on the first layout of the map; the second layout of 24.11.2026,
with the west front along the main river, has not been measured.)
`river_fork_front` (3.6 km × 2.0 km sketch map, **deliberately not
symmetric**: team 0 north-east on wavy heights, team 1 south-west on the
plain, two trench systems across a fork of three rivers) needed no AI
change; measured on 24.11.2026. Battle mode, `--max-ticks 18000`
(15 min), sides alternating every game, `--jobs 4`. Since prompt 125 the
`--verbose` lines of `ai-tournament` name the winning side (`side 0` /
`side 1`), so equal AIs measure the map's side balance.

The default armies (de north, su south) carry a faction bias (as on
`river_front`), so the balance was measured with equal factions on both
sides, equal AIs, wins north / south / timeouts:

| Version | normal vs normal, 4 factions × 40 games (de, su, us, uk each on both sides) | hard vs hard, 4 × 20 | north share of decided games |
|---|---|---|---|
| first draft (crest 26 m, south starts y = 900; 20 games each) | 35 / 18 / 27 | 12 / 7 / 21 | 66 % (normal), default factions 17 / 0 |
| crest 18 m | 63 / 37 / 60 (seed 101) | 35 / 23 / 22 | 62 % |
| crest 18 m, south starts y = 760 | 24 / 46 / 90 | 3 / 23 / 54 | 28 % |
| **final**: crest 18 m, south starts y = 830 | 51 / 54 / 55 (seed 101), 54 / 46 / 60 (seed 901), de and uk 41 / 8 / 31 (seed 501) | 18 / 14 / 48 | **57 %** (normal, 254 decided), 56 % (hard) |

So no side wins clearly over 60 % over all factions, but single factions
lean one way: us north 36 / 11, su south 23 / 47, uk north 51 / 20, de
even 36 / 30 [provisional; the outcome reacts strongly to small changes:
moving the south starts by 70 m turned 61 % north into 72 % south, the
games are deterministic and a slightly different first contact grows].
Other pairings (final map): hard vs normal 9 / 5 / 6 (wins north /
south / timeouts; hard won 50 %, normal 20 %), normal vs easy 14 / 4 / 2
(normal 60 %, easy 30 %), hard vs passive 4 / 0 / 6 (the attacker only
wins from the heights; 6 timeouts), default armies normal vs normal
14 / 3 / 3; 2v2 normal vs normal de 1 / 2 / 7, uk 3 / 5 / 2, hard vs
normal 3 / 0 / 3. Many games run out of time: the armies take routes
through different sectors between the rivers (the north tanks down the
east lane, the south ones up through the west), meet late and the
attacker of a start fails against the defender. [open: AI for attacks
over rivers and against heights, and for the trenches and gun positions,
is a prompt of its own (not part of prompt 125)]
`just stuck-audit --map river_fork_front`: 145 group orders, 464 units,
all arrived.

### The trench line (prompt 124)

> Measured on the first layout of the map (two straight trench lines). The
> second layout (ring, diagonal brook, jagged trenches; docs/MAPS.md) is not
> measured again yet.

`trench_line` (2.4 km × 2.0 km sketch map, point-symmetric, two trench
systems with wire and a cratered no man's land between them, two lanes
for vehicles; team 0 north, team 1 south) needed no AI change; measured on
23.11.2026. Battle mode, `--max-ticks 18000` (15 min), sides
alternating, `--jobs 4`, win rate A / B / timeouts, 1v1 (start 0
north-west against start 1 south-east): hard vs normal 50 / 40 / 1, hard
vs passive 50 / 20 / 3, normal vs easy 60 / 40 / 0, normal vs normal
(20 games) 40 / 55 / 1; 2v2 hard vs normal (6 games) 16 / 50 / 2. The
attacker has a hard time: even against the passive AI three games of ten
run out of time. The armies cross no man's land through the lanes (the
path search takes the firm grass: a traced game sends the tanks of
both sides down the west lane) and the infantry through the wire gaps;
nobody uses the trenches as positions on purpose. Base mode (hard vs
normal, 2 games of 30 minutes) ends in a **standstill** (both timeouts),
as on `river_front`. [open: AI for trench fighting, breakthroughs and
the gun positions is a prompt of its own (not part of prompt 124)]
`just stuck-audit --map trench_line`: 95 group orders, 304 units, all
arrived.

### The river front (prompt 123)

`river_front` (3.6 km × 2.0 km sketch map, mirror-symmetric north against
south over a broad river with three islands and ten long stone bridges,
four prepared gun positions per side; team 0 north, team 1 south) needed
no AI change; measured on 23.11.2026. Battle mode, `--max-ticks 18000`
(15 min), sides alternating, `--jobs 4`, win rate A / B / timeouts. 1v1
(start 0 north-west against start 1 south-west, 1.6 km apart): hard vs
normal 50 / 50 / 0, hard vs passive 70 / 30 / 0, normal vs easy
60 / 40 / 0, normal vs normal 50 / 40 / 1 (10 games each). 2v2 (6
games): hard vs normal 50 / 50 / 0, normal vs normal 66 / 33 / 0. The
battle armies (eight tanks and two rifle squads, no artillery) cross
over the west island (both its bridges) and fight at the bridgeheads;
games end after 4–8 minutes. `just stuck-audit --map river_front`: 110
group orders, 352 units, 348 arrived, 3 still on the way, one rifle
squad gave up at the south end of the west end bridge (another unit in
the way).

Base mode (hard vs normal, 4 games of 30 minutes, `--max-ticks 36000`)
ends in a **standstill**: every game ran out of time with 74–87 units a
side, and each side kept 94 % of its units on its own bank (north 296
of 315 counted units, south 301 of 319). The AI builds mortars and
rocket launchers (no howitzers) and keeps them with the army on its own
bank (26 and 37 indirect units, none on the islands); it does not use
the prepared gun positions (the artillery follows the army at 80 m,
`crates/ai/src/artillery.rs`) and fires no counter-battery. [open: AI
for gun positions, counter-battery fire and river crossings is a prompt
of its own]

**Side bias of equal AIs.** Although map, open path cells and path
costs are mirrored exactly (test), equal AIs do not win equally often on
both sides, and the side depends on the factions: normal vs normal, 20
games each, wins north / south: default armies (de north, su south)
2 / 16, `--factions de,su` 18 / 2, `su,de` 11 / 9, `de,de` 10 / 9,
`su,su` 8 / 10; `us,us` (40 games) 31 / 9, while `us,us` on
`test_small`, `city_large` and `river_fields` gives 15 / 25, 17 / 22
and 16 / 24 for player 0 / 1. The map is not the cause; small
differences (the vision grid lies 2 m off the mirror axis, the path
search may pick another of two nearly equal routes on the mirrored side,
units act in id order) grow in the deterministic games. [open: worth a
look at the AI's choice of crossing per side]

### The castle rock (prompt 127)

`castle_rock` (3.6 km × 3.4 km, a city on volcanic rocks: the castle on
its rock in the west, the old town on the ridge, the valley with the
railway, the new town, two hills in the east, a small river in the north;
starts **north against south-east**: team 0 in the parks beyond the river,
team 1 in the park at the foot of the big hill, 3.2–3.4 km apart) needed
no AI change. Battle mode, `--max-ticks 18000` (15 min), sides
alternating, `--jobs 4`, `--verbose` (win rate A / B / timeouts). 1v1:
hard vs normal 60 / 30 / 1, normal vs normal 50 / 30 / 2, hard vs passive
10 / 0 / 9, normal vs easy 50 / 50 / 0 (10 games each). 2v2 normal vs
normal 33 / 50 / 1 (6 games). Base mode (30 000 ticks): hard vs passive
0 of 4 won, 4 timeouts (the attacker does not finish the defender in 25
minutes). `just stuck-audit --map=castle_rock`: 130 group orders, 416
units, 415 arrived, one rifle squad gave up at the deck of a stone bridge
over the river.

What the AI does here: **the side decides, not the difficulty.** Of the
36 decided games the north (player 0) won 31: hard/normal 6 of 9,
normal/normal 8 of 8, normal/easy 10 of 10 (the easy AI wins every game
it plays from the north), 2v2 5 of 5. It is not the factions: with the
factions swapped (`--factions su,de`, normal vs normal) the north still
won 7 of 9. The path lengths to the supply points are even (0.5 km over
eight points), so the advantage lies in where the armies meet: in the
parks and the street grid between the river and the new town, nearer to
the north start, where the south-east arrives in columns through the
narrow streets of the old town, round the small hill and over the steel
bridge. No column was seen hanging in the valley or in the closes (the
stuck audit above; the closes are foot ways, not streets, and the valley
floor is open ballast and park, cut only under the steel bridge). The
castle and the summit play no part in battle mode. Against the passive AI
the hard attacker halts before the south-east start in nine of ten games
(the known pattern of the coastal heights). [provisional; open: move the
south-east start west or nearer to the old town and measure again]

### The landing gulf (prompt 121)

`landing_gulf` (5.0 km × 4.4 km, a gulf in the middle, a village at its
head, a plain behind it, hills on both sides; starts **hinterland against
coast**: team 0 on the plain in the north, team 1 west of the village and
on the coast plain) needed no AI change. Battle mode, `--max-ticks 18000`
(15 min), sides alternating, `--jobs 4`, win rate A / B / timeouts. 1v1
(start 0 on the plain against start 1 west of the village, 2.0 km
apart): hard vs normal 50 / 50 / 0, hard vs passive 80 / 10 / 1, normal
vs easy 80 / 20 / 0, normal vs normal 70 / 30 / 0 (10 games each; equal
AIs on alternating sides, so the side does not decide). 2v2 (6 games):
hard vs normal 50 / 50 / 0, normal vs normal 50 / 50 / 0. Base mode
(30 000 ticks): hard vs passive 3 of 4 won, 1 timeout. `just stuck-audit
--map landing_gulf`: 50 group orders, 160 units, all arrived (0 stuck).

What the AI does here: decided games end after 5–8 minutes on average
(mean 282–489 s); the beach plays no special part (there are no boats in
free play). Hard against normal is even here (10 and 6 games), where it
wins on most maps [provisional: small series].

### The rhine bridge (prompt 118)

`rhine_bridge` (3.0 km × 3.2 km, a town north of a river 140–160 m wide
with one steel bridge, the polder in the south; starts on both banks as
on `river_crossing`: team 0 west, team 1 east) needed no AI change.
Battle mode, `--max-ticks 18000` (15 min), sides alternating, `--jobs 4`,
win rate A / B / timeouts. 1v1 (both starts in the town north of the
river, 2.3 km apart): hard vs normal 60 / 30 / 1, hard vs passive
70 / 10 / 2, normal vs easy 50 / 40 / 1, normal vs normal 40 / 60 / 0
(10 games each). 2v2 (each team one start on each bank, 6 games): hard
vs normal 33 / 50 / 1, normal vs normal 50 / 33 / 1. Base mode (30 000
ticks): hard vs passive 4 of 4 won. `just stuck-audit --map
rhine_bridge`: 110 group orders, 352 units, all arrived (0 stuck).

What the AI does here: the 1v1 games end after 5–7 minutes of fighting
in the town; in 2v2 games the AIs use the bridge in both directions.
Hard vs normal in 2v2 is lost more often than won (6 games; the polder
starts are far from the town fight) [provisional]. Open: the computer
player does not hold or blow the bridge on purpose (as on
`river_crossing`).

### The river crossing (prompt 116)

`river_crossing` (3.6 km × 3.2 km, a town on a terrace, a river 250–400 m
wide with one long steel bridge, polder and meadows; starts on both
banks: team 0 west, team 1 east) needed no AI change. Battle mode,
`--max-ticks 18000` (15 min), sides alternating, win rate A / B /
timeouts. 1v1 (both starts on the south bank, 2.4 km apart): hard vs
normal 10 / 50 / 4, hard vs passive 70 / 30 / 0, normal vs easy
60 / 40 / 0, normal vs normal 30 / 30 / 4 (10 games each). 2v2 (each
team one start on each bank, 6 games): hard vs normal 16 / 33 / 3,
normal vs normal 33 / 0 / 4. Base mode (30 000 ticks): hard vs passive
4 of 4 won (the first geodata map where the attacker wins every base
game). `just stuck-audit --map river_crossing`: 100 group orders, 320
units, 317 arrived; one Tiger of a heavy group turned back beside the
south end of the bridge deck (it tried to get onto the deck from the
ramp road beside it, 204 re-plans) [provisional, open].

What the AI does here: games end early (mean 5–9 minutes in 1v1): the
two sides meet in the town and on the meadows south of the river; with
four players the AIs drive over the bridge in both directions (tank
crews of both teams at the bridgeheads after the games), so holding it
does not lock a team out (each team has a start on each bank).
Equal AIs (normal vs normal): of 10 games the west won 3, the east 1,
6 timeouts [provisional: the west's supply points are a little closer].

Open (not changed within prompt 116): the computer player does not
demolish or defend bridges on purpose, has no boats and sends no
paratroopers over the river; the hard AI loses more 1v1 games against
normal here than on other maps (the town fight, not the bridge).

### Balance (prompt 045)

Prompt 045 changed rule values the AI plays with (income, start account,
resupply cost, unit prices by range, a truck as start vehicle, more Soviet
and US tanks in battle mode when factions are chosen); the tables above
were measured before. Faction balance with equal AIs, the statistics per
unit type and the new tools (`just balance`, `just duel`) are in
[BALANCE.md](BALANCE.md). The AI weaknesses the statistics showed are
listed there under "Open" (useless Soviet AT squads built against armor,
lost engineers rebuilt again and again).

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
- `crates/ai/tests/commander.rs` (prompt 051): Hard surrenders when its
  rifles see only tanks, the old Hard does not; Hard attacks a passive
  tank platoon from the flank, not along the direct line.
- `crates/ai/src/flank.rs` (prompt 110): flanked guns and tanks; the
  measurement `crates/ai/tests/flank_scenario.rs` (ignored, prints a
  table).
- `crates/ai/tests/economy.rs` (prompt 023): every level builds barracks
  and depot and produces units against the passive enemy; base matches
  between two AIs are reproducible and both sides build.
- `crates/ai/src/support.rs` and `crates/ai/tests/support.rs` (prompt
  195): support vehicles hold behind their escorts, back off from the
  front, drive back without escorts, spare lone squads, fire at squads in
  houses and back off to reload; the village attack prints a table.

## Great Britain (prompt 117)

The computer plays `uk` with the same code as the other factions: its
production lists in `data/economy.toml` keep the slots the mixes index
(0 armoured car, 1 carrier, 2 medium tank, 3 tank destroyer, 4 heavy tank,
5 self-propelled gun, 6 truck, 7 AT gun, 8 howitzer). `uk` has no rocket
launcher, so the mixes build the heavy tank (Churchill VII) in its place.
British transports drop the airborne section (`paratroopers`,
`data/air.toml`).

`just balance --pairs uk-de,uk-su,uk-us` (hard vs hard, 20 games per row,
X / Y / undecided in percent), the final settings (Firefly in slot 2,
battle groups `uk = 112 %`):

| Mode | Map | uk – de | uk – su | uk – us |
|---|---|---|---|---|
| battle | test_small | 55 / 45 / 0 | 50 / 50 / 0 | 0 / 100 / 0 |
| battle | skirmish | 50 / 30 / 20 | 45 / 55 / 0 | 10 / 90 / 0 |
| base | test_small | 70 / 20 / 10 | 45 / 55 / 0 | 50 / 35 / 15 |
| base | skirmish | undecided (as every pairing, docs/BALANCE.md) | | |

Adjusted **[provisional]**: with the Cromwell in slot 2 the British won
only 13 % of the base matches (10 / 2 / 4 of 20 against de / su / us; the
Cromwell took 1.4 enemies per loss, the M4A3 4.6): the Sherman Firefly
(17-pounder) is in slot 2 now, the Cromwell further back. Battle groups of
118 % and 125 % were measured and rejected (uk–de 75 %, uk–su 95 %).
**Known imbalance:** in battle mode the Cromwell loses to the M4A3 (its
64 mm front is vertical, the Sherman's sloped); the catalog values stay.
Mirror tournament `--factions uk,uk`: hard vs passive 82 % (3 timeouts),
hard vs normal 35 / 65 (us,us for comparison: 57 % and 47 / 52).

## Open

- Commander (prompt 051): 90 % against the passive enemy on every map and
  a clear win against the old Hard are not reached (see "Results of the
  commander"); base matches between two AIs on 1–2 km maps stay
  undecided (economy balance, prompt 045). Protecting own trucks beyond
  keeping them behind the army, observers for the artillery, terrain
  (crests, woods, houses) in the choice of axes, and coordinating allies
  beyond shared sightings are open.

- Infantry tactics (prompt 178 did stances and houses for defenders):
  moving from cover to cover, buildings and wood edges as intermediate
  goals, fire and movement of two squads, positions of tanks and guns by
  the enemy's approach. Decide after more tournament numbers whether that
  is worth a prompt; base matches between two AIs are now even more often
  undecided. The end game of base matches (see above),
  using terrain (hull-down positions, woods, crests). Artillery (prompt
  032) is Hard only and simple: no observers sent forward, no
  counter-battery, smoke only when the enemy first appears.
- Air support (prompt 033) is simple: no recon before attacks, no flak
  placed by threat direction, no heavy flak, no call on structures.
- Meeting engagements on large maps for a better spread of the levels.
- Formation facing and speed matching (pathfinding, see ARCHITECTURE.md).
