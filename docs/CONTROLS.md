# Controls

How to play Mud and Steel with mouse and keyboard. Keys marked *(rebindable)*
can be changed in **Settings → Controls**; the command bar always shows the
current key. Technical details: [ARCHITECTURE.md](ARCHITECTURE.md#selection-presentation-only)
and [the HUD section](ARCHITECTURE.md#hud-presentation-only). The tutorial
(prompt 039) builds on this page.

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
| Ctrl + 1…9 (Cmd on macOS) | Store the selection as group |
| 1…9 | Recall the group; twice quickly: camera there |
| Unit panel: click a tile | Select only that unit |
| Unit panel: Shift + click a tile | Remove that unit from the selection |

## Orders

| Input | Effect |
|---|---|
| Right click on the ground | Move there (in formation) |
| Shift + right click | Add a waypoint: the units drive there after their current orders; lines and markers show the chain |
| Right click on an enemy | Attack it |
| Ctrl + right click (Cmd on macOS) | Fire at the ground point |
| Right click on a building (infantry) | Occupy it; vehicles drive there |
| Minimap: right click (Shift: waypoint) | Move there |
| H *(rebindable)* | Stop (also drops the waypoints) |
| F *(rebindable)* | Hold fire / fire at will |
| Z / X / V / B *(rebindable)* | Infantry: stand / kneel / prone / automatic stance |
| G / T *(rebindable)* | Mortars and guns: barrage / smoke at the ground point under the cursor (prompt 032) |

**Targeted orders** from the command bar or their key wait for a left click
(the cursor turns into a cross, a hint shows at the top); right click
cancels. Shift + left click keeps the order active for more targets (moves
become waypoints).

| Key *(rebindable)* | Order |
|---|---|
| M | Move to the clicked point |
| R | Attack the clicked enemy |
| L | Fire at the clicked ground point |
| P | Patrol: drive to the clicked point and back, again and again |
| U | Occupy the clicked building (infantry) |
| Barrage / Smoke buttons | Mortars and guns fire at the clicked point (the keys G / T fire at the cursor at once) |

## Base building (skirmish)

| Input | Effect |
|---|---|
| Command bar: Build | Open the build menu (price, greyed out with the reason) |
| Build menu: a building | Place it: left click (Shift: place more), right click or Esc cancels |
| Select an own barracks or factory | Production bar above the command bar: queue units, cancel the last |
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
- **Esc**: pause menu (settings, surrender, quit).
- **F3**: debug menu.
