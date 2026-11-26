# Prompt 242 – Hochkant-Short „Ostfront-Dorfchaos“ (Karte, Massen-Infanterie-Gefecht und 9:16-Video)

> In einer frischen Session im Repo-Ordner ausführen. **Empfohlen nach Prompts
> 238–241** (9:16-Hochkant-Pipeline, kontrastreiche Kettenspuren/Karrenwege/Krater-Decals,
> zerfetzte Ziegeldächer & Trümmerhaufen, Dorf-Details wie Blütenbäume,
> Sonnenblumen, Ziehbrunnen, Holzschuppen & Gemüsegärten) sowie **Prompt 231**
> (zügiger Infanterie-Gefechtslauf).
> Lies zuerst `CLAUDE.md`, `docs/MAPS.md`, `docs/MISSIONS.md`,
> `docs/ARCHITECTURE.md` („Trailer shots“) sowie als direkte Vorbilder
> `prompts/done/221-video-szene-bocage-artillerieschlag.md`,
> `prompts/226-video-szene-herbst-artillerieangriff.md` und
> `prompts/periodic/082-video-hinterhalt-am-waldrand.md`.
> Im Code:
> `data/map_sketches/`,
> `data/trailer/bocage_barrage.toml`,
> `data/trailer/autumn_barrage.toml`,
> `godot/scenes/tools/trailer_shots.gd`,
> `tools/trailer-shots.sh`,
> `tools/trailer-scout.sh`,
> `tools/trailer-cut.sh`.
>
> **Stil-Referenz (nur lokal ansehen, nichts ins Repo übernehmen):**
> `.temp/vorlage/This WWII RTS is Pure Chaos #panzerstrike [lskjKn-C9yo].webm`
> (gesamter Clip, `00:00–00:21`, `9:16` Hochkant, 60 fps, 5 harte Schnitte
> zwischen zerschossenem Ostfront-Dorf und angrenzender Birken-Wiesenflanke).
> **Keine Bilder, Tonspuren, Texte, Namen oder Assets aus dem Video ins Repo**;
> Vergleichsbilder nur lokal in `build/chaos_compare/` ablegen (`build/` ist
> gitignored).
> Alle Bewegungen, Schüsse, Treffer und Verluste müssen echte deterministische
> Simulations-Befehle sein – **keine Cheats, keine Godot-Physik, Golden Hashes
> unverändert**.

## 1. Ziel

Ein deterministisch reproduzierbares, direkt in Godot aufgezeichnetes
**21-Sekunden-Hochkant-Kurzvideo (9:16 Short)** (`1080 × 1920 @ 60 fps`, optional
`1440 × 2560`, mit Original-Spielton auf `-14 LUFS`), das die rohe
Schlachtfeld-Dichte, den Rhythmus, die steile RTS-Bildkomposition und das
„Pure Chaos“-Gefühl der Referenz in *Mud and Steel* erreicht – rein mit unseren
eigenen WW2-Modellen, eigener Skizzenkarte und echter Simulation:

- **Format & Kamera-Stil:**
  - Hochkant **`9:16`** (Prompt 238), klassische steile RTS-Draufsicht
    (`perspective = "classic"`, `fov ≈ 26–30°`, `pitch ≈ 52–58°`, `dof = "off"`,
    `hud = false`; optional dezente `unit_bars = true` für einen kurzen Moment
    oder komplett HUD-frei – im Vergleich prüfen).
  - **Sommertag** (`biome = "forest"`, klares warmes Tageslicht mit harten
    Sonnenschatten und satten Grüntönen, sodass rote Ziegeldächer, rosa
    Blütenbäume, gelbe Sonnenblumen, weiße Birkenstämme und dunkle
    Kettenspuren/Krater maximal kontrastieren).
