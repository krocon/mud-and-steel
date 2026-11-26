# Prompt 228 – Sowjetische 152-mm-Kanonenhaubitze M1937 (ML-20)

> In einer frischen Session im Repo-Ordner ausführen. **Empfohlen nach Prompt
> 227** (gespreizte Holme `Trail_L`/`Trail_R` in Feuerstellung).
> Lies zuerst `CLAUDE.md`, `docs/SOURCES.md`, `docs/SCALE.md`, `docs/ASSETS.md`,
> `data/catalog/SCHEMA.md`, `data/catalog/su.toml` (insb. `su_m30` und
> `su_isu152`), `data/economy.toml`, `data/indirect.toml`, `data/transport.toml`
> sowie `assets-src/blueprints/su/su_m30.toml` und `tools/blender/kit/weapons.py`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/152 mm howitzer-gun M1937 (ML-20) ｜ Sudden Strike 5 #shorts #shortsvideo #gaming [l77ywaqaGrg].webm`
> (schwere sowjetische 152-mm-Kanonenhaubitze ML-20 mit langem Rohr, markanter
> geschlitzter Mündungsbremse, Rücklauf-/Vorhol-Zylindern, schwerem Schild,
> massiven Lafettenrädern und weit gespreizten Kastenholmen mit Erdspornen).
> **Keine Daten, Bilder, Texturen oder Assets aus dem Video oder anderen Spielen
> ins Repo – jeder Katalogwert braucht eine echte historische Quelle nach
> `docs/SOURCES.md`.**

## 1. Problem

Die Sowjetunion besitzt im Katalog bisher nur die leichtere 122-mm-Haubitze
M-30 (`su_m30`) als gezogenes indirektes Geschütz; die berühmte 152-mm-Waffe
**ML-20** existiert ausschließlich als Fahrzeugkanone ML-20S in den
Selbstfahrlafetten `su_isu152` und `su_su152`. Als schweres gezogenes
Korpsgeschütz mit großer Reichweite, schwerem 152-mm-Sprenggeschoss und
markanter geschlitzter Mündungsbremse fehlt die **152 mm M1937 (ML-20)** bisher
vollständig.

## 2. Ziel

1. **Historisch belegter Katalogeintrag `su_ml20` in `data/catalog/su.toml`:**
   - Alle technischen Daten vor dem Eintragen gegen primäre/sekundäre historische
     Quellen (`docs/SOURCES.md`, z. B. englische und russische Wikipedia
     `152 mm howitzer-gun M1937 (ML-20)`, Schirokorad/Zaloga) verifizieren,
     jedes Feld mit `source = [...]` und ehrlichem `uncertain = [...]` versehen.
   - Eckwerte zur Prüfung gegen die Quellen:
     - `id = "su_ml20"`, `faction = "su"`, `category = "gun_howitzer"`,
       `tier = 2`
     - `name_de = "152-mm-Kanonenhaubitze ML-20"`,
       `name_en = "152 mm Gun-Howitzer M1937 (ML-20)"`
     - `designation = "152-mm gaubitsa-pushka obr. 1937 (ML-20)"`
     - `year_from = 1937`, `year_to = 1945`
     - Maße (Marschstellung wie bei `su_m30` und `de_lefh18`, Quelle prüfen):
       ca. `length_mm = 7210` (ohne Protze; mit Protze ~8070 – exakt nach
       Quelle festlegen und in `note` erklären), `width_mm = 2310`–`2345`,
       `height_mm = 2260`–`2270`, Gefechtsmasse `mass_kg ≈ 7270` (Marsch ~7930).
     - `crew = 9`, `gun_shield = true` (Schildstärke `armor_turret_front_mm`
       nur setzen, wenn eine Quelle sie belegt).
     - Bewaffnung: `152.4 mm ML-20 L/29`, `caliber_mm_x10 = 1524` (oder `1520`
       konsistent mit `su_isu152` – prüfen), `ammo = "OF-540 43.56 kg HE-frag, BR-540 APHE, G-530 concrete-piercing"`,
       `rate_rpm = 3` (praktisch 3–4 Schuss/min), `muzzle_velocity_mps = 655`,
       `range_effective_m = 17230` (Maximale indirekte Reichweite; direkte
       Reichweite in `note` festhalten).
   - `just catalog-doc` laufen lassen (`docs/UNIT_CATALOG.md` aktualisieren).
