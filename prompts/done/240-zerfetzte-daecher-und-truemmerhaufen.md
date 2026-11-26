# Prompt 240 – Zerfetzte Ziegeldächer mit offenen Dachstühlen und Ziegel-Trümmerhaufen

> In einer frischen Session im Repo-Ordner ausführen. Benötigt Blender
> (`docs/VERSIONS.md`). Lies zuerst `CLAUDE.md`, `docs/ASSETS.md` (Abschnitte
> „Polygon and texture budgets“, „Environment objects“, „Surfaces per building“)
> sowie `prompts/done/219-normandie-dorfhaeuser-und-kirche.md` und
> `prompts/done/075-dach-patina.md`.
> Im Code:
> `tools/blender/kit/buildings.py` (`gable_roof`, `rubble`, `ruin_top`, `house`),
> `tools/blender/kit/env.py`,
> `godot/assets/shaders/env.gdshader`,
> `godot/assets/shaders/env_roof.gdshaderinc`,
> `assets-src/blueprints/neutral/env_house_farm.toml`,
> `assets-src/blueprints/neutral/env_house_farm_brick.toml`,
> `assets-src/blueprints/neutral/env_barn.toml`,
> `assets-src/blueprints/neutral/env_house_log.toml`,
> `assets-src/blueprints/neutral/env_house_thatch.toml`,
> `assets-src/blueprints/neutral/env_house_stone.toml`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/This WWII RTS is Pure Chaos #panzerstrike [lskjKn-C9yo].webm`
> (Sekunden `00:00–00:02` und `00:05–00:09`: Fachwerk- und Backstein-Bauernhäuser
> mit aufgerissenen roten Ziegeldächern, unregelmäßigen Einschusslöchern,
> freiliegenden zerbrochenen dunklen Dachsparren/Latten über schwarzem
> Dachboden-Inneren, teilweise weggesprengten Giebelspitzen sowie dichten,
> roten Ziegel- und Holzbalken-Trümmerhaufen an eingestürzten Häusern).
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> **Wichtig:** Reine Modellbau- und Darstellungsverbesserung an bestehenden
> Gebäuden – **keine Änderungen in `crates/sim`**, Golden Hashes unverändert!

## 1. Problem

In der Referenz erzeugen die **halbzerschossenen roten Ziegeldächer und
eingestürzten Dorfruinen** auf den ersten Blick das Bild eines stundenlang
umkämpften Frontdorfs. In unserem Baukasten (`tools/blender/kit/buildings.py`)
wirken `Damaged` (`damage == 1`) und `Ruin` (`damage == 2`) ländlicher Häuser
dagegen noch deutlich zu geometrisch und brav:

1. **Glatte Vollstreifen-Lücken in `gable_roof()`:**
   - Bei `damage == 1` lässt `gable_roof()` aktuell einfach 2–3 ganze
     Rechteck-Streifen (`strips = 8`) sauber von der Traufe bis zum First weg
     und setzt zwei parallele, unversehrte Holzbalken quer durch die Lücke.
   - Es fehlen **unregelmäßige Einschusslöcher mitten in der Dachfläche**
     (mit ausgefransten/gezackten Ziegelrändern), **gebrochene Sparren und
     Querlatten**, ein **dunkler, geschwärzter Dachboden-Untergrund** (damit
     man durch die Dachlöcher in ein tiefschwarzes ausgebranntes Inneres
     blickt statt auf helle Innenwände) und **Ruß-/Brandspuren** direkt um
     die Dachdurchbrüche.
2. **Teil-Schäden an Giebeln & First:**
   - Bei `damage == 1` bleibt der Giebel (`gable()`) bisher vollständig intakt
     und symmetrisch; in der Referenz fehlen oft einzelne Dach-Ecken, ein Stück
     First oder eine Ecke der Giebelwand, durch die das Dachgebälk frei herausragt.
