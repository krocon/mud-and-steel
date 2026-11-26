# Prompt 215 – 3D-Modelle verfeinern: Polygon-Budget umschichten (Panzer, Flugzeuge, Häuser)

> In einer frischen Session im Repo-Ordner ausführen. Benötigt Blender (`docs/VERSIONS.md`).
> Empfohlen nach Prompt 203 (Shader-Fixes) und Prompt 214 (AO-Bake).
> Lies zuerst `CLAUDE.md`, `docs/ASSETS.md` (Abschnitte „Part kit“, „Polygon and texture
> budgets“, „New unit in 5 steps“) und `docs/SOURCES.md`. Im Code:
> `tools/blender/kit/wheels.py`,
> `tools/blender/kit/running_gear.py`,
> `tools/blender/kit/hull.py`,
> `tools/blender/kit/turret.py`,
> `tools/blender/kit/weapons.py`,
> `tools/blender/kit/attachments.py`,
> `tools/blender/kit/aircraft.py`,
> `tools/blender/kit/buildings.py`,
> `tools/blender/kit/blocks.py`,
> `godot/assets/shaders/env.gdshader` (`sash_window`).
> Reine Asset-Verbesserung – **keine Änderungen in `crates/sim`**, Golden Hash unverändert.

## 1. Problem

Die quantitative und visuelle Inspektion der `.glb`-Modelle und Thumbnails
(`de_pz4_h.png`, `de_fw190_a8.png`, `us_m4.glb`, `env_town_hall_med.glb`) zeigt eine
deutliche Schieflage in der Polygon-Verteilung:

1. **Panzer verschenken 40–50 % ihrer Dreiecke an winzige Laufrollen & Antriebsräder, während die Hero-Silhouette karg bleibt:**
   - Beispiel `de_pz4_h.glb` (7.448 Tris): `Sprocket_L` + `Sprocket_R` verbrauchen allein
     **744 Tris** (mehr als **Turm 294 Tris + Kanone 280 Tris zusammen**!), und die 31
     Lauf-/Stützrollen-Nodes verbrauchen **2.930 Tris (40 %)** – obwohl sie beim Pz. IV H
     weitgehend hinter den Schürzen (`Skirt_L/R`) liegen und das Antriebsrad in
     `de_pz4_h.png` von außen wie eine platte gelbe Scheibe wirkt.
   - Gleichzeitig wirken Wannendeck, Turmfront, Blende und Kotflügel recht kastenförmig:
     Motordeck-Lüftungsgitter (Rippen/Lamellen), Sichtschlitze/Periskope, abgesetzte
     Blendenkragen/Rohrwiegen und markantere Naben-/Speichendetails an Sprockets fehlen oder
     sind bloße Quader.
2. **Ältere Flugzeug-Blueprints wirken deutlich kantiger als der Prompt-202-Stand:**
   - Beispiel `de_fw190_a8.png`: kantiger Sternmotor-Rumpf (wenige Ringsegmente), rechteckige
     Flügel- und Leitwerksenden statt abgerundeter Fw-190-Form, platte graue Strich-Propeller
     ohne Blattprofil/Spinner-Übergang, kastenförmige Kanzel ohne Strebenrahmen und dünne
     Stab-Fahrwerksbeine.
   - Dabei hat `kit/aircraft.py` in Prompt 202 (für die Spitfire `uk_spitfire_ix`) bereits
     `ground_angle = "fit"`, `fin_outline`/`rudder_outline`, `planform = "elliptical"`,
     verbesserte `propeller` (`spinner`, `blade_width`) und `canopy` (`profile`) bekommen!
3. **Gebäude in `buildings.py` verschwenden ~72 Tris pro Fenster:**
   - `tools/blender/kit/buildings.py:112-123` baut jedes Fenster aus **6 vollen 3D-Boxen**
     (Glas + vertikaler Pfosten + Querholz + Fensterbank + 2 Fensterläden). Bei 60–90
     Fenstern kommt `env_town_hall_med.glb` auf **12.764 Tris gesamt** (4.648 Tris allein für
     `Intact_Walls`)!
   - Wohnblöcke (`kit/blocks.py`) zeichnen dagegen mit `sash_window()` (`env.gdshader:150-173`,
     `Glass` mit `UV.x = 102 + ...`) gestochen scharfe, anti-aliasierte Sprossenfenster mit
     verschiedenen Rahmenfarben und Vorhängen auf **einem einzigen Quad (2 Tris)**.

## 2. Ziel

### Teil A – Panzer-Baukasten: Budget umschichten & Silhouette schärfen
1. **Laufwerk schlanker & lesbarer machen (`wheels.py`, `running_gear.py`):**
   - `sprocket`: Zähne vereinfachen (weniger unsichtbare Innenflächen/Zahn-Polygone;
     optional abgesetzte Metall-Nabe / Speichenandeutung statt platter gelber Scheibe) →
     Ziel: `Sprocket_L+R` von ~744 Tris auf ~240–320 Tris senken.
   - Verdeckte Innenflächen von Zwillingsrollen/Stützrollen (besonders hinter Schürzen)
     weiter straffen; `segments(r)` prüfen, ob kleine Rollen mit 8–10 Segmenten auskommen.
