# Sources and legal rules

Every value in the unit catalog (`data/catalog/`) names its source in the entry's
`source` list, together with what the source covers. Values that sources disagree on,
or that had to be estimated, are listed in the entry's `uncertain` field and
explained in `note`. They are marked, not guessed. The per-entry sources appear in
the generated [UNIT_CATALOG.md](UNIT_CATALOG.md).

## Preferred sources

In this order:

1. **Primary documents in the public domain**: US War Department technical manuals
   (TM 9-…), field manuals (FM …), tables of organization (T/O&E) and TM-E 30-451
   *Handbook on German Military Forces* (1945). As works of the US government they are
   free to use.
2. **Museum data**: The Tank Museum Bovington, Kubinka, US Army Ordnance Museum,
   Smithsonian National Air and Space Museum, RAF Museum, Deutsches Panzermuseum Munster.
3. **Standard literature**: Jentz/Doyle *Panzer Tracts*, Chamberlain/Doyle
   *Encyclopedia of German Tanks of World War Two*, Hunnicutt (*Sherman*, *Pershing*,
   *Half-Track*), Zaloga (Osprey and others), Gander/Chamberlain *Small Arms,
   Artillery and Special Weapons of the Third Reich*.
4. **Secondary web sources** such as Wikipedia, cited with the article URL. Values
   from here should be cross-checked where they matter for gameplay.

## Vehicle database

`data/vehicles/*.json` (prompt 064, schema in `data/vehicles/SCHEMA.md`) holds the
technical data of the armored vehicles from the reference folder
`docs/.vorlagen/fahrzeuge/`, loaded with `just vehicle-refs` from onwar.com (Ralph
Zuljan, "Tanks of World War II"). Each entry names its page (`source.url`), the day it
was fetched and the literature codes the page cites. onwar.com is a secondary web
source (rank 4): the database is a reference work for display, and where it disagrees
with the catalog, the catalog's own sources win; the field is then listed in the
catalog entry's `uncertain` and the difference named in its `note`
(`just vehicle-db-check` enforces that). Only facts are taken over: no drawings, no
photos, no file names of them, no text except the short notes of the data tables.

## What may be used how

- **Facts and dimensions** (lengths, masses, armor, speeds, calibers) are not
  copyrightable and may be used freely, with the source named.
- **Drawings, plans and photos by third parties** may be used only as modeling
  references on the developer's machine. They are never committed to the repository
  and never shipped.
- **Nothing from other games**: no models, textures, sounds, names or data from other
  games, and no game wikis as sources (War Thunder, World of Tanks, Company of Heroes,
  Steel Division and the like). Their values are often balanced, not historical, and
  their assets are protected.