- **Schauplatz (zwei unmittelbar benachbarte Zonen auf derselben Karte):**
  1. **Das zerschossene Ostfront-Dorf (West/Mitte):**
     - Eng gruppierte ländliche Fachwerk-, Backstein- und Blockhäuser
       (`env_house_farm`, `env_house_farm_brick`, `env_barn`, `env_house_log`,
       `env_shed_wood`), davon ca. **60–70 % bereits `Damaged` (zerfetzte rote
       Ziegeldächer mit offenen Dachstühlen aus Prompt 240) oder `Ruin`
       (Ziegel- und Balken-Trümmerhaufen)**.
     - Authentische Dorf-Details (Prompt 241): hölzerner **Ziehbrunnen**
       (`env_well_shadoof`) auf dem sandigen Dorfplatz, kleine rechteckige
       **Gemüsegarten-Beete** mit Staketenzäunen (`env_fence_picket`) am
       südlichen Dorfrand, **Sonnenblumen-Horste** (`env_sunflowers`) am
       Nordost- und Südrand, **2–3 rosa blühende Bäume** (`env_tree_blossom`)
       zwischen Laubbäumen und Koniferen.
     - Verzweigte schmale **zweispurige Dorf-Karrenwege** (Prompt 239) und
       dichtes **Granatkrater-Feld** (`[[crater_field]]` mit scharfen
       Krater-Decals) rund um und zwischen den Häusern.
  2. **Die Birken-Wiesenflanke (unmittelbar östlich/südöstlich des Dorfs):**
     - Offene, kraterübersäte grüne Sommerwiese, durchzogen von einem
       **geschwungenen schmalen Doppelspur-Karrenweg** in der unteren Hälfte.
     - In der oberen Bildhälfte **zwei markante weißstämmige Birken
       (`env_tree_birch`)** mit umgebenden Sträuchern (`env_bush`), zerbombter
       brauner Erde und 2–3 bereits **brennenden Panzer-/Halbketten-Wracks**
       als Brennpunkt.
     - Über die gesamte untere Wiese ziehen sich **dunkle, gerippte
       Panzer-Kettenspuren** (Prompt 239) mit Wendeschleifen und Anfahrtsbahnen
       aus der Vorlauf-Phase (`pre_roll_s`).
- **Einheiten & Gefechts-Choreografie (alles echte Modelle im Katalog!):**
  - **Deutsche Verteidiger (Player 0):**
    - *Wiesenflanke:* `de_jagdpanther` zentral am Karrenweg, links daneben
      `de_flakpz38t` (20-mm-Schnellfeuer!), rechts auf dem Karrenweg
      `de_sdkfz222`, davor in lockerer Linie eine deutsche Schützengruppe
      (`de_inf_rifle_squad`), und zeitlich gestaffelt rollt im finalen Cut von
      links eine **`de_hummel`** (oder `de_wespe`) ins Bild und feuert schweres
      HE in die anstürmende Welle!
    - *Dorf & Südrand:* `de_pz4_h`, `de_stug3_g`, `de_marder3_m`,
      `de_sdkfz251_1`, `de_opel_blitz` (einer davon wird im Dorf-Cut spektakulär
      mit gelb-orangem Feuerball vernichtet!), `de_inf_mg42_team` und
      `de_inf_rifle_squad` in und zwischen den beschädigten Häusern.
  - **Sowjetische Angreifer (Player 1):**
    - **Massive, gestaffelte Infanteriewellen** (`14–20× su_inf_rifle_squad`,
      also ~140–200 Soldaten!) greifen sowohl durch die Dorfruinen als auch in
      überwältigender Masse von Norden an den beiden Birken und brennenden
      Wracks vorbei direkt über die kraterübersäte Wiese an. Durch den
      Vorlauf liegen bereits Dutzende gefallene Soldaten über Dorf und Wiese
      verstreut (`infantry_view.gd` behält alle Gefallenen auf dem Feld!).
    - **Sowjetische Panzer im Dorf:** `su_t34_85_1944`, `su_is2` und `su_su85`
      drücken aus Nordwesten ins brennende Dorf, feuern durch Mündungsrauch
      und Rauchschwaden auf die Verteidiger.
    - **Begleitender Mörser-/HE-Beschuss** (`su_pm38` / `de_inf_mortar_medium`
      außerhalb des Bildes) sorgt durchgängig für knackige graue
      Explosions-Staubwolken, Erdfontänen und frische Krater inmitten der
      Infanteriewellen.

