# Prompt 008 – Restliche Einheiten Stufe 2 (parallel)

> **Dieser Prompt läuft neunmal parallel**: drei Sessions pro Fraktion, jede in
> einem eigenen Git-Worktree/Branch `assets/<fraktion>-<bereich>`:
>
> - „Bitte führe `prompts/008-einheiten-stufe2.md` für `de`, Bereich `vehicles` aus.“
> - „… für `de`, Bereich `guns` aus.“
> - „… für `de`, Bereich `infantry_air` aus.“
> - dasselbe für `su` und `us`.
>
> Voraussetzungen: Prompts 003–007 sind erledigt, die Vorab-Session von 008 (Baukasten
> Stufe 2) ist auf `main` gemergt. Lies `CLAUDE.md`, `docs/ASSETS.md` (vor allem die
> Abschnitte „Part kit“, „Prompt 008 assemblies“ und „Hierarchy and names“),
> `docs/INFANTRY.md`, `docs/SCALE.md`, `docs/SOURCES.md` und den Katalog deiner Fraktion.

## 1. Ziel

Alle Katalog-Einträge mit `tier = 2` als fertige, geprüfte Modelle in allen drei
Tarnungen. Danach hat jeder Katalog-Eintrag ein Modell oder eine begründete Lücke im
Bericht.

## 2. Aufteilung

Warum diese Aufteilung statt `vehicles_guns` / `infantry` / `aircraft_fortifications`:
Die Infanterie ist nach der Vorab-Session fast nur noch Trupp-Dateien (alle Soldaten-
Rollen sind gebaut), dafür sind Fahrzeuge und Geschütze zusammen elf Modelle pro
Fraktion. So hat jede Session ähnlich viel Arbeit (sieben Modelle, sieben Modelle,
zehn Trupps plus vier Flugzeuge), und Geschütze und Befestigungen teilen die Werkzeuge
für statische Stellungen (Besatzungs-Sockets, Umwelt-Generatoren).

Bereits als Baukasten-Beispiel gebaut (nur prüfen und verbessern, nicht neu anlegen):
`de_stug3_g`, `de_bunker_tobruk`, `de_minefield_at`, `su_ba64b`, `us_bofors_m1`,
`us_m2_42in`, `us_l4`.

### `vehicles` – Panzer, Jagdpanzer, Sturmgeschütze, Selbstfahrlafetten, Rad-Fahrzeuge

| Fraktion | Einheit | Baugruppe und Teile (Vorschlag) |
|---|---|---|
| `de` | `de_pz2_l` Luchs | `tank`, `interleaved` (rows 2), `plate_turret` |
| `de` | `de_tiger1_e` | `tank`, `interleaved` (rows 3), `plate_turret` (Hufeisen-Grundriss), Mündungsbremse `double_baffle` |
| `de` | `de_jgdpz38t` Hetzer | `casemate` (`top_plan` für die stark geneigten Platten), `leaf_spring_bogie` (große Räder), Kanone mit `x`-Versatz rechts, Blende `saukopf`, MG `pintle_mg` (Fernbedienung) |
| `de` | `de_stug3_g` | Beispiel vorhanden: Details (Schürzen, Ketten-Glieder, Antenne) |
| `de` | `de_opel_blitz` | `wheeled` (wie `su_zis5`) |
| `de` | `de_sdkfz222` | `armored_car`, `plate_turret` mit `open_top`, Gitter-Aufsatz als Faktions-Teil |
| `de` | `de_wespe` | `casemate` mit `open = true`, `torsion_bar` (`twin = false`, Pz II), Blende `ball` oder `box` |
| `su` | `su_t70m` | `tank`, `torsion_bar`, `plate_turret` |
| `su` | `su_is2` | `tank`, `torsion_bar`, `cast_turret`, DShK als `mg` mit `pintle` |
| `su` | `su_su85` | `casemate` auf T-34-Fahrgestell (`christie`), Blende `ball` |
| `su` | `su_su76m` | `casemate` mit `open = true`, `torsion_bar` |
| `su` | `su_isu152` | `casemate`, `torsion_bar`, Blende `box` |
| `su` | `su_ba64b` | Beispiel vorhanden |
| `su` | `su_m3a1_scout` | `armored_car` ohne Turm, `section_hull` mit `open`, MGs als `pintle_mg` (Katalog-Kategorie `halftrack` passt nicht: der M3A1 Scout Car ist ein 4×4-Radfahrzeug; im Bericht melden, Katalog nur mit Begründung ändern) |
| `us` | `us_m5a1` | `tank`, `vvss_bogie`, `plate_turret` |
| `us` | `us_m26` | `tank`, `torsion_bar` mit Rücklaufrollen, `cast_turret` |
| `us` | `us_m10` | `tank`, `vvss_bogie`, `plate_turret` mit `open_top` (Fünfeck) |
| `us` | `us_m4a3_105` | `tank` auf Basis von `us_m4a3_75`, Haubitzen-Rohr |
| `us` | `us_gmc_cckw` | `wheeled` mit drei Achsen (`twin` hinten) |
| `us` | `us_m8_greyhound` | `armored_car` mit drei Achsen, `plate_turret` mit `open_top` |
| `us` | `us_m7_priest` | `casemate` mit `open = true`, `vvss_bogie`, MG-Kanzel (`cylinder_prop` + `pintle_mg`) |

