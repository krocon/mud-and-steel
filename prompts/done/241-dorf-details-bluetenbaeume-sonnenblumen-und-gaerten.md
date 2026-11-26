# Prompt 241 – Ost-/Mitteleuropäische Dorf-Details: Blütenbäume, Sonnenblumen, Ziehbrunnen, Holzschuppen und Gemüsegärten

> In einer frischen Session im Repo-Ordner ausführen. Benötigt Blender
> (`docs/VERSIONS.md`). Lies zuerst `CLAUDE.md`, `docs/ASSETS.md` (Budgets,
> Umwelt-Objekte, Laub-Shader-Muster), `docs/SOURCES.md`, `docs/SCALE.md`,
> `data/catalog/SCHEMA.md` und `docs/MAPS.md`.
> Im Code:
> `tools/blender/kit/env.py`,
> `tools/blender/kit/buildings.py`,
> `godot/assets/shaders/env_foliage.gdshader`,
> `godot/assets/shaders/env_common.gdshaderinc`,
> `godot/assets/materials/terrain_fields.gdshaderinc`,
> `data/catalog/neutral.toml`,
> `data/objects.toml`,
> `data/destruction.toml`,
> `data/vision.toml`,
> `data/fire.toml`,
> `crates/mapgen/src/gen_sketch/` (`schema.rs`, `settlements.rs`, `mod.rs`),
> `godot/scenes/editor/editor_data.gd`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/This WWII RTS is Pure Chaos #panzerstrike [lskjKn-C9yo].webm`
> (Sekunden `00:00–00:02` und `00:05–00:09`: blühende rosa-purpurne Zier-/Obstbäume
> im Dorf, gelbe Sonnenblumen-Horste am Dorfrand, hölzerner Ziehbrunnen mit
> langem Schwengelbaum auf dem Dorfplatz, winzige Holzschuppen mit rotem
> Ziegeldach neben den Höfen, eng bepflanzte Gemüsegarten-Beete in dunkler Erde
> und niedrige ländliche Staketen-/Flechtzäune).
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> **Hinweis zum Golden Hash:** Neue Umwelt-Objekte in `data/catalog/neutral.toml`,
> `data/objects.toml`, `data/destruction.toml` (und ggf. `data/vision.toml`)
> sind ein gewollter Regelzuwachs und ändern den Golden Hash – Gegenprobe ohne
> neue Einträge durchführen, Ursache dokumentieren und Golden Hash sauber
> aktualisieren.

## 1. Problem

Unser Umwelt-Katalog deckt bereits Waldbäume (`env_tree_deciduous`,
`env_tree_conifer`, `env_tree_birch`), Bauernhäuser (`env_house_farm`,
`env_house_farm_brick`, `env_barn`, `env_house_log`) und Weidezäune
(`env_fence_wood`) ab. Für ein lebendiges ost-/mitteleuropäisches Sommerdorf
wie in der Referenz fehlen jedoch fünf charakteristische Dorf-Bausteine:

1. **Blühende Obst-/Zierbäume (`env_tree_blossom`):**
   - Im Sommerdorf setzen einzelne rosa-magenta/purpurn blühende Bäume
     (Blutpflaume / Zierkirsche / Fliederbaum) sofort einen warmen, natürlichen
     Farbkontrast zwischen dem satten Wiesengrün und den roten Ziegeldächern.
2. **Sonnenblumen-Horste (`env_sunflowers`):**
   - Kräftig gelb blühende Sonnenblumen-Gruppen entlang von Gartenzäunen und
     Dorfrändern prägen sofort das Sommer-Ostfront-Flair.
3. **Hölzerner Ziehbrunnen (`env_well_shadoof`):**
   - Bisher existiert im Katalog nur der gemauerte Wüstenbrunnen
     `env_well_desert`. Ein typischer europäischer/osteuropäischer Dorf-Ziehbrunnen
     (Holzkasten, Stützpfahl, langer schräger Schwengelbaum mit Gegengewicht
     und Eimerstange) fehlt.
4. **Kleiner Hof-/Gartenschuppen (`env_shed_wood`):**
   - Neben großen Bauernhäusern und Scheunen stehen in Dörfern winzige
     Holz- und Putz-Nebenschuppen (`~3,5 × 2,5 m`, rotes Ziegeldach), die die
     Dorf-Silhouetten im Maßstab auflockern und als zerstörbare Kleinbauten
     dienen.
5. **Gemüsegarten-Beete & ländlicher Staketenzaun:**
   - Direkt hinter den Bauernhöfen liegen in der Referenz kleine rechteckige
     **Hausgärten** mit dunkler, umgegrabener Erde und dichten grünen
     Gemüsereihen (`~0,8–1,2 m` Reihenabstand) sowie **dichte ländliche
     Staketen-/Flechtzäune** (enger und niedriger als der offene Weidezaun
     `env_fence_wood`).

## 2. Ziel

