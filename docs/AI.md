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
known, else it hunts on its own):

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
to infantry. The other units keep firing at will; return fire and close
range are not used by the AI yet (finding for prompt 178: defenders in
woods could wait with "close range only").

## Target choice (prompt 188)

The AI does not pick its units' targets: the simulation's score
(ARCHITECTURE.md "Combat") does it for every player alike. With it the
tournament shifted little (BALANCE.md "Target choice by score").

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