## 2. Umsetzung in zwei Stufen

### Stufe 1 – Karte, deterministische Schlacht & 5-Cut-Schnitt aufsetzen und scouten

1. **Skizzenkarte `data/map_sketches/eastern_village_chaos.toml` anlegen** und
   mit `just gen-sketch eastern_village_chaos` erzeugen (`just map-check` und
   `just map-reach` grün; auch als reguläre Skirmish-Karte mit 2 Starts und
   Versorgungspunkten spielbar machen):
   - `biome = "forest"`, `size_m = [800, 800]` (kompakt für schnelle
     Aufnahme und dichte Szenerie).
   - Klar gegliederter Dorfkern (West/Mitte) mit direktem Übergang zur
     Birken-Wiesenflanke (Ost), sodass beide Schauplätze innerhalb derselben
     laufenden Schlacht liegen und sich sogar gegenseitig hören/beeinflussen.
   - Dichte `[[crater_field]]`-Zonen rund um Nordrand, Dorfplatz und Birken
     (mit `lane`-Aussparung für den geschwungenen Karrenweg).
2. **Szenen-Skript `data/trailer/eastern_chaos.toml` schreiben** (Format:
   `docs/MISSIONS.md`, ausschließlich reguläre Simulationsbefehle `order`,
   `fire`, `barrage`, `when = "time"` / `when = "destroyed"`):
   - **`pre_roll_s` (~12–18 s normal abgespielt vor dem ersten Cut):**
     - Die deutschen Fahrzeuge auf der Wiese (`de_jagdpanther`, `de_flakpz38t`,
       `de_sdkfz222`) fahren zuerst eine kurze Manöverschleife über die grüne
       Wiese in ihre Feuerstellungen, damit zu Shot-Beginn markante dunkle
       **Kettenspuren-Bögen und Wendekreise** im Gras liegen!
     - Frühe Vorhut-Trupps und 3–4 Opferfahrzeuge im Dorf und bei den Birken
       werden früh bekämpft, sodass zu `start_s` bereits **brennende Wracks
       mit schwarzen Rauchsäulen** und **Dutzende Gefallene** das Schlachtfeld
       zeichnen.
   - **Laufende 21-Sekunden-Schlacht (`start_s` bis `start_s + 21.2 s`):**
     - Kontinuierliche gestaffelte sowjetische Infanteriewellen strömen an den
       Birken vorbei auf die deutsche Linie zu; `de_flakpz38t`, `de_jagdpanther`,
       `de_sdkfz222` und die deutsche Schützenlinie feuern ununterbrochen;
       ca. 16 s nach Schnittbeginn rollt die `de_hummel` von links ins Bild und
       greift direkt ein.
     - Parallel tobt im Dorf der Nahkampf zwischen `su_t34_85_1944`/`su_is2` +
       sowjetischer Infanterie und den deutschen Verteidigern; im 4. Cut
       (`t_cut ≈ 6,0 s`) geht ein Fahrzeug am unteren linken Dorfrand mit
       hellem Feuerball hoch.
   - Mit `just trailer-scout` einen deterministischen `seed` und das exakte
     `start_s`-Fenster ermitteln, in dem alle 5 Cuts maximale Action liefern.