### `guns` – Flak, Haubitzen, schwere Mörser, Befestigungen

| Fraktion | Einheit | Baugruppe und Teile (Vorschlag) |
|---|---|---|
| `de` | `de_flak38` | `aa_gun`, kleine Plattform, `aa_mount` mit Schild |
| `de` | `de_flak36` | `aa_gun`, `cruciform_platform`, zwei Achsen (Sonderanhänger) optional |
| `de` | `de_lefh18` | `towed_gun` (`split_trail_carriage`, `spokes` > 0 nur wenn belegt) |
| `de` | `de_grw42` | `mortar`, Bodenplatte rund oder eckig, Transport-Räder optional |
| `de` | `de_bunker_tobruk`, `de_minefield_at` | Beispiele vorhanden |
| `de` | `de_minefield_ap` | `minefield` mit `mine = "ap"` |
| `su` | `su_61k`, `su_52k` | `aa_gun` (vier Räder, Ausleger) |
| `su` | `su_m30` | `towed_gun` |
| `su` | `su_pm38` | `mortar` mit Rad-Fahrgestell (`[[axle]]`) |
| `su` | `su_bunker_dzot` | `log_bunker` (`walls = "logs"`) |
| `su` | `su_minefield_at`, `su_minefield_ap` | `minefield` |
| `us` | `us_bofors_m1`, `us_m2_42in` | Beispiele vorhanden |
| `us` | `us_90mm_m2` | `aa_gun`, `cruciform_platform`, Achsen |
| `us` | `us_m2a1_105` | `towed_gun` |
| `us` | `us_bunker_mg` | `log_bunker` (`walls = "sandbags"`) |
| `us` | `us_minefield_at`, `us_minefield_ap` | `minefield` |

Befestigungen sind Umweltobjekte in deinem Fraktionsordner: Bauplan mit
`assembly = "env"`, bauen mit `just env <id>`, prüfen mit `just validate-env <id>`.
Minenfelder haben im Katalog Höhe 0; die Variante gibt die Markierungshöhe als
`height_m` an.

### `infantry_air` – Trupps und Flugzeuge

Alle Soldaten-Rollen für die Stufe-2-Trupps sind gebaut (`just roles <fraktion>` zeigt
Rollen, Waffen und die noch fehlenden Trupp-Dateien). Pro Trupp eine Datei
`assets-src/blueprints/<fraktion>/squads/<id>.toml` wie in Prompt 006, Mitglieder laut
Katalog-Waffenliste, dann `just squad <fraktion> <id>`.

