# Unit page

A static page that presents the game and its unit catalog: a hero with a live 3D
model, the three armies, a duel that compares two units, and the full catalog with a
data sheet per entry (3D view with dimension lines, armor, weapons, sources). Start it
with `just units-web` (serves the repository root on `http://localhost:8137/web/units/`;
`--port N`, `--no-open`). `…/web/units/#<id>` opens the data sheet of that entry.

There is no build step. Served locally, the page reads at runtime (paths from the
repository root):

| What | From |
|---|---|
| Units and their real values | `data/catalog/*.toml` |
| Base buildings | `data/economy.toml` |
| Technical data of armored vehicles | `data/vehicles/{de,su,us}.json` (prompt 064, `data/vehicles/SCHEMA.md`) |
| Prices and build times | `cli economy-table`, written to `web/units/economy.md` by `tools/units_web.py` |
| Game values and the five ratings | `cli unit-stats` (`sim::unit_stats`), written to `web/units/stats.json` by `tools/units_web.py` |
| Models, squad scenes | `godot/assets/models/` (`.glb`, squads from their `.tscn`) |
| Camouflage (forest, desert, winter) | `godot/assets/materials/camo/*.tres` |
| Infantry colors | `godot/assets/models/infantry/uniform_palettes.json` |
| List pictures | `godot/assets/thumbnails/<id>.png`, else a snapshot of the 3D model |

The website serves `web/units/` itself as its root, so `app.js` resolves the paths
above against that root. `just units-web-sync` copies every file in the table to the
same path under `web/units/` (`data/`, `godot/`), removes copies nothing loads any more
and writes `economy.md`. Run it after changes to the catalog, models, palettes or
thumbnails and before uploading `web/units/`; never edit the copies by hand.

Files:

| File | Holds |
|---|---|
| `index.html` | Page structure and the texts about the game (German) |
| `style.css` | Look and motion; the biome (`html[data-biome]`) sets accent, sky and tint |
| `app.js` | Data loading and every section: hero, armies, duel, catalog, data sheet |
| `viewer.js` | 3D view: models, camouflage shader, camera, dimension lines, snapshots |
| `fx.js` | Weather over the hero (rain, dust, snow), counting numbers, scroll reveals |
| `stats.js` | Data sheet blocks: rating bars, game values, technical data with the armor table |
| `vendor/` | three.js, smol-toml and the fonts with their licenses |

The camouflage switch changes the whole page: camouflage of every model, accent color,
sky and weather. Visitors who ask for reduced motion get a still page.

## Data sheet blocks (prompt 065)

- **Kennzahlen im Spiel:** five bars 0–100 (speed, protection, against vehicles,
  infantry, structures); the list can be sorted by each (the choice is kept in
  `localStorage` with army and biome).
- **Technische Daten:** only for vehicles with an entry in `data/vehicles/`. Groups as
  in the database; its armor table (every assembly and face, thickness and angle)
  replaces the small armor diagram. A value the catalog also holds is shown once, in the
  catalog block; if the database differs by more than 5 %, its row appears with the
  catalog value next to it (the reason is in the entry's note). Values with an original
  text show both. German labels are `FIELD_NAMES` in `stats.js`; a field without a label
  appears under its key.
- **Spielwerte:** the groups of the unit's card, real and game values side by side.

### `stats.json`

`{ "reference": {...}, "units": { "<id>": card } }`, integers only, unit in the key
(`_x10` = tenths; `game` = what the simulation computes with, `real` = the historical
value behind it). `reference` holds the comparison distances, weapons, targets and
objects. A card has, where it applies: `ratings`, `movement`, `protection` (with
`versus`: per comparison weapon the real distance up to which it penetrates each face,
`-1` = never), `weapons` (with `penetration_mm` at the reference distances),
`indirect`, `against_vehicles` (with `targets`), `against_infantry`,
`against_structures` (with `objects`), `other`, `object`. Where every value comes from:
`docs/ARCHITECTURE.md`, "Unit stats".

## Legal texts

"Impressum" and "Datenschutz" are two small links in the footer and under every data
sheet; both texts stand once, in the `<dialog id="legal">` of `index.html`. The privacy
text describes what the page does (no cookies, no tracking, no requests to other
servers, `localStorage` for army, biome and sorting). It still has the placeholder
`[Hosting-Anbieter, Anschrift]` / `[Speicherdauer der Logs]`: fill it in before
publishing. It is no legal advice.

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
- The page asks no other server: three.js 0.170.0 (MIT), smol-toml 1.3.1 (BSD-3-Clause)
  and the fonts (Big Shoulders, Big Shoulders Stencil, IBM Plex Sans, IBM Plex Mono; SIL
  OFL 1.1, Latin and Latin Extended) lie under `vendor/` with their license texts. To
  update one, replace its files there and the version in this line.
- No formula of the simulation lives in this folder. `stats.js` only formats what
  `stats.json` holds; a new game value is added in `crates/sim/src/unit_stats.rs`.
- Fields the page does not know are listed under their catalog name, so new
  catalog fields show up without a change here.
