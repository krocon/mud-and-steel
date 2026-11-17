# Sound page

A local page to listen to every sound of the game (prompt 083). Each row is one sound
class, weapon recording, ambience layer or music mood, with a remark on when the game
plays it, one button per file (length, origin, whether the game plays it) and the
manifest values of the class. Start it with `just sounds-web`: it serves the repository
root on `http://localhost:8138/web/sounds/` (`--port N`, `--no-open`) next to
`just units-web` (8137). `…/web/sounds/#<name>` jumps to a row (`#shot_mg`,
`#weapon/mg_42`, `#ambience/rain`, `#music/combat`).

The page plays the files as they are, or with the level (`volume_db`) and random pitch
(`pitch_spread`) of their class ("wie im Spiel"). Distance, low-pass, voice budget, fog
of war, reverb and delay are left out: `just sound-gallery` and the game play those.

## Files

| File | Holds |
|---|---|
| `index.html`, `style.css`, `app.js` | The page (German texts, fonts from `web/units/vendor/fonts/`), no build step, no dependencies; playback with Web Audio |
| `usage.toml` | The remarks, by hand, in German |
| `index.json` | Written by `tools/sounds_web.py` on every start (gitignored): every file with its path, length, origin and whether a private file displaces it, the manifest values and the remarks |

The page loads `index.json` and the audio files straight from `godot/assets/audio/`.
Origin comes from the file name: `<class>_NN` synthesized, `ext_*` a free recording,
`priv_*` from the private sound repository. A folder with a `priv_*` file plays only
those in the game (`sound_library.gd`; not for ambience and music); the page marks the
other files "im Spiel verdrängt" and still plays them.

## Adding a remark

`usage.toml` has one table per kind of folder: `[class]` (manifest classes, folder
`godot/assets/audio/<class>/`), `[weapon]` (`weapon/<slug>/`), `[ambience]`
(`ambience/<layer>/`), `[music]` (mood of `music/<mood>_NN`) and `[voice]` (callout of
`voice/<faction>/<callout>/`). One sentence of at most about 120 characters that names
the occasion, not the sound, taken from the code (`godot/scenes/audio/`, `air_view.gd`,
`artillery_view.gd`, `destruction_view.gd`, `crater_view.gd`), with the thresholds that
choose the sound ("Kanonen ab 75 mm"). A weapon remark names the catalog weapon and the
class it replaces. A class the code never plays reads "Derzeit ungenutzt."

The tool reads the file without a TOML library (Python 3.10 has none): only `[table]`
headers, comments and `key = "text"` lines.

`just sounds-web --check` (in `just check` and CI) fails when a folder with audio files
has no remark, a remark has no folder, or a manifest class has no folder or no page
section (`CLASS_SECTIONS` in `tools/sounds_web.py`). It reads only folder and file
names, so it works without Git LFS and without the private repository; `priv_*` files
do not count. Callout remarks may exist before any voice file ships.

## Why the page stays local

The private `priv_*` sounds may ship inside the game but must not be passed on as single
files (`docs/AUDIO_LIBRARY.md`). A copy of the audio files under `web/` would be a step
towards uploading them, so unlike the unit page this page has no `sync` step and is
never uploaded: it only runs from the repository through `just sounds-web`.