| Fraktion | Trupps (je `*_inf_*`) | Flugzeuge |
|---|---|---|
| `de` | `hmg_team`, `mortar_light`, `mortar_medium`, `recon`, `sniper`, `medic`, `engineer`, `at`, `flamethrower`, `command` | `de_fw190_a8` (`fixed_guns`, `bomb_load`), `de_ju87_d5` (`wing` mit `kink`, `landing_gear` mit `fixed` und `spats`, `bomb_load` mit `swing_arm`, `flex_mg` mit `twin`), `de_he111_h16` (`fuselage` mit `glass_from_y`, `nacelle`, `bomb_bay`, `flex_mg`), `de_fi156` (`wing_struts`, `fixed`) |
| `su` | dieselben zehn | `su_la5fn`, `su_il2m3` (`rocket_rails`, `bomb_bay`/`bomb_load`, `flex_mg`), `su_pe2` (`nacelle`, `bomb_bay`, `twin_fins`), `su_po2` (`upper_wing`, `wing_struts`, `skid`) |
| `us` | dieselben zehn | `us_p51d`, `us_p47d` (`rocket_rails`, `bomb_load`), `us_b25j` (`nacelle`, `landing_gear` mit `nose`, `gun_turret`, `bomb_bay`, `twin_fins`), `us_l4` (Beispiel vorhanden) |

Rollen-Hinweise: Pistolen-Schützen ohne Langwaffe sind `ammo_carrier` (de, us);
Karabiner-Schützen `carbineer` (su Mosin M1938, us M1 Carbine); Späher und
Scharfschützen-Beobachter `scout`; Offiziere `officer`; Panzerabwehr-Helfer
`at_assistant`; zweiter Mann am Flammenwerfer ist ein normaler `rifleman` bzw.
`smg_gunner`. Abweichungen (fehlende Waffe, ähnlichste Rolle) in der Trupp-Datei
begründen, wie in Prompt 006.

Flugzeuge: Jedes Flugzeug bekommt `shadow_proxy` als letztes `[[part]]`, mindestens ein
`Socket_Fire_NN`, Hoheitszeichen auf Rumpf und Tragflächen. Fahrwerk ausgefahren
(Modell steht darauf); einziehbare Fahrwerke tragen das automatisch in den Extras
ein. Propeller als eigenes Objekt (`Propeller`, bei zwei Motoren `Propeller_L/_R`).
Bomben und Raketen als `Bomb_NN` / `Rocket_NN` mit Abwurf-Sockets, interne Bomben nur
als Sockets im `bomb_bay`.

## 3. Regeln für die parallele Arbeit

- Nur in eigene Pfade schreiben: deine Baupläne in `assets-src/blueprints/<fraktion>/`
  (Bereich `infantry_air` zusätzlich `squads/`), die zugehörigen `.blend` in
  `assets-src/blender/<fraktion>/`, Modelle und Trupp-Szenen in
  `godot/assets/models/<fraktion>/`, die `model`-Zeilen **deiner** Einträge in
  `data/catalog/<fraktion>.toml`, und dein eigenes Teile-Modul
  `tools/blender/kit/parts_<fraktion>_<vehicles|guns|air>.py` (bereits angelegt und
  registriert; Teile mit Präfix `<fraktion>.`, Umwelt-Generatoren mit
  `@env_part("<fraktion>_<name>")`).
- `parts_<fraktion>.py` und alle gemeinsamen Baukasten-Dateien nicht ändern. Fehlt
  etwas Grundsätzliches im gemeinsamen Baukasten, baue es als Faktions-Teil in deinem
  Modul und schlage die Verallgemeinerung im Bericht vor.
- Soldaten: keine neuen Rollen, außer es geht wirklich nicht anders; dann am Ende von
  `tools/blender/soldier/loadouts.py` in einem Block `# --- prompt 008 <fraktion>` (nur
  `infantry_air`; bekannter, trivialer Merge-Konflikt) und Soldat mit
  `just soldier <fraktion> <rolle>` bauen.
