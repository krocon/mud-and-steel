# Unit browser

A static page that shows every entry of the unit catalog with a picture, a 3D view
and its values. Start it with `just units-web` (serves the repository root on
`http://localhost:8137/web/units/`; `--port N`, `--no-open`).

There is no build step and no copy of the data. The page reads at runtime:

| What | From |
|---|---|
| Units and their real values | `data/catalog/*.toml` |
| Base buildings | `data/economy.toml` |
| Prices and build times | `cli economy-table`, written to `build/units-web/economy.md` by `tools/units_web.py` |
| Models, squad scenes | `godot/assets/models/` (`.glb`, squads from their `.tscn`) |
| Camouflage (forest, desert, winter) | `godot/assets/materials/camo/*.tres` |
| Infantry colors | `godot/assets/models/infantry/uniform_palettes.json` |
| List pictures | `godot/assets/thumbnails/<id>.png`, else a snapshot of the 3D model |

Limits:

- `viewer.js` contains a GLSL port of `godot/assets/shaders/unit_camo.gdshader`.
  If that shader changes, port the change by hand.
- Soldiers and environment objects use flat slot colors, not the game's procedural
  shaders; markings (decals) are not shown.
- three.js and the TOML parser come from a CDN (pinned versions in `index.html`),
  so the page needs internet access.
- Fields the page does not know are listed under their catalog name, so new
  catalog fields show up without a change here.