2. **3D-Modell & Blueprint `assets-src/blueprints/su/su_ml20.toml`:**
   - Auf Basis der `towed_gun`-Baugruppe (und `Trail_L`/`Trail_R` aus Prompt
     227, falls vorhanden; sonst `split_trail_carriage` mit passender
     Feuerstellungs-Spreizung):
     - Langes, kräftiges 152-mm-Rohr (ca. 4,2–4,4 m inkl. Mündungsbremse) mit
       **großer, klar lesbarer geschlitzter Mündungsbremse**
       (`muzzle_brake = "slotted"` – falls `slotted` in `tools/blender/kit/weapons.py`
       aus Spielhöhe zu unauffällig ist, Schlitze/Kastenform sauber nachschärfen,
       ohne andere Modelle negativ zu verändern!).
     - Markante lange Wiege und zylindrische Rücklaufbremse/Vorholer unter und
       über dem Rohr, kräftiger Schraubenverschluss, Ausgleicher (Equilibrator)
       an der Oberlafette, Richtoptik links und Handräder.
     - Schwerer, leicht geneigter Schutzschild mit Rohrausschnitt und unterer
       Schürze, große schwere Vollgummi-/Speichen- oder Doppelreifen-Räder
       (typisch ML-20: wuchtige breite Räder mit charakteristischen Speichen-
       oder Lochscheibenfelgen), massive gespreizte Kastenholme mit großen
       Erdspornen und Stützrollen/Griffen.
     - **9 `Socket_Crew_NN`** sauber ohne Clipping hinter dem Schild und
       entlang/zwischen den gespreizten Holmen verteilt.
     - Dreiecksbudget (`gun_howitzer ≤ 3000` Tris) und Maßtoleranz (±2 %) mit
       `just unit su su_ml20` und `just validate-assets` erfüllen.
3. **Spiel-Integration (`data/economy.toml`, Zugfähigkeit, Szenarien, Tests):**
   - `su_ml20` ans **Ende** von `[produce.vehicles] su` in `data/economy.toml`
     anhängen (nicht dazwischenschieben, da KI-Mixes die vorderen Indizes
     nutzen – siehe Kommentar in `data/economy.toml`!). Prüfen, dass sich die
     automatischen Einheitenkosten aus `[unit_cost]` sinnvoll gegen `su_m30` und
     `su_isu152` absetzen (höhere Munitions-/Mannkraftkosten, langsamere
     Feuerrate, größere Reichweite & Sprengwirkung).
   - **Schleppen prüfen (`data/transport.toml`):** Mit `mass_kg ≈ 7270` und
     `min_hp_per_tonne = 8` ergibt sich:
     - `su_m3a1_scout` (5.670 kg, 110 PS): `110 / 12,94 = 8,5 hp/t` → **kann
       die ML-20 schleppen**!
     - `su_zis5` (3.100 kg, 73 PS): `73 / 10,37 = 7,0 hp/t` → historisch
       stimmig zu schwach für eine 7,3-t-Korpshaubitze!
     Das in einem kurzen Kommentar/Test prüfen und im Ergebnis dokumentieren
     (kein unhistorisches Aufweichen von `min_hp_per_tonne`).
   - **Schuss-Sound & Effekte:** Prüfen, ob `sound_classes.gd` (`gun_howitzer`
     → `shot_howitzer`), `effects.gd` (152-mm-Skalierung + Mündungsbremse +
     schwere Einschläge ≥ 100 mm) und `gun_crew_view.gd` (9 Mann, `BIG_CALIBER_MM`)
     automatisch greifen.
   - `su_ml20` in `data/scenarios/gun_crews.toml` (für Sichtprüfung und
     Bedienungs-Test) sowie in `data/scenarios/all_units.toml` aufnehmen.
   - **Golden Hashes (`crates/sim/tests/determinism.rs`):** Prüfen, ob der neue
     Katalog-/Economy-Eintrag den Golden Hash ändert; falls ja, Ursache exakt
     benennen und bewusst im Commit dokumentieren (niemals Tests abschwächen).

## 3. Prüfen und zeigen

- **Bilder** (nah und aus Spielhöhe):
  - `su_ml20` allein und im Größenvergleich direkt neben `su_m30` (122 mm) und
    `su_zis5` / `su_m3a1_scout` (der deutliche Sprung zur schweren 152-mm-Korpshaubitze
    muss sofort lesbar sein!).
  - `su_ml20` beim Abfeuern (erhöhtes Rohr, Rohrrücklauf, Mündungsfeuer inkl.
    Mündungsbremse, 9-köpfige Bedienung).
- **Thumbnails & Doku:**
  - `just thumbnails --only=su_ml20`
  - `just units-web-sync` (prüfen, dass kein Thumbnail fehlt)
  - `just catalog-doc`
- **Tests:**
  - `just validate-assets`, `just check`, `just selftest` (inkl.
    `--gun-crew-test`) grün.

## 4. Nicht Teil dieses Prompts

