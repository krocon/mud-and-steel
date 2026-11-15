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
