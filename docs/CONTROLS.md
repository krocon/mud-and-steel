# Controls

How to play Mud and Steel with mouse and keyboard. Keys marked *(rebindable)*
can be changed in **Settings → Controls**; the command bar always shows the
current key. Technical details: [ARCHITECTURE.md](ARCHITECTURE.md#selection-presentation-only)
and [the HUD section](ARCHITECTURE.md#hud-presentation-only). The tutorial
(prompt 039, main menu → Tutorial, [MISSIONS.md](MISSIONS.md)) builds on
this page.

## Camera

| Input | Effect |
|---|---|
| W A S D, arrow keys *(rebindable)*, mouse at the screen edge | Pan |
| Mouse wheel, trackpad pinch | Zoom (towards the cursor) |
| Middle mouse drag, Option/Alt + mouse move, two-finger trackpad pan | Grab the map |
| Q / E *(rebindable)* | Turn by 45° |
| N *(rebindable)* | Face north |
| Double click on empty ground | Glide there |
| C *(rebindable)* | Follow the selection (any pan ends it) |
| Minimap: left click or drag | Look there |
| Space *(rebindable)* | Jump to the newest event message |

## Selecting

| Input | Effect |
|---|---|
| Left click on an own unit | Select it |
| Left click on empty ground | Clear the selection |
| Shift + left click | Add or remove a unit |
| Left drag | Selection box (buildings only if nothing else is in it) |
| Double click on a unit | All visible own units of that type |
| Ctrl + 1…9 (Cmd on macOS) | Store the selection as group (with the current formation) |
| 1…9 | Recall the group (and its formation); twice quickly: camera there |
| Unit panel: click a tile | Select only that unit |
| Unit panel: Shift + click a tile | Remove that unit from the selection |
| Shift + I *(rebindable)* | Unit card of the selected unit (the first of several); again closes it (prompt 066) |
| Unit panel: "Values" in the corner of the picture | The same as the key |

## Orders

| Input | Effect |
|---|---|
| Right click on the ground | Move there in the formation of the command bar; infantry halts to fight enemies in range on the way (prompt 046) |
| Right drag on the ground | Move to where the drag started; the formation faces the drag direction and the units (tanks: their front armor) turn that way on arrival |
| Shift + right click | Add a waypoint: the units drive there after their current orders; lines and markers show the chain |
| Right click on an enemy | Attack it |
| Ctrl + right click (Cmd on macOS) | Fire at the ground point |
| Right click on a building (infantry) | Occupy it; vehicles drive there |
| Minimap: right click (Shift: waypoint) | Move there |
| H *(rebindable)* | Stop (also drops the waypoints) |
| F *(rebindable)* | Hold fire / fire at will |
| Z / X / V / B *(rebindable)* | Infantry: stand / kneel / prone / automatic stance |
| G / T *(rebindable)* | Mortars and guns: barrage / smoke at the ground point under the cursor (prompt 032) |
| Right click with squads on an own truck, halftrack or tank | Mount it (if its seats suffice; prompt 034) |
| Right click with vehicles on an own towed gun | The nearest vehicle strong enough hitches it |
| J *(rebindable)* | Passengers of the selected vehicles get off once the vehicle stands |
| O *(rebindable)* | Selected vehicles drop their towed guns; selected towed guns are unhitched |
| K *(rebindable)* | Mark the ground point under the cursor for your allies (ping, prompt 037) |
| , *(rebindable)* | Next formation: Auto, Line, Wedge, Column, Broad wedge, Dispersed (prompt 046) [provisional] |
| Minimap: Alt + left click | Mark that point for your allies |

**Targeted orders** from the command bar or their key wait for a left click
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
tanks selected the command bar shows Lay bridge (then a left click near a
narrow river or ditch: the tank lays its bridge across the nearest gap its
bridge spans) and Take up (it fetches its laid bridge). With vehicles or
guns selected, Camouflage puts up branches and nets (Uncover takes them off
when all of them have them). [provisional: no keys yet]

Passengers are hidden in their vehicle and cannot be selected; the unit
panel shows "Passengers 9 / 10", "towing …" or "towed by …" and "loading"
while mounting, dismounting, limbering or unlimbering runs. Dismount and
Unlimber are also buttons of the command bar.

## Formations (prompt 046)

The first two cells of the command bar choose the formation of the next
moves (key `,` switches). **Auto** looks at the ground at the target:
column on roads and in narrows, dispersed in woods, wedge with vehicles and
line for infantry in the open. Every formation stands across its facing (the
direction of travel, or the right-drag direction). Selected units show an
attack-move route in orange and a secured area as a red circle. Control
groups remember their formation; there are no named squads [provisional].

## Air support (prompt 033)

| Input | Effect |
|---|---|
| Air bar (above the command bar): Recon / Fighters / Attack / Bombers / Paratroopers | Wait for a left click on the map, then call the mission there; right click or Esc cancels |
| Shift + 1 / 2 / 3 / 4 / 5 *(rebindable)* | The same as the five buttons [provisional] |
| Air bar: Paratroopers (prompt 055) | Transport aircraft drop three squads of paratroopers around the clicked point |
| Drag instead of click (prompt 055) | The aircraft fly along the dragged line; bombers lay a carpet over it (at most 300 m), transports drop along it |

The buttons show the price (fuel / ammunition / manpower) or the seconds
until the mission can be called again; the tooltip names the aircraft. Own
missions show their approach as a dashed line to a ring on the target with
the time until the aircraft arrive. Enemy aircraft coming into sight post
**Enemy aircraft!** in the event feed.

## Heavy strike (prompt 054)

| Input | Effect |
|---|---|
| Strike bar (above the command bar, base matches): Rockets | Wait for a left click on the map, then call the rocket strike there; right click or Esc cancels |
| Shift + 6 *(rebindable)* | The same as the button (Shift + 5 is the paratroopers of prompt 055) [provisional] |

The button shows the price, the time until the strike unlocks, the
cooldown, or "no battery" while there is no completed rocket battery
position. A called strike shows red smoke and a ring on the target, sounds a
siren and counts down at the top of the screen; an enemy strike also marks
the enemy's battery position (beam and ring) and pings it on the minimap.

## Base building (skirmish)

| Input | Effect |
|---|---|
| Command bar: Build | Open the build menu (price, greyed out with the reason) |
| Build menu: a building | Place it: left click (Shift: place more), right click or Esc cancels |
| Select an own barracks or factory | Production bar above the command bar: queue units, cancel the last |
| Build menu / production bar: tabs | Show one kind (base, defense, infantry, vehicles, guns, air); a tile shows the five ratings of its unit, its tooltip their numbers, a few values and what it is good against and vulnerable to (prompt 066) |
| Build menu / production bar: right click on a tile | Unit card of that unit; the left click still builds it |
| Alt + 1 … 9, 0 | Pick the first … tenth tile of the open tab: of the production bar if it shows, else of the build menu (prompt 058, [provisional]) |
| Right click with only own buildings selected | Rally point |

## Screen

- **Minimap** (bottom right): terrain, own units green, enemies in sight red,
  fog of war, the camera's view as a trapezoid, red rings where own units are
  hit.
- **Unit panel** (bottom center): one unit with picture, hit points, armor,
  weapons, stance and fire mode; several units as tiles with health bars.
- **Command bar** (bottom left): every order with its key; buttons that do
  not fit the selection are greyed out.
- **Events** (left edge): unit lost, building completed, unit ready, enemy
  sighted, under fire. Click one to look there.
- **Health bars**: over damaged and selected units; Settings → Interface
  can limit them to selected units or hide them.
- **Unit card** (top right, prompt 066): all game values of a unit type and,
  for vehicles of the vehicle database, its technical data, in two tabs. It
  opens by a right click on a tile of the build menu or the production bar,
  by Shift + I for the selected unit and by the button in the unit panel;
  Esc or its ✕ closes it. The match goes on while it is open and every
  order still works. Plain I is "Secure"; every letter key has an order, so
  the card got Shift + I. The same card for everything in the catalog, with
  the 3D model to turn and a camouflage choice: main menu → Units. What the
  five ratings mean: [GAME_DESIGN.md](GAME_DESIGN.md#what-a-unit-can-do-the-five-ratings-provisional).
- **Esc**: closes the unit card if it is open; else the pause menu
  (settings, surrender, quit).
- **F3**: debug menu.
