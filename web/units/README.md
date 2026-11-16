# Unit page

A static page that presents the game and its unit catalog: a hero with a live 3D
model, the three armies, a duel that compares two units, and the full catalog with a
data sheet per entry (3D view with dimension lines, armor, weapons, sources). Start it
with `just units-web` (serves the repository root on `http://localhost:8137/web/units/`;
`--port N`, `--no-open`). `…/web/units/#<id>` opens the data sheet of that entry.

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

Files:

| File | Holds |
|---|---|
| `index.html` | Page structure and the texts about the game (German) |
| `style.css` | Look and motion; the biome (`html[data-biome]`) sets accent, sky and tint |
| `app.js` | Data loading and every section: hero, armies, duel, catalog, data sheet |
| `viewer.js` | 3D view: models, camouflage shader, camera, dimension lines, snapshots |
| `fx.js` | Weather over the hero (rain, dust, snow), counting numbers, scroll reveals |

The camouflage switch changes the whole page: camouflage of every model, accent color,
sky and weather. Visitors who ask for reduced motion get a still page.

Limits:

- `viewer.js` contains a GLSL port of `godot/assets/shaders/unit_camo.gdshader`.
  If that shader changes, port the change by hand.
- Soldiers and environment objects use flat slot colors, not the game's procedural
  shaders; markings (decals) are not shown.
- Models are expected to look along -Z; camera, light and dimension lines rely on it.
- The texts about the game in `index.html` are written by hand; keep them in line with
  `docs/GAME_DESIGN.md`, `docs/SCALE.md`, `docs/TEAMS.md` and `docs/AI.md`.
- The hero shows the units listed in `FEATURED`, the duel starts with `DUEL_DEFAULT`
  (both in `app.js`).
- three.js, the TOML parser and the fonts come from a CDN (pinned versions in
  `index.html`), so the page needs internet access.
- Fields the page does not know are listed under their catalog name, so new
  catalog fields show up without a change here.
