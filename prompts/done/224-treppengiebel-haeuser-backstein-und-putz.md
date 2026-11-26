# Prompt 224 – Niederländisch-flämische Treppengiebel-Häuser (Backstein, Putz und Giebel-Baukasten)

> In einer frischen Session im Repo-Ordner ausführen. Benötigt Blender
> (`docs/VERSIONS.md`). Baut auf Prompt 215 (`sash_window`) und Prompt 219
> (`buildings.py`-Erweiterungen) auf.
> Lies zuerst `CLAUDE.md`, `docs/ASSETS.md` (Abschnitte „Polygon and texture
> budgets“, „Environment objects“, „Town and winter buildings“, „Surfaces per
> building“), `docs/SOURCES.md`, `docs/SCALE.md` und `data/catalog/SCHEMA.md`,
> sowie als Vorbilder `prompts/done/219-normandie-dorfhaeuser-und-kirche.md`
> und `prompts/184-arkaden-stadthaus.md`. Im Code:
> `tools/blender/kit/buildings.py`,
> `tools/blender/kit/env.py`,
> `godot/assets/shaders/env.gdshader` (`sash_window`),
> `godot/assets/shaders/env_surface.gdshaderinc`,
> `godot/assets/shaders/env_roof.gdshaderinc`,
> `assets-src/blueprints/neutral/env_house_farm_brick.toml`,
> `assets-src/blueprints/neutral/env_house_stone.toml`,
> `assets-src/blueprints/neutral/env_church.toml`,
> `data/catalog/neutral.toml`,
> `data/objects.toml`,
> `data/destruction.toml`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:20–00:30`
> (eingeschossiges rotes Backsteinhaus im rechten Vordergrund, geschlossenes
> Treppengiebel-Dorf in der oberen Bildmitte mit dunklen Backsteinhäusern,
> hellem Ocker-Putzhaus und zerschossener Treppengiebel-Ruine, sowie Gehöft
> und großer Backsteinbau links am Bildrand).
> **Keine Bilder, Namen, Texturen oder Assets aus dem Video ins Repo.**
> **Hinweis zum Golden Hash:** Neue Gebäude-IDs in `data/catalog/neutral.toml`,
> `data/objects.toml` und `data/destruction.toml` sind ein gewollter
> Regelzuwachs und ändern den Golden Hash – Ursache prüfen, bestehenden
> Vergleichslauf ohne neue Bauten gegentesten, Hash sauber aktualisieren und im
> Commit nennen.

## 1. Problem

Die Gebäude in der Referenz (Sek. 00:20–00:30) prägen sofort den unverwechselbaren
nordwesteuropäischen (niederländisch-flämisch / niederrheinischen) Charakter der
Szene – und fehlen bisher in unserem Gebäudekatalog:

1. **Treppengiebel mit Zwillings-Giebelschornsteinen:**
   - Alle Dorfhäuser tragen beidseitig **gemauerte Treppengiebel** (5–6 Stufen
     pro Schräge) über dunklen Schiefer-/Anthrazitdächern, gekrönt von
     **gemauerten Schornsteinen direkt auf beiden Giebelspitzen** (`+Y` und `-Y`).
   - Im zerschossenen Dorf bleiben markante **gezackte Treppengiebel-Ruinen**
     stehen, deren gestufte Silhouetten im Rauch sofort als Dorf lesbar sind.
   - `tools/blender/kit/buildings.py` kennt bisher nur glatte Dreiecksgiebel
     (`h.gable`) und einen einzelnen First-Schornstein (`chimney_cap`).
2. **Klare Klinker- und Putz-Kontraste mit weißen Kreuzstock-Fenstern:**
   - **Vordergrund rechts:** ein eingeschossiges, dunkles rotes Backstein-/Klinkerhaus
     mit sauberem hellem Werkstein-/Betonsockel, kleiner heller Eingangsstufe,
     weißer Traufkante, 4 dunklen Fenstern mit **kräftigem weißem Kreuzstock**
     und hellen waagerechten Werkstein-Stürzen/Sohlbänken + 1 dunkler Tür an der
     Längsseite (2 Fenster je Giebelseite) sowie dunklem Schieferdach.
   - **Dorfzentrum & links:** dicht gruppierte **zweigeschossige rote
     Backsteinhäuser** mit derselben Treppengiebel-Form, weißen Kreuzstockfenstern
     in zwei Stockwerken (4–5 Achsen) und Zwillings-Schornsteinen – im wirkungsvollen
     Wechsel mit einem **zweigeschossigen hellen Ocker-/Sandputz-Haus** mit
     Treppengiebeln und dunklem Schieferdach als hellem Blickfang in der Mitte.
   - **Randbebauung links / hinten rechts:** großer, steil gedeckter dunkler
     Backsteinbau (Kirche bzw. großes Backstein-Langhaus mit Strebepfeilern).

## 2. Ziel

### Teil A – Baukasten `tools/blender/kit/buildings.py` & Shader rückwärtskompatibel erweitern

Alle neuen Parameter per Default aus, sodass **alle bestehenden Gebäude unverändert**
bleiben! Zudem `stepped_gable` allgemein genug anlegen, damit auch das spätere
schmale Kanal-Giebelhaus (`env_house_gable` in Prompt 151) ihn direkt wiederverwenden
kann:

1. **`stepped_gable = N`** (int, Default `0`; z. B. `5` oder `6` Stufen je Schräge):
   - Erzeugt an beiden Giebelseiten (`±Y`) eine echte gemauerte Treppengiebel-Wand,
     die ca. `0,25–0,45 m` über die Dachfläche hinaussteht, mit schmaler
     Abdeckplatte/Stufenkrone (`Stone` Kind 1 oder `wall`) je Stufe.
   - Die Dachfläche (`gable_roof`) schließt bündig hinter den Giebelscheiben ab
     (`verge = 0.0`), sodass die Treppensilhouette von allen Seiten scharf wirkt.
   - **Schadensstufen:** In `Damaged` brechen einzelne obere Stufen und Dachteile
     aus; in `Ruin` bleibt mindestens ein gezackter Treppengiebel-Rest über dem
     Schutt (`_Debris`) als ikonische Silhouette erhalten.
2. **`gable_chimneys = true`** (Default `false`):
   - Setzt gemauerte Schornsteine direkt auf beide Giebelspitzen (`+Y` und `-Y`)
     auf die oberste Treppenstufe (streng `≤ H`, damit `fit = false` bei
     `Damaged` nicht über `Intact` hinausragt).
3. **`lintels = true` & `eave_trim = true`** (Default `false`):
   - `lintels`: helle Werkstein-Fensterstürze und -Sohlbänke (`Stone` Kind 1)
     über/unter Fenstern und Türen, **ohne** Eckquader (`quoins`) zu erzwingen
     – wichtig für glatte Backsteinfassaden.
   - `eave_trim`: schmale, klar abgesetzte weiße/helle Traufleiste entlang der
     Längsseiten unter dem dunklen Schieferdach (und optional kleine helle
     Eingangsstufe an der Tür).
4. **Kräftige weiße Kreuzstock-Fenster (`env.gdshader`):**
   - Prüfen, ob das bestehende `sash_window` (`UV.x = 102`, Prompt 215/219) aus
     50–120 m auf dunklem rotem Backstein ausreichend kontrastiert. Da `sash_window`
     30 % der Rahmen dunkel färbt und feine Sprossen nutzt, eine klare
     Kreuzstock-Variante (z. B. `UV.x = 103`, immer weiß/hellgrau gestrichener
     Rahmen mit gut lesbarem T-/Kreuzstock, anti-aliased über `fwidth`) bzw.
     Blueprint-Option `cross_windows = true` ergänzen, damit die weißen
     Fensterkreuze auf dem Backstein auch aus Weitwinkel-Distanz scharf lesbar
     sind.

### Teil B – Neue Gebäude-Assets, Katalog & Regeln

Historische nordwesteuropäische (niederländisch-flämische) Bautypen, Maße 1:1 in
echten Metern mit Quellenvermerk nach `docs/SOURCES.md` (`uncertain` wo geschätzt),
jeweils `Intact`, `Damaged`, `Ruin`, alle im Dreiecks-Budget (`≤ 3.000 Tris` für
Häuser, `≤ 5.000 Tris` für Kirche):

1. **`env_house_stepped_cottage`** („Treppengiebel-Klinkerhaus“ / „Stepped-gable brick cottage“):
   - Maße ca. **13,0 × 7,5 × 6,8 m** (Länge Y, Breite X, Firsthöhe Z; Traufseite `+X`).
   - Eingeschossiges rotes Backstein-/Klinkerhaus (`wall = "Stone", wall_kind = 3`,
     `[surface] brick = "red", roof = "slate"`, dunkler Ziegelton prüfen), heller
     Beton-/Werksteinsockel (`plinth ≈ 0.55 m`), weiße Traufleiste, 4 weiße
     Kreuzstockfenster + 1 dunkle Tür längs, 2 Kreuzstockfenster je Giebelwand,
     Treppengiebel (`stepped_gable = 5`) + Zwillings-Giebelschornsteine
     (`gable_chimneys = true`), dunkles Schieferdach.
2. **`env_house_stepped_brick`** („Treppengiebel-Backsteinhaus“ / „Stepped-gable brick house“):
   - Maße ca. **13,0 × 8,5 × 9,8 m** (2 Vollgeschosse + Dach).
   - Zweigeschossiges rotes Backstein-Dorfhaus (`[surface] brick = "red", roof = "slate"`),
     heller Sockel, 4–5 Fensterachsen mit weißen Kreuzstockfenstern und hellen
     Stürzen/Sohlbänken, weiße Traufleiste, Treppengiebel (`stepped_gable = 6`)
     + Zwillings-Giebelschornsteine, dunkles Schieferdach.
3. **`env_house_stepped_plaster`** („Treppengiebel-Putzhaus“ / „Stepped-gable plaster house“):
   - Maße ca. **12,5 × 8,5 × 9,6 m** (2 Vollgeschosse + Dach).
   - Zweigeschossiges verputztes Bürger-/Dorfhaus (`wall = "Plaster"`,
     `[surface] plaster = "ochre"` oder `"cream"`, `roof = "slate"`), dunkle
     Sprossenfenster (`sash_windows = true`), Treppengiebel (`stepped_gable = 6`)
     + Zwillings-Giebelschornsteine, dunkles Schieferdach – heller Kontrast
     zwischen den dunklen Backsteinhäusern im Dorfzentrum.
4. **`env_church_brick`** („Backsteinkirche“ / „Brick church“, prüfen/empfohlen):
   - Maße ca. **26,0 × 12,0 × 26,0 m** (Budget `≤ 5.000 Tris`), baut auf `church()`
     in `buildings.py` auf: dunkler roter Backstein (`[surface] brick = "red",
     roof = "slate"`), Strebepfeiler, hohes steiles Schieferdach, schlichter
     Westturm (oder alternativ Backstein-Giebelscheune, falls der Randbau eher
     als großes Backstein-Langhaus wirkt – in der Session anhand der Referenz
     sauber entscheiden und begründen).

Regeleinträge in `data/catalog/neutral.toml`, `data/objects.toml` und
`data/destruction.toml` analog zu `env_house_farm_brick`, `env_house_stone` und
`env_church` (Besetzbarkeit, Deckung, Trefferpunkte, Brennbarkeit, Einsturzstufen
nach Prompt 197 mit passenden Höhenbändern!). Lokalisierung (`godot/locale/`
bzw. `l10n_check.py`) und `just catalog-doc` nachziehen.

## 3. Prüfen und zeigen

- **Vorschau-Blätter (`just building-preview`):**
  - Für jedes neue Gebäude ein vollständiges Blatt (nah + Spielhöhe, Wald,
    Herbst [falls Prompt 223 fertig] und Winter, sowie `Intact`, `Damaged`,
    `Ruin` – besonders die stehenbleibende Treppengiebel-Ruine prüfen!).
  - Stichprobe bestehender Häuser (`env_house_stone`, `env_house_farm_brick`):
    unverändert.
- **Pflicht-Checks (`CLAUDE.md`):**
  - `just validate-assets` und `just glb-check` (Budgets, Maße ±2 %, Varianten).
  - `just import`, `just gallery-check`.
  - `just thumbnails --only=env_house_stepped_cottage,env_house_stepped_brick,env_house_stepped_plaster,env_church_brick`
    (alle neuen IDs) und `just units-web-sync` + `just catalog-doc`.
  - `just check`, `just selftest` grün; Golden-Hash-Änderung ausschließlich durch
    die neuen Katalog-/Regeleinträge belegen und aktualisieren.

## 4. Nicht Teil dieses Prompts

- Schmale 3–4-geschossige Kanal-Reihenhäuser und Windmühlen für das Polderland
  (bleiben in Prompt 151, können `stepped_gable` dann direkt nutzen).
- Herbstlandschaft/Shader (Prompt 223), Artillerie-Rauchbögen (Prompt 225) und
  die fertige Video-Szene (Prompt 226).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/224-treppengiebel-haeuser-backstein-und-putz.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Architektur-Kit & Kreuzstock-Fenster:** `tools/blender/kit/buildings.py` rückwärtskompatibel um `stepped_gable` (inkl. bodenverankerter gezackter Treppengiebel-Ruine ab `damage >= 2`), `gable_chimneys`, `cross_windows`, `lintels`, `eave_trim`, `plinth_kind`, `door_step` sowie `wall_kind` für `church()` erweitert; `cross_window()` (`UV.x > 102.5`) in `godot/assets/shaders/env.gdshader` ergänzt.
- **Vier neue zerstörbare Gebäude:** `env_house_stepped_cottage` (1670/3000 Tris), `env_house_stepped_brick` (2644/3000 Tris), `env_house_stepped_plaster` (2372/3000 Tris) und `env_church_brick` (1962/5000 Tris) als Blueprints in `assets-src/blueprints/neutral/`, registriert in `data/catalog/neutral.toml`, `objects.toml`, `destruction.toml`, `fire.toml`, `infantry.toml`, `vision.toml`, `tools/blender/kit/env.py`, `tools/glb_check.py`, `building_preview.gd` und `editor_data.gd`.
- **Assets, Vorschau & Tests:** Gebaut, validiert (`just validate-env`, `just glb-check` 458 GLBs ok), importiert (`just import`), alle 4 `just building-sheet`-Bögen visuell geprüft, Thumbnails (`godot/assets/thumbnails/env_*.png`), `just units-web-sync` und `just catalog-doc` aktualisiert; erwartete Golden Hashes in `crates/sim/tests/determinism.rs` (`0xda23_c26e_c657_755c`) und `crates/sim/tests/economy.rs` (`0xbd2b_2f9d_1484_eade`) gezielt für die vier neuen Regel-Einträge nachgezogen. Werte vorläufig.