### Teil A – Neue Umwelt-Modelle, Katalog & Regeln (1:1 in Metern, Quellen in `docs/SOURCES.md`)

Alle neuen Einträge erhalten historische/reale Maßangaben mit Quellenvermerk
(`uncertain` wo typisch/geschätzt), saubere LOD-/Schatten-Meshes im Budget
(`docs/ASSETS.md`), zerstörbare Stufen, Karteneditor-Eintrag (`editor_data.gd`)
und Thumbnails:

1. **`env_tree_blossom`** („Blütenbaum“ / „Blossoming tree“):
   - Maße ca. **7,5 × 7,5 × 8,5 m** (etwas kompakter und breiter als ein hoher
     Waldbaum, passend für Dorfgärten).
   - Varianten `Intact`, `Bare` (kahl nach schwerem Beschuss wie Prompt 085,
     auch im Winter kahl), `Fallen` (umgestürzt) + `_Far`/`_Shadow`.
   - Laubfarbe in `env_foliage.gdshader` über eigenes Blüten-Muster oder
     Farb-Slot/Vertex-Tint in sattem, natürlichem **Purpur-/Altrosa** (keine
     grell-künstliche Neonfarbe! Schattierte Unterseite tief bordeaux-violett,
     Sonnenseite warm rosa-magenta; bei `env_autumn` bzw. Winter sinnvoll
     gedämpft/kahl).
   - Regeln wie `env_tree_deciduous` (Deckung, Sicht, umstürzbar/entlaubbar,
     von Panzern umfahrbar).
2. **`env_sunflowers`** („Sonnenblumen“ / „Sunflowers“):
   - Maße ca. **3,5 × 3,5 × 1,8 m** (lockerer, unregelmäßiger Horst aus
     ~15–25 Stängeln, grünen Blättern und leuchtend gelben Blütenköpfen mit
     dunkelbraunem Zentrum, `≤ 600 Tris`, 2 Varianten `A`/`B` + `_Far`).
   - Im Spiel reine weiche Vegetation / leichte Infanterie-Sichtdeckung (wie
     `env_bush`, problemlos von Fahrzeugen überrollbar, in `destruction.toml`
     zerstörbar).
3. **`env_well_shadoof`** („Ziehbrunnen“ / „Shadoof well“):
   - Maße ca. **5,5 × 2,2 × 4,8 m** (quadratischer Holzbohlen-Brunnenkasten
     `~1,3 × 1,3 × 0,9 m`, senkrechter Gabelpfahl `~3,0 m`, schräger
     Schwengelbaum `~5,0 m` mit Stein-/Holz-Gegengewicht und dünner
     Eimerstange + kleinem Holzeimer, `≤ 900 Tris`).
   - Varianten `Intact` und `Ruin` (gebrochener Schwengelbaum & Holztrümmer).
   - Hindernis/leichte Deckung analog `env_well_desert`, zerstörbar.
4. **`env_shed_wood`** („Holzschuppen“ / „Wooden shed“):
   - Maße ca. **4,0 × 2,8 × 3,2 m** (kleiner Bretter-/Fachwerk-Nebenschuppen
     mit rotem Ziegeldach, kleiner Holztür und Fenster, `≤ 1200 Tris`).
   - Varianten `Intact`, `Damaged` (aufgerissenes Dach nach Prompt 240, falls
     vorhanden, sonst Dachloch), `Ruin` (Holz- und Ziegeltrümmer).
   - Zerstörbares Kleingebäude (nicht oder nur mit kleinem Trupp besetzbar –
     konsistent mit `data/infantry.toml` / `data/objects.toml` entscheiden und
     dokumentieren).
5. **`env_fence_picket`** („Staketenzaun“ / „Picket fence“; oder neue
   Blueprint-Variante, falls klarer als eigenes Objekt):
   - Maße pro Segment ca. **4,0 × 0,2 × 1,1 m** (`Segment`, `End`, `Corner`,
     `Broken`/`Ruin`; `≤ 300 Tris`), dicht stehende verwitterte Holzlatten mit
     zwei Querriegeln – ideal für Dorf- und Gemüsegärten.
   - Regeln wie `env_fence_wood` (von Panzern niederwalzbar, leichte Deckung).

### Teil B – Dorf-Gemüsegärten & Staketenzäune im Skizzen-Generator (`gen_sketch`)

