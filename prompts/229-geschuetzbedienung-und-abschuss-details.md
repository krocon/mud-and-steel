# Prompt 229 – Geschützbedienung verfeinern und Artillerie-Abschussdetails

> In einer frischen Session im Repo-Ordner ausführen. **Empfohlen nach Prompt
> 227** (gespreizte Holme) und harmoniert direkt mit **Prompt 228** (`su_ml20`).
> Lies zuerst `CLAUDE.md`, `docs/INFANTRY.md`, `docs/PERFORMANCE.md` sowie die
> erledigten Prompts `prompts/done/193-geschuetzbedienung-als-figuren.md`,
> `prompts/done/206-infanterie-skript-und-geschuetzbedienung-beschleunigen.md`
> und `prompts/done/213-infanterie-animationen-teilen-und-smooth-shading.md`.
> Im Code:
> `godot/scenes/world/gun_crew_view.gd`,
> `godot/scenes/world/gun_kick_view.gd`,
> `godot/scenes/world/effects.gd`,
> `godot/scenes/world/effect_library.gd`,
> `godot/scenes/world/unit_merge.gd`,
> `tools/blender/soldier/animation.py`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/152 mm howitzer-gun M1937 (ML-20) ｜ Sudden Strike 5 #shorts #shortsvideo #gaming [l77ywaqaGrg].webm`
> (Sek. `00:15–00:35`: arbeitende Bedienung ohne Gewehre in den Händen,
> deutliches Zurückzucken/Abducken beim Abschuss, danach Laden am Verschluss,
> ausgeworfene Messinghülsen am Boden hinter den Holmen, bereitgestellte
> Munition, kräftiger Mündungsbremsen-Feuerstoß und aufgewirbelter Bodenstaub
> um das Geschütz).
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hashes
> unverändert.

## 1. Problem

Seit Prompt 193 haben alle gezogenen Geschütze und schweren Mörser echte
animierte Soldatenfiguren (`gun_crew_view.gd`), und seit Prompt 077/208 haben
Geschütze Rücklauf, Wanken und Mündungsstaub (`gun_kick_view.gd`, `effects.gd`).
Im nahen Vergleich (z. B. `su_m30.png`, `de_lefh18.png` und im laufenden Gefecht)
fallen jedoch fünf konkrete Details auf, die eine feuernde Artilleriebatterie
noch deutlich lebendiger und glaubwürdiger machen:

1. **Gewehre in den Händen der Bedienung:** `gun_crew_view.gd` nutzt
   `rifleman.glb`/`carbineer.glb`. Außerhalb von `gun_aim` und `gun_load`
   (also beim Knieen, Warten, Ducken und beim Geschützführer) halten die
   Soldaten ihre Gewehre quer über die Knie oder vor der Brust, statt die Hände
   am Geschütz bzw. an der Munition zu haben.
2. **Kein eigener Abschuss-Schutzclip & leere Hände beim Laden:** In
   `gun_crew_view.gd` (Zeile 26) steht explizit *„no own clip for covering the
   ears“* – beim Schuss einer 122-mm- oder 152-mm-Haubitze gehen alle außer dem
   Ladeschützen nur kurz in `crouch`, und der Ladeschütze bewegt in `gun_load`
   leere Hände Richtung Verschluss (keine sichtbare Granate, keine
   Bereitschaftsmunition neben dem Geschütz).
3. **Keine ausgeworfenen Hülsen am Boden:** Nach dem Abschuss liegen bei uns
   keine leeren Messinghülsen hinter dem Verschluss – dabei erzählen gerade die
   glänzenden Hülsen zwischen den Holmen nach 2–3 Salven sofort, wie intensiv
   eine Batterie gefeuert hat.