3. **Trümmerhaufen (`Ruin`, `damage == 2`) zu dünn:**
   - Unsere `Ruin`-Varianten ländlicher Häuser zeigen oft hohe glatte Restwände
     mit nur wenigen kleinen `rubble()`-Quadern. In der Referenz liegen an
     komplett zerstörten Häusern **satt geschichtete Haufen aus roten Ziegeln,
     Putzbrocken und kreuz und quer liegenden verkohlten Dachbalken** innerhalb
     niedriger, gezackter Mauerreste.

## 2. Ziel

### Teil A – Realistisch zerfetztes Satteldach in `tools/blender/kit/buildings.py`

1. **2D-Dachraster mit ausgefransten Einschusslöchern (`damage == 1`):**
   - `gable_roof()` für `damage == 1` (standardmäßig aktiv für alle
     Satteldach-Häuser oder sauber über `strips` × `rows`, z. B. 8–10 Streifen
     längs × 4–5 Reihen von Traufe bis First, sodass Dreiecksbudget locker
     eingehalten bleibt!):
     - Deterministisch aus `self.r` 1–3 **unregelmäßige Einschuss-Cluster**
       (jeweils 2–5 angrenzende Rasterzellen, teils mitten in der Dachfläche,
       teils bis an First oder Traufe reichend) aussparen.
     - Benachbarte Dachzellen am Lochrand leicht unregelmäßig anschrägen bzw.
       kleine gekippte/abgerutschte Dachplatten-Stücke am Lochrand ergänzen.
     - **Freiliegender Dachstuhl im Loch:** Dunkle/verkohlte Sparren (entlang
       der Schräge, teils abgebrochen mit gezacktem Ende) plus 2–3 dünne
       Querlatten (parallel zum First) quer über die Einschusslöcher legen.
     - **Geschwärztes Dachboden-Inneres:** Eine leicht vertiefte dunkle
       Dachboden-Decke / verkohlte Innenverkleidung (Vertex-Farbe / `Wood`
       `kind = 3` verkohlt) direkt unter dem Dach, sodass die Löcher aus
       isometrischer Draufsicht tief, dunkel und kontrastreich wirken.
     - **Gebrochener First:** Den Firstziegel (`z_ridge`) bei `damage == 1`
       über den betroffenen Einschussstellen unterbrechen statt ganz wegzulassen.
2. **Angeschlagene Giebelecken bei `damage == 1`:**
   - Optional (deterministisch aus `self.r`) an einer Giebelseite eine Ecke
     oder obere Kante leicht ausbrechen lassen, falls ein Dachloch bis an den
     Giebel reicht – natürlich immer innerhalb der `Intact`-Hülle (`fit = false`).
3. **Satte Ziegel- und Balken-Trümmerhaufen bei `damage == 2` (`Ruin`):**
   - `rubble()` erweitern (rückwärtskompatibel über Standardwerte oder für
     ländliche Häuser):
     - Flächiger, leicht hügeliger Schuttsockel im Grundriss (gemischt aus
       roten Ziegel-/Stein- und dunklen Schutt-Clustern, Material `Roof` +
       `Stone` / `wall_mat`),
     - Kreuz und quer hineingekippte, teils verkohlte **Holzbalken und
       Sparrenreste** (`Wood`, `kind = 3`), die schräg aus dem Schutt ragen,
     - Niedrige, deutlich gezackte Mauerreste (`ruin_top` mit markanteren
       Ausbrüchen), damit eine totale Dorfruine von oben sofort als
       Trümmerhaufen neben halbzerstörten Häusern lesbar ist.

### Teil B – Ruß-Akzent im Dach-Shader (optional, geringster Aufwand)

- Prüfen, ob `env_roof.gdshaderinc` / `env.gdshader` bereits `env_char` (aus
  Prompt 020/078) auf `Damaged`/`Ruin`-Dächern sichtbar genug darstellt; falls
  Dachränder um Einschusslöcher noch zu sauber hellrot wirken, über bestehende
  Vertex-Farbe (`COLOR.r` / AO-Bake aus `bake.py`) oder eine gezielte
  Schmauch-Abdunklung an beschädigten Dachkanten nachschärfen – **ohne**
  `Intact`-Dächer zu verändern!