2. **Eingesparte Dreiecke in sichtbare Hero-Details investieren (rückwärtskompatibel im Kit!):**
   - **Motordeck-Lüftungsgitter** (neues kleines Kit-Detail oder Option an `deck_box`, z. B.
     `grille = true`: dunkle Metall-Lamellen/Vertiefung statt glatter Paint-Box).
   - **Sichtblöcke/Periskope & Fahrer-Sehschlitz** mit schmaler Metall-/Glas-Blende.
   - **Kanonenblende & Rohr** (`weapons.py`): sauberer Blendenkragen/Übergang zur Rohrwiege
     und realistischere Verjüngung; bei `double_baffle`-Mündungsbremsen seitliche Öffnungen
     dunkel absetzen, damit sie aus Spielhöhe als Mündungsbremse lesbar sind.
   - An den Kern-Panzern (`de_pz4_h`, `de_tiger1_e`, `de_panther_g`, `su_t34_76`, `su_t34_85`,
     `us_m4`, `us_m4a3_76w_hvss`, `uk_cromwell` o. ä.) exemplarisch anwenden und alle
     Budgets (`docs/ASSETS.md`) strikt einhalten.

### Teil B – Ältere Flugzeuge auf Prompt-202-Qualität heben
1. `kit/aircraft.py`:
   - Rumpf-Standard-`segments` (z. B. 16 statt 12 für Sternmotor-Nasen) und saubere
     Smooth-Übergänge; Kanzel-Streben (schmale Paint-Bügel über `Glass`) optional per Parameter;
     Propellerblätter mit realistischer Breite/Verjüngung statt dünner Stäbe.
2. Die älteren Jäger-/Schlachtflugzeug-/Bomber-Blueprints (mindestens `de_fw190_a8`,
   `de_bf109_g6`, `de_ju87_d`, `su_yak3`, `su_il2_m3`, `us_p51d`, `us_p47d`) mit
   `ground_angle = "fit"`, sauberen `fin_outline`/`rudder_outline`, runden Randbögen und
   schöneren Propellern/Kanzeln aufrüsten (Katalogmaße ±2 % und Budgets bleiben exakt!).

### Teil C – Gebäude-Fenster in `buildings.py` auf `sash_window`-Quads umstellen
1. In `tools/blender/kit/buildings.py` für intakte/beschädigte Fenster (`damage < 2`) das
   `sash_window`-Quad aus `env.gdshader` (`Glass` mit `UV.x = 102 + u`, `UV.y = v`)
   einsetzen (plus schlanke Fensterbank und ggf. flache 2-Tri-Fensterläden bei `shutters=true`
   statt 12-Tri-Boxen).
2. Sicherstellen, dass Gebäude-Dreieckszahlen bei großen Häusern (`env_town_hall_med`,
   `env_house_med_row_corner`, `env_town_hall`, `env_station`) um **~40–65 %** sinken und
   die Optik aus Spielhöhe *und* Nahansicht gewinnt (Vorher/Nachher mit `just building-preview`!).
3. Prüfen, ob zerstörte Fenster (`damage == 2`, Ruinen) weiterhin echte Öffnungen bleiben
   (wichtig für Sicht ins Innere und Besatzung!).

## 3. Prüfen und zeigen

- **Vorher/Nachher-Vergleiche:**
  - Turntables & `just ref-compare` für die überarbeiteten Panzer und Flugzeuge
    (insbesondere `de_pz4_h`, `de_fw190_a8`, `us_m4`).
  - `just building-preview` für `env_town_hall_med`, `env_house_town`, `env_house_farm`.
  - Tabelle der Dreieckszahlen vorher/nachher.
- **Pflichtpipeline (`CLAUDE.md`):**
  - `just validate-assets` (alle Budgets & ±2 %-Katalogmaße grün!).
  - `just import`, `just gallery-check`.
  - `just thumbnails --only=<alle geänderten ids>` und `just units-web-sync`.
  - `just check`, `just selftest` grün; Golden Hash unverändert.

## 4. Nicht Teil dieses Prompts

- Neue Katalog-Einheiten oder Maßänderungen im Katalog (`data/catalog/*.toml` bleibt unverändert).
- Soldaten-Modelle (Prompt 213).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/215-3d-detail-budget-panzer-flugzeuge-und-haeuser.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (2026-11-26)

