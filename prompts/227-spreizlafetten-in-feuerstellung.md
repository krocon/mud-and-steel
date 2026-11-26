# Prompt 227 – Spreizlafetten in Feuerstellung und Klapp-Holme beim Schleppen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ASSETS.md` (Namens- und Hierarchietabelle, Budgets),
> `tools/blender/kit/artillery.py`, `tools/blender/kit/assemblies.py`,
> `tools/blender/validate.py`, `godot/scenes/world/unit_merge.gd`,
> `godot/scenes/world/gun_crew_view.gd` und `godot/scenes/world/transport_view.gd`
> sowie die bestehenden Spreizlafetten-Blueprints unter `assets-src/blueprints/`
> (`de_lefh18`, `de_pak40`, `su_m30`, `su_zis3`, `us_m2a1_105`, `us_57mm_m1`,
> `uk_17pdr`, `uk_6pdr`).
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/152 mm howitzer-gun M1937 (ML-20) ｜ Sudden Strike 5 #shorts #shortsvideo #gaming [l77ywaqaGrg].webm`
> (weit gespreizte Kastenholme mit Erdspornen in Feuerstellung, Bedienung
> luftig zwischen und neben den Holmen).
> **Keine Bilder, Werte oder Assets aus dem Video ins Repo.**
> Reine Darstellung und Modellbau – **keine Änderungen in `crates/sim`**, Golden
> Hashes bleiben unverändert.

## 1. Problem

Alle unsere gezogenen Spreizlafetten-Geschütze (`de_lefh18`, `de_pak40`,
`su_m30`, `su_zis3`, `us_m2a1_105`, `us_57mm_m1`, `uk_17pdr`, `uk_6pdr`) sind
bisher mit **geschlossenen Marschholmen (`trail_spread_deg = 0`)** gebaut.
Grund war eine reine Prüfgrenze (`de_lefh18.toml`: *„spread trails would be far
wider than the catalog width“*): `validate.py` prüft die gesamte Modellbreite
gegen die Marschbreite `width_mm` des Katalogs (±2 %).

Dadurch wirken abgeprotzte, feuernde Haubitzen und Panzerabwehrkanonen (gut
sichtbar z. B. in `godot/assets/thumbnails/su_m30.png` und `de_lefh18.png`) wie
im Marschzustand, und die Bedienung hockt eng gedrängt außen entlang zweier
paralleler Balken statt zwischen weit gespreizten Holmen hinter dem Schild.

## 2. Ziel

1. **Bewegliche Holme `Trail_L` / `Trail_R` im Blender-Baukasten (`tools/blender/kit/artillery.py`):**
   - `split_trail_carriage` baut die beiden Lafettenholme (inkl. Erdsporn und
     Handgriff) bei Spreizlafetten (`trail_spread_deg > 0` oder explizitem Flag)
     als eigene Kinder **`Trail_L`** und **`Trail_R`** unter `Carriage` mit
     Pivot direkt am Holmgelenk hinter der Achse (`±hinge_x, ay - 0.1, z0`),
     ab Werk in **Feuerstellung** (gespreizt auf `trail_spread_deg`, typisch
     `45–55°`).
   - Beim `towed_gun`-Assembly (`tools/blender/kit/assemblies.py`) beachten:
     Die Holmlänge wird weiterhin aus der Katalog-Marschlänge bestimmt (oder so
     berechnet, dass die projizierte Länge bei `trail_spread_deg = 0` exakt der
     Kataloglänge entspricht; `exclude_length` / `exclude_width` sauber prüfen,
     damit die Maßtreue nach `docs/SCALE.md` nicht aufweicht!).
   - `Socket_Tow` bleibt am hinteren Zugende (bzw. bei zusammengeklappten
     Holmen an der Zugöse); die Kastenlafette `uk_25pdr` (und `uk_75mm_pack`,
     falls Einholm) behält `trail_spread_deg = 0` ohne `Trail_L`/`Trail_R`.
   - `tools/blender/kit/bake.py`: `Trail_L` und `Trail_R` im AO-Bake als
     bewegliche Baugruppen behandeln (wie `Gun`/`Turret` aus Prompt 214), damit
     beim Zusammenklappen keine schwarzen AO-Schatten am Gelenk zurückbleiben.