3. **5 Hochkant-Shots in `godot/scenes/tools/trailer_shots.gd` eintragen**
   (alle mit derselben `scene = "eastern_chaos"`, demselben `seed`,
   `size = Vector2i(1080, 1920)` bzw. `aspect = "9:16"`, fortlaufendem
   `start_s` ohne Zeitsprung-Lücken):
   - **`chaos_01_village_close`** (`~1,6 s`): Steile Nah-Mittelaufnahme aufs
     zerschossene Dorfzentrum (aufgerissenes rotes Ziegeldach, brennende
     Wracks, rosa Blütenbaum, feuernder sowjetischer Panzer im Rauch).
   - **`chaos_02_village_wide`** (`~1,0 s`): Weiterer Dorf-Überblick mit
     Ziehbrunnen, Gemüsegarten, Karrenwegen und Gefechtslinien.
   - **`chaos_03_meadow_wave`** (`~2,7 s`): Schnitt auf die Wiesenflanke mit
     Birken, Karrenweg, dunklen Kettenspuren und der ersten anstürmenden
     Infanteriewelle.
   - **`chaos_04_village_climax`** (`~4,2 s`): Zurück auf die weite
     Dorf-Ansicht (leichter langsamer Kameraschwenk) – Feuerball-Abschuss
     unten links, Sonnenblumen oben, dichter Rauch, Panzer & Infanterie im
     gesamten Dorf.
   - **`chaos_05_meadow_hold`** (`~11,7 s`): Langer Höhepunkt auf der
     Wiesenflanke – `de_jagdpanther`, `de_flakpz38t`, `de_sdkfz222`, deutsche
     Schützenlinie + von links ins Bild fahrende `de_hummel` halten gegen die
     endlose sowjetische Infanteriewelle an den brennenden Wracks & Birken!
4. **Standbilder ziehen & visuell iterieren:**
   - `just trailer-shots --only=chaos_01_village_close,chaos_02_village_wide,chaos_03_meadow_wave,chaos_04_village_climax,chaos_05_meadow_hold --stills`
   - Standbilder lokal in `build/chaos_compare/` direkt neben die
     Referenz-Frames (Sek. `01, 04, 07, 11, 18` aus der Vorlage) legen;
     Kamerahöhe, Dorf-Dichte, Zerstörungsgrad, Kettenspuren-Verlauf,
     Kraterdichte, Infanteriewellen-Timing und Rauch feinjustieren, bis jede
     Einstellung sitzt.

### Stufe 2 – 9:16-Video aufnehmen & schneiden

1. **Vollständige 60-fps-Hochkant-Aufnahme** aller 5 Shots über
   `just trailer-shots --only=chaos_01_village_close,chaos_02_village_wide,chaos_03_meadow_wave,chaos_04_village_climax,chaos_05_meadow_hold`.
2. **Schnitt & Ton-Mastering** über `tools/trailer-cut.sh chaos build/trailer/chaos_final.mp4`
   (harte Schnitte im Rhythmus der 5 Einstellungen, Original-Spielton mit
   1,5-Frame-Anti-Klick-Fades auf `-14 LUFS`, Peak-Limiter `-1,5 dB`).
   - Hinweis zu Captions/Voiceover: Standard-Ausgabe `chaos_final.mp4` bleibt
     **pures Gameplay mit Original-Spielton** ohne Fremdtexte. Falls der Nutzer
     eigene Shorts-Hook-Texte oder eine eigene lokale Sprecherspur möchte,
     lässt sich das direkt über die in Prompt 238 geschaffenen `CAPTIONS=` /
     `VOICEOVER=`-Optionen von `tools/trailer-cut.sh` ergänzen (keine fremden
     Texte oder Stimmen aus der Vorlage übernehmen!).
3. Aus `build/trailer/chaos_final.mp4` 10 Kontroll-Frames (`t = 1, 3, 5, 7, 9,
   11, 13, 15, 17, 20 s`) ziehen und visuell auf Artefakte prüfen (kein
   Flimmern an Kettenspuren/Gras/Zäunen, saubere 9:16-Proportionen, keine
   abgeschnittenen Schatten, voller druckvoller Schlachtton ohne Übersteuerung).

## 3. Prüfen und zeigen

- **Vergleichs-Kontaktbogen** lokal in `build/chaos_compare/contact_sheet.png`
  (Schlüssel-Frames aller 5 Cuts) und fertiger Video-Pfad
  `build/trailer/chaos_final.mp4` dem Nutzer zeigen.