- Own assets are built by us or are CC0.
- Sounds: see [Sounds](#sounds).

## Markings and symbols

- National markings: **Balkenkreuz** for `de`, **red star** for `su`, **white star**
  for `us`.
- **No swastika and no SS runes**, anywhere (models, textures, UI, names). Showing
  them is restricted in Germany (§ 86a StGB) and conflicts with USK rating and Steam
  distribution. Unit selection avoids SS formations.
- Tactical symbols, unit numbers and turret numbers are allowed.
- **Medics without the Red Cross.** The red cross is a protected emblem under the
  Geneva Conventions and its use is restricted. Medics are marked by a plain white
  armband and a medical bag instead.

## Fire, towns and dumps (prompt 041)

- **Fire values** (`data/fire.toml`: chances, reaches, burn times, the
  occupant damage, the dump blasts) are game rules, not historical data, and
  marked provisional; the catalog has no flammability.
- **New buildings** (town house, town hall, warehouse, log house) are
  generic types with judgement dimensions; their catalog entries cite
  general references (Wikipedia: Terraced house, Town hall, Warehouse, Log
  house; storey heights) and list every dimension as `uncertain`. No real
  building is reproduced.
- **Dumps**: drum size from the 200 l steel drum (Wikipedia: Drum
  (container)); field storage of ammunition in small dispersed stacks on
  dunnage after FM 9-6 *Ammunition Supply* (US War Department, public
  domain). No markings, stencils or national symbols on crates and drums.
- **Maps**: generated maps carry no real place names; signposts (open, not
  built yet) get blank boards.

## Desert village (prompt 059)

- **Houses, walls, mosque, police post** are generic types of North
  African villages 1940–43 (Libya, Tunisia, Egypt) with judgement
  dimensions; the catalog cites general references (Wikipedia: Adobe,
  Mudbrick, Courtyard house, Flat roof, Barrel vault, Djerba, Arcade,
  Souq, Mosque, Minaret, Italian Libya) and lists every dimension as
  `uncertain`. Facts from literature and period photographs serve as
  reference only; no real building is reproduced and nothing comes from
  other games.
- **No writing, symbols or insignia**: the mosque has no inscriptions,
  ornaments with meaning or crescent; the police post has no insignia,
  flags or fasces. Nothing modern (no plastic tanks, satellite dishes, air
  conditioners, power lines).
- **Rules** (hit points, garrison capacities, cover) are game rules, marked
  provisional. The map carries no real place names.

## Grown rock (prompt 074)

- **Slabs, outcrops, ridge pieces** are generic natural forms with
  judgement dimensions; the catalog cites general geology references
  (Wikipedia: Outcrop, Limestone pavement, Tor (rock formation), Joint
  (geology), Ridge, Crag) like `env_rock`, and lists every dimension as
  `uncertain`. Sizes were estimated from the scale of houses, poles and
  vehicles in reference screenshots of another game; those pictures served
  for form and effect only: no cut-outs, textures or values from them, all
  models from the kit, all patterns from the shader.
- **Rules** (blocking, sight heights, box margins) are game rules.

## Apartment blocks (prompt 078)

- **Blocks** are generic types with judgement dimensions: interwar
  concrete-frame apartment blocks with flat roofs, roof terraces and
  balconies of Mediterranean and North African cities (Wikipedia: White
  City (Tel Aviv), Flat roof), and a European tenement with a carriage
  passage to its courtyard (de.wikipedia: Mietskaserne, Berliner
  Traufhöhe: eaves at most 22 m). Storey heights from Wikipedia: Storey
  (about 3.0 m, 3.1 m for apartments). Pavlov's House (Wikipedia) backs
  the garrison cap (a four-storey block held by about 25-30 men). Every
  dimension is `uncertain`; no real building is reproduced.
- **Look:** two screenshots of a present-day desert city from another
  strategy game served as a reference for form, size and effect only: no
  cut-outs, textures or values from them; nothing after 1940 (no air
  conditioners, antennas, dishes). All patterns come from the shader.
- **Rules** (hit points, capacities, exposure, ruin heights) are game
  rules, marked provisional.

## Rocket salvo effects (prompt 076)

- **M-13 (132 mm, BM-13 "Katyusha")**: reactive powder charge burning time
  0.85 s, length without fuse 1,415 mm, caliber 132 mm, highest speed
  355 m/s, speed leaving the rail 70 m/s
  (https://en.missilery.info/node/13291, a single secondary source, not
  cross-checked). The game's flame burns 1.0 s of game time (0.43 s would
  be the real burn at time factor 2): a judged deviation, see
  docs/ARCHITECTURE.md "Rocket salvo (prompt 076)". The body is drawn at
  1.41 m. The other factions' rockets differ only by their caliber
  (data/strike.toml); no per-faction burn times were looked up.
- **Look** (flare, flame, trail, haze): three stills from a promotional
  video of another strategy game served as a reference for effect and
  sequence only: no cut-outs, textures or values were taken from them;
  sizes and times are judgement or from the source above, textures and
  shapes are generated in shaders.

## Muzzle blast (prompt 077)

- **Flash zones**: the intermediate flash appears about 20-25 calibers
  before the muzzle of large guns, the secondary flash (re-ignited
  propellant gas) beyond it as a large yellow-white fireball (Tony
  DiGiulian, "Muzzle Flash", NavWeaps, History and Technology,
  https://navweaps.com/index_tech/tech-090.php; Wikipedia "Muzzle flash"
  for the phases, no sizes). The game's fire tongue is 40 calibers long (a
  judged middle of the secondary flash, 3 m for 7.5 cm); no duration in
  milliseconds was found in an accessible source, the times are judged so
  the flash is seen for a few frames.
- **Size of the cloud and dust**: a screenshot of another game served as a
  reference for effect and size only (cloud 1.5-2 vehicle lengths, flat
  brown dust); no cut-outs, textures or values from it.
- **Recoil** 0.3-0.6 m for 7.5-12.2 cm guns and the hull's rocking are
  judged orders of magnitude, not looked up per gun.
- **Muzzle velocities** steer the tongue's length; they come from the
  catalog (its own sources) through the simulation's shell speed.

## Infantry camouflage (prompt 042)

The camouflage prints of the soldiers are procedural imitations by period
descriptions; colors are sRGB approximations (`uniform_palettes.json`, `camo`).
No photos or prints are copied.