2. **Validator & Doku (`tools/blender/validate.py`, `docs/ASSETS.md`):**
   - `Trail_[LR]` in `NAMES` (erlaubter Parent: `Carriage`) aufnehmen und in
     `docs/ASSETS.md` dokumentieren.
   - Die bestehenden `[validate]`-Ausschlüsse (`exclude_width = ["Trail_L", "Trail_R"]`,
     und falls die projizierte Feuerstellung kürzer ist als die Marschlänge,
     sauber begründet dokumentieren oder die Holmlänge aus der Marschlänge
     prüfen) so nutzen, dass **Spurweite/Radbreite (`width_mm`), Höhe
     (`height_mm`) und Rohrlänge** weiterhin streng innerhalb ±2 % gegen den
     Katalog geprüft werden.
3. **Bestehende Spreizlafetten-Blueprints & `Socket_Crew_NN` aktualisieren:**
   - Alle Spreizlafetten (`de_lefh18`, `de_pak40`, `su_m30`, `su_zis3`,
     `us_m2a1_105`, `us_57mm_m1`, `uk_17pdr`, `uk_6pdr`) auf typische
     Feuerstellungs-Spreizung (`~46–54°`) umstellen.
   - Alle `Socket_Crew_NN` der betroffenen Geschütze neu setzen: Richtschütze
     (`01`) links direkt an Optik/Handrädern hinter dem Schild, Ladeschütze
     (`02`) rechts am Verschluss, Munitionsträger (`03`) und Geschützführer
     (letzter Socket) luftig zwischen/hinter den gespreizten Holmen, restliche
     Bedienung ohne Überlappung mit Rädern oder Holmen.
   - Alle betroffenen Modelle neu bauen (`just unit ...`), exportieren und mit
     `just validate-assets` prüfen (Dreiecksbudget `gun_* ≤ 3000` einhalten!).
4. **Dynamisches Auf- und Zuklappen in Godot (`unit_merge.gd`, `gun_crew_view.gd` / `transport_view.gd`):**
   - `godot/scenes/world/unit_merge.gd`: `Trail_L` und `Trail_R` als bewegliche
     Teile kennzeichnen (nicht in `Carriage` einbacken).
   - Im Spiel (reine Darstellung):
     - Abgeprotzt (Feuerstellung, `carriers[i] < 0`): `Trail_L` und `Trail_R`
       stehen in Ruhe-Transform (gespreizt).
     - Angehängt am Zugfahrzeug (`carriers[i] >= 0`): `Trail_L` und `Trail_R`
       drehen um ihr Gelenk (`rotation.y`) auf geschlossene Marschstellung
       (`±trail_spread_deg / 2` zurück auf `0°`, Winkel aus dem Modell bzw.
       Metadaten ableiten, kein Hardcoding pro Einheitentyp).
     - Beim Abprotzen / Anhängen weicher Übergang (~0,8 s) statt hartem Sprung.

## 3. Prüfen und zeigen

- **Vorher/Nachher-Bilder** (nah und aus Spielhöhe):
  - `su_m30` und `de_lefh18` in Feuerstellung (gespreizte Holme + Bedienung
    sauber ohne Clipping an den Holmen/Rädern) sowie `de_pak40` / `su_zis3`.
  - Ein geschlepptes Geschütz am LKW (`su_zis5` + `su_m30` oder `de_opel_blitz`
    + `de_lefh18`): Holme sauber zusammengeklappt hinter der Anhängerkupplung;
    nach dem Abprotzen sauber gespreizt.
- **Thumbnails & Einheiten-Webseite:**
  - `just thumbnails --only=de_lefh18,de_pak40,su_m30,su_zis3,us_m2a1_105,us_57mm_m1,uk_17pdr,uk_6pdr`
  - `just units-web-sync`
- **Tests & Performance:**
  - `just validate-assets`, `just check`, `just selftest` (inkl.
    `--gun-crew-test`) grün; Golden Hashes unverändert.
  - `just bench-battery` prüfen (keine nennenswerte Draw-Call- oder
    Frame-Regression durch die 2 zusätzlichen Mesh-Nodes pro Spreizlafette).

## 4. Nicht Teil dieses Prompts

- Neue Geschütztypen wie die 152-mm-ML-20 (Prompt 228).
- Neue Soldaten-Animationen, Granaten-Props, Hülsenauswurf oder Abschuss-Effekte
  (Prompt 229).
- Regeländerungen an Auf-/Abprotzzeiten oder Marschgeschwindigkeit in
  `crates/sim`.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/227-spreizlafetten-in-feuerstellung.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.
