# Controls

How to play Mud and Steel with mouse and keyboard. Keys marked *(rebindable)*
can be changed in **Settings → Controls**; every tile of the order grid
shows its current key in its corner and in its tooltip. Technical details: [ARCHITECTURE.md](ARCHITECTURE.md#selection-presentation-only)
and [the HUD section](ARCHITECTURE.md#hud-presentation-only). The tutorial
(prompt 039, main menu → Tutorial, [MISSIONS.md](MISSIONS.md)) builds on
this page; since prompt 180 it has five chapters (basics, sight, infantry,
tanks, support), and its hints name the keys as currently bound.

**Size of the interface:** Settings → Interface → "Interface scale" and
"Font size" change every text and every tile of the HUD (as far as the
window has room: on a window 1280 × 720 interface pixels large the tiles
grow by 15 % at most, so the corners never grow into each other). On a
dense screen (Retina, high DPI) the game starts from the system's scale.
Nothing folds any more (prompt 095): the HUD shows only what there is
something to do with, see "Screen". All keys work whether their tile shows
or not.

## Camera

| Input | Effect |
|---|---|
| W A S D, arrow keys *(rebindable)*, mouse at the screen edge | Pan |
| Mouse wheel, trackpad pinch | Zoom (towards the cursor; towards the screen center while the camera follows a unit) |
| Middle mouse drag, Option/Alt + mouse move, two-finger trackpad pan | Grab the map |
| Q / E *(rebindable)* | Turn by 45° |
| N *(rebindable)* | Face north |
| Page up / Page down *(rebindable)* | Tilt the camera steeper / flatter by 10° (prompt 096) |
| Camera control above the minimap: drag | Sideways turns the camera by any angle, up tilts steeper, down flatter (Settings → Camera: "Invert tilting" swaps up and down) |
| Camera control: click on N, E, S or W | Turn to look that way |
| Camera control: mouse wheel | Tilt in steps (instead of zoom) |
| Camera control: house button, or double click on the dial | Back to north and to the normal tilt |
| Shift + middle mouse drag, Shift + Option/Alt + mouse move | Turn and tilt around the screen center, on the map |
| Double click on empty ground | Glide there |
| Shift + C *(rebindable)*, info field: button with the ring | Glide to the active unit (see "Selecting"); zoom and rotation stay (prompt 094) |
| C *(rebindable)*, info field: toggle with the frame | Follow the active unit: it stays in the screen center while it drives. Zoom and rotation keep the mode; moving the map by hand (keys, screen edge, drag, trackpad, minimap, double click on the ground, Space) ends it. Without a selection it waits for the next one. The game remembers the choice for the next match |
| Minimap: left click or drag | Look there |
| Space *(rebindable)* | Jump to the newest event message |
| Shift + P *(rebindable)*, or the pause tile beside the minimap | Tactical pause (single player, prompt 102): the game stands, camera, selection and orders keep working; orders start with the first tick after the pause. A line at the top says so; the music plays quieter. Esc opens the menu as before; closed again, the pause goes on. Not in network matches |
| ] (right of Ü on German keyboards) *(rebindable)* | Game speed 1×, 2×, ½× (single player) |
| Shift + O *(rebindable)* | Overview (prompt 103): the camera glides out to up to 1200 m at the steepest tilt, shadows and small decoration off, every own unit shows its ring. Again: back to the view before; a zoom in returns centered on the point under the pointer |
| F1 *(rebindable)* | Hide or show the HUD and every mark on the map; orders keep working, a short line names the key |
| F2 *(rebindable)* | Photo mode (single player and replays): the game stops, HUD and marks go, the camera goes closer, flatter and steeper; field of view, depth of field (off / subtle / miniature; tilted flat the foreground stays sharp and only the upper part of the picture is soft, a narrow sharp band needs miniature and a steep view), fog of war off (only in replays or after the match), "Save picture" writes a PNG to the user folder `photos/`. F2, the button or Esc leave and restore everything |
| Shift + Space *(rebindable)* | Jump to the newest alarm (fire on or loss of own units and buildings out of view, units stuck out of view since prompt 173); pressed again within 4 s: the next older one (prompt 101) |

**Tilt** (prompt 096): the zoom sets how steeply the camera looks down
(45° close, 65° far away), as before; your tilt is added to that and stays
while you zoom. The view goes from 25° (flat, towards the horizon) to 85°
(nearly straight down). Rotation and tilt are not saved: every match
starts facing north with the normal tilt. Turning and tilting do not end
the follow mode. The camera control can be switched off in Settings →
Camera; the keys and the drag on the map work without it.

## Selecting

| Input | Effect |
|---|---|
| Left click on an own unit | Select it |
| Left click on empty ground | Clear the selection |
| Shift + left click | Add or remove a unit |
| Left drag | Selection box (buildings only if nothing else is in it) |
| Double click on a unit | All visible own units of that type |
| Ctrl + double click (Cmd on macOS) on a unit | All own units of that type on the whole map (prompt 101) |
| Shift + A *(rebindable)* | The whole army: every own unit with a weapon, no structures; unarmed trucks, tractors and other carriers stay out (prompt 101) |
| . *(rebindable)* | Next idle own unit (stands without an order: no move, waypoint, attack, target it fires at, barrage, attack-move, secured area or engineer job), the camera goes there; again: the next one |
| Shift + . *(rebindable)* | Every idle own unit |
| Ctrl + 1…9 (Cmd on macOS) | Store the selection as group (with the current formation) |
| 1…9 | Recall the group (and its formation); twice quickly: camera there |
| Unit panel: click a tile | Select only that unit |
| Unit panel: Shift + click a tile | Remove that unit from the selection |
| Shift + I *(rebindable)* | Unit card of the active unit; again closes it (prompt 066) |
| Unit panel: "Values" in the corner of the picture | The same as the key |

**Active unit** (prompt 094): the selected unit you clicked last (on the
map, with Shift + click, or its tile). Its tile in the info field has a
bright frame; the camera buttons and the unit card refer to it. A selection
box, a double click or a group keep it if it is among the selected units;
otherwise the unit nearest to the box or the screen center (the first of a
group) becomes active. If it is destroyed, mounts or is deselected, the
nearest selected unit takes over.

## Orders

| Input | Effect |
|---|---|
| Right click on the ground | Move there in the formation of the order grid; infantry halts to fight enemies in range on the way (prompt 046) |
| Right drag on the ground | Move to where the drag started; the formation faces the drag direction and the units (tanks: their front armor) turn that way on arrival |
| Hold right button for a move | Target preview: a mark per unit where it will stand (grey: blocked spot), prompt 099 |
| ^ (key under Esc; \` on US keyboards) *(rebindable)*, held | Range and sight circles of the selection (prompt 099; Settings → Interface) |
| Shift + right click | Add a waypoint: the units drive there after their current orders; lines and markers show the chain |
| Right click on an enemy | Attack it |
| Ctrl + right click (Cmd on macOS) | Fire at the ground point |
| Right click with tanks | Never breaks a wall on purpose: the path search decides. Tanks drive through mud walls (from 8 t) and dry-stone walls (from 18 t) where the way around is clearly longer, and run over enemy soldiers in their way (prompt 093) |
| Right click on a building (infantry) | Occupy it; vehicles drive there |
| Minimap: right click (Shift: waypoint) | Move there |
| H *(rebindable)* | Stop (also drops the waypoints) |
| F *(rebindable)* | Next fire mode: fire at will → return fire → hold fire (prompt 176) |
| Z / X / V / B *(rebindable)* | Infantry: stand / kneel / prone / automatic stance |
| G / T *(rebindable)* | Mortars, guns and rocket launchers: barrage / smoke at the ground point under the cursor (prompt 032; a rocket launcher fires whole salvos, prompt 062) |
| Right click with squads on an own truck, halftrack or tank | Mount it (if its seats suffice; prompt 034) |
| Right click with vehicles on an own towed gun | The nearest vehicle strong enough hitches it |
| J *(rebindable)* | Passengers of the selected vehicles get off once the vehicle stands |
| Info field: click on a rider tile | Only this squad gets off (the gun is unhitched); once outside it alone is selected (prompt 109) |
| Info field: Shift + click on more rider tiles | Collects them: each gets off at once, all are selected together once they stand outside |
| O *(rebindable)* | Selected vehicles drop their towed guns; selected towed guns are unhitched |
| K *(rebindable)* | Mark the ground point under the cursor for your allies (ping, prompt 037) |
| , *(rebindable)* | Next formation: Auto, Line, Wedge, Column, Broad wedge, Dispersed (prompt 046) [provisional] |
| Minimap: Alt + left click | Mark that point for your allies |

**Targeted orders** from the order grid or their key wait for a left click
(the cursor turns into a cross, a hint shows at the top); right click
cancels. Shift + left click keeps the order active for more targets (moves
become waypoints).

| Key *(rebindable)* | Order |
|---|---|
| M | Move to the clicked point; nobody halts on the way (also infantry) |
| Y | Attack-move ("Advance"): every unit halts to fight enemies in range on the way, then drives on (prompt 046) [provisional] |
| I | Secure: take up the formation around the clicked point and hold 80 m around it; enemies inside are attacked, the units return to their places [provisional] |
| R | Attack the clicked enemy |
| L | Fire at the clicked ground point |
| Shift + L | Shell a building, wall or tree (prompt 093): the object under the cursor that can be shelled is outlined (the cursor shows a cross on a target, a "forbidden" sign elsewhere); the click orders tanks and guns to drive into range and shell it until it is a ruin or gone, then they stop by themselves. Only guns from 20 mm fire at it; machine guns keep firing at soldiers. Shift + click keeps the order armed for the next building (the new order replaces the old one: orders on objects are not queued) |
| P | Patrol: drive to the clicked point and back, again and again |
| U | Occupy the clicked building (infantry) |
| Barrage / Smoke buttons | Mortars and guns fire at the clicked point (the keys G / T fire at the cursor at once) |

**Engineers** (prompt 035): while engineer squads are selected, the command
bar shows AT mines, AP mines, Wire, Obstacles, Foxhole, Trench, Clear and
Bridge. Mines, foxhole and clear take one left click; wire, obstacles and
trench take two (start and end of the line); Bridge takes a click near a
bridge (an intact one is blown up, a destroyed one repaired). The tooltip
names the price of one piece. Known mines are marked on the ground (own
yellow, found enemy mines red). [provisional: no keys yet]

Positions (prompt 052): Comm. trench and Teeth take two clicks like a
trench; Vehicle pit, MG nest, Dugout and Camo net one click; Bunker two
(its place at a trench edge, then the direction it faces). While a line
waits for its end, a strip from its start to the cursor shows its length,
the number of pieces and the total price (and the price per meter). A
vehicle that stands in an own vehicle pit for 8 s is dug in; any move order
drives it out.

**Bridge-laying tanks and camouflage** (prompt 055): with bridge-laying
tanks selected the page "More orders" of the order grid shows Lay bridge (then a left click near a
narrow river or ditch: the tank lays its bridge across the nearest gap its
bridge spans) and Take up (it fetches its laid bridge). With vehicles or
guns selected, Camouflage puts up branches and nets (Uncover takes them off
when all of them have them). [provisional: no keys yet]

Passengers are hidden in their vehicle and cannot be selected; the info
field shows "Passengers 9 / 10", "towing …" or "towed by …" and "loading"
while mounting, dismounting, limbering or unlimbering runs. Dismount and
Unlimber are also tiles of the order grid (page "More orders").

**Rider tiles** (prompt 109): with an own vehicle selected (one unit, or the
active one of several) the info field lists under "Aboard …" one tile per
squad riding in it and per towed gun: picture, short name, soldiers alive /
squad size, wounded men as an orange number (details in the tooltip), a
health bar; dim with "getting off" while it gets off. A click puts exactly
that squad off where J puts everybody off (same order, only its id), or
unhitches that gun (Unlimber with the gun's id). The vehicle then leaves
the selection and the squad (or gun) alone is selected once it stands
outside; Shift + click collects several, selected together once all are
out. Selecting something else in between drops the switch. J still
unloads everybody. Vehicles of other players show no rider tiles.

## Formations (prompt 046)

The formation tile of the order grid (third row; its symbol is the
formation in use) opens the six formations and returns after a choice; it
sets the formation of the next moves (key `,` switches). **Auto** looks at the ground at the target:
column on roads and in narrows, dispersed in woods, wedge with vehicles and
line for infantry in the open. Every formation stands across its facing (the
direction of travel, or the right-drag direction). Selected units show an
attack-move route in orange and a secured area as a red circle. Control
groups remember their formation; there are no named squads [provisional].

**Route lines (prompt 166).** Units that got the same order share one
thin dashed line from their middle to the order's target, small dots at
their formation places and the waypoint chain once (yellow; patrols blue,
attack-move orange). The lines are strong for about 2.5 s after an order,
then faint; strong again while **Shift** (setting waypoints) or **Tab**
(show all own units) is held. The full path with every bend shows for a
single selected unit or the selected unit under the mouse pointer. Units
off the screen draw no line. Settings → Interface → "Route lines": off,
targets only, briefly after an order (default), always (every unit's full
route, as before). F5 still shows every path for debugging.

## Air support (prompt 033)

| Input | Effect |
|---|---|
| Support column (right edge, under the resources): Recon / Fighters / Attack / Bombers / Paratroopers | Wait for a left click on the map, then call the mission there; right click or Esc cancels |
| Shift + 1 / 2 / 3 / 4 / 5 *(rebindable)* | The same as the five tiles [provisional] |
| Support column: Paratroopers (prompt 055) | Transport aircraft drop three squads of paratroopers around the clicked point |
| Drag instead of click (prompt 055) | The aircraft fly along the dragged line; bombers lay a carpet over it (at most 300 m), transports drop along it |

Each tile shows the mission's symbol and its digit in the corner, under it
the price in gold (fuel / ammunition / manpower; red when it cannot be
paid) or in blue the seconds until the mission can be called again, with a
dark sector over the tile that shrinks meanwhile; a greyed tile cannot be
called, its tooltip says why and names the aircraft. The hint "Air support:
left click the target …" shows at the top only while a target is picked. Own
missions show their approach as a dashed line to a ring on the target with
the time until the aircraft arrive. Enemy aircraft coming into sight post
**Enemy aircraft!** in the event feed.

## Heavy strike (prompt 054)

| Input | Effect |
|---|---|
| Support column (last tile, base matches): Rockets | Wait for a left click on the map, then call the rocket strike there; right click or Esc cancels |
| Shift + 6 *(rebindable)* | The same as the tile (Shift + 5 is the paratroopers of prompt 055) [provisional] |

The tile shows the price, the time until the strike unlocks ("in 9:58"),
the cooldown, or "–" while there is no completed rocket battery position
(the tooltip says so). A called strike shows red smoke and a ring on the target, sounds a
siren and counts down at the top of the screen; an enemy strike also marks
the enemy's battery position (beam and ring) and pings it on the minimap.

## Base building (skirmish)

| Input | Effect |
|---|---|
| Top bar: the hammer left of the resources | Open or close the build menu above the order grid (price, greyed out with the reason) |
| Build menu: a building | Place it: left click (Shift: place more), right click or Esc cancels |
| Select an own barracks or factory | Production bar above the order grid: queue units. The order grid shows the building's page: Repair, Sell, Cancel last order |
| Build menu / production bar: tabs | Show one kind (base, defense, infantry, vehicles, guns, air); a tile shows the five ratings of its unit, its tooltip their numbers, a few values and what it is good against and vulnerable to (prompt 066) |
| Build menu / production bar: right click on a tile | Unit card of that unit; the left click still builds it |
| Alt + 1 … 9, 0 | Pick the first … tenth tile of the open tab: of the production bar if it shows, else of the build menu (prompt 058, [provisional]) |
| Right click with only own buildings selected | Rally point |

## Screen

Every part has its corner (prompt 095). Always there: the minimap and the
top bar. Everything else appears when there is something to do with it.

| Where | What | Shows |
|---|---|---|
| bottom left | Minimap with the "N" mark; right of it its buttons | always |
| above the minimap | Camera control: a dial with a compass rose and a house button | unless switched off (Settings → Camera) |
| bottom center | Info field of the selection | with a selection |
| bottom right | Order grid, 5 × 3 tiles | with own units (or one own building) selected |
| above the order grid | Build menu / production bar | while open / with a producing building selected |
| top right | Resources, Build, "?" and the menu button | always (resources and Build in matches with base building) |
| right edge, under the resources | Support column | when you have air support or a rocket strike |
| top center | Tabs of the control groups in use; under them the hint of a pending order | groups in use |
| left edge | Events | messages |

- **Minimap** (bottom left): terrain, own units green, enemies in sight red,
  supply points and mission objectives as marks (see below),
  everything out of sight dimmed (the whole map is drawn from the start),
  the camera's view as a trapezoid, red rings where own units are
  hit. North is always up ("N" on the frame); the map does not turn with
  the camera, so a click always means the same place, and the trapezoid
  shows where you look (wider and farther the flatter the camera is
  tilted). Buttons right of it: **zoom** (the magnifier: camera back to its
  starting distance), **pause** (opens the pause menu; missing in network
  matches, which cannot pause), and, only while the camera control is
  switched off, **north** (the needle turns with the camera and shows
  where north is on the screen; a click or N turns the camera back).
- **Camera control** (above the minimap, prompt 096): a dark dial with a
  compass rose that lies on the ground as the camera sees it. Its gold
  "N" always points to the map's north on the screen, and the tilt
  squashes the rose: a narrow ellipse for a flat view, nearly a circle
  when you look straight down. Drag on it to turn and tilt, click a
  letter to look that way, use the wheel to tilt; the house button beside
  it (or a double click on the dial) restores north and the normal tilt.
  See "Camera" for all inputs.
- **Info field** (bottom center): one unit with its name and rank, the kind
  of unit (top right), picture, hit points and soldiers as numbers with a
  symbol, bars for health, ammunition, fuel and experience (numbers in the
  tooltips), armor, weapons, stance and fire mode, the five ratings and the
  buttons "Values", center and follow; several units as tiles with health
  bars. Very faint behind it: the marking of the unit's nation.
- **Order grid** (bottom right): symbols instead of words; the tooltip
  names the order, its key and what it does, the key stands in the tile's
  corner, a gold dot marks a lasting state (holding fire, camouflaged,
  repairing, the formation in use). Every order has a fixed place:

  | | 1 | 2 | 3 | 4 | 5 |
  |---|---|---|---|---|---|
  | **Movement** | Move (M) | Advance (Z) | Patrol (P) | Secure (I) | Stop (H) |
  | **Combat** | Attack (R) | Ground (L) | Shell (Shift + L) | Barrage (G) | Smoke (T) |
  | **Posture, more** | Fire ▸ (F cycles) | Stance ▸ | Formation ▸ (,) | Occupy (U) | More orders ▸ |

  An order that does not fit the selection stays in its place, greyed, so
  the others never move. There are more than 15 orders, so four tiles
  open a page of their own and the last place of such a page leads back:
  **Fire** (prompt 176: fire at will, return fire, hold fire, and the
  switches close range only and vehicles only; the tile shows the mode of
  the selection, a dot when it is not "fire at will" or a limit is on),
  **Stance** (Stand Y, Kneel X, Prone V, Auto B; returns after a choice),
  **Formation** (the six formations; returns after a choice) and **More
  orders**: Dismount (J), Unlimber (O), Camouflage, Lay bridge, Take up in
  the first row, the engineers' obstacles in the second and third (mines,
  wire, hedgehogs, dragon's teeth, clear, bridge) and **Positions ▸**
  (foxhole, trench, communication trench, MG nest, dugout, vehicle pit,
  camouflage net, bunker). On these pages a place stays empty when no
  selected unit can do that. Why pages and not a second grid: the orders
  used all the time keep one fixed place on the first page, and the rare
  ones (stances have keys, engineer works are picked once and then drawn)
  cost one more click. The grid returns to the first page when the
  selection changes. With one own building selected the grid shows Repair,
  Sell and "Cancel last order" in the third row.
- **Top bar** (top right): fuel, ammunition and manpower with a symbol,
  the income per minute small beside the number (all three in the tooltip),
  red while a stock is empty; the hammer opens the build menu; "?" opens
  the list of keys (Settings → Controls), the three lines the pause menu.
- **Group tabs** (top center): a small tab per control group that has
  units; a click selects the group like its number key (twice: the camera
  goes there). The tab of the group that is selected right now is lit.
- **Events** (left edge): unit lost, building completed, unit ready, enemy
  sighted, under fire. Click one to look there.
- **Alarms** (prompt 101): fire on own units and buildings and own losses
  sound, ping the minimap and show "under fire" / "building under fire" /
  "lost" only where the camera does not look. In view a hit stays quiet
  and a loss shows in the feed without the alarm sound. Out of view a small
  arrow at the screen edge points at the place (yellow: fire, red: loss)
  for 6 s; click it to look there. Events within 60 m are one place: it
  alarms again after 10 s (losses 3 s), another place alarms at once.
  Losses out of view stay 30 s in the feed and are not pushed out by
  newer messages. Replays and spectators get no alarms.
- **Stuck units** (prompt 173): a unit that gets no closer to its target
  for 20 s, or that the movement rules stop short of it, stops where it is.
  The feed says "Cannot get through: <unit>" (in view too); out of view it
  also pings the minimap and shows a sand-colored edge arrow (no sound),
  and Shift + Space jumps there like to the other alarms. Until it gets a
  new order the unit carries a state mark (a wall with an arrow).
- **Health bars**: short bars over damaged units and over own units that
  are selected (yellow), under the mouse pointer or shown with **Tab**
  (blue in the standard palette, the own color in the other palettes);
  own squads show one bar per man (at most six, men grouped beyond),
  stacked like stairs. Allied bars have a light rim. Bars fade out when
  they are no longer wanted. Settings → Interface can limit them to
  selected and marked units or hide them (prompt 167).
- **Order feedback** (prompt 175):
  - An order the game drops shows its reason beside the pointer for 2.5 s
    and in the event feed, with a quiet click.
  - The target preview of a move marks in orange where a unit really stops
    when its slot cannot be reached (across water without a crossing it may
    use, beyond the map edge), with a dashed line from the clicked slot and
    "Target not reachable – drives up to here".
  - Greyed order tiles name the reason in their tooltip.
  - The info field of an own unit starts with what it does ("Activity:
    drives", "in combat with T-34/76", "cannot get through") and, if armed,
    whether it fires and why not ("Fire: no enemy in sight", "enemy out of
    range", "weapons cannot hurt the enemy", "target not in sight", "holds
    fire", "reloads", "aims at", "pinned down", "out of ammunition"). With
    several units the header counts them ("4 in combat, 2 driving").
  - With own armed units selected, the pointer over an enemy shows a light:
    three dots for front, side and rear (green: penetrated, red: not) and
    "Penetrates", "Only from the side or rear" or "Does not penetrate", for
    the active (or first armed) unit at its current distance, head-on.

  Reasons for a dropped order:

  | Reason | When |
  |---|---|
  | Target outside the map | a move, attack, barrage, air call or engineer line beyond the map |
  | None of these units can move | a move order (also patrol, waypoint, formation, secure, occupy) for own units that are all buildings, unhitched guns, aboard or towed, out of fuel or immobilized |
  | That is an own or allied unit | attacking a friend |
  | The target no longer exists | attacking a destroyed or unknown unit |
  | No weapon of these units can hurt the target | attacking a tank with rifles only (at any distance) |
  | Out of ammunition | attacking with units that are all out of ammunition |
  | This cannot be shelled | shelling an object without hit points, or already destroyed |
  | No gun that damages buildings | shelling with units without such a gun |
  | No own vehicle with free seats | mounting an enemy, destroyed, towed or seatless vehicle |
  | No infantry squad selected | mounting or occupying without squads |
  | Not an own free gun | towing a unit that is no own gun, is towed already or is being hitched |
  | No selected vehicle can pull this gun | no selected vehicle is strong enough or free |
  | No towed gun to unlimber | unlimbering without a towed gun |
  | Squads cannot enter this | occupying an object that is no enterable building |
  | It cannot be built here | placing a building where it does not fit |
  | Not enough resources | building, producing, air support, heavy strike |
  | Not available now | producing what the building does not make or while it is built, air support or a strike the faction lacks or that is in its cooldown |
  | The production queue is full | producing beyond the queue |
  | No selected unit fires indirectly | a barrage without artillery or mortars |
  | No engineers selected | mines, wire, obstacles, positions or clearing without engineers |

- **Gun crews** (prompt 177): the info field of a towed gun shows its
  crew ("Crew 4 / 6"); a gun whose men are all hit says "Abandoned" and
  does not fire. Machine guns hurt the crew, not the gun, unless its shield
  faces them.
- **State marks** (prompt 105): small symbols above the health bar tell
  a unit's state without selecting it, at most two side by side (from far
  away only the most important one; the rest is in the info field):

  | Symbol | Color | Means |
  |---|---|---|
  | flame | orange | stands in the flames of a fire |
  | wheel, struck through | red | immobilized (too damaged to drive) |
  | broken track | red | lost its track to a mine |
  | helmet pressed down by an arrow | white | suppressed or pinned down |
  | crosshair with an arrow back | grey | own unit returns fire only (prompt 176) |
  | crosshair, struck through | grey | own unit holds fire (prompt 176) |
  | arrow stopped at a wall | sand | stuck: stopped short of its target, waits for a new order (prompt 173) |
  | shell, struck through | amber | out of ammunition |
  | drop, struck through | blue | out of fuel |
  | wrench with a plus | green | being repaired or resupplied |

  Over enemies in sight only what can be seen from outside: flames,
  immobilized, track off, suppressed (never out of ammunition or fuel).
  Settings → Interface → State marks: own units and visible enemies,
  selected units only, or never.
- **Own units** (prompts 164, 167): a small light ring with a dark rim
  under every own soldier. Only the unit whose model is directly under the
  mouse pointer (a few pixels of tolerance; on free ground between units
  nothing) gets a thin outline along its silhouette: own units blue (with
  their blue health bar), enemies red, allies in their color; a squad as a
  whole. A click selects exactly what is outlined (a click just beside a
  small unit still takes the nearest one). No rings and no lighting up on
  hover. Hold **Tab** *(rebindable)* to show every own unit (ring, bars,
  bright markers). Settings → Interface → "Own units": markers and outline
  (default), outline under the pointer only, or off. Pictures:
  `just unit-marker-shots` (`build/unit-visibility/`).
- **Hidden units** (prompt 165): the parts of own units hidden behind
  houses, trees, hills or vehicles show as a flat silhouette in the own
  color (selected units and those near the pointer brighter), allied ones
  in the ally color; never enemy units, not through tall grass or smoke,
  not for soldiers inside a building or a trench. A click on a silhouette
  selects the unit. Settings → Interface → "Hidden units as silhouette":
  own and allied (default), own only, or off. Hidden with the HUD (F1) and
  in photo mode. Pictures: `just hidden-unit-shots` (`build/occluded-units/`).
- **Target preview** (prompt 099): while the right button is held for a
  move (and for a second after any move, attack-move or secure order) every
  selected unit shows a flat mark where it will stand: a rectangle with a
  nose for vehicles, a disc for squads, in the own color, turned to the
  facing of the drag. A grey mark is a spot the unit cannot stand on
  (water, house, rock); it stops at the nearest reachable spot instead. A
  tractor towing a gun shows the gun's range circle at its mark.
- **Mark switches** (prompt 103): Settings → Interface has a box for
  selection rings, routes and waypoints, supply and repair radii, flags,
  capture rings and borders of supply points, impact markers, mission rings, the
  ally / enemy tint and the edge arrows, and "Lines on the ground" strong or
  subtle (half strength for routes, previews, supply and range circles).
- **Supply points** (prompt 107): the edge of the capture area is a line
  on the ground in the holder's color: pale and dashed while nobody holds
  it, solid for you and your allies, with short ticks inwards for the
  enemy; it blinks while both sides stand in the area. On the minimap each
  point is a mark: hollow circle neutral, filled circle yours, square an
  ally's, diamond the enemy's, with an arc for the capture progress; an
  enemy point out of sight shows its holder as last seen. Rest the mouse
  on the area for the holder, the income per minute and the progress.
  Settings → Interface → "Border of supply point areas": strong, subtle
  or off. The event feed says "supply point taken", "lost" and "under
  attack" (click or Space to jump there). The top bar shows the income
  per minute beside each stock. Active mission objectives are gold rings
  with four ticks on the minimap.
- **Ground tooltip** (prompt 100): rest the mouse on the map for 0.4 s and
  a small box beside the pointer names the ground or the object under it
  (meadow, road, ford, woods, farmhouse, wall, trench …) and says what it
  does: movement for vehicles and infantry (or, when all selected units
  share one running gear, for tracks, wheels, half-tracks or foot) as a
  word and the percent of the cross-country speed, with rain or snow
  softening the ground; "road speed" on roads; impassable; the slope when
  it slows uphill noticeably; **concealment** (woods, buildings, rock,
  smoke hide what is in or behind them) and, separately, **cover** (heavy
  where an object stops bullets, otherwise the share of hits soft cover,
  a dug position or a crater catches). Field works of the enemy show only
  where you see them. No box during a box selection, a right-drag, a
  camera drag, while placing a building or over a HUD panel. Settings →
  Interface → "Ground tooltip": when the mouse rests (default), only while
  **-** (right of the period on German keyboards, / on US ones;
  *rebindable*) is held, or off.
- **Range circles** (prompt 099): the fire range of the selected units as
  a thin yellow circle on the ground (the longest-reaching weapon; mortars
  and guns firing indirectly: minimum and maximum, the minimum dotted
  orange; flak: its anti-aircraft range in light blue), the sight range
  dotted pale blue. A tractor shows the circle of its towed gun, a vehicle
  those of its riders, dashed and pale: they cannot fire like this. Up to
  8 selected units get circles, more: only the active unit. Hold **^**
  (the key under Esc; \` on US keyboards, *rebindable*) to show them;
  Settings → Interface → "Range circles" / "Sight circles": always with a
  selection (range default), only while the key is held (sight default),
  or never. A circle is only a distance: woods, houses and slopes that
  block the view or the shot are not cut out.
- **Unit card** (under the top bar, left of the support column, prompt 066): all game values of a unit type and,
  for vehicles of the vehicle database, its technical data, in two tabs. It
  opens by a right click on a tile of the build menu or the production bar,
  by Shift + I for the selected unit and by the button "Values" in the info field;
  Esc or its ✕ closes it. The match goes on while it is open and every
  order still works. Plain I is "Secure"; every letter key has an order, so
  the card got Shift + I. The same card for everything in the catalog, with
  the 3D model to turn and a camouflage choice: main menu → Units. What the
  five ratings mean: [GAME_DESIGN.md](GAME_DESIGN.md#what-a-unit-can-do-the-five-ratings-provisional).
- **Esc**: closes the unit card if it is open; else the pause menu
  (settings, surrender, quit).
- **F3**: debug menu.