### Teil C – Ländliche Dorfgebäude neu bauen & prüfen

- Alle ländlichen Dorfgebäude mit Satteldach neu aus Blender bauen und
  exportieren:
  - `env_house_farm` (Fachwerk-Bauernhaus),
  - `env_house_farm_brick` (rotes Backstein-Bauernhaus),
  - `env_barn` (Holzscheune),
  - `env_house_log` (Blockhaus),
  - `env_house_thatch` (Reetdach-Kate),
  - `env_house_stone` (ländliches Steinhaus).
- **Strenge Qualitäts- und Budget-Kontrolle:**
  - `just validate-assets` und `just glb-check` komplett grün (alle
    Dreiecksbudgets `≤ 3000 Tris` je Variante eingehalten, keine schwebenden
    Teile, `Damaged`/`Ruin` innerhalb `Intact`-Fußabdruck!).
  - `building_collapse.gd` (`just collapse-shots` oder Selftest) prüfen, dass
    der schichtweise Einsturz mit den neuen `Damaged`-/`Ruin`-Meshes sauber
    aussieht.
  - `just thumbnails --only=env_house_farm,env_house_farm_brick,env_barn,env_house_log,env_house_thatch,env_house_stone`
    und `just units-web-sync` ausführen.

## 3. Prüfen und zeigen

- **Vorher/Nachher-Übersichtsbogen** (`just building-sheet` oder lokaler
  Kontaktbogen in `build/roof_damage_compare/`) für `env_house_farm`,
  `env_house_farm_brick`, `env_barn` und `env_house_log` jeweils in `Intact`,
  `Damaged` und `Ruin` aus steiler RTS-Draufsicht (`pitch ≈ 55°`).
- **Tests:**
  - `just validate-assets`, `just glb-check`, `just check`, `just selftest`
    grün; Golden Hashes unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an `crates/sim` oder Schadens-/Trefferpunkt-Werten in
  `data/destruction.toml`.
- Neue Gebäudetypen oder Dorf-Props (Prompt 241).
- Treppengiebel-Gebäude (Prompt 224) oder Wüsten-/Stadt-Hochhäuser umbauen
  (falls `gable_roof()` sie mit verbessert, sicherstellen, dass ihre
  `validate-assets`-Budgets grün bleiben!).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/240-zerfetzte-daecher-und-truemmerhaufen.md prompts/done/`
verschieben, Ergebnisabschnitt (Dreieckszahlen vorher/nachher, neue
`gable_roof`/`rubble`-Parameter, Vergleichsbilder in `build/`) ergänzen und den
Status in `prompts/README.md` auf „erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Zerfetzte Satteldächer & Trümmerhaufen (`tools/blender/kit/buildings.py`):**
  - `House.gable_roof()` für `damage == 1` auf ein 2D-Raster (`strips × 3`) umgestellt: unregelmäßige Einschusslöcher mitten in der Dachfläche und an Rändern, freiliegende verkohlte Dachsparren und Querlatten, tiefdunkler geschwärzter Dachboden-Untergrund sowie unterbrochener First.
  - `House.rubble()` für `damage >= 2` um einen flächigen Schuttsockel aus roten Dachziegel- (`Roof`) und Mauerbrocken sowie schräg gekreuzte verkohlte Dachbalken erweitert.
- **Neu gebaute & geprüfte Gebäude:**
  - `env_house_farm`, `env_house_farm_brick`, `env_barn`, `env_house_log`, `env_house_thatch`, `env_house_stone` in Blender neu gebaut, importiert, Thumbnails neu gerendert und visuell geprüft (`build/preview/env_house_farm/`, `env_house_farm_brick/`).
- **Prüfung:** Alle 6 Gebäude unter Budget (`≤ 3000 Tris`), `just validate-env`, `just glb-check`, `just units-web-sync`, `just check` und `just selftest` grün; keine Änderungen in `crates/sim`.