4. **Mündungsbremsen-Erkennung & schwerer Bodenstaub:** `unit_merge.gd`
   (`has_muzzle_brake`) prüft die Verdickung an der Mündung – bei schlankeren
   `slotted`-Mündungsbremsen (wie an der ML-20) prüfen, dass sie zuverlässig
   erkannt werden, damit die seitlichen Feuer-/Gasstrahlen auslösen. Zudem
   schaltet `_muzzle_dust` in `effects.gd` ab `RAISED_BARREL = 0.35` (≈ 20,5°)
   komplett auf das flache runde `blast_dust` um, sodass bei einer 122–152-mm-
   Haubitze (25° Standard-Elevation) der markante vorwärts/seitlich gefächerte
   Abschussstaub und träge Pulverrauch rund um Räder und Mündung zu schwach
   ausfällt.
5. **Rohrrücklauf-Deckel bei Großkalibern:** `gun_kick_view.gd` deckelt
   `RECOIL_MAX_M = 0.6 m`, sodass eine 152-mm-Haubitze keinen längeren Rücklauf
   zeigt als eine 122-mm- oder 105-mm-Haubitze.

## 2. Ziel

Reine Darstellung (`crates/sim` unverändert, Golden Hashes identisch, strikt im
Performance-Budget aus `docs/PERFORMANCE.md`):

1. **Freie Hände, Granate & Bereitschaftsmunition (`gun_crew_view.gd`):**
   - Bei Figuren der Geschützbedienung das `Weapon`-Mesh (Gewehr/Karabiner)
     standardmäßig ausblenden (`visible = false`, Ausnahme ggf. Fernglas/Gear),
     damit niemand beim Knieen, Warten oder Richten am Geschütz ein Gewehr im
     Anschlag hält.
   - **Artilleriegranate beim Laden:** Während der Trage- und Einschubphase von
     `gun_load` (Ladeschütze, optional Munitionsträger) ein kleines, geteiltes
     Granaten-Mesh (Messing-/Oliv-Zylinder mit Spitze, nach Kaliber skaliert)
     zwischen den Händen einblenden und nach dem Einschieben (ab ~2,1 s im Clip)
     ausblenden.
   - **Munitionskiste / Bereitschaftsgranaten am Geschütz:** Bei abgeprotzten
     Geschützen (`gun_howitzer`, `gun_at`, `mortar_heavy`) eine kleine
     prozedurale/geteilte Munitionskiste (oder 2–3 bereitgelegte Granaten) auf
     dem Boden seitlich/hinter dem rechten Holm platzieren (verschwindet beim
     Anhängen, 1 gemeinsames MultiMesh oder extrem leichtgewichtiges geteiltes
     Mesh, in `NEAR_M`/`MID_M` sichtbar).
2. **Großkaliber-Abschussreaktion `gun_brace` (`tools/blender/soldier/animation.py` + `gun_crew_view.gd`):**
   - Neuen kompakten Clip `gun_brace` (ca. `1.0 s`, geduckter Oberkörper, leicht
     vom Verschluss weggedreht, beide Hände an die Ohren) in
     `tools/blender/soldier/animation.py` anlegen und über die geteilten
     Animationsbibliotheken aus Prompt 213 exportieren.
   - In `gun_crew_view.gd`: Bei Kaliber `>= 100 mm` spielen alle Bediener in der
     unmittelbaren Abschussphase (~`0.8–1.0 s`; beim Ladeschützen kurz ~`0.45 s`
     unmittelbar nach dem Schuss, bevor er den nächsten Ladevorgang beginnt)
     `gun_brace` statt `crouch`! Kleinere Kaliber (`< 100 mm`) behalten das
     schnelle `crouch`.
