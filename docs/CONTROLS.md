# Controls

How to play Mud and Steel with mouse and keyboard. Keys marked *(rebindable)*
can be changed in **Settings → Controls**; every tile of the order grid
shows its current key in its corner and in its tooltip. Technical details: [ARCHITECTURE.md](ARCHITECTURE.md#selection-presentation-only)
and [the HUD section](ARCHITECTURE.md#hud-presentation-only). The tutorial
(prompt 039, main menu → Tutorial, [MISSIONS.md](MISSIONS.md)) builds on
this page.

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
| Double click on empty ground | Glide there |
| Shift + C *(rebindable)*, info field: button with the ring | Glide to the active unit (see "Selecting"); zoom and rotation stay (prompt 094) |
| C *(rebindable)*, info field: toggle with the frame | Follow the active unit: it stays in the screen center while it drives. Zoom and rotation keep the mode; moving the map by hand (keys, screen edge, drag, trackpad, minimap, double click on the ground, Space) ends it. Without a selection it waits for the next one. The game remembers the choice for the next match |
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
| Shift + right click | Add a waypoint: the units drive there after their current orders; lines and markers show the chain |
| Right click on an enemy | Attack it |
| Ctrl + right click (Cmd on macOS) | Fire at the ground point |
| Right click with tanks | Never breaks a wall on purpose: the path search decides. Tanks drive through mud walls (from 8 t) and dry-stone walls (from 18 t) where the way around is clearly longer, and run over enemy soldiers in their way (prompt 093) |
| Right click on a building (infantry) | Occupy it; vehicles drive there |
| Minimap: right click (Shift: waypoint) | Move there |
| H *(rebindable)* | Stop (also drops the waypoints) |
| F *(rebindable)* | Hold fire / fire at will |
| Z / X / V / B *(rebindable)* | Infantry: stand / kneel / prone / automatic stance |
| G / T *(rebindable)* | Mortars, guns and rocket launchers: barrage / smoke at the ground point under the cursor (prompt 032; a rocket launcher fires whole salvos, prompt 062) |
| Right click with squads on an own truck, halftrack or tank | Mount it (if its seats suffice; prompt 034) |
| Right click with vehicles on an own towed gun | The nearest vehicle strong enough hitches it |
| J *(rebindable)* | Passengers of the selected vehicles get off once the vehicle stands |
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

## Formations (prompt 046)

The formation tile of the order grid (third row; its symbol is the
formation in use) opens the six formations and returns after a choice; it
sets the formation of the next moves (key `,` switches). **Auto** looks at the ground at the target:
column on roads and in narrows, dispersed in woods, wedge with vehicles and
line for infantry in the open. Every formation stands across its facing (the
direction of travel, or the right-drag direction). Selected units show an
attack-move route in orange and a secured area as a red circle. Control
groups remember their formation; there are no named squads [provisional].

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
| bottom left | Minimap with the "N" mark; right of it three buttons | always |
| bottom center | Info field of the selection | with a selection |
| bottom right | Order grid, 5 × 3 tiles | with own units (or one own building) selected |
| above the order grid | Build menu / production bar | while open / with a producing building selected |
| top right | Resources, Build, "?" and the menu button | always (resources and Build in matches with base building) |
| right edge, under the resources | Support column | when you have air support or a rocket strike |
| top center | Tabs of the control groups in use; under them the hint of a pending order | groups in use |
| left edge | Events | messages |

- **Minimap** (bottom left): terrain, own units green, enemies in sight red,
  fog of war, the camera's view as a trapezoid, red rings where own units are
  hit. North is always up ("N" on the frame); the map does not turn with
  the camera, so a click always means the same place, and the trapezoid
  shows where you look. Buttons right of it: **north** (the needle turns
  with the camera and shows where north is on the screen; a click or N
  turns the camera back), **zoom** (the magnifier: camera back to its
  starting distance), **pause** (opens the pause menu; missing in network
  matches, which cannot pause).
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
  | **Posture, more** | Hold fire (F) | Stance ▸ | Formation ▸ (,) | Occupy (U) | More orders ▸ |

  An order that does not fit the selection stays in its place, greyed, so
  the others never move. There are more than 15 orders, so three tiles
  open a page of their own and the last place of such a page leads back:
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
- **Health bars**: over damaged and selected units; Settings → Interface
  can limit them to selected units or hide them.
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