- **Teil A – Panzer-Baukasten (Budget umschichten & Silhouette schärfen):**
  - `tools/blender/kit/wheels.py`: `sprocket()` begrenzt die sichtbaren Zähne auf max. 12 an einem äußeren Metall-Zahnring, ersetzt den verdeckten inneren Zahnring durch eine 10-eckige Trommel und fügt eine versenkte Metall-Nabe plus 4 Speichenrippen hinzu (bündig mit `x_out`, ohne Breitenänderung); `segments(r)` gestrafft (`14 / 10 / 8`).
  - `tools/blender/kit/hull.py`: `deck_box` rückwärtskompatibel um `grille = true` & `louvers` erweitert (gerahmtes dunkles Metall-Lamellengitter bei identischen Außenmaßen).
  - `tools/blender/kit/attachments.py`: neues `@part("vision_block")` (gepanzerte Periskop-/Sehschlitz-Haube mit `Glass`-Spalt und Metall-Blende).
  - `tools/blender/kit/weapons.py`: Blendenkragen am Rohrwiegen-Übergang in `tank_gun` (für Rohre ≥ 0,15 m, sodass MG-Stummel nicht über `Socket_Muzzle` hinausragen) und dunkle seitliche Öffnungen (`Rubber`) an `double_baffle`/`double_baffle_flat`/`single_baffle`-Mündungsbremsen.
  - Auf die 8 Kern-Panzer angewendet (`de_pz4_h`, `de_tiger1_e`, `de_panther_g`, `su_t34_76_1943`, `su_t34_85_1944`, `us_m4`, `us_m4a3_76w_hvss`, `uk_cromwell`).
- **Teil B – Flugzeuge auf Prompt-202-Qualität:**
  - `tools/blender/kit/aircraft.py`: `fuselage` Standard `segments = 16`, `wing`/`tail` um `planform = "rounded"` (runde Randbögen bei exakter Spannweite) erweitert, `propeller` mit 3-Stationen-Blattprofil/Verjüngung, `canopy` um optionale `frames`-Strebenbügel erweitert, `landing_gear` mit zweistufigem Federbein und `ground_z`-Unterstützung bei `spats = true`.
  - Alle 6 älteren Jäger-/Schlachtflugzeug-Blueprints (`de_fw190_a8`, `de_ju87_d5`, `su_la5fn`, `su_il2m3`, `us_p51d`, `us_p47d`) auf `ground_angle = "fit"`, `fin_outline`/`rudder_outline`, runde/elliptische Randbögen, Spinner, breite Blätter und Kanzelprofile/-streben umgestellt (alle innerhalb ±2 % Katalogmaß & Budget).
- **Teil C – Gebäude-Fenster in `buildings.py`:**
  - `self.sash_windows` standardmäßig `True` für alle intakten/beschädigten Fenster (`damage < 2` → 1-Quad-`sash_window` mit `UV.x = 102..103`), flache 2-Tri-Fensterläden (`_shutter_quad` statt 12-Tri-Boxen); Ruinen (`damage == 2`) bleiben echte offene Durchbrüche.
- **Dreieckszahlen vorher / nachher:**
  - `de_pz4_h.glb`: **7 448 → 7 052 Tris** (`Sprocket` 744 → 404, `Wheels/Rollers/Idlers` 2 720 → 2 384, `Hull` 1 660 → 1 888, `Gun` 280 → 332).
  - `de_tiger1_e.glb`: **6 524 → 6 268 Tris** (`Sprocket` 680 → 404, `Wheels` 2 496 → 2 224, `Hull` 1 048 → 1 288).
  - `us_m4.glb`: **6 332 → 6 256 Tris** (`Sprocket` 520 → 404, `Wheels` 2 112 → 1 856, `Hull` 1 808 → 2 076).
  - `uk_cromwell.glb`: **4 796 → 4 580 Tris** (`Sprocket` 616 → 404, `Hull` 588 → 788).
  - Flugzeuge (Budget 6 000): `de_fw190_a8` 1 658 → 1 886, `de_ju87_d5` 1 950 → 2 250, `su_la5fn` 1 674, `su_il2m3` 3 266, `us_p51d` 1 694 → 1 982, `us_p47d` 2 874 → 3 322 Tris.
  - Gebäude (`Intact_Walls` / gesamt): `env_town_hall_med` **4 648 → 2 846 (−38,8 %)** / 12 764 → 9 296, `env_house_med_row_corner` **3 544 → 1 924 (−45,7 %)** / 9 524 → 6 554, `env_town_hall` **2 464 → 1 506 (−38,9 %)** / 7 748 → 5 934, `env_station` **2 752 → 1 698 (−38,3 %)** / 9 380 → 7 374, `env_house_town` **2 128 → 1 346 (−36,7 %)** / 6 776 → 5 348.
- **Prüfung:** `just validate-assets`, `just import`, `just thumbnails`, hochauflösende Godot-Prüfbilder (`de_pz4_h`, `us_m4`, `de_fw190_a8`, `us_p51d`, `env_town_hall_med`, `env_house_town`), `just units-web-sync` und `just selftest` grün; `crates/sim` und Golden Hashes unverändert.