| Pattern | Facts used | Source |
|---|---|---|
| Splittertarn (Heer) | introduced 1931 (Zeltbahn), smocks and helmet covers from 1942, winter suit with a white reverse; light field-grey/tan base, wood-brown and medium-green polygons, dark-green rain strokes; not used by the Waffen-SS | https://en.wikipedia.org/wiki/Splittertarnmuster |
| Erbsentarn | 1944 pattern, Waffen-SS issue: **not used** (rule above) | https://en.wikipedia.org/wiki/List_of_camouflage_patterns |
| Soviet camouflage suit | "amoeba" pattern overgarment for snipers (and scouts) | https://en.wikipedia.org/wiki/Military_camouflage (section "From the Second World War") |
| M1942 frogskin | reversible: five-color green side (green and brown shapes on pale green), three-color brown side; USMC in the Pacific, US Army in Normandy in summer 1944 (41st Armored Infantry), withdrawn as it was mistaken for Waffen-SS camouflage | https://en.wikipedia.org/wiki/Frog_skin_camouflage, https://en.wikipedia.org/wiki/Military_camouflage (section "Uniforms") |
| Pistols | P 38 216 / 125 mm, TT-33 195 / 116 mm, M1911A1 216 / 127 mm (overall / barrel length) | https://en.wikipedia.org/wiki/Walther_P38, https://en.wikipedia.org/wiki/TT_pistol, https://en.wikipedia.org/wiki/M1911_pistol |

## Sounds

Prompt 029. Every file in `godot/assets/audio/` names its origin here.

| Files | Origin | License |
|---|---|---|
| `godot/assets/audio/<class>/<class>_NN.wav`, every class of `sounds.json` | Synthesized by `tools/audio/synth.py` (`just audio`): noise, sine partials, envelopes, filters and a convolution tail, a recipe per class, seeded per variant. No recordings, samples or presets of anyone else. | Own work, same license as the code |
| Stage-2 classes (prompt 053): `crack_bullet`, `crack_shell` (N-wave pressure signature plus air hiss), `explosion_tail`, `flak_burst`, `engine_aircraft`, `engine_bomber` (propeller blade pass and cylinder buzz, detuned engines beating), `fire_large` | Synthesized by `tools/audio/synth.py` like the classes above; no recordings. | Own work, same license as the code |
| `godot/assets/audio/weapon/<slug>/`, `godot/assets/audio/voice/<faction>/<callout>/` (prompt 053) | **Empty.** Folders for real weapon recordings and battle voices (docs/ASSETS.md); every file gets its own row here before it is committed. | – |
| `godot/assets/audio/music/<mood>_NN.mp3` (prompt 030) | Composed and rendered by `tools/audio/music.py` (`just music`): chord progressions and melodies written for this game (melodies partly from a seeded random walk on the scale), additive instruments (string pad, low strings, a soft piano-like pluck, timpani) and a convolution hall, all numpy; encoded with LAME. No samples, loops, presets, MIDI files or melodies of anyone else; no music from games, films or historical recordings. | Own work, same license as the code |
| `godot/assets/audio/ambience/<layer>/<layer>_NN.wav` (prompt 030) | Synthesized by `tools/audio/music.py`: wind, insects, leaves, water, rain from shaped noise and short tones; bird calls from frequency sweeps; rumble from low-passed noise. No recordings. | Own work, same license as the code |
| `assets-src/audio/weapon/{kar98k,kar98k_mit_zf_39}/wwm802675_01.wav` | Walking.With.Microphones, "Mauser 98k Shooting" (blanks at a reenactment), https://freesound.org/people/Walking.With.Microphones/sounds/802675/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/weapon/{mg_42,mg_34_mg_42,mg_42_on_lafette_42}/lubini338242_0N.wav` (last round of each burst) | Lubini, "MG 42 (Solo) WW2.wav", https://freesound.org/people/Lubini/sounds/338242/ , downloaded 2026-11-16 | CC BY 4.0, credited in `CREDITS.md` |
| `assets-src/audio/weapon/mp_40/lubini338046_0N.wav` (MP 38 as stand-in) | Lubini, "mp 38 schmeisser (Variations) WW2.wav", https://freesound.org/people/Lubini/sounds/338046/ , downloaded 2026-11-16 | CC BY 4.0, credited |
| `assets-src/audio/weapon/ppsh_41/lubini338323_0N.wav` | Lubini, "PPSh-41 - Soviet Submachine Gun.wav", https://freesound.org/people/Lubini/sounds/338323/ , downloaded 2026-11-16 | CC BY 4.0, credited |
| `assets-src/audio/shot_mortar/qubodup184382_0N.wav` | qubodup, "Mortar Shots.flac" (from a US government video), https://freesound.org/people/qubodup/sounds/184382/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/shot_howitzer/qubodup182822_0N.wav` | qubodup, "Artillery Shots.flac" (from US government videos), https://freesound.org/people/qubodup/sounds/182822/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/shot_howitzer/inmotion702245_01.wav` | InMotionAudio, "GUNArtl_69thFieldArtilleryGun01", https://freesound.org/people/InMotionAudio/sounds/702245/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/building_collapse/julujanus768369_01.wav` | julujanus, "Power plant chimney detonation collapse", https://freesound.org/people/julujanus/sounds/768369/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/building_collapse/xkeril703247_01.wav` | xkeril, "Big falling debris (crash)", https://freesound.org/people/xkeril/sounds/703247/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/fire/kyles637523_01.wav` (6 s loop, crossfaded) | kyles, "fire small campfire crackling short +air tone", https://freesound.org/people/kyles/sounds/637523/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/shot_cannon_small/qubodup854183_0N.wav` (one shot, four pitches, synthesized low body) | qubodup, "Autocannon Shot 1" (likely a 25 mm M242, from a US Marine Corps video), https://freesound.org/people/qubodup/sounds/854183/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/shot_cannon_medium/ylearkisto332935_0N.wav` (the five shots of the tape, synthesized crack) | YleArkisto, "Tykki, laukaus / Big gun, cannon, 76 mm, single shots, gunshots" (76 mm field gun, 1958), https://freesound.org/people/YleArkisto/sounds/332935/ , downloaded 2026-11-16 | CC BY 4.0, credited |
| `assets-src/audio/shot_cannon_large/qubodup189344_0N.wav` (first four shots, synthesized low body) | qubodup, "Tanks Shooting.flac" (from a US government video), https://freesound.org/people/qubodup/sounds/189344/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/hit_penetration/lpa134329515_0N.wav` (one hit, three pitches, synthesized bang inside the hull) | LPA134, "Big stereo metal hit" (hits on a lift door), https://freesound.org/people/LPA134/sounds/329515/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/hit_ricochet/cv523403_0N.wav` (lowered pitch) | C-V, "22 caliber with ricochet.wav", https://freesound.org/people/C-V/sounds/523403/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/{explosion_vehicle,hit_shell,hit_shell_small}/modusmogulus737321_0N.wav` (pitched and shortened per class) | modusmogulus, "Pipe Bomb" (designed from the author's own recordings), https://freesound.org/people/modusmogulus/sounds/737321/ , downloaded 2026-11-16 | CC0 |
| debris layer in `assets-src/audio/explosion_vehicle/` | SamsterBirdies, "Scrap metal dropping / crashing", https://freesound.org/people/SamsterBirdies/sounds/587443/ , downloaded 2026-11-16 | CC0 |
| `assets-src/audio/explosion_tail/ylearkisto256296_0N.wav` | YleArkisto, "Räjähdys maastossa / Explosion in the woods little further away, echoing" (1984), https://freesound.org/people/YleArkisto/sounds/256296/ , downloaded 2026-11-16 | CC BY 4.0, credited |
| `assets-src/audio/tracks/ylearkisto386660_01.wav` (8 s loop, high-passed, crossfaded) | YleArkisto, "Panssarivaunut kadulla / Tanks on the street, tracks creaking" (1969), https://freesound.org/people/YleArkisto/sounds/386660/ , downloaded 2026-11-16 | CC BY 4.0, credited |

