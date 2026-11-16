# Audio library: free recordings for real sounds

Research result of prompt 061 (November 2026). It lists free recordings that may be
used in a commercial game, as candidates to replace or layer the synthesized classes in
`godot/assets/audio/sounds.json` (prompt 053, "real recordings"). Rows 1, 14, 16, 17,
21, 22, 23, 24, 28 and 34 are in the game since 2026-11-16 (the Sounds table in
SOURCES.md names the exact files); every further file goes through the rules in
[SOURCES.md](SOURCES.md#sounds) (a row there before the commit, downloads agreed first).

How it was checked: every row of the sound table comes from fetching the sound's own
page (Freesound file page, Commons file page); license, format, sample rate, size and
description were read there, not from search listings. Spot checks repeated for
Walking.With.Microphones 802675, Fight2FlyPhoto 142902, Lubini 338242, the Commons
Sherman files and the Sonniss license. OpenGameArt and ZapSplat blocked automated
access (HTTP 403); their entries are marked **unverified** and must be checked in a
browser. Values a page does not state are "n/s".

Licenses: CC0 = https://creativecommons.org/publicdomain/zero/1.0/ ,
CC BY 3.0 = https://creativecommons.org/licenses/by/3.0/ ,
CC BY 4.0 = https://creativecommons.org/licenses/by/4.0/ ,
CC BY-SA 3.0 = https://creativecommons.org/licenses/by-sa/3.0/ ,
Sonniss GDC = https://sonniss.com/gdc-bundle-license/ .

## Key decisions

| Topic | Decision |
|---|---|
| Repo-safe licenses | CC0 and public domain: commit freely. CC BY / CC BY 4.0: commit, and add the author to the credits screen (prompt 044). CC BY-SA: commit; the edited sound file itself stays CC BY-SA (code is not affected); credit needed. |
| Sonniss GDC bundles | Commercial use, no attribution, but the license forbids supplying the sounds "as sound effects" to others, modified or not, and forbids AI training. Fine for the shipped game and a private repository; **not** for a repository that is or may become public. Keep them out of git (a local `assets-src/audio-private/`, gitignored) until the repository's visibility is decided. |
| Pixabay, Mixkit, 99Sounds, ZapSplat | Commercial use allowed, but standalone redistribution forbidden (and ZapSplat's free tier needs credit). Same handling as Sonniss; low priority, origin of uploads uncertain. |
| Excluded on purpose | CC BY-NC / CC BY-ND (e.g. most Fight2FlyPhoto P-51 and Bf 109 taxi files, smill.and.welson, HDVideoGuy M1 Carbine, RavenWolfProds); craigsmith "Vintage/SSE" packs on Freesound (labelled CC0, but transfers of 1930s–60s Hollywood and TV studio effects: film content, unclear rights chain); morganpurkis "MG42 SFX" (edited game sound); "Stuka siren" uploads (look like film rips); OpenGameArt "Tiny Naval Battle Sounds Set" (provenance complaint). |
| Historical archive audio | WW2-era recordings are mostly still protected (US: until about 2040–2046 unless a US government work). Speeches and broadcasts are excluded anyway (no ideology). NARA, LoC and archive.org need a rights check per item; none passed. Use modern recordings of surviving hardware instead. |
| Modern government footage (DVIDS, qubodup's US Army extracts) | Generally public domain as US government works, but DVIDS warns about third-party content and the source videos are often gone. Usable with a per-clip note; modern weapons, video-quality audio. |

## Best individual sounds

Sorted by quality and authenticity, then fit, license safety and availability.
"Attr." = attribution needed. All are available as direct downloads from the page
(Freesound needs a free login).

| # | Sound | Page | Author / origin | License | Comm. | Attr. | Format | Rate / bit | Size / length | Quality | Use (class) |
|---|---|---|---|---|---|---|---|---|---|---|---|
| 1 | Mauser 98k Shooting | [802675](https://freesound.org/people/Walking.With.Microphones/sounds/802675/) | Walking.With.Microphones, K98k blanks at a reenactment, NT5 ORTF + NTG3 | CC0 | Yes | No | WAV st | 48 / 24 | 818 KB / 2.1 s | Excellent, real weapon | `shot_rifle` (de) |
| 2 | Mauser 98k – bolt-action mechanism | [802673](https://freesound.org/people/Walking.With.Microphones/sounds/802673/) | same session | CC0 | Yes | No | WAV | 48 / 24 | 820 KB / 2.1 s | Excellent | reload, bolt |
| 3 | Gun salute with blanks (rifles, revolvers, MGs) | [802672](https://freesound.org/people/Walking.With.Microphones/sounds/802672/) | same recordist, reenactment | CC0 | Yes | No | WAV | 48 / 24 | 9.6 MB / 34.7 s | Very good; models not named | volleys, `shot_mg`, distant fire |
| 4 | Spitfires and Bf-109 Flyby | [142902](https://freesound.org/people/Fight2FlyPhoto/sounds/142902/) | Fight2FlyPhoto, airshow: Bf 109E-3 (DB 601) + Spitfire Vc/IXe | CC BY 3.0 | Yes | Yes | WAV st | 44.1 / 16 | 3.0 MB / 18 s | Very good, real aircraft | aircraft flyby (de/uk) |
| 5 | Bf-109 – Daimler-Benz Run-Up | [142898](https://freesound.org/people/Fight2FlyPhoto/sounds/142898/) | Fight2FlyPhoto, Bf 109E-3 close | CC BY 3.0 | Yes | Yes | WAV | 44.1 / 16 | 12.4 MB / 1:13 | Very good | aircraft engine (de) |
| 6 | Spitfire and B-17 Taxi | [142897](https://freesound.org/people/Fight2FlyPhoto/sounds/142897/) | Fight2FlyPhoto, B-17G + 2 Spitfires | CC BY 3.0 | Yes | Yes | WAV st | 44.1 / 16 | 8.0 MB / 47 s | Very good | bomber engine, airfield |
| 7 | Spitfire 2 passes Military od | [170477](https://freesound.org/people/Cheeseheadburger/sounds/170477/) | Cheeseheadburger, Sound Devices 744 | CC BY 4.0 | Yes | Yes | WAV st | 96 / 24 | 26.6 MB / 48 s | Best technical quality; 1st pass has chatter | fighter flyby |
| 8 | AEROMil Croxall Lakes, Spitfire passing by | [827214](https://freesound.org/people/JW_Audio/sounds/827214/) | JW_Audio, cleaned in RX11 | CC0 | Yes | No | WAV st | 96 / 32 | 108 MB / 2:28 | Very good, distant birds | distant flyby |
| 9 | spitfire close 1 | [160242](https://freesound.org/people/beerbelly38/sounds/160242/) | beerbelly38 | CC0 | Yes | No | WAV st | 44.1 / 24 | 11.2 MB / 45 s | Good, some birds | fighter flyby (CC0 option) |
| 10 | Sherman tank, starten en rijden | [Commons](https://commons.wikimedia.org/wiki/File:Sherman_tank,_starten_en_rijden_-_SoundCloud_-_Beeld_en_Geluid.ogg) | Beeld en Geluid (NL); 6 more Sherman files in the same series (driving, idle/stop, column, high speed) | CC BY-SA 3.0 | Yes | Yes | OGG (lossy, 586 kbps) | n/s | 2.45 MB / 35 s | Only real Sherman found; lossy; Commons tags it "to be checked" | `engine_track_medium` (us), start |
| 11 | t26_moving_01 | [329800](https://freesound.org/people/greatmganga/sounds/329800/) | greatmganga, real T-26, Parola tank museum | CC0 | Yes | No | WAV st | 44.1 / 16 | 2.4 MB / 14 s | Authentic, camcorder quality | `engine_track_light` (su) |
| 12 | Tanks on terrain, tracks creaking | [386661](https://freesound.org/people/YleArkisto/sounds/386661/) | YleArkisto (Finnish broadcaster), Nagra 1984 | CC BY 4.0 | Yes | Yes (Yle) | WAV st | 48 / 24 | 26.9 MB / 1:37 | Professional; model n/s | `engine_track_heavy`, tracks |
| 13 | Tank passing by and reversing on sand | [386590](https://freesound.org/people/YleArkisto/sounds/386590/) | YleArkisto, DAT 1999 | CC BY 4.0 | Yes | Yes | WAV st | 48 / 16 | 15.5 MB / 1:25 | Professional; likely post-war tank | `tracks`, pass-by |
| 14 | MG 42 (Solo) WW2 | [338242](https://freesound.org/people/Lubini/sounds/338242/) | Lubini, "real MG 42 + few additional layers" | CC BY 4.0 | Yes | Yes | WAV st | 48 / 24 | 1.9 MB / 7 s | Good; commenter: rate of fire too low | `shot_mg` (de) |
| 15 | Heavy machine guns around (background) WW2 | [338241](https://freesound.org/people/Lubini/sounds/338241/) | Lubini, documentary shoot | CC BY 4.0 | Yes | Yes | WAV st | 48 / 24 | 4.2 MB / 15 s | Very useful at RTS distance | distant MG, ambience |
| 16 | mp 38 schmeisser (variations) WW2 | [338046](https://freesound.org/people/Lubini/sounds/338046/) | Lubini, documentary shoot | CC BY 4.0 | Yes | Yes | WAV | 48 / 24 | 2.1 MB / 7.7 s | Good; MP38 ≈ MP40 | SMG (de) |
| 17 | PPSh-41 – Soviet submachine gun | [338323](https://freesound.org/people/Lubini/sounds/338323/) | Lubini, Carpathians | CC BY 4.0 | Yes | Yes | WAV | 48 / 24 | 2.3 MB / 8.3 s | Good | SMG (su) |
| 18 | Battle in the forest – Mosin vs Mauser | [338240](https://freesound.org/people/Lubini/sounds/338240/) | Lubini, near Lviv | CC BY 4.0 | Yes | Yes | WAV st | 48 / 24 | 3.6 MB / 13 s | Good | rifle exchange, battle bed |
| 19 | Bolt Action Rifle Reload | [508747](https://freesound.org/people/AugustSandberg/sounds/508747/) | AugustSandberg, real K98, indoors | CC0 | Yes | No | WAV st | 96 / 24 | 14.1 MB / 26 s | Very good | reload |
| 20 | mortar line | [240567](https://freesound.org/people/greenbastardd/sounds/240567/) | greenbastardd, Canadian artillery exercise | CC0 | Yes | No | WAV st | 44.1 / 16 | 16.7 MB / 1:39 | Authentic; cut out radio/voices | `shot_mortar` |
| 21 | GUNArtl 69th Field Artillery Gun 01 | [702245](https://freesound.org/people/InMotionAudio/sounds/702245/) | InMotionAudio; origin n/s | CC0 | Yes | No | WAV st | 96 / 24 | 1.2 MB / 2.1 s | Best technical gun shot | `shot_howitzer`, `shot_cannon_large` |
| 22 | Power plant chimney detonation collapse | [768369](https://freesound.org/people/julujanus/sounds/768369/) | julujanus, real demolition, Zoom H4n Pro | CC0 | Yes | No | WAV st | 44.1 / 32 | 4.5 MB / 13 s | Very good | `building_collapse`, large explosion |
| 23 | Artillery Shots | [182822](https://freesound.org/people/qubodup/sounds/182822/) | qubodup, from 4 US government videos | CC0 | Yes | No | FLAC mono | 48 / 16 | 1.5 MB / 41 s | Real, modern guns, video audio | `shot_howitzer` variants |
| 24 | Mortar Shots | [184382](https://freesound.org/people/qubodup/sounds/184382/) | qubodup, US government video | CC0 | Yes | No | FLAC st | 48 / 16 | 611 KB / 12 s | Real, video audio | `shot_mortar` |
| 25 | EOD Explosion | [855798](https://freesound.org/people/qubodup/sounds/855798/) | qubodup, DVIDS video | CC0 | Yes | No | FLAC mono | 48 / 16 | 77 KB / 1.7 s | Real detonation, de-clipped | `hit_shell`, mine |
| 26 | distant explosions | [320788](https://freesound.org/people/Kostrava/sounds/320788/) | Kostrava; origin n/s | CC0 | Yes | No | WAV st | 44.1 / 16 | 10.8 MB / 1:04 | Good bed | distant artillery ambience |
| 27 | small explosion in city far away | [613560](https://freesound.org/people/Garuda1982/sounds/613560/) | Garuda1982, Leipzig (probably fireworks) | CC0 | Yes | No | WAV st | 192 / 16 | 11.3 MB / 15 s | Real urban echo tail | explosion tails |
| 28 | Big falling debris (crash) | [703247](https://freesound.org/people/xkeril/sounds/703247/) | xkeril, Zoom H5n | CC0 | Yes | No | WAV st | 48 / 16 | 876 KB / 4.7 s | Good | debris layer |
| 29 | Concrete Breaks Several Denoised | [843339](https://freesound.org/people/loganzsound/sounds/843339/) | loganzsound | CC0 | Yes | No | WAV st | 44.1 / 24 | 2.2 MB / 8.8 s | Clean | `building_damage`, concrete |
| 30 | Foley Impact Metal Long Mono | [569555](https://freesound.org/people/Nox_Sound/sounds/569555/) | Nox_Sound, 15 hits | CC0 | Yes | No | WAV mono | 48 / 24 | 5.5 MB / 40 s | Clean foley | `hit_bullet_armor`, `hit_penetration` layer |
| 31 | Heavy rain and thunderstorm, Baldham | [839570](https://freesound.org/people/danfloyd/sounds/839570/) | danfloyd, Tascam DR-05 | CC0 | Yes | No | WAV st | 44.1 / 16 | 33.9 MB / 3:21 | Good, compressed | weather (prompt 040) |
| 32 | Summer forest winds 18 July 2019 | [477187](https://freesound.org/people/doxent/sounds/477187/) | doxent, Zoom H4n | CC0 | Yes | No | WAV st | 48 / 16 | 225 MB / 20:29 | Good; cut a loop, never commit the whole file | forest ambience |
| 33 | Open field winds summer ambience | [523374](https://freesound.org/people/nickmaysoundmusic/sounds/523374/) | nickmaysoundmusic | CC0 | Yes | No | WAV st | 48 / 24 | 7.7 MB / 28 s | Good; distant traffic to avoid | field ambience |
| 34 | fire small campfire crackling | [637523](https://freesound.org/people/kyles/sounds/637523/) | kyles | CC0 | Yes | No | FLAC mono | 48 / 24 | 3.4 MB / 35 s | Clean, professional | `fire` loop |
| 35 | truck large diesel / APC pass by | [405086](https://freesound.org/people/kyles/sounds/405086/) | kyles, Colombia 2016 | CC0 | Yes | No | FLAC mono | 48 / 24 | 7.6 MB / 1:31 | Clean, modern vehicle | `engine_wheel` (truck stand-in) |
| 36 | Machine gun firing (blanks, studio shoot) | [205901](https://freesound.org/people/Cymeon/sounds/205901/) | Cymeon, film set recording (not taken from a film) | CC0 | Yes | No | WAV | 44.1 / 24 | 9.6 MB / 38 s | Good; fan noise; model n/s | `shot_mg` generic |
| 37 | Distant Gunfire | [842326](https://freesound.org/people/iainmccurdy/sounds/842326/) | iainmccurdy, hunt in Provence | CC0 | Yes | No | WAV | 96 / 24 | 4.2 MB / 7.6 s | Good, real distance | distant rifle layer |
| 38 | Bouncing Shell Casings (various sizes) | [318964](https://freesound.org/people/GryffDavid/sounds/318964/) | GryffDavid | CC0 | Yes | No | WAV | 44.1 / 16 | 6.9 MB / 41 s | OK, USB mic | casings |
| 39 | Hammering Wood | [204946](https://freesound.org/people/CapsLok/sounds/204946/) | CapsLok, Zoom H6 | CC0 | Yes | No | WAV st | 44.1 / 24 | 3.4 MB / 13 s | Clean | construction |
| 40 | Ratchet Wrench Fast Multiple | [591529](https://freesound.org/people/Rudmer_Rotteveel/sounds/591529/) | Rudmer_Rotteveel | CC0 | Yes | No | WAV st | 44.1 / 16 | 874 KB / 5 s | Clean | repair |
| 41 | OLD ARMY RADIO STATIC | [203255](https://freesound.org/people/metrostock99/sounds/203255/) | metrostock99, synth | CC0 | Yes | No | WAV st | 44.1 / 16 | 5.2 MB / 31 s | Good | radio bed under voice lines |
| 42 | Lots of Morse Code | [204968](https://freesound.org/people/Buss1000/sounds/204968/) | Buss1000, 40 m amateur band | CC0 | Yes | No | WAV st | 44.1 / 16 | 23.6 MB / 2:20 | OK, some buzz | HQ radio ambience |

Also checked and usable as second choice: Churchill engine at Tankfest
([702457](https://freesound.org/people/AdamHarvester/sounds/702457/), CC0, 32 kHz mono,
announcer in the background), monosfera tank_engine
([572294](https://freesound.org/people/monosfera/sounds/572294/), CC0, 48/16, 1:19,
model n/s), P-47 and P-40 start-ups by Fight2FlyPhoto
([142900](https://freesound.org/people/Fight2FlyPhoto/sounds/142900/),
[142899](https://freesound.org/people/Fight2FlyPhoto/sounds/142899/), CC BY 3.0),
M1 Garand clip ping and empty reload by serøutōnin--deprivəd
([725398](https://freesound.org/people/ser%C3%B8ut%C5%8Dnin--depriv%C9%99d/sounds/725398/),
[725399](https://freesound.org/people/ser%C3%B8ut%C5%8Dnin--depriv%C9%99d/sounds/725399/), CC0,
real rifle not confirmed), Mosin bolt cycle
([370345](https://freesound.org/people/Zott820/sounds/370345/), CC0, hobby quality),
welding ([454351](https://freesound.org/people/kyles/sounds/454351/), CC0), metal
workshop ambience ([189817](https://freesound.org/people/iankath/sounds/189817/), CC0,
MP3), crosscut saw ([841708](https://freesound.org/people/RdmnPrd/sounds/841708/), CC0),
turret traverse stand-in ([607311](https://freesound.org/people/lorefold/sounds/607311/),
CC0, designed from a motor), Firefight distant bed
([404334](https://freesound.org/people/TheBuilder15/sounds/404334/), CC0, 20 s), whistle
for `shell_incoming` ([398255](https://freesound.org/people/morganpurkis/sounds/398255/),
CC0, synthesized; the same uploader's MG42 is a game rip, so check before use).

## Ten best free packs

| # | Pack | Link | License | Repo-safe | Covers |
|---|---|---|---|---|---|
| 1 | Sonniss GDC 2018 (29.3 GB) | [gameaudiogdc](https://sonniss.com/gameaudiogdc/) | Sonniss GDC v2.0: commercial, no credit, no redistribution, no AI training | No (private only) | Pole Position: Sherman M4A1/M4A3, Stuart M5A1, Chaffee M24, M18 Hellcat, M16 halftrack, Spitfire Mk XIV, C-46; Burning House; gun acoustics; distant blasts; fire, wind, thunder, rain; tools; UI |
| 2 | Sonniss GDC 2020 (48.2 GB) | same | same | No | Pole Position: MG 42, MP 40, K98k, Springfield, Thompson, PPSh-41, P38, M1911, M10, TKS; Super Thump WW2 weapons; FLYSOUND T-44; tank explosions; radio transmission; casings; debris; British soldier voices; UI |
| 3 | Sonniss GDC 2016 (12.9 GB) | same | same | No | Pole Position: T-34-85, F6F Hellcat, BAR, M1919A4, Thompson, M3 grease gun, gun foley; overhead aircraft; soldier footsteps; wind; construction |
| 4 | Sonniss GDC 2019 (26.2 GB) | same | same | No | Airborne Sound battlefield howitzers, attack aircraft; Spitfire Mk IX; military radio voices; battle crowd; debris; storm; tools; UI |
| 5 | Sonniss GDC 2017 (19.2 GB) | same | same | No | Pole Position Warfare Library, F8F Bearcat, Corbitt cargo truck; designed explosions; signal communication; rain/thunder; wood debris; UI |
| 6 | Lubini WW2 documentary recordings | [Freesound](https://freesound.org/people/Lubini/sounds/) | CC BY 4.0 | Yes (credit) | MG42, MP38, PPSh-41, TT-33, P38, rifle exchanges, distant heavy MG |
| 7 | Fight2FlyPhoto warbirds (BY files only) | [Freesound](https://freesound.org/people/Fight2FlyPhoto/sounds/) | per file: CC BY 3.0 or CC BY-NC (check each) | Yes for BY files (credit) | Bf 109, Spitfire, B-17, P-47, P-40 |
| 8 | Beeld en Geluid on Commons | [category](https://commons.wikimedia.org/wiki/Category:Sounds_from_Geluid_van_Nederland) | CC BY-SA 3.0 | Yes (credit, share-alike) | Sherman (start, idle, drive, column), Centurion, DC-3, Ford trucks |
| 9 | qubodup "military sounds" | [Freesound pack 4366](https://freesound.org/people/qubodup/packs/4366/) | CC0 (US government sources) | Yes, with per-clip origin note | Artillery, mortars, EOD blasts (modern) |
| 10 | Kenney Interface Sounds / UI Audio / Impact Sounds | [kenney.nl](https://kenney.nl/assets/interface-sounds) | CC0 | Yes | UI clicks, select, confirm, impacts |

Worth a manual look (unverified, OpenGameArt blocked automated access): "The Free
Firearm Sound Library" (CC0, 194 MB), "150 Combat Sound Effects" (per-file licenses in
LICENSES.txt), qubodup "Sci-Fi RTS War Unit Sounds" (CC0, unit-response templates).
BigSoundBank (CC0/WTFPL, redistribution explicitly allowed) was not searched for
military content. Sonniss GDC 2026 lists 344 Audio "Historical Weapons Vol. 2" and
Sonic Bat "Vintage Radio" (contents not checked). 2021, 2022 and 2024 track lists were
not available.

## Gaps

No free recording with a safe license was found for:

- **German and Soviet armor:** Tiger I/II, Panther, Panzer IV, T-34 (only the T-34-85 and
  T-44 in Sonniss). Opel Blitz, Willys, CCKW, Sd.Kfz. 251 (only Sonniss M16 halftrack and
  modern trucks).
- **Turret traverse, gun elevation, tank braking**: only designed stand-ins.
- **Ju 87 engine and dive siren, Fw 190, P-51** (P-51 recordings found are NC), dives.
- **MG34, Bren, DP-28, M1 Carbine shot, clean MP40**; magazine and belt changes.
- **Period guns** (10.5 cm leFH, 25-pounder) and a good **shell whistle**.
- **Complete masonry house collapse**, field telephone, military alert UI.
- **Wartime radio traffic** with clear rights.

For these the synthesis in `tools/audio/` stays, layered with the recordings above
(e.g. a Stuka siren from a synthesized siren, distant versions by low-pass and the
existing convolution tail), or own recordings (voice lines through a band-pass over the
CC0 static). Paid libraries exist for most gaps (Pole Position, BOOM WW2 tanks and
firearms) and are a later budget decision.

## Library layout

The game keeps one flat folder per sound class (`godot/assets/audio/<class>/`), because
`sounds.json` and the voice pools work per class. Long raw recordings and their edits
live in a source library sorted by subject, from which the class files are cut:

```
assets-src/audio/                      # repo-safe: CC0, CC BY, CC BY-SA, own work
  _raw/                                # untouched downloads (Git LFS), named <source>_<id>_<slug>.<ext>
  vehicles/tanks/<vehicle>/            # start, idle, drive, accel, brake, turn, tracks, turret
  vehicles/halftracks/  vehicles/trucks/  vehicles/jeeps/
  weapons/rifles/  weapons/smg/  weapons/machineguns/  weapons/pistols/
  weapons/foley/                       # reload, bolt, magazine, casings
  artillery/guns/  artillery/howitzers/  artillery/mortars/  artillery/incoming/
  explosions/small/  explosions/medium/  explosions/large/  explosions/distant/  explosions/tails/
  impacts/metal/  impacts/earth/  impacts/concrete/  impacts/wood/  impacts/water/
  destruction/buildings/  destruction/debris/  destruction/trees/
  aircraft/fighters/  aircraft/bombers/  aircraft/dive/
  ambience/battle_distant/  ambience/weather/  ambience/nature/  ambience/fire/
  radio/                               # static, morse, own voice lines
  buildings/construction/  buildings/repair/  buildings/production/
  ui/
  <class>/                             # finished files that replace a synthesized class (SOURCES.md)
assets-src/audio-private/              # gitignored: Sonniss and other no-redistribution packs
```

Rules for the library:

1. Every file in `_raw/` gets a row in `docs/SOURCES.md` (page URL, author, license,
   date downloaded) before it is committed; CC BY/BY-SA authors also go to the credits.
2. Edits keep the source id in the name (`k98k_shot_wwm802675_01.wav`), so the license
   can always be traced.
3. Finished game files: WAV 48 kHz / 16 bit (one-shots mono, loops and ambience stereo),
   peak-normalized per class; long ambience may stay OGG.
4. No full 20-minute field recordings in git: cut the parts actually used.