- Schwere Korpshaubitzen der übrigen Fraktionen (`de_sfh18`, `us_m1_155`,
  `uk_55in` – können nach demselben Muster in einem Folge-Prompt folgen).
- Ein eigener schwerer sowjetischer Artillerieschlepper (z. B. S-65 Stalinets /
  Ya-12 – eigener Katalog-Prompt, falls gewünscht).
- Neue Soldaten-Animationen oder Hülsenauswurf (Prompt 229).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/228-sowjetische-152mm-haubitze-ml20.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.

## Ergebnis (Nachtlauf 26.11.2026)

- **Katalog (`data/catalog/su.toml`, `docs/UNIT_CATALOG.md`):**
  - Historisch belegter Eintrag `su_ml20` (`gun_howitzer`, `tier = 2`, 1937–1945) aus en./ru. Wikipedia (`152 mm howitzer-gun M1937 (ML-20)`): `length_mm = 8180` (Marschmaß konsistent mit `su_m30`), `width_mm = 2345`, `height_mm = 2270`, `mass_kg = 7270`, `crew = 9`, `gun_shield = true`, 152,4 mm ML-20 L/29 (`caliber_mm_x10 = 1524`, `rate_rpm = 3`, `muzzle_velocity_mps = 655`, `range_effective_m = 17230`, OF-540 / BR-540 / G-530). `just catalog-doc` ausgeführt.
- **3D-Modell & Baukasten (`tools/blender/kit/weapons.py`, `assets-src/blueprints/su/su_ml20.toml`):**
  - Neuer Mündungsbremsentyp `slotted_heavy` (wuchtiger Kasten mit 3 dunklen seitlichen Schlitzen, Breite > `BRAKE_RATIO = 1.45` für automatische Mündungsbremsen-Erkennung in Godot; bestehende `slotted`-Modelle unverändert).
  - `su_ml20` auf `towed_gun` mit 50°-Spreizlafette (`Trail_L`/`Trail_R` aus Prompt 227), 8-Speichen-Schwerlast-Rädern, Wiege + oberem Vorholer + seitlichen Ausgleichern, Schutzschild + Schürze und 9 `Socket_Crew_NN` gebaut und validiert (`1540/3000 tris`, exakt `8.180 / 2.345 / 2.270 m`).
- **Spiel-Integration & Tests (`data/economy.toml`, `crates/sim/tests/transport.rs`, `crates/cli/src/map_sight.rs`, `data/scenarios/gun_crews.toml`, `all_units.toml`):**
  - `su_ml20` ans Ende von `[produce.vehicles] su` angehängt: automatische Kosten `0 Treibstoff / 397 Munition / 54 Mannkraft / 64,3 s` (sauber zwischen `su_m30` `0/258/48` und `su_isu152` `184/395/30`).
  - Zugfähigkeit in neuem Sim-Test `ml20_towed_by_m3a1_scout_not_zis5` verifiziert: `su_m3a1_scout` (8,5 PS/t ≥ 8) schleppt `su_ml20`, `su_zis5` (7,0 PS/t < 8) nicht.
  - `crates/cli/src/map_sight.rs`: Die 5 älteren Karten-Layout-Tests (Prompts 123–133) grenzen jetzt explizit Divisionsartillerie (< 15 km) von schwerer Korpsartillerie (`su_ml20`, 17,23 km = 2,15 km im Spiel) ab, ohne eine bestehende Divisions-Reichweitenprüfung abzuschwächen (`map_sight::indirect_weapons()` führt `su_ml20` weiterhin voll mit).
  - Golden Hashes bewusst aktualisiert (`determinism.rs`: `0x2f94_7b05_45b0_32ce`, `economy.rs`: `0xa12e_0f6d_30e1_34c6`) wegen des neuen Eintrags in `[produce.vehicles] su` (`demo_battle_is_decided` unverändert Tick 715, Spieler 0).
- **Prüfungen:** `just thumbnails --only=su_ml20`, `just units-web-sync` (0 fehlende Thumbnails), `just kit-test`, `just check` (782 Tests + 464 GLBs), `just selftest` (293 Einheiten in Referenz, 13 Geschütze inkl. `su_ml20` mit 9 Mann im `--gun-crew-test`) alle grün.
- **Vorläufig (Nachtlauf):** Marschlänge `length_mm = 8180` (analog `su_m30` Marschlänge inkl. Protzenmaß aus Wikipedia-Infobox statt 7210 mm ohne Protze), `caliber_mm_x10 = 1524`, neuer Bremsentyp `slotted_heavy` sowie Korpsartillerie-Grenze `>= 15 km` in den älteren `map_sight`-Karteninvarianten.