1. **`[[garden]]` (oder `kind = "garden"` in `[[field]]` / `[[fence]]`-Erweiterung) in `crates/mapgen/src/gen_sketch/`:**
   - Alle bestehenden Skizzenkarten bleiben **Bit-für-Bit unverändert**!
   - Optionales **`kind = "garden"`** (oder `[[garden]]`-Block):
     - Malt ein kompaktes rechteckiges Beet aus `field_ploughed` (dunkle Erde)
       und/oder `field_green` mit **engen Gemüsereihen** (z. B. durch kleine
       grüne `env_bush`-Mini-Skalierung oder ein eigenes niedriges
       Gemüsereihen-Objekt `env_garden_rows`, bzw. dichtere Reihenfrequenz in
       `terrain_fields.gdshaderinc` – einfachste, flimmerfreie Lösung wählen,
       die aus steiler RTS-Kamera sofort wie ein echter Bauerngarten mit grünen
       Beetreihen auf dunkler Erde aussieht!).
     - Optional automatisch an 2–4 Seiten von `env_fence_picket` eingerahmt
       (mit Lücke an Wegen).
   - In `[[fence]]` (aus Prompt 226) optionales `kind = "picket"` (Default
     weiterhin `"wood"` → `env_fence_wood`), damit Staketenzäune auch frei
     entlang von Hofgrenzen gezogen werden können.

## 3. Prüfen und zeigen

- **Übersichts- und Ingame-Bilder** in `build/village_props/`:
  - Alle neuen Modelle (`env_tree_blossom`, `env_sunflowers`, `env_well_shadoof`,
    `env_shed_wood`, `env_fence_picket`) in der Modellgalerie / per
    `just env-lineup` und in einer kleinen Dorf-Testkomposition aus steiler
    RTS-Draufsicht (`pitch ≈ 55°`).
  - Gemüsegarten-Beet neben `env_house_farm` und `env_shed_wood` prüfen.
- **Asset-, Katalog- und Spiel-Checks:**
  - `just validate-assets`, `just glb-check`, `just catalog-doc`.
  - `just thumbnails --only=env_tree_blossom,env_sunflowers,env_well_shadoof,env_shed_wood,env_fence_picket`
    und `just units-web-sync`.
  - Gegenprobe: bestehende Skizzenkarten bleiben Bit-für-Bit unverändert.
  - `just check` und `just selftest` grün; Golden Hashes sauber begründet
    aktualisiert.

## 4. Nicht Teil dieses Prompts

- Bau der neuen Ostfront-Dorfkarte und des Hochkant-Shorts (Prompt 242).
- Umbau bestehender Karten.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/241-dorf-details-bluetenbaeume-sonnenblumen-und-gaerten.md prompts/done/`
verschieben, Ergebnisabschnitt (neue IDs, Maße, Dreieckszahlen, Quellen,
Golden-Hash-Änderung) ergänzen und den Status in `prompts/README.md` auf
„erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Fünf neue Dorf-Umweltobjekte (Blender-Baukasten, Katalog, Regeln, Editor, Thumbnails):**
  - `env_tree_blossom` (`Blossom`, `BlossomYoung`, `Bare`, `Fallen`, `_Far`/`_Shadow`; neues Blütenlaub-Muster `CARD_BLOSSOM = 14` / `CARD_CROWN_BLOSSOM = 15` in `env.py` & `env_foliage.gdshader`, inkl. Ortho-Thumbnail-Tint und `LEAF_COLORS` in `effects.gd`).
  - `env_sunflowers` (`A`, `B`, `_Far`; neues `@env_part("sunflowers")` in `vegetation.py` + `CARD_SUNFLOWER = 16` in `env_foliage.gdshader` mit 12 gekreuzten aufrechten/geneigten Karten).
  - `env_well_shadoof` (`Intact`, `Ruin`; `@env_part("well_shadoof")` in `parts_neutral.py`, 200 Tris, sauber zentrierter Bodenkontakt).
  - `env_shed_wood` (`Intact`, `Damaged`, `Ruin`; kleiner Holz-/Ziegelschuppen).
  - `env_fence_picket` (`Segment`, `End`, `Corner`, `Ruin`; `@env_part("fence_picket")` in `parts_neutral.py`).
  - Registriert in `data/catalog/neutral.toml`, `data/objects.toml`, `data/destruction.toml`, `data/vision.toml`, `data/fire.toml` und `godot/scenes/editor/editor_data.gd`; `just catalog-doc` und `just units-web-sync` aktualisiert.
- **Gemüsegärten & Staketenzäune im Skizzen-Generator (`crates/mapgen/src/gen_sketch/`):**
  - Optionales `kind = "picket"` in `[[fence]]` sowie optionales `[[garden]]` (`GardenDef`, `garden_beds()`) rückwärtskompatibel ergänzt (alle bestehenden Skizzenkarten unverändert).
- **Prüfung & Golden Hashes:**
  - `just validate-env`, `just glb-check` (463 GLBs ok), `just check` (781 Rust-Tests + alle Geo-/Asset-/l10n-Checks grün) und `just selftest` (292 Einheiten/Objekte mit 3D-Modell) grün.
  - Golden Hashes bewusst und dokumentiert aktualisiert (`determinism.rs`: `0xda23_c26e_c657_755c` → `0xe29d_c11d_7451_05cb`, `economy.rs`: `0xbd2b_2f9d_1484_eade` → `0x5c7c_594e_0721_4657`) durch die 5 neuen neutralen Einträge in den gehashten Objekt-/Zerstörungs-/Feuer-/Sichtregeln.
