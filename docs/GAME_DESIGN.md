# Game design: base building and resources

Prompt 023. This document fixes the game rules for building a base, gathering
resources and producing units. The numbers live in `data/economy.toml`
(compiled into the simulation, part of the state hash) and can be changed
there without touching code; how the simulation implements them is in
[ARCHITECTURE.md](ARCHITECTURE.md#economy-base-building-and-production).

> **Status: provisional (vorläufig, vom Nutzer zu bestätigen).** The user asked
> for these decisions to be made without a round of questions. Every section
> below marked **[provisional]** is a design decision that still needs the
> user's confirmation; the reasons are given so that each one can be reversed
> on its own.

## Guiding idea

A WW2 real-time strategy game in the spirit of the classics of the genre:
the tactical weight of real units (armor faces, ranges, sight) combined with
a compact base, a few resources and supply points worth fighting over. A solo
developer has to be able to build, balance and maintain it, so:

- **Few moving parts.** Three resources, four base buildings, two kinds of
  fortification, no harvester units, no tech tree beyond one prerequisite.
- **Everything is a unit.** Buildings, construction sites and fortifications
  are stationary units of the simulation. Combat, vision, destruction,
  victory conditions and the computer opponent work with them unchanged.
- **Numbers from data.** Unit costs follow from catalog values by a formula
  (mass, crew, weapons), so all 133 catalog entries have a price without a
  hand-made table; single units can be overridden.

## Resources [provisional]

| Resource | Used for | Comes from |
|---|---|---|
| **Fuel** (Treibstoff) | vehicles (by mass), vehicle works | HQ, supply depots, supply points |
| **Ammunition** (Munition) | every armed unit (by its weapons), fortifications | HQ, supply depots, supply points |
| **Manpower** (Personal) | soldiers and crews, every building | HQ, supply depots, supply points |

- Every player has an account of the three resources; it starts with
  **600 fuel, 600 ammunition, 400 manpower**. There is no storage limit.
- Income is paid continuously (every second of game time, in thousandths, so
  nothing is lost to rounding) from:

| Source | Fuel / min | Ammunition / min | Manpower / min |
|---|---|---|---|
| Headquarters (completed) | 40 | 70 | 80 |
| Supply depot (completed, each, at most 3) | 40 | 50 | 30 |
| Supply point (held, each) | 60 | 60 | 30 |

The first AI matches showed manpower and then ammunition as the bottleneck
(the vehicle works was never affordable), so the headquarters and depot
income was raised from the first draft (40/50/60 and 40/40/20).

- **Supply points** are neutral spots on the map (map markers named
  `supply…`, see [MAPS.md](MAPS.md)). A player takes one by having units (not
  buildings) within **25 m** while no enemy unit is there; after **20 s** of
  uncontested presence the point is theirs. Taking an enemy point first
  neutralizes it (20 s), then captures it (20 s). Held points pay until they
  are lost. This is where the supply war happens: the cheapest way to more
  income is the map, not the base.

**Why these three and why no harvesting [provisional]:** fuel, ammunition and
manpower are what an army of the period really ran on, and each one limits a
different part of the army (tanks need fuel, artillery and AT need
ammunition, infantry needs manpower). Harvester units (trucks driving between
a source and a depot) would add path traffic, AI logistics and a unit type
that is never fun to protect; captured supply points give the same "fight
for resources on the map" with one simple rule.

### Consumption [provisional]

Since prompt 036 units use up ammunition and fuel and are resupplied from
the account, see [Supply and repair](#supply-and-repair-provisional). The
price of a unit still contains its first load.

## Buildings [provisional]

Buildings are game objects, not catalog entries: their sizes match the
models they borrow (`docs/ASSETS.md`), their strength is a game rule like the
hit points of map objects in `data/destruction.toml`.

| Building | Size (m) | Hit points | Armor (mm) | Cost F / A / M | Build time | Needs | Does |
|---|---|---|---|---|---|---|---|
| Headquarters (`base_hq`) | 10 × 8 × 9 | 6000 | 40 | 200 / 0 / 300 | 60 s | – | income; build radius 120 m |
| Barracks (`base_barracks`) | 12 × 8 × 7.5 | 3000 | 25 | 0 / 0 / 150 | 25 s | HQ | produces infantry |
| Supply depot (`base_depot`) | 14 × 9 × 8 | 2500 | 25 | 50 / 0 / 100 | 25 s | HQ | income (max. 3) |
| Vehicle works (`base_factory`) | 20 × 10 × 9 | 4000 | 30 | 150 / 50 / 200 | 45 s | HQ, depot | produces vehicles |

Every player starts with a completed headquarters, a halftrack and two
rifle squads (scenario `data/scenarios/skirmish.toml`). None of them can
hurt a building, so the first tanks or AT squads decide when a base falls.
The bases stand at up to 70 % of half the map size from the center, on the
line through the map's start positions (`start_distance_pct`; closer in
where map objects or impassable ground are in the way): the test maps have
their starts only 120 m apart, which is right for the battle but leaves no
room for two bases. On `test_large` the bases end up 0.9 and 1.4 km from the
center; on `test_small` the woods around the start meadow keep them at
about 90 m. Fortifications come from the catalog:

| Fortification | Catalog ids (de / su / us) | Hit points | Armor (mm) | Cost F / A / M | Build time | Needs |
|---|---|---|---|---|---|---|
| MG bunker | `de_bunker_tobruk`, `su_bunker_dzot`, `us_bunker_mg` | 2500 | 60 | 0 / 80 / 60 | 20 s | barracks |
| Light flak (prompt 033) | `de_flak38`, `su_61k`, `us_bofors_m1` | 1000 (rules) | – | formula | formula | barracks |
| Heavy flak (prompt 033) | `de_flak36`, `su_52k`, `us_90mm_m2` | 1000 (rules) | – | formula | formula | vehicle works |

- Flak fires at aircraft (`data/air.toml`) and with its catalog weapon at
  ground targets. Flak positions are fortifications: they cannot move or be
  towed.
- Bunkers fight with their catalog weapons; they cannot move. The AT gun
  position of prompt 023 is gone since prompt 034: AT guns and howitzers are
  produced in the vehicle works and towed by trucks and halftracks
  (ARCHITECTURE.md "Transport and towing"). [provisional]
- Armor of buildings is the same on every face. 25–60 mm keep rifles and
  machine guns out (their penetration is ≈ 14 mm) but not tank and AT guns:
  **buildings need heavy weapons**, which keeps infantry rushes from razing a
  base and gives AT squads and tanks a job.

### Placing and construction [provisional]

- Buildings are ordered by the player, not by a builder unit (one command,
  like the classics of the genre; no engineer pathing, no idle builders for
  the AI to manage).
- A building can be placed within the **build radius** of an own completed
  building: 120 m around the headquarters, 60 m around the others
  (fortifications extend nothing).
- The site must be flat enough (height difference over the footprint at most
  12 % of its larger side), on passable ground (no water, no forest, map
  objects or wrecks), inside the map, and free of units. The preview shows
  this in green or red; the simulation checks the same rule again.
- The full cost is paid when the site is placed. At most **2 construction
  sites** per player at a time.
- A construction site is a unit at once: it starts with 10 % of its hit
  points and gains the rest linearly until it is completed (damage taken on
  the way stays). It can be shot and destroyed; it does not fire, produce or
  pay income until it is completed.
- Destroyed buildings stay as ruins that block their ground (like wrecks);
  their production queue is lost.

## Production [provisional]

- The **barracks** produces the infantry of the player's faction, the
  **vehicle works** its vehicles (lists in `data/economy.toml`):

| Faction | Barracks | Vehicle works |
|---|---|---|
| de | rifle squad, MG 42 team, HMG team, AT squad, 8 cm mortar team, engineer squad | Sd.Kfz. 222, Sd.Kfz. 251/1, Panzer IV H, StuG III G, Tiger I, Wespe, Opel Blitz, 7.5 cm Pak 40, 10.5 cm leFH 18 |
| su | rifle squad, MG team, HMG team, AT squad, 82 mm mortar team, sapper squad | BA-64B, T-70M, T-34/76, SU-85, IS-2, ISU-152, ZiS-5, ZiS-3, M-30 |
| us | rifle squad, MG team, HMG team, AT squad, 81 mm mortar team, engineer squad | M8 Greyhound, M3 halftrack, M4A3 (75), M10, M26, M7 Priest, GMC CCKW, 57 mm M1, 105 mm M2A1 |

- Each producing building has a queue of up to **5** orders. The cost is paid
  when an order is queued; cancelling the last order refunds it in full.
- Only the first order is worked on. When it is done, the unit appears at a
  free spot beside the building and drives to the building's **rally point**
  (set by right-clicking with the building selected; default 20 m in front of
  it).
- A player can have at most **80** living units besides buildings;
  production waits while the limit is reached.

### Unit costs (formula) [provisional]

From the catalog entry of the unit (`data/economy.toml`, `[unit_cost]`):

- **Manpower** = 6 × soldiers (squad size, or crew of vehicles and guns).
- **Fuel** = 4 per tonne of combat mass (vehicles only).
- **Ammunition** = 10 + Σ over the weapons the simulation uses
  (caliber in mm)² × count / 60.
- **Build time** (game seconds) = 8 + (fuel + ammunition + manpower) / 8.

Examples (`cli economy-table` prints the full table):

| Unit | Fuel | Ammunition | Manpower | Time |
|---|---|---|---|---|
| Rifle squad (de) | 0 | 18 | 54 | 17.0 s |
| AT squad (de, Panzerschreck) | 0 | 140 | 12 | 27.0 s |
| AT squad (us, bazooka) | 0 | 71 | 12 | 18.3 s |
| Panzer IV H | 100 | 103 | 30 | 37.1 s |
| T-34/76 (1943) | 123 | 106 | 24 | 39.6 s |
| Tiger I | 228 | 139 | 30 | 57.6 s |
| IS-2 | 184 | 258 | 24 | 66.2 s |
| M4A3 (75) | 126 | 103 | 30 | 40.3 s |
| Pak 40 position | 0 | 103 | 36 | 25.3 s |

The code computes the values (`sim::economy::EconomyRules::unit_cost`);
this table is a snapshot, `cli economy-table` shows the current ones. The
formula only counts the weapons the simulation fires (prompt 017), so the
Soviet AT squad (PTRD rifle, 14.5 mm) is cheap and weak against armor;
single units can get their own price with an override if that matters.

## Field works [provisional]

Engineer squads (barracks, last in the list) build field works for
resources (prompt 035, `data/works.toml`; mechanics in
[ARCHITECTURE.md](ARCHITECTURE.md#field-works)). Prices are per piece and
paid when the site is placed; times are game seconds for one squad (two
squads are twice as fast).

| Work | Piece | Price (fuel / ammo / manpower) | Time |
|---|---|---|---|
| Anti-tank mines | a 20 m × 20 m section (20 mines, catalog) | 0 / 60 / 0 | 40 s |
| Anti-personnel mines | a section (40–50 mines) | 0 / 40 / 0 | 30 s |
| Barbed wire | 4 m of line | 0 / 0 / 6 | 6 s |
| Anti-tank obstacles | one hedgehog every 3 m | 0 / 0 / 10 | 10 s |
| Foxhole | one | free | 15 s |
| Trench | 4 m of line | free | 20 s |
| Demolish a bridge | – | 0 / 40 / 0 (when done) | 30 s |
| Repair a bridge | – | 40 / 0 / 80 (when done) | 90 s |

Positions (prompt 052, `[positions]` of `data/works.toml`):

| Work | Piece | Price (fuel / ammo / manpower) | Time |
|---|---|---|---|
| Communication trench | 4 m of line | free | 15 s |
| Vehicle pit | one | free | 60 s |
| MG nest (at a trench) | one | 0 / 0 / 10 | 25 s |
| Dugout (at a trench) | one | 0 / 0 / 20 | 60 s |
| Camouflage net | one | 0 / 0 / 10 | 10 s |
| Bunker (at a trench) | the faction's fortification | 0 / 80 / 60 | 60 s |
| Concrete teeth | one tooth every 1.5 m | 0 / 0 / 15 | 15 s |

Clearing takes 3 s per mine, 5 s per wire piece, 8 s per hedgehog (30 s per
tooth, 4 s per net). At most
24 unfinished sites per player; sites left alone for 30 s are given up
(refunded if not started). Mines do not go off under the own team; the
enemy sees them only after his engineers found them or one went off.

## Air support [provisional]

Prompt 033, rules in `data/air.toml`, technical details in
[ARCHITECTURE.md](ARCHITECTURE.md#air-support-and-anti-aircraft-fire).
Every player can call aircraft of their faction on a ground point, paid
from the account (also in battles without base building, where the start
account is all there is):

| Mission | Aircraft (de / su / us) | Cost F / A / M | Cooldown | Effect |
|---|---|---|---|---|
| Recon | Fi 156 / Po-2 / L-4 | 40 / 0 / 10 | 60 s | one aircraft reveals 500 m around its track |
| Fighters | Fw 190 A-8 / La-5FN / P-51D | 60 / 60 / 10 | 90 s | two aircraft strafe the point at 30 m |
| Ground attack | Ju 87 D-5 / Il-2M3 / P-47D | 80 / 100 / 10 | 120 s | two aircraft bomb from 150 m and strafe |
| Bombers | He 111 H-16 / Pe-2 / B-25J | 120 / 160 / 20 | 180 s | three aircraft drop sticks from 500 m |

No buildings are needed: air support is the commander's tool in both
modes. Light and heavy flak are the answer (fortifications above), and
tanks of the US and the late Soviet heavies carry AA machine guns.

## Supply and repair [provisional]

Prompt 036, rules in `data/logistics.toml`. The user was not available for
the design question of the prompt ("how much logistics is fun"); it was
decided along the prompt's proposal and **every point of this section is
provisional** until the user confirms it. The reasons per decision are in
`prompts/done/036-nachschub-reparatur.md`.

**Idea: a stock per unit, resupply in a radius, no micromanagement of single
shells.** An offensive has to bring trucks along or come back to the base;
the player never loads trucks or picks ammunition types.

| Stock | Who has it | Full load | Used by | Empty |
|---|---|---|---|---|
| Ammunition | every armed unit but buildings, bunkers and flak positions | salvos of the main weapon: catalog `ammo_carried` / barrels (Panzer IV 87, Tiger 92), else 150 (infantry), 100 (vehicles), 60 (guns); at least 12, at most 2000 | every salvo of the main weapon (shells of mortars and howitzers) | the unit fires no weapon |
| Fuel | vehicles | road range × 1/20 (`data/scale.toml`, `[range] fuel`): Panzer IV 10.5 km, Tiger 9.75 km, Opel Blitz 16 km | every meter driven | the vehicle stands |

- **Immobilized:** a vehicle at or below **35 %** of its hit points cannot
  move (it still fires) until it is repaired above that line. One 8.8 cm
  hit (774 of 1000 points) or three 5.7 cm hits do it.
- **Resupply:** completed headquarters and supply depots (**60 m**) and
  every **truck** (**40 m**) resupply the units of their player and of
  their allies: **5 %** of a full load per second, ammunition and fuel at
  once. The receiving player pays from the account: a full load costs half
  the ammunition (fuel) price of the unit. Without the money nothing is
  refilled. Trucks have no stock of their own.
- **Repair** (free, it costs time): the vehicle works repairs vehicles and
  towed guns within **50 m** by **2 %** of their hit points per second,
  trucks within **25 m** and engineers (category `inf_engineer`) within
  **20 m** by **1 %**. The best repairer in reach counts. Infantry is not
  repaired (replacing soldiers is open).
- **Recovery** of an immobilized tank is repair on the spot until it can
  drive again; tanks are not towed (no recovery vehicles in the catalog).
- **Buildings:** the owner orders a repair of a completed, damaged
  building (`Repair`): **2 %** per second while the account pays; a full
  repair costs half the building's price. **Selling** (`Sell`) a completed
  building refunds **50 %** of its price, cancelling a construction site
  **75 %**; queued production is refunded in full. A sold building is gone
  (its ground is free); for the victory condition it is lost like a
  destroyed one.
- **HUD:** the unit panel shows ammunition and fuel bars and warns about
  low ammunition (≤ 25 %), low fuel (≤ 20 %) and immobilized vehicles;
  selected suppliers and repairers show their radius on the ground; the
  building bar has Repair and Sell (Cancel for a site).
- **Computer players** send units with low stocks or (mobile) vehicles
  below half their hit points back to the nearest own supplier or
  repairer and repair damaged buildings when they can pay
  ([AI.md](AI.md)).

## Wounds, medics and crews [provisional]

Prompt 049, rules in `data/care.toml`, details in
[INFANTRY.md](INFANTRY.md#wounds-and-buddy-aid-prompt-049), reasons in
`prompts/done/049-verwundung-kameradenhilfe.md`. A hit a soldier survives
wounds him: lightly (he bandages himself and limps) or heavily (down, out of
the fight, dies after 90 s game without a medic). Comrades drag the wounded
into cover and carry them along (the squad slows down); medic teams fetch
them, two-man teams on a stretcher, and patch them up, so the man limps back
into the fight. Survivors of destroyed vehicles bail out as small crew squads
with pistols and a submachine gun; they do not keep a player in the match.
Not in this version: prisoners, re-crewing vehicles.

## Experience and ranks [provisional]

Prompt 050, rules in `data/experience.toml`, reasons per decision in
`prompts/done/050-erfahrung-raenge.md`. Every value is provisional until
balancing (prompt 045).

**Idea: men who survive and fight get better, a little.** A veteran crew is
worth protecting, but a fresh Tiger still beats an elite Panzer IV: ranks
never change armor, penetration, range or hit points.

| Points (XP) | For |
|---|---|
| 100 | all the damage it takes to destroy one enemy unit, shared out by damage (a hit that takes a quarter of a squad's hit points: 25) |
| +25 | the hit that destroys an enemy unit (a kill) |
| +2 | surviving a tick with a damaging enemy hit |
| +1 | every game second a squad is pinned down (suppressed) and alive |

| Rank | From XP | Dispersion | Reload time | Pinned down |
|---|---|---|---|---|
| Recruit | 0 | 100 % | 100 % | 100 % |
| Experienced | 100 | 92 % | 96 % | 85 % |
| Veteran | 300 | 84 % | 92 % | 70 % |
| Elite | 700 | 76 % | 88 % | 55 % |

- Experience belongs to the unit: a vehicle with its crew, a gun, a whole
  squad (its survivors keep it). Every unit starts as a recruit.
- Only enemy units count; hits by mines and aircraft have no shooter.
  Blast damage of shells and bombs around the impact counts only through
  the kill (the shell's direct target gets its share).
- A tank that destroys its first enemy tank is experienced (125); about
  three destroyed units make a veteran, six an elite.
- **Visible:** the unit panel shows the rank insignia (German sleeve
  chevrons, US chevrons, Soviet shoulder-board stripes), rank name, kills
  and a bar to the next rank; multiple selection tiles show the insignia.
  Soldiers of experienced squads wear sun-bleached, worn uniforms; German
  and Soviet veterans got hold of camouflage smocks or suits (half of a
  veteran squad, all of an elite one, forest biome). Vehicles and guns
  carry a white ring round the barrel per destroyed enemy vehicle. Nothing
  modern: no optics, lights, body armor.
- **Computer players** pull veterans back earlier: on Normal a veteran or
  elite unit below 40 % of its hit points leaves the fight (recruits fight
  on), on Hard below 50 % (others below 30 %).

## Victory [provisional]

Unchanged from prompt 027: a player who has no living unit left is defeated.
Buildings and construction sites are units, so a player with a base is not
defeated while one building stands. A completed bunker alone keeps a player
in the game, which is acceptable for now.

## Game modes

- **Base building** (new default in the single-player setup): the skirmish
  scenario, every player with a headquarters, a few starting units and the
  start resources.
- **Battle**: the fixed armies of the demo scenario, as before (also used by
  the golden hash, the LAN tests and the self-test matches 1–3). Supply
  points and accounts exist there too but have nothing to buy.
- LAN and online matches: the host chooses the mode (prompt 037).
- **Teams** (prompt 037, [TEAMS.md](TEAMS.md)): 2 against 2 and 3 against
  3 with humans and computer players in both modes; allies share their
  sight (option), a team loses when all its players are defeated.

## Computer opponent [provisional]

The AI (prompt 024) plays the economy with the same commands as a human:

- a fixed build order (barracks, depot, vehicle works, depot, MG bunker),
  placed on the first free spot of a spiral around its headquarters (the
  AT gun position of Hard is gone since prompt 034: Normal produces a towed
  AT gun instead);
- keeps both production queues filled while it can pay, choosing units from
  a per-difficulty mix (Hard prefers tanks);
- keeps the price of the next building of its build order in reserve, so
  production does not starve the base;
- holds its army near its base and the nearest supply points until an attack
  wave is complete (Easy 8, Normal 6, Hard 5 units), then attacks as before;
  enemies in sight are always engaged; buildings are attacked last, by
  armor and by squads with AT weapons.

On `test_small` (500 m) everything is in sight of everything, and two
computer players of similar strength can end in a stalemate after the first
base falls; on larger maps the waves matter more. Tuning the end game is
open.

## Balancing principles [provisional]

1. **Real values decide the fight, the economy decides the mix.** Costs scale
   with catalog mass, crew and weapons, so a Tiger costs about two Panzer IV;
   a stronger unit is never cheaper.
2. **Each resource limits one arm.** Fuel limits armor, ammunition limits
   guns and AT, manpower limits infantry and buildings. A player who holds
   the supply points gets more of all three.
3. **The map pays more than the base.** One supply point is worth 1.5 depots
   and cannot be built, so armies have to leave the base.
4. **No instant power.** Build and production times grow with the price;
   the starting resources buy a barracks and a few squads or one tank, not an
   army.
5. **Static defense is real but finite.** Bunkers and towed guns use real
   weapons, but buildings cannot hurt tanks with small arms and every
   fortification costs ammunition the army also needs.
6. **Change one number at a time** in `data/economy.toml`, run
   `just ai-tournament --mode base --max-ticks 30000` and a few matches, and
   note the reason in the commit.

## Open (not in this version)

Replacing soldiers of squads (and what replacements do to a squad's
experience, prompt 050), veteran units at the start of missions,
experience for artillery spotting and blast damage, captured weapons as
a visible veteran kit (all prompt 050), towing immobilized tanks, recovery
vehicles, supply trucks with a stock of their own (all prompt 036), upgrades, aircraft and artillery production, walls as
buildable objects (mines, wire, obstacles and dug positions are built by
engineers since prompt 035, see "Field works"), supply points in the map
editor, sharing
resources with allies, tuning the AI's end game in base matches (prompt
037 made a first step, see [AI.md](AI.md#teams-prompt-037)).
