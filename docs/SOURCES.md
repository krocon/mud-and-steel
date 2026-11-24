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

## Mediterranean biome (prompt 120)

- **Plants**: sizes from Wikipedia (Pinus pinea: 12–20 m, crown over 8 m;
  Cupressus sempervirens: up to 35 m, columnar forms under a tenth as wide
  as tall; Olive: rarely over 8–15 m; Maquis shrubland: shrubs up to
  4–5 m; Arbutus unedo 4–7 m; Spartium 2–4 m), the chosen values are
  judgement values below the maxima and marked `uncertain`.
- **Houses, row houses, casale, barn, church with campanile, coastal
  tower, town hall, field-stone and terrace walls** are generic types of
  Italian villages around 1940 with judgement dimensions; the catalog cites
  general references (Wikipedia: Vernacular architecture, Imbrex and
  tegula, Terraced house, Casale, Campanile, Coastal towers in Sardinia,
  Dry stone, Terrace (earthworks)). Photographs serve as reference only; no
  real building is reproduced, nothing comes from other games, no writing,
  symbols or insignia (no fasces on the town hall).
- **Light and ground** of the biome are presentation values chosen by
  comparison with the `forest` and `desert` presets (ARCHITECTURE.md).
  The cicadas are the repository's own synthesized cicada bed
  (`tools/audio/music.py`); nothing was downloaded.
- **Rules** (hit points, capacities, cover, crush limits) are game rules,
  marked provisional.

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

## Tanks pushing trees over (prompt 106)

- **What tanks can push over:** US Army FM 7-20, appendix D (tank
  capabilities: tanks push through small trees and vegetation;
  globalsecurity.org/military/library/policy/army/fm/7-20/Appd.htm) and
  FM 5-33 *Terrain Analysis* (1990, vegetation: huge trees seldom stand so
  close that a tank cannot pass between them, closely spaced small trees
  can often be pushed over but the pile-up may stop the tank, wheeled
  vehicles are more restricted; archive.org/download/MManuals/
  UsArmyTerrainAnalysis1990.pdf). An unofficial summary of US tank
  doctrine names "small trees up to 12 inches in diameter" (inetres.com/gp/
  military/cv/US_tank_doctrine.html); the official text found lacks the
  number, so it is no basis for a value.
- **Dense stands:** US Army subcourse IN0201 (heavy stands of trees 8 in
  and more in diameter spaced less than 20 ft apart build up into an
  obstacle when tracked vehicles push through them;
  globalsecurity.org/military/library/policy/army/accp/in0201/ch2j.htm).
- **Trunk thickness per model variant** (`data/destruction.toml`) is a
  judgement from the catalog heights of the trees; the catalog sources
  (Wikipedia) give heights, not trunk diameters. The mass limits (18 t,
  40 t) and the stand rule (6 trees within 10 m) are game values, marked
  provisional: no source gives them. No values from other games.

## Maps from real geodata (prompt 081)

