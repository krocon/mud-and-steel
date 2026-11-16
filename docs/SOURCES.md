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

Files of other origin go to `assets-src/audio/<class>/` (music:
`assets-src/music/<mood>_NN.<ext>`, which replaces the composed piece of
that name) and get a row here before they are committed:

- **CC0 recordings:** link to the file page (e.g. on freesound.org with
  the CC0 mark), author, date downloaded. Only CC0; attribution licenses
  (CC BY) need a credits screen first.
- **AI-generated sounds** (text-to-sound tools): tool and version, the
  full prompt, date, and the license of the output; it must allow
  commercial use.
- **Never** sounds from other games or game mods, from films, or
  "royalty-free" packs whose license forbids redistribution in source
  form (the repository ships the WAV files).
- Downloads are agreed with the maintainer first.

Candidate recordings with checked licenses, the free packs worth using, known gaps
and the source library layout: [AUDIO_LIBRARY.md](AUDIO_LIBRARY.md) (prompt 061).