The recordings above are cut from Freesound's public HQ previews (MP3, 128 kbps)
and converted to 44.1 kHz 16-bit mono; the original WAV/FLAC files need a Freesound
login. Replacing a file with a cut from the original keeps its name and row. The
classes and weapons with files in `assets-src/audio/` play only those recordings
(`tools/audio/synth.py`). The cuts of prompt 069 (cannon shots, hits, explosions,
tracks) are made by `tools/audio/cuts.py` from the downloads in `docs/.sfx/freesound/`
(not in git); the layers it adds are synthesized there.

Files of other origin go to `assets-src/audio/<class>/` (music:
`assets-src/music/<mood>_NN.<ext>`, which replaces the composed piece of
that name) and get a row here before they are committed:

- **CC0 recordings:** link to the file page (e.g. on freesound.org with
  the CC0 mark), author, date downloaded. CC0 or CC BY; attribution licenses
  (CC BY) also get a row in `CREDITS.md`, which the main menu shows.
- **AI-generated sounds** (text-to-sound tools): tool and version, the
  full prompt, date, and the license of the output; it must allow
  commercial use.
- **Never** sounds from other games or game mods, from films, or
  "royalty-free" packs whose license forbids redistribution in source
  form (the repository ships the WAV files).
- Downloads are agreed with the maintainer first.

**Private sounds** (prompt 069): sounds that may ship inside the game but not as single
files (ElevenLabs output, Pixabay, Sonniss) are not in this repository. Release builds
take them from the private repository `mud-and-steel-sfx`, whose own `SOURCES.md` names
every file; there they replace the public sounds of their class (so far, all generated
with ElevenLabs on a paid plan: `shot_mg`, single shots cut from a machine-gun burst,
and the loops of `engine_track_heavy` and `engine_track_light`).

Candidate recordings with checked licenses, the free packs worth using, known gaps
and the source library layout: [AUDIO_LIBRARY.md](AUDIO_LIBRARY.md) (prompt 061).