3. **Ausgeworfene Messinghülsen am Boden (`shell_casings.gd`, angebunden an `effects.gd`):**
   - Neues schlankes Modul `godot/scenes/world/shell_casings.gd` mit **einem
     einzigen `MultiMeshInstance3D`** (Ringpuffer, z. B. `MAX_CASINGS = 512`,
     einfacher 6-seitiger Messing-Zylinder, **1 Draw-Call!**).
   - Bei jedem Schuss eines gezogenen Geschützes (`gun_howitzer`, `gun_at`,
     `gun_aa_heavy`, ab `caliber >= 50 mm`, sichtbar im Fog-of-War) landet ca.
     `0,4 s` nach dem Schuss (deterministisch leicht gestreut) eine
     kaliberskalierte Messinghülse hinter dem Verschluss auf dem Boden
     (`sim_host.height_at`), leicht schräg/rollend abgelegt.
   - Nach mehreren Salven bildet sich ein realistisches kleines Häuflein
     glänzender Hülsen hinter den Holmen; auf Grafikqualität `Low` aus, im
     Nebel des Krieges ausgeblendet (oder nur bei Sichtbarkeit gespawnt), keine
     Schattenwürfe (`SHADOW_CASTING_SETTING_OFF`).
4. **Wuchtigerer schwerer Haubitzen-Abschuss (`effects.gd`, `gun_kick_view.gd`, `unit_merge.gd`):**
   - `unit_merge.gd` (`has_muzzle_brake`): Gegen alle Geschütze mit
     Mündungsbremse prüfen (insb. `de_lefh18`, `de_pak40`, `uk_25pdr` und
     `su_ml20` mit `slotted`-Bremse); falls `slotted` unter `BRAKE_RATIO`
     rutscht, sauber nachjustieren (ohne falsche Treffer bei bremsenlosen
     Rohren wie `su_m30` oder `us_m2a1_105`).
   - `effects.gd` (`_muzzle_dust` & `MUZZLE_CLASSES["howitzer"]`): Bei
     Haubitzen mit `caliber >= 100 mm` und typischer indirekter Elevation
     (20–45°) zusätzlich zum runden Bodenstoß einen kräftigen, vorwärts und
     seitlich abfließenden **Bodenstaub-Fächer** auslösen (am stärksten auf
     Erde/Feld/Sand, gedämpft auf nassem Boden/Straße), plus spürbar volleren
     warm-orangen Mündungsblitz und träger abziehenden braungrauen Pulverrauch.
   - `gun_kick_view.gd`: `RECOIL_MAX_M` für schwere Geschütze (`>= 120 mm`) auf
     `~0.75–0.80 m` prüfen (solange der Verschluss sauber in der Wiege bleibt)
     und das kurze Setzen/Zurückwippen der Lafette (`howitzer.rock`) bei
     schweren Kalibern fein abstimmen.

## 3. Prüfen und zeigen

- **Vorher/Nachher-Bilder & kurzer Clip** (nah auf `su_m30`, `de_lefh18` und
  – falls Prompt 228 fertig – `su_ml20`):
  - Bedienung ohne Gewehre in den Händen; Ladeschütze mit sichtbarer Granate;
    Abschussmoment mit `gun_brace` (Hände an den Ohren), Mündungsfeuer +
    Mündungsbremsen-Seitenstrahlen + Bodenstaub; nach 3–4 Schüssen deutlich
    sichtbare Messinghülsen am Boden hinter den Holmen + Munitionskiste.
- **Thumbnails neu rendern** (da keine Gewehre mehr in den Händen der
  Geschützbedienung liegen!):
  - Alle `gun_*`- und `mortar_heavy`-Thumbnails neu (`just thumbnails --only=...`),
    `just units-web-sync`.
- **Performance & Tests:**
  - `just bench-battery` und `just effects-bench` vorher/nachher messen und in
    `docs/PERFORMANCE.md` festhalten (Ziel: ≤ 1 zusätzlicher Draw-Call für alle
    Hülsen zusammen, keine spürbare Frame-Zeit-Regression).
  - `just selftest` (inkl. `--gun-crew-test`), `just check` grün; Golden Hashes
    unverändert.

## 4. Nicht Teil dieses Prompts

- Regeländerungen an Nachladezeit, Streuung, Salven-Staffelung oder Schaden in
  `crates/sim`.
- Neue Geschützmodelle (Prompt 228) oder Spreizlafetten-Geometrie (Prompt 227).
- Sandsack-Stellungen für Panzer/Geschütze (Prompt 230).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/229-geschuetzbedienung-und-abschuss-details.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.