The map `harbor_city` (in the game "Hafenstadt" / "Harbor city") is built
by `tools/geo_import.py` from open geodata of **Monaco** and the
neighbouring French communes (Cap-d'Ail, Beausoleil, La Turbie,
Roquebrune-Cap-Martin). The place is named only here, in
[MAPS.md](MAPS.md#maps-from-real-geodata-prompt-081) and in
`assets-src/geo/harbor_city/`; the map files, markers, menus and
translations carry no real name (map names stay generic, see
"Maps" above). The battle on it is invented; no coats of arms, flags or
names of streets, shops or people.

| Data | Source | License and attribution |
|---|---|---|
| Coastline, roads, building footprints and storeys, land use, parks, woods | [OpenStreetMap](https://www.openstreetmap.org), Overpass API (`https://overpass-api.de/api/interpreter`), one query (in `assets-src/geo/harbor_city/site.json`, `sources.osm_query`) over the box 43.712–43.768 N, 7.392–7.462 E, retrieved 2026-11-17 18:25 UTC (OSM base of that minute); raw answer 20.1 MB JSON, reduced to `osm.json.gz` (1.4 MB) | [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/), "© OpenStreetMap contributors" ([copyright](https://www.openstreetmap.org/copyright)) |
| Heights | Copernicus DEM GLO-30 (30 m), tile `Copernicus_DSM_COG_10_N43_00_E007_00_DEM.tif` from the AWS open data bucket `copernicus-dem-30m` (12.6 MB GeoTIFF), retrieved 2026-11-17; a 257 × 206 pixel crop in `dem.json.gz` | free use with attribution: "produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved" |

- `assets-src/geo/harbor_city/osm.json.gz` and the map files of
  `data/maps/harbor_city/` (heights, terrain, object positions) are derived
  from ODbL data, a derivative database in the sense of the license, and
  are available under ODbL 1.0 as well; the game itself is a produced work
  that names the source in its credits (`CREDITS.md`, main menu →
  Credits). [provisional: the exact terms of both licenses should be read
  once more before a release]
- Not used: Google Maps or any map service whose terms forbid extraction,
  the commercial 3D city model whose pictures (`.temp/`, not in the
  repository) served only to compare the result by eye; nothing was
  measured, traced or taken from them.
- The buildings are the game's catalog models standing where real blocks
  stand, chosen by footprint and storeys (stand-ins, not reproductions); no
  real building is modelled.

## Maps from real geodata: the brick quarter (prompt 112)

The map `brick_quarter` (in the game "Backsteinviertel" / "Brick
quarter") is built by `tools/geo_import.py` from open geodata of
**Manhattan around Greenwich Village** (New York City): the West Village,
the Village, SoHo and Hudson Square, the south of Chelsea and the blocks
east of Fifth Avenue up to Union Square, with the Hudson waterfront. The
place is named only here, in
[MAPS.md](MAPS.md#brick_quarter-backsteinviertel) and in
`assets-src/geo/brick_quarter/`; the map files, markers, menus and
translations carry no real name. It shows the town as it stood around
1944 as far as the data allow (see MAPS.md); the battle is invented (the
city saw no fighting), no flags, names of streets, shops or people.

| Data | Source | License and attribution |
|---|---|---|
| Coastline, piers, roads, building footprints, heights and storeys, `start_date`, land use, parks | [OpenStreetMap](https://www.openstreetmap.org), Overpass API (`https://overpass-api.de/api/interpreter`): the query of 081 over the box 40.712–40.754 N, 74.028–73.976 W, retrieved 2026-11-23 06:14 UTC (raw answer 35.1 MB JSON), and a second small query for pier multipolygons (`osm_query_extra`, 06:23 UTC, 29 KB); both in `assets-src/geo/brick_quarter/site.json`, reduced to `osm.json.gz` (2.1 MB) | [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/), "© OpenStreetMap contributors" ([copyright](https://www.openstreetmap.org/copyright)) |
| Heights | Copernicus DEM GLO-30, tiles `Copernicus_DSM_COG_10_N40_00_W074_00_DEM.tif` (15.8 MB) and `..._N40_00_W075_00_DEM.tif` (48.2 MB; the site crosses 74° W) from the AWS open data bucket `copernicus-dem-30m`, retrieved 2026-11-23; a 192 × 156 pixel mosaic crop in `dem.json.gz` (46 KB) | as for `harbor_city`: "produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved" |

- **NYC Building Footprints** (NYC Open Data, dataset `5zhs-2jue`, with
  `construction_year` and `height_roof`) were checked as a second source
  for the year of construction and **not used**: the dataset carries no
  license (its metadata has `"license": null`), and the portal's terms of
  use refer to the NYC.gov terms, which reserve all rights in the
  materials of the site; whether redistribution of derived data in a
  game is allowed could not be settled from these texts. Without a
  settled source the prompt's rule applies: a footprint taller than 45 m
  stands for a later building and gets the district's estimate instead of
  its own height; so does a footprint whose OSM `start_date` is 1946 or
  later (about 200 footprints carry a date). [provisional: ask the NYC
  Open Data team or find an explicit license before using the years]
- `assets-src/geo/brick_quarter/osm.json.gz` and the map files of
  `data/maps/brick_quarter/` are derived from ODbL data and available
  under ODbL 1.0 as well (as for `harbor_city`); the credits name both
  sources (`CREDITS.md`, main menu → Credits).
- The buildings are the game's catalog models standing along the real
  block edges, chosen by the footprint's height and kind (stand-ins: town
  houses for brownstones, tenement blocks for walk-ups and lofts,
  warehouses at the quays); no real building is modelled. The waterfront
  sheds are placed along today's quays and piers (`sheds` in `site.json`),
  not from a historical plan.

## Maps from real geodata: the coastal heights (prompt 113)

The map `coastal_heights` (in the game "Küstenhöhen" / "Coastal
heights") is built by `tools/geo_import.py` from open geodata of the
**Marin Headlands** north of San Francisco (California): Point Bonita,
Rodeo Beach and Rodeo Lagoon, Fort Cronkhite and Fort Barry in the west,
the ridges of Hawk Hill and Wolf Ridge, the Rodeo Valley, Fort Baker,
Horseshoe Bay and the north end of the Golden Gate Bridge in the east, the
south of Sausalito in the north-east corner. The place is named only here,
in [MAPS.md](MAPS.md#coastal_heights-küstenhöhen) and in
`assets-src/geo/coastal_heights/`; the map files, markers (neutral names
such as `supply_south_ridge`), menus and translations carry no real name.
It shows the headlands around 1944 as far as the data allow (see
MAPS.md); the battle is invented (the forts never saw fighting), no flags,
names of roads, batteries or people.

| Data | Source | License and attribution |
|---|---|---|
| Coastline, roads and tracks, building footprints, land use, woods, scrub, beaches, the lagoon | [OpenStreetMap](https://www.openstreetmap.org), Overpass API (`https://overpass-api.de/api/interpreter`): the query of 081 over the box 37.810–37.864 N, 122.552–122.466 W, retrieved 2026-11-23 07:21 UTC (raw answer 8.3 MB JSON), in `assets-src/geo/coastal_heights/site.json`, reduced to `osm.json.gz` (0.7 MB; besides the tags of 081 it keeps `historic` and `military`, `sources.extra_tags`) | [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/), "© OpenStreetMap contributors" ([copyright](https://www.openstreetmap.org/copyright)) |
| Places of the coast batteries | the same Overpass endpoint, a small query for `military=*`, `historic=*`, `building=bunker` (`sources.fortifications_query`, 07:22 UTC, 59 KB, `out tags center`); the positions are copied by hand into `fortifications.works` of `site.json` with the feature named in `source` | ODbL 1.0 as above |
| Heights | Copernicus DEM GLO-30, tile `Copernicus_DSM_COG_10_N37_00_W123_00_DEM.tif` (20.1 MB) from the AWS open data bucket `copernicus-dem-30m`, retrieved 2026-11-23 07:22 UTC; a 315 × 199 pixel crop in `dem.json.gz` (92 KB) | as for `harbor_city`: "produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved" |

- `assets-src/geo/coastal_heights/osm.json.gz` and the map files of
  `data/maps/coastal_heights/` are derived from ODbL data and available
  under ODbL 1.0 as well (as for `harbor_city`); the credits name both
  sources (`CREDITS.md`, main menu → Credits).
- The state of 1944 is judged from general knowledge of the place, not
  from a historical map: the Nike missile site (1950s), the radar on the
  ridge, the post-war family housing in the Rodeo Valley and its streets
  are left out (district and road polygons in `site.json`); the forts'
  buildings are the game's catalog houses (there is no barracks object
  for maps); the batteries are field works of the game
  (`fort_bunker_mg` standing in for the concrete, trenches and sandbag
  positions) at the OSM places of the batteries, facing the sea by hand;
  no real gun or battery is modelled. Battery Construction 129 on the
  south ridge was built 1942–44 and is kept.

## Maps from real geodata: the dune coast (prompt 114)

The map `dune_coast` (in the game "Dünenküste" / "Dune coast") is built
by `tools/geo_import.py` from open geodata of **Kampen on Sylt**
(Schleswig-Holstein, Germany): the North Sea beach and the Rotes Kliff
in the west, the dunes with the Uwe-Düne (about 52 m), the heath and the
village in the middle, the tidal flats of the Wadden Sea in the east,
the north edge of Wenningstedt in the south. The place is named only
here, in [MAPS.md](MAPS.md#dune_coast-dünenküste) and in
`assets-src/geo/dune_coast/`; the map files, markers (neutral names such
as `supply_high_dune`), menus and translations carry no real name. It
shows the place around 1944 as far as the data allow (see MAPS.md); the
battle is invented (there was no fighting on the island), no flags,
emblems or names of people.

| Data | Source | License and attribution |
|---|---|---|
| Coastline, roads and tracks, building footprints, land use, heath, woods, beaches, tidal flats | [OpenStreetMap](https://www.openstreetmap.org), Overpass API (`https://overpass-api.de/api/interpreter`): the query of 081 plus `aeroway` over the box 54.928–54.987 N, 8.280–8.405 E, retrieved 2026-11-23 09:05 UTC (raw answer 7.6 MB JSON), in `assets-src/geo/dune_coast/site.json`, reduced to `osm.json.gz` (0.9 MB; besides the tags of 081 it keeps `historic`, `military`, `wetland` and `aeroway`) | [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/), "© OpenStreetMap contributors" ([copyright](https://www.openstreetmap.org/copyright)) |
| Fortifications | the same endpoint, a small query for `military=*`, `historic=*`, `building=bunker` (`sources.fortifications_query`, 09:06 UTC, 8 KB): one bunker ruin of the Second World War, just outside the frame; no other work in the area | ODbL 1.0 as above |
| Heights | **DGM1 Schleswig-Holstein**, the official digital terrain model with 1 m cells (ground only: no buildings, no trees), from the open geobasis data portal (OpenGBD) of the Landesamt für Vermessung und Geoinformation Schleswig-Holstein (`https://geodaten.schleswig-holstein.de/gaialight-sh/_apps/dladownload/dl-dgm1.html`): 28 tiles of 1 km × 1 km in ETRS89 / UTM zone 32N (EPSG:25832) as `x y z` text (27 MB each, 749 MB in all), heights over NHN in meters, tiles of the survey 2024 and five of 2005 at the edges; tile names in `site.json` (`sources.dgm_tiles`), the list of all tiles from the portal's index (`sources.dgm_index_url`), retrieved 2026-11-23 09:08 UTC; reduced to 2 m cells (mean of 2 × 2 pixels) over the map frame plus 100 m in `dem.json.gz` (4.2 MB) | [CC BY 4.0](https://creativecommons.org/licenses/by/4.0/) (terms of use of the portal, `lizenz.html`, as of 1 August 2022: any use, also commercial, sharing and adapting allowed); attribution as the portal asks: "© GeoBasis-DE/LVermGeo SH/CC BY 4.0", for changed data "© GeoBasis-DE/LVermGeo SH/CC BY 4.0 (Quelle verändert)" |
| Comparison only | Copernicus DEM GLO-30 tile `Copernicus_DSM_COG_10_N54_00_E008_00_DEM.tif` (10.3 MB, AWS bucket `copernicus-dem-30m`, 09:08 UTC), read by `just gen-geo-compare`; not part of the map and not in the repository | as for `harbor_city` |

- The licence was checked before any tile was loaded (section 2 of the
  prompt): CC BY 4.0 allows keeping the reduced data in the repository
  and using it in a game, with the attribution above; it is in
  `CREDITS.md` (main menu → Credits) and in
  `assets-src/geo/dune_coast/README.md`.
- `osm.json.gz` and the map files are derived from ODbL data and
  available under ODbL 1.0 as well (as for `harbor_city`).
- The state of 1944 is judged from general knowledge of the place, not
  from a historical map: the village keeps its buildings only inside the
  old core (a district polygon; the holiday houses around it, mostly
  built after 1950, are left out), at most about 270 catalog houses
  (thatched houses as an own model are open); the north edge of the
  neighbouring village and its streets are left out (a polygon); the
  modern residential lanes are kept as sand tracks (`dirt_track`, as
  sandy lanes of the time); service roads, parking aisles and footpaths,
  including the wooden beach crossings, are dropped by class. The golf
  course stays open ground. The air base south of the frame is outside.
- The field works (`fort_bunker_mg`, trenches and sandbag positions) on
  ten dune crests are **invented positions** (the island was fortified in
  the war, but OSM carries no work inside the frame); each is marked as
  invented in `site.json`. No real battery or gun is modelled.

## Long steel bridges (prompt 115)

- **Waal bridge, Nijmegen** (nl.wikipedia: Waalbrug Nijmegen): main span
  244.1 m between the piers, total length 604 m, width 26.5 m, arch top
  about NAP +60 m, navigation clearance NAP +24.75 m, built 1931–1936,
  taken nearly undamaged on 20 September 1944. Gives `Arch244`, the
  catalog width and height (water about NAP +8 m assumed) and the deck
  18 m above the water.
- **Rhine bridge, Arnhem** (nl.wikipedia: John Frostbrug; en.wikipedia:
  John Frost Bridge): built 1932–1935, largest span 120 m, width about
  23.8 m, navigation height NAP +21.86 m; bombed on 7 October 1944. Gives
  `Arch120` (deck 15 m above the water). The prompt's "about 140 m" is
  not used: the source gives 120 m.
- **Approach spans:** only the total of approaches and ramps follows from
  the sources (604 m against 244 m); the single spans (60 m, 30 m), the
  pier height and the girder depth are judgement (en.wikipedia: Girder
  bridge for the type), marked `uncertain`.
- Photos served only as a reference for form; no measured drawings were
  used. Hit points, railing cover and the shelling caliber are game rules,
  marked provisional.

## Maps from real geodata: the river crossing (prompt 116)

The map `river_crossing` (in the game "Flussübergang" / "River crossing")
and its mission "The great bridge" (`bridge_assault`) are built after
**Nijmegen and the Waal bridge** (Gelderland, Netherlands) in September
1944: the old town with the Valkhof (castle hill) and the Hunnerpark at
the south end of the bridge, the Waal with the bridge of 1936, the
floodplain, the dike and the polder with the village of Lent in the
north, the Ooijpolder in the east. The place is named only here, in
[MAPS.md](MAPS.md#river_crossing-flussübergang),
[MISSIONS.md](MISSIONS.md) and in `assets-src/geo/river_crossing/`; the
map files, markers (neutral names such as `supply_castle_hill`), the
mission file, menus and translations carry no real name, unit or date
[provisional: the prompt leaves naming the place and the date to the
user; nobody could be asked during the run]. No flags, emblems or names
of people; the German side uses the generic `de` catalog without SS
insignia.

| Data | Source | License and attribution |
|---|---|---|
| Rivers and water areas, roads, building footprints (with their year of construction, `start_date`, from the Dutch building register), land use, parks | [OpenStreetMap](https://www.openstreetmap.org), Overpass API (`https://overpass-api.de/api/interpreter`): the query of 112 plus `water=*` ways and relations, `waterway` relations and `man_made=dyke/embankment`, over the box 51.828–51.879 N, 5.830–5.910 E, retrieved 2026-11-23 11:01 UTC (OSM base 11:00:36 UTC; raw answer 52 MB JSON), in `assets-src/geo/river_crossing/site.json`, reduced to `osm.json.gz` (3.1 MB) | [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/), "© OpenStreetMap contributors" |
| Heights | **AHN4 DTM 0.5 m** (Actueel Hoogtebestand Nederland, ground only: no buildings, trees or bridges; no data under buildings and over water), from the PDOK coverage service `https://service.pdok.nl/rws/ahn/wcs/v1_0` (coverage `dtm_05m`, GetCoverage in RD New, EPSG:28992): 30 tiles of 1 km × 1 km at 1 m (x 185–190 km, y 427–431 km; float32 GeoTIFF, deflate, 61 MB in all), retrieved 2026-11-23 11:03 UTC; reduced to 2 m cells (mean of the 1 m pixels) over the map frame plus 100 m, the gaps filled from their neighbours, in `dem.json.gz` (3.6 MB) | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/) ("Geen beperkingen", access constraints of the service's capabilities); no attribution required, named in `CREDITS.md` |
| Reference only | Kadaster topographic map of 1944 (Topotijdreis tiles, service `Historische_tijdreis_1944` on `tiles.arcgis.com`, 49 tiles of level 10, retrieved 2026-11-23 under `build/`, not in the repository): looked at for the village of Lent, the dike and the floodplain north of the river; the service names no licence (Kadaster publishes Topotijdreis as open data, CC BY 4.0); nothing was traced from it, and its tiles could not be placed exactly over the map (about 300 m and a factor of scale off, open) | – |
| Bridge | dimensions of prompt 115 (nl.wikipedia: Waalbrug Nijmegen) | – |
| Events | general knowledge of Operation Market Garden: the 82nd Airborne Division and the Guards Armoured Division took the bridge on 20 September 1944, after paratroopers had crossed the Waal in boats west of the railway bridge under fire; the bridge was prepared for demolition and fell intact | – |

- The **state of 1944**: the side channel dug in 2013–2016 west and
  north of the bridge (with the island in it and the dike moved inland)
  is land again at the height of the polder (`coast.skip_water_at_m`,
  `coast.dry_polygons_m`); the old dike remains on the former island.
  The modern bridge west of the frame's centre (2013) and the cycle and
  rail bridges are not in the map; the railway bridge lies in the frame
  but is left out (no model, prompt 116). North of the river and in the
  meadows east of the town every footprint built from 1946 on is dropped
  (the register's years; the new town on the north bank goes, the old
  village stays), with the minor streets there. In the town the
  footprints of all years stand for the houses of 1944 (the town was
  rebuilt on its street plan after the bombing of February 1944 and the
  fighting); a share of ruins and damaged houses in the old town stands
  for the destruction [provisional].
- `osm.json.gz` and the map files are derived from ODbL data and
  available under ODbL 1.0 as well.

## Maps from real geodata: the rhine bridge (prompt 118)

The map `rhine_bridge` (in the game "Rheinbrücke" / "Rhine bridge") and
its mission "Holding the bridge" (`bridge_hold`) are built after **Arnhem
and its road bridge over the Nederrijn** (Gelderland, Netherlands) in
September 1944: the old town and the quarters north of the river with the
north end of the bridge of 1935, the river, the floodplain and the polder
of Malburgen in the south. The place is named only here, in
[MAPS.md](MAPS.md#rhine_bridge-rheinbrücke), [MISSIONS.md](MISSIONS.md)
and in `assets-src/geo/rhine_bridge/`; the map files, markers, the
mission file, menus and translations carry no real name of the town,
the units or people and no date beyond the month (as decided for prompt
116 [provisional]); the menu name "Rhine bridge" names only the river,
as the prompt asks. The German side uses the generic `de` catalog
without SS insignia (the units at the bridge in 1944 belonged to the
II SS-Panzerkorps; the game shows neither their names nor insignia).

| Data | Source | License and attribution |
|---|---|---|
| Rivers and water areas, roads, building footprints (with their year from the Dutch building register), land use, parks | [OpenStreetMap](https://www.openstreetmap.org), Overpass API (`https://overpass-api.de/api/interpreter`): the query of 116 over the box 51.957–52.000 N, 5.882–5.948 E, retrieved 2026-11-23 14:07 UTC (OSM base 14:07:19 UTC; raw answer 47 MB JSON), in `assets-src/geo/rhine_bridge/site.json`, reduced to `osm.json.gz` (2.9 MB) | [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/), "© OpenStreetMap contributors" |
| Heights | **AHN4 DTM 0.5 m** from the PDOK coverage service `https://service.pdok.nl/rws/ahn/wcs/v1_0` (coverage `dtm_05m`, RD New): 25 tiles of 1 km × 1 km at 1 m (x 189–193 km, y 441–445 km), retrieved 2026-11-23 14:09 UTC; reduced to 2 m cells over the frame plus 100 m, gaps filled, in `dem.json.gz` (3.5 MB) | [CC0 1.0](https://creativecommons.org/publicdomain/zero/1.0/); named in `CREDITS.md` |
| The core of 1944 (blocks, ponds, parks around the north end) | Kadaster topographic map, Topotijdreis tile service `Historische_tijdreis_1944` (`tiles.arcgis.com`, RD New tiling, level 11; 100 tiles of 256 px under `build/`, not in the repository), retrieved 2026-11-23; in this place an edition from before 1935 (no bridge yet; the editions 1938–1950 of the service show the same sheet). Traced by colour into `core_1944.json` (outlines of blocks, not the map picture) | The service names no licence; the Kadaster publishes its topographic data (BRT, historical TOPraster) under CC BY 4.0, so the traced outlines are attributed "Kadaster" in `CREDITS.md` [provisional: the terms of the Topotijdreis layer itself should be confirmed]. Street and block positions are facts; no map picture is shipped |
| Reference only | Aerial photograph "De Rijnbrug van Arnhem 5 dagen voor de gevechten om de brug", 13 September 1944 (Nationaal Archief, via Wikimedia Commons, public domain), downloaded under `build/` for the comparison picture, not in the repository: shows the long north ramp down to a roundabout and the houses on both sides, the harbour basin west of the bridge on the south bank and rows of houses in the polder | public domain |
| Bridge | dimensions of prompt 115 (nl.wikipedia: John Frostbrug: built 1932–1935, total length 601 m, largest span 120 m, about 23.8 m wide); the line is today's bridge (rebuilt on the same piers' line, 1948–1950) | – |
| Events | general knowledge of Operation Market Garden: a British parachute battalion reached the north end of the road bridge in the evening of 17 September 1944 and held the houses at the ramp; on the morning of 18 September a German reconnaissance unit with armoured cars and half-tracks tried to cross from the south and was shot up on the ramp; infantry, tanks and artillery attacked from the town over the next days; supplies were dropped from the air, many into German-held ground; the relief from the south (XXX Corps) did not arrive, the last defenders were overrun on 21 September; the bridge was bombed by the Allies on 7 October 1944 | – |

- The **state of 1944**: inside the core the town of the old map stands
  (blocks, streets between them), with the ramp of 1935 and the houses
  beside it added by hand [provisional: their outlines are judged; the
  aerial photograph confirms houses on both sides]. Outside, the town
  keeps today's footprints as stand-ins; south of the river and on the
  fields east of the town the register's post-war buildings and the
  minor streets of the post-war quarters are left out, and 17 lakes and
  ponds dug or made later are not read. The modern bridge west of the
  core (Nelson Mandelabrug) and its interchange are not in the map.
- `osm.json.gz` and the map files are derived from ODbL data and
  available under ODbL 1.0 as well.

## Maps from real geodata: the landing gulf (prompt 121)

The map `landing_gulf` (in the game "Landung im Golf" / "Landing gulf")
and its mission "Landing in the gulf" (`gulf_landing`) are built after the
**Golfo di Campo on the island of Elba** (Tuscany, Italy) and the landing
of **Operation Brassard** on 17 June 1944: Marina di Campo with its
harbour and the long sand beach at the head of the gulf, the plain of
Campo behind it, Monte Poro in the south-west and the hills towards Punta
di Fonza in the east. The place is named only here, in
[MAPS.md](MAPS.md#landing_gulf-landung-im-golf),
[MISSIONS.md](MISSIONS.md#landing-in-the-gulf-prompt-121) and in
`assets-src/geo/landing_gulf/`; the map files, markers, the mission file,
menus and translations carry no real name of the place, the units or
people and no date (as decided for prompt 116 [provisional]). There is
no French nation: the French troops of 1944 (commandos, colonial
infantry), equipped almost entirely with American gear, are US units in
the game; the German side is the generic `de` catalog without insignia.
What happened to the civilians after the landing is not part of the game.

| Data | Source | License and attribution |
|---|---|---|
| Coastline, roads, building footprints, land use, walls and retaining walls | [OpenStreetMap](https://www.openstreetmap.org), Overpass API (`https://overpass-api.de/api/interpreter`): query over the box 42.712–42.790 N, 10.175–10.300 E (also `barrier=wall`/`retaining_wall`, `aeroway`, `tourism`), retrieved 2026-11-23 17:49 UTC (OSM base 17:48:49 UTC; raw answer 7.0 MB JSON), in `assets-src/geo/landing_gulf/site.json`, reduced to `osm.json.gz` (0.6 MB); a second query for military and historic features (17:51 UTC, answer not kept: no bunker or battery in the frame, two lighthouses, the coastal tower, churches) | [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/), "© OpenStreetMap contributors" |
| Heights | Copernicus DEM GLO-30, tile N42 E010 (`https://copernicus-dem-30m.s3.amazonaws.com/Copernicus_DSM_COG_10_N42_00_E010_00_DEM/…`), retrieved 2026-11-23 17:52 UTC (5.6 MB), cropped to `dem.json.gz` (0.2 MB) | "Produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved" (free use with this attribution); in `CREDITS.md` |
| Events, batteries | English Wikipedia, "Invasion of Elba" (retrieved 2026-11-23): Free French forces with British and American naval support, 17–19 June 1944; Royal Navy commandos entered the bay of Marina di Campo at 03:50 under fire; main landings planned for 04:00 on two beaches; fire from the hills above the beaches forced craft to withdraw; the flak ship at the jetty of Marina di Campo; the battery at Poro (one 102 mm gun, four 90 mm guns) covering the gulf; Senegalese troops cleared the village after several hours and fought in the hills with flamethrowers. Theatrum Belli, "17-18 juin 1944: le Bataillon de choc et le Groupe de commandos d'Afrique … libération de l'île d'Elbe" (retrieved 2026-11-23): the commandos landed ahead of the main landing to silence the coast batteries so the landing at Marina di Campo could go in; the battery of Poro surrendered to a detachment that kept its crews from the guns; detachments took Fonza and Capo Stella; the beach of Marina di Campo was "minée et barbelée"; the commandos landed on a small beach further east and went up the ridge to outflank the village | facts only, no text or picture taken |

- **Batteries and works** (map objects, `fortifications.works`): Poro
  (two works on the height above the lighthouse; Wikipedia, Theatrum
  Belli), Fonza (one work on the height above the point; Theatrum Belli),
  the ridge east of the beach (one work, by hand: the commandos' route),
  the harbour and the south end of the beach (two works, by hand: the
  village was defended). Pillboxes, trench sections and sandbag
  positions are the game's placeholders. Barbed wire along the beach (in
  runs with gaps) and two anti-personnel mine sections on the beach
  follow "minée et barbelée"; places by hand. No hedgehogs: no source
  names anti-tank obstacles on this beach.
- **Guns of the mission:** light field howitzers (`de_lefh18`) stand in
  for the 90 mm and 102 mm guns of the batteries; there are no heavy
  coast guns in the catalog (open). The counterattack with an assault gun
  and a half-track is not taken from a source [provisional].
- **The state of 1944** (by general knowledge of the place, no
  historical map used): the airfield in the plain (opened after the war),
  hotels, camp sites and the town around the old village are left out;
  the old village around the harbour and the main street stays (a
  polygon by hand); farms in the plain and on the slopes stand as
  Mediterranean farmhouses. The harbour's jetty is kept (the sources name
  it in 1944); floating pontoons are not read.
- `osm.json.gz` and the map files are derived from ODbL data and
  available under ODbL 1.0 as well.

## Maps from real geodata: the castle rock (prompt 127)

The map `castle_rock` (in the game "Burgfelsen" / "Castle rock") is built
after the **inner city of Edinburgh** (Scotland) around 1944: Edinburgh
Castle on the Castle Rock, the Old Town along the Royal Mile, Princes
Street Gardens and Waverley station in the valley, the New Town, Calton
Hill, Holyrood Park with Arthur's Seat and Salisbury Crags, and in the
north the Water of Leith with Stockbridge and Canonmills. No battle was
fought here; the fight on the map is fictional. The place is named only
here, in [MAPS.md](MAPS.md#castle_rock-burgfelsen) and in
`assets-src/geo/castle_rock/`; the map files, markers, menus and
translations carry no real name (checked by the map test).

| Data | Source | License and attribution |
|---|---|---|
| Roads, railways, building footprints, land use, water, cliffs | [OpenStreetMap](https://www.openstreetmap.org), Overpass API (`https://overpass-api.de/api/interpreter`): query over the box 55.930–55.972 N, 3.245–3.140 W (with `railway`, `historic`, `barrier` walls), retrieved 2026-11-24 04:05 UTC (OSM base 04:03:48 UTC; raw answer 89 MB JSON), in `assets-src/geo/castle_rock/site.json`, reduced to `osm.json.gz` (4.8 MB) | [ODbL 1.0](https://opendatacommons.org/licenses/odbl/1-0/), "© OpenStreetMap contributors" |
| Heights | Copernicus DEM GLO-30, tile N55 W004 (`https://copernicus-dem-30m.s3.amazonaws.com/Copernicus_DSM_COG_10_N55_00_W004_00_DEM/…`), retrieved 2026-11-24 04:04 UTC (30 MB), cropped to `dem.json.gz` (86 KB) | "Produced using Copernicus WorldDEM-30 © DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved" (free use with this attribution); in `CREDITS.md` |
| Heights of the rock and the hill | English Wikipedia, "Castle Rock, Edinburgh" (retrieved 2026-11-24): summit 130 m above sea level, cliffs up to 80 m on the south, west and north, the only easy approach from the east; "Salisbury Crags": cliffs of 46 m; "Arthur's Seat": 251 m | facts only; the castle's plateau is raised to 121 m (`elevation.lifts`), the cliff steps come from the elevation model along the OSM lines |

- **Not used:** the Scottish public sector LiDAR (Scottish Remote
  Sensing Portal, British National Grid). The prompt names the Open
  Government Licence; the portal is a script page and its licence text
  could not be read in this session, so it is **not confirmed** and the
  importer has no reader for its grid yet [open: confirm the licence
  before using it].
- **The state of 1944** (by general knowledge of the place, no
  historical map used): the Scottish Parliament (2004), Dynamic Earth
  (1999) and the offices beside them stand as warehouses of an industrial
  district (the brewery ground of 1944); the shopping centres of today
  (St James Quarter, the Omni centre) stand as rows of town houses (the
  tenements before them); trams and their stops are not read; streets
  closed to traffic later are streets; railway lines closed after 1944
  are kept, tunnels are not built. Footprints taller than 45 m or with a
  `start_date` from 1946 get the district's estimate.
- **Castle models** (`env_castle_wall`, `env_castle_gatehouse`,
  `env_castle_bastion`, `env_castle_hall`, `env_castle_chapel`, sources
  in `data/catalog/neutral.toml`): Wikipedia articles on curtain walls,
  gatehouses, bastions, the castle, the Half Moon Battery and St
  Margaret's Chapel give the kinds of buildings and rough sizes; no
  building is copied, sizes are judgement values. The tower on the small
  hill is the coastal watch tower as a stand-in.
- `osm.json.gz` and the map files are derived from ODbL data and
  available under ODbL 1.0 as well.

## Boats (prompt 119)

Catalog values of the six boats (`data/catalog/*.toml`, each `source`
names what it covers):

- **LCVP, LCM(3):** ONI 226 *Allied Landing Craft and Ships* (US Office of
  Naval Intelligence, 7 Apr 1944; US government work, public domain),
  https://www.ibiblio.org/hyperwar/USN/ref/ONI/ONI-226/ONI-226.pdf, LCVP
  p. 48, LCM(3) p. 52; Wikipedia for the plywood hull, the M1919 guns and
  the years. "Tons" there are short tons (the LCM's "30-ton tank" equals
  its 60,000 lb).
- **Assault boat M2:** *Stream Crossing Equipment*, Information Bulletin
  No. 120, Office of the Chief of Engineers (July 1943), paras 8–9, as
  transcribed by Lone Sentry (archived copy).
- **Sturmboot 39, Pionierlandungsboot 39:** Lexikon der Wehrmacht and
  Navypedia (they disagree on the Sturmboot's length and the PiLB's speed;
  the catalog notes which was taken); TM-E 30-451 (1943) pp. 136, 144.
- **A-3 boat (Soviet):** Russian Wikipedia "ПА-3", which cites the 1931
  engineer manual and Ilyin-Mitkevich's 1941 handbook. The DL-10 assault
  boat had no data to be found, so the A-3 of the divisional pontoon park
  stands for the Soviet river-crossing boat.

Not found and marked `uncertain`: the hull depth (`height_mm`) of the
LCVP, LCM, Sturmboot and PiLB (estimated from photos and drawings), the
paddled speeds (4 km/h), the A-3's mass and draft, the PiLB's vehicle
count. No source gives ramp or unloading times: those of
`data/boats.toml` are game rules. Models are built from these dimensions
by the part kit; no drawings or data of other games were used.

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

## Blast throws (prompt 194)

- **Throw by charge** (`[knockdown]` in `data/infantry.toml`): the throw
  grows linearly with the caliber after Hopkinson-Cranz cube-root scaling
  (blast effects at a distance scale with the cube root of the charge
  mass, the charge of similar shells with the cube of the caliber; G. F.
  Kinney, K. J. Graham, *Explosive Shocks in Air*, 2nd ed., Springer 1985,
  chapter on scaling laws). How far a man is thrown is not
  given by any source found; the factor (30 mm per mm of caliber, at
  most 12 m) is judgement and marked provisional. Bombs use their shell
  equivalent of `data/air.toml`.

## Running and hitting the dirt (prompt 196)

- **Run speed** (`[rush] speed_pct` in `data/tactics.toml`): 3.6 m/s, the
  speed the `run` clip is authored for (docs/INFANTRY.md "Animations"),
  about 13 km/h: a run with rifle and kit, well below a free sprint
  (6-8 m/s). Against the catalog's cross-country march speed (4 km/h)
  that is about three times: 300 %. Advancing by short rushes from cover
  to cover under fire is the drill of the period's infantry manuals (US
  War Department FM 7-10, *Rifle Company, Infantry Regiment*, 1942); how
  long a loaded man can run before he is spent (20 s real), how long he
  needs to recover and how much worse he aims is judgement and marked
  provisional.
- **Hitting the dirt** (`[drop]`): the radius per caliber (150 mm per mm:
  8 cm 12 m, 15 cm 22.5 m) is judgement in the order of the lethal radius
  commonly given for 81/82 mm mortar rounds (15-25 m); the reaction to a
  burst within 3 m and the 3 s real on the ground are judgement.

## Infantry behind tanks (prompt 198)

- **Practice** (`[escort]` in `data/tactics.toml`): riflemen accompanying
  tanks in the attack follow them closely, using the hull as cover against
  small-arms fire, and protect them against close-in anti-tank teams; the
  US War Department's FM 17-36, *Employment of Tanks with Infantry* (1944),
  describes infantry advancing with and behind the tanks (title and
  subject known, the manual was not read in full here). No distances were
  taken from a manual: 9 m from the tank's center to the middle of the
  squad (the front row about 5 m behind the center, 2 m behind the hull of
  a Panzer IV), two files 1.4 m apart inside the hull's width (Panzer IV H
  2.88 m, catalog), 2 m between rows, the pacing distances and the
  300 m (game) in which an enemy in sight starts the escort are judgement
  and marked provisional. The cover itself is no value: the hull stops the
  rounds by the projectile simulation (`sim/combat.rs`).

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

## Vehicle paint by year

German vehicles wear the paint of the year they reached the front
(`UnitCamo.palette_for`, `godot/assets/materials/camo/de_*_early.tres`):
catalog `year_from` before 1943 means plain Dunkelgrau Nr. 46, later ones
the Dunkelgelb three-tone scheme. Colors are sRGB approximations of the RAL
references, the grey lightened a little for the game distance.

| Paint | Facts used | Source |
|---|---|---|
| Dunkelgrau Nr. 46 (RAL 7021) | standard paint of army vehicles from 1937; until 1940 with Dunkelbraun RAL 7017 patches, then plain; replaced by Dunkelgelb RAL 7028 by the order of February 1943 | https://en.wikipedia.org/wiki/Panzer_grey, https://en.wikipedia.org/wiki/List_of_RAL_colours |
| Dunkelgelb (RAL 7028) with Olivgrün RAL 6003 and Rotbraun RAL 8017 | base coat from February 1943, the two colors issued as paste for camouflage sprayed in the field | https://en.wikipedia.org/wiki/Panzer_grey, https://en.wikipedia.org/wiki/List_of_RAL_colours |
| Soviet 4BO | protective green, the standard paint of Soviet armour 1941–1945, plain on forest maps (disruptive patches were rare) | https://en.wikipedia.org/wiki/Military_camouflage |

Simplification: older vehicles repainted Dunkelgelb after 1943 stay grey.

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