- `docs/`, `justfile`, `prompts/`, Galerie- und Import-Skripte nicht ändern.
  `docs/UNIT_CATALOG.md` zeigt keine `model`-Felder, bleibt beim Eintragen der Modelle
  also unverändert. Nur wenn du Katalogwerte korrigierst (mit Quelle), `just
  catalog-doc` ausführen und das im Bericht nennen; die zusammenführende Session
  erzeugt das Dokument ohnehin neu.
- Prompt 011 (Terrain) und 012 (Wegfindung) laufen eventuell parallel: `crates/`,
  `godot/scenes/terrain/` und die Bridge nicht anfassen.
- Commits nur auf dem eigenen Branch. Kein Merge, kein Push. Binärdateien über Git LFS.
- Hand-over-Notiz in den gemeinsamen Notizordner der Session-Gruppe
  (`008-<fraktion>-<bereich>-handover.md`): Branch, Commit, fertige Einheiten,
  berührte Dateien außerhalb der eigenen Pfade (sollte keine geben).

## 4. Arbeitsweise pro Einheit

Wie in `docs/ASSETS.md`, „New unit in 5 steps“: Katalogwerte prüfen, nächstliegenden
Bauplan kopieren (die Beispiele der Vorab-Session zeigen jede neue Baugruppe), bauen
(`just unit <fraktion> <id>`, Vorschau in `build/renders/`), prüfen
(`just validate-assets`), in Godot ansehen (`just import`, `just gallery`).
Katalogwerte nie im Bauplan wiederholen; Schätzwerte im Bauplan-Kommentar mit Herkunft.
Typische Merkmale müssen in der Draufsicht erkennbar sein.

## 5. Akzeptanzkriterien (pro Session)

1. `just check`, `just validate-assets`, `just kit-test`, `just gallery-check` und
   `just selftest` grün auf dem Branch (`infantry_air` zusätzlich
   `just infantry-selftest`).
2. Alle Einheiten des Bereichs in der Galerie, in allen drei Tarnungen; Trupps mit
   Animationen; Befestigungen unter „Environment“.
3. Aufstellung des Bereichs mit Referenzfigur im Scratchpad (`just lineup`,
   `just env-lineup`, `just soldier-lineup`); im Bericht die Größenverhältnisse.
4. Bericht: fertige Einheiten, Abweichungen von historischen Vorlagen, unsichere
   Daten, vorgeschlagene Baukasten-Erweiterungen, Lücken mit Begründung.

## 6. Zusammenführen (eigene kurze Session, wenn alle neun fertig sind)

„Bitte führe die Branches `assets/*-vehicles`, `assets/*-guns`, `assets/*-infantry_air`
zusammen.“ Diese Session merged nacheinander (Konflikte erwartet nur in
`data/catalog/<fraktion>.toml`, falls benachbarte Zeilen, und im `loadouts.py`-Block),
erzeugt `docs/UNIT_CATALOG.md` neu (`just catalog-doc`), führt `just import` aus, lässt
alle Prüfungen laufen (`just check`, `just validate-assets`, `just kit-test`,
`just gallery-check`, `just infantry-selftest`, `just selftest`), rendert eine
Gesamt-Aufstellung aller Einheiten im Scratchpad mit bewerteten Größenverhältnissen,
listet jeden Katalog-Eintrag ohne Modell mit Begründung, übernimmt sinnvolle
Baukasten-Vorschläge aus den Berichten, verschiebt diese Datei mit `git mv` nach
`prompts/done/` und aktualisiert `prompts/README.md`. Kein Push ohne meine Freigabe.

## 7. Arbeitsweise

Mit mir Deutsch, im Repo Englisch. Entscheidungen selbst treffen und begründen; fragen
nur bei Installationen oder Downloads. Mein laufendes Blender nicht anfassen (nur
headless). Kein Test wird abgeschwächt.