- **Reproduzierbarkeit & Tests:**
  - `just map-check eastern_village_chaos` und
    `just map-reach eastern_village_chaos` grün.
  - `ffprobe` auf `build/trailer/chaos_final.mp4`: `1080 × 1920` (oder
    `1440 × 2560`), `60 fps`, `yuv420p`, `~21 s`, `-14 LUFS`.
  - Zweiter Lauf von `just trailer-scout eastern_chaos ...` liefert ein
    Bit-für-Bit identisches Ereignisprotokoll.
  - `just check`, `just selftest` grün; Golden Hashes unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an Spielregeln in `crates/sim`.
- Committen von Videodateien, Referenzbildern oder lokalen Voiceover-Aufnahmen
  (bleiben in `build/` bzw. `.temp/`).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/242-video-szene-ostfront-dorfchaos-short.md prompts/done/`
verschieben, Ergebnisabschnitt (Seed, `start_s`, alle 5 Shots mit Kamera-Werten,
`ffprobe`-Daten, Vergleichsbeobachtungen) ergänzen und den Status in
`prompts/README.md` auf „erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Skizzenkarte `eastern_village_chaos` (`data/map_sketches/eastern_village_chaos.toml` → `data/maps/eastern_village_chaos/`):**
  - 800 × 800 m Sommer-Ostfront-Karte (`biome = "forest"`, `weather = "clear"`): zerstörtes Dorf (11 beschädigte/eingestürzte Fachwerk-, Backstein-, Scheunen- und Blockhäuser, 3 Holzschuppen, Ziehbrunnen `env_well_shadoof` am Dorfplatz, Gemüsegarten-Beete, 3 rosa Blütenbäume, 4 Sonnenblumen-Horste, Staketenzäune, 106 Krater) unmittelbar neben der offenen Birken-Wiesenflanke mit geschwungenem Karrenweg.
  - `just map-check eastern_village_chaos` (0 Fehler, 0 Warnungen) und `just map-reach eastern_village_chaos` (100 % Infanterie / 95,9 % Fahrzeuge im Hauptteil, alle Start- und Versorgungspunkte angebunden) grün.
- **Deterministische Schlacht & 5 dynamische 9:16-Shots (`data/trailer/eastern_chaos.toml`, `godot/scenes/tools/trailer_shots.gd`):**
  - Reine Simulationsbefehle (`order`, `fire`, `barrage`): Vorlauf-Manöver stempeln tiefe Kettenspuren ins Gras, 6 früh brennende Wracks, 24+ gestaffelte sowjetische Schützentrupps, T-34-85, IS-2, SU-85, Jagdpanther, Flakpz 38(t), Sd.Kfz. 222, Opel Blitz und spät anrollende Hummel.
  - 5 dynamisch choreographierte 9:16-Kamerafahrten (`chaos_01_village_close` 1,6 s Push-In/Orbit, `chaos_02_village_wide` 1,0 s Crane-Out/Pan, `chaos_03_meadow_wave` 2,7 s Track von den Birken zur deutschen Linie, `chaos_04_village_climax` 4,2 s 36°-Arc-Schwenk inkl. abfliegendem T-34-Turm, `chaos_05_meadow_hold` 11,7 s 4-Keyframe-Fahrt bis zur anrollenden Hummel).
- **Fertiges Video & Prüfungen:**
  - `build/trailer/chaos_final.mp4`: `1080 × 1920`, `60 fps`, `yuv420p`, `sar 1:1`, `21.20 s`, AAC `-14.0 LUFS` (`ffprobe` geprüft).
  - 10-Frame-Kontrollbogen in `build/chaos_compare/contact_sheet.png` visuell geprüft (keine Artefakte, kein Flimmern, Hummel & Ziehbrunnen sauber im Bild).
  - Determinismus über zwei unabhängige `just trailer-scout chaos_01_village_close 46 1`-Läufe Bit-für-Bit bestätigt (`diff` leer); `just check` und `just selftest` grün.
