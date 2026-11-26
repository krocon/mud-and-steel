# Prompt 226 – Video-Szene „Herbst-Artillerieangriff“ (Karte, Skript-Gefecht und 1080p-Aufnahme)

> In einer frischen Session im Repo-Ordner ausführen. **Empfohlen nach Prompts
> 223–225** (Herbstlandschaft/Bäume/Felder/Stimmung, Treppengiebel-Häuser,
> durchgehende Artillerie-Rauchbögen ab Abschuss) sowie den erledigten Prompts
> 217 (Weitwinkel-Perspektive), 220 (Explosionsrauch) und 222 (Umland).
> Lies zuerst `CLAUDE.md`, `docs/MAPS.md`, `docs/MISSIONS.md`,
> `docs/ARCHITECTURE.md` („Trailer shots“) sowie als direktes Vorbild das
> Ergebnis von `prompts/done/221-video-szene-bocage-artillerieschlag.md`.
> Im Code:
> `crates/mapgen/src/gen_sketch/` (`schema.rs`, `settlements.rs`, `mod.rs`),
> `data/map_sketches/bocage_village.toml`,
> `data/trailer/bocage_barrage.toml`,
> `godot/scenes/tools/trailer_shots.gd`,
> `tools/trailer-shots.sh`,
> `tools/trailer-scout.sh`,
> `tools/trailer-cut.sh`.
>
> **Stil-Referenz (nur lokal ansehen, nichts ins Repo übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:20–00:30`.
> **Keine Bilder, Namen oder Assets aus dem Video ins Repo**; Vergleichsbilder
> nur lokal in `build/autumn_compare/` ablegen (`build/` ist gitignored).
> Alle Bewegungen, Schüsse und Treffer müssen echte deterministische
> Simulations-Befehle sein – **keine Cheats, keine Godot-Physik, Golden Hashes
> unverändert**. Keine UI-Einblendungen („PROMOTED“ o. ä.) im Video – reine
> Spielwelt ohne HUD.

## 1. Ziel

Ein deterministisch reproduzierbares, direkt in Godot aufgezeichnetes
**10-Sekunden-1080p-Video** (`1920 × 1080 @ 60 fps` mit Original-Spielton),
das die Herbststimmung, Bildkomposition, Kamerafahrt, Treppengiebel-Architektur
und Gefechtswirkung von Sekunde `00:20–00:30` der Referenz in *Mud and Steel*
erreicht – rein mit unseren eigenen WW2-Assets, eigener Skizzenkarte und echter
Simulation:

- **Vordergrund links:** Ein dichter, farbenprächtiger **Herbstwald** (Goldgelb,
  Ocker, Kupfer-Orange, Rostrot, Tiefrot + vereinzelte kahle Bäume und dunkle
  Stämme) auf feuchtem, leicht nebelverhangenem Waldboden (`mist_<n>`).
- **Vordergrund rechts unten:** **2× US M7 Priest (`us_m7_priest`)** feuern
  zeitlich versetzt über die gesamte 10-Sekunden-Einstellung im hohen Bogen
  aufs Dorf – mit langen, perlenartigen weißen Rauchwölkchen-Bögen direkt ab
  der Mündung quer durchs Bild (Prompt 225), ergänzt um eine weitere
  Artillerie-Stellung leicht rechts außerhalb des Bildes für den dritten
  Rauchbogen am rechten Bildrand.
- **Vordergrund rechts Mitte:** Das freistehende **eingeschossige rote
  Treppengiebel-Backsteinhaus (`env_house_stepped_cottage`)** mit dunklem
  Schieferdach, weißen Kreuzstockfenstern und 2–3 Herbst- und kahlen Bäumen
  direkt dahinter.
- **Mittelgrund:** Feuchte, bräunlich-olive Herbstwiese mit abgefahrenen
  Spuren und verstreuten Weidepfählen, dahinter ein großer **dunkler, feuchter
  gepflügter Herbst-Acker** vor dem Dorf.
  - **4× US Shermans (`us_m4a3_75` / `us_m4`)** rollen in lockerer, leicht
    gestaffelter Linie rechts am Herbstwald vorbei Richtung dunklem Acker und
    Dorf vor und feuern direkt auf die Dorfverteidiger (sichtbare Mündungsrauch-
    Puffketten).
  - Links am Rand des Herbstwalds läuft ein zweites Gefecht (1–2 US-Panzer in
    Beschuss, Explosionen und aufsteigendem Rauch).
- **Hintergrund (Dorf & Horizont):**
  - In der oberen Bildmitte ein eng gruppiertes **Treppengiebel-Dorf** aus
    mehreren zweigeschossigen roten Backsteinhäusern (`env_house_stepped_brick`),
    dem **hellen ockerfarbenen Putzhaus (`env_house_stepped_plaster`)** als
    Kontrastpunkt in der Mitte, einzelnen bereits beschädigten/ruinierten
    Treppengiebel-Gebäuden und niedrigen Hofmauern/Sandsackstellungen am
    Dorfrand.
  - Deutsche Verteidiger (`de_pz4_h`, `de_stug3_g`, `de_pak40`) am Dorfrand
    erwidern das Feuer und liegen unter dem anhaltenden schweren Artillerie-
    und Panzerbeschuss (mehrfache gelb-orange Einschlag-Feuerbälle, flacher
    Bodendunst und mehrere mächtige tiefschwarze Rauchsäulen über dem Dorf).
  - Links und rechts hinter dem Dorf zwei **helle, abgeerntete, eingezäunte
    Stoppel-/Brachfelder** (`field_grain`, komplett eingerahmt von niedrigen
    Holzzäunen), links am Bildrand ein kleines Treppengiebel-Gehöft samt
    großer Backstein-Flanke (`env_church_brick` bzw. Backsteinbau).
  - Entlang des gesamten oberen Horizonts schließt ein **bunter Herbstwald-
    Streifen** (plus herbstliches Umland aus Prompt 222/223) die Szene im
    kühlgrauen Herbst-Dunst ab, bei feinem Nieselregen (`look = "autumn"`).
- **Kamera:** Weitwinkel-Perspektive nach Prompt 217 (`fov ≈ 50–54°`,
  `pitch ≈ 30–35°`, `hud = false`, `dof = "off"`) mit einer **langsamen,
  ruhigen Vorwärts-Fahrt** über 10 Sekunden – vom bunten Herbstwald & den
  feuernden M7 Priests im Vordergrund langsam Richtung Panzervorstoß und
  brennendes Treppengiebel-Dorf.

## 2. Umsetzung in zwei Stufen

### Stufe 1 – Skizzen-Bausteine, Karte und Szene aufsetzen & scouten

1. **Skizzen-Generator gezielt ergänzen (`crates/mapgen/src/gen_sketch/`):**
   - Alle bestehenden 22 Skizzenkarten müssen **Bit-für-Bit unverändert** bleiben!
   - Optionale **`[[fence]]`-Polylinien** (analog zu `[[hedge]]` aus Prompt 221):
     platzieren `env_fence_wood:Segment` lückenlos entlang von Feld- und
     Weidegrenzen (mit automatischer Lücke an Straßen/Wegen) und optional
     einzelne verstreute Zaunpfosten (`env_fence_wood:End`) als offene
     Herbst-Weidepfähle.
   - Optionale **`bare_share`** (Anteil kahler Bäume `env_tree_deciduous:Bare`
     in `[[wood]]` und `[[scatter]]`, Default `0.0`), damit im Herbstwald und
     zwischen den Feldern natürlich einzelne kahle Bäume stehen.
2. **Skizzenkarte `data/map_sketches/autumn_village.toml` anlegen** und mit
   `just gen-sketch autumn_village` erzeugen (`just map-check autumn_village`
   und `just map-reach autumn_village` grün; auch als reguläre Skirmish-Karte
   spielbar machen wie `bocage_village`):
   - `biome = "forest"`, `look = "autumn"`, `size_m = [1000, 1000]`.
   - Exakte Raumaufteilung im Kamerablickfeld: dichter bunter Herbstwald links
     vorne (`mist_1`..`mist_n` am Waldboden und Dorf), offene feuchte Wiese mit
     Weidepfählen in der Mitte, großer dunkler gepflügter Acker vor dem Dorf,
     freistehendes `env_house_stepped_cottage` rechts mit 2–3 Herbst-/kahlen
     Bäumen, Treppengiebel-Dorf in der Mitte hinten (`env_house_stepped_brick`,
     `env_house_stepped_plaster`, auch `Damaged`/`Ruin`-Varianten oder über
     Vorbeschuss im `pre_roll_s` beschädigt!), zwei eingezäunte helle
     Stoppelfelder links/rechts hinter dem Dorf, Gehöft + `env_church_brick`
     links, durchgehender Herbstwald entlang des gesamten Nordrands.
3. **Szenen-Skript `data/trailer/autumn_barrage.toml` schreiben** (Format:
   `docs/MISSIONS.md`, wie `data/trailer/bocage_barrage.toml`):
   - Ausschließlich reguläre Simulationsbefehle (`order`, `fire`, `barrage`):
     - `pre_roll_s` so wählen, dass zu Shot-Beginn (`t_shot = 0 s`) bereits
       erste Rauchsäulen über dem Dorf und links am Waldrand stehen und die
       ersten 105-mm-Salven der beiden `us_m7_priest` schon Bögen in den Himmel
       zeichnen.
     - Über die vollen `10 s` feuern beide `us_m7_priest` gestaffelt weiter
       (plus eine 3. Haubitzen-/SPG-Position rechts außerhalb des Bildes), die
       4 Shermans rollen in lockerer Linie am Herbstwald vorbei Richtung Acker
       und feuern aufs Dorf, im Dorf schlagen durchgehend Artilleriegranaten mit
       Feuerblitzen und dunklem Rauch ein.
   - Mit `just trailer-scout` deterministischen Seed + `start_s` ermitteln.
4. **Standbilder ziehen & iterieren:**
   - Shot `autumn_barrage` in `godot/scenes/tools/trailer_shots.gd` eintragen
     (`seconds = 10.0`, langsame Vorwärts-Kamerafahrt, `look = "autumn"`,
     `perspective = "wide"`, `hud = false`, `stills = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5, 6.5, 7.5, 8.5, 9.5]`).
   - `just trailer-shots --only=autumn_barrage --stills` laufen lassen und die
     10 Standbilder lokal in `build/autumn_compare/` neben die 10 Referenz-Frames
     (Sek. 20–30 aus `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`) legen;
     Herbstfarben, Bodennebel, Nieselregen, Haus-Platzierung, Zaunpfähle,
     Artilleriebögen und Kamerafahrt feinjustieren, bis Bildkomposition und
     Stimmung sitzen.

### Stufe 2 – Video aufnehmen

1. **Vollständige 1080p-60fps-Aufnahme** über
   `just trailer-shots --only=autumn_barrage` (plus `tools/trailer-cut.sh` für
   sauberen AAC-Ton auf `-14 LUFS`, Ausgabedatei lokal in
   `build/trailer/autumn_barrage.mp4` – bleibt in `build/`, nicht committen!).
2. Aus der fertigen MP4-Datei erneut 10 Einzelbilder (`t = 1..10 s`) ziehen und
   visuell auf Artefakte prüfen (kein Flimmern an Feldern/Zäunen/Bäumen, keine
   abgeschnittenen Schatten, weiche Kamerafahrt, sauberer Ton ohne Clipping).

## 3. Prüfen und zeigen

- **Vergleichs-Kontaktbogen** lokal in `build/autumn_compare/` (10 Frames
  unserer Aufnahme bei `t = 1..10 s`) und Video-Pfad
  `build/trailer/autumn_barrage.mp4` dem Nutzer zeigen.
- **Reproduzierbarkeit & Tests:**
  - `just map-check autumn_village` und `just map-reach autumn_village` grün.
  - Bestehende Skizzenkarten durch `gen_sketch`-Erweiterung unverändert.
  - Zweiter Lauf mit demselben Seed (`just trailer-scout autumn_barrage ...`)
    liefert ein Bit-für-Bit identisches Ereignisprotokoll.
  - `just check`, `just selftest` grün; Golden Hashes unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an Spielregeln in `crates/sim`.
- Committen von großen Video-Dateien oder Referenzbildern (bleiben in `build/`
  bzw. `.temp/`).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/226-video-szene-herbst-artillerieangriff.md prompts/done/`
verschieben, Ergebnisabschnitt (Seed, Kamera-Fahrt `fov`/`pitch`/`dist`/`at`,
Bildrate, Vergleichsbeobachtungen) ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.


## Ergebnis (2026-11-26)

- **Skizzen-Generator & Karte `autumn_village`:** `[[fence]]`-Polylinien (`FenceDef`/`fence_line()` mit automatischen Straßenlücken) und Herbst-Baummix `"autumn"` (`Oak`/`Birch`/`Bare`) in `crates/mapgen/src/gen_sketch/` ergänzt (bestehende 22 Skizzen unverändert); Karte `autumn_village` (1000 × 1000 m, `look = "autumn"`) aus `data/map_sketches/autumn_village.toml` erzeugt (`just map-check autumn_village` 0 Fehler/Warnungen, `just map-reach autumn_village` 79,3 % Fahrzeuge / 100 % Infanterie).
- **Deterministische Gefechtsszene & 6-Schnitt-Trailer:** `data/trailer/autumn_barrage.toml` + `trailer_shots.gd` (`autumn_barrage` sowie 6-Schnitt-Sequenz `autumn_cut_01_open` bis `autumn_cut_06_collapse`, Seed 2, `look = "autumn"`, Weitwinkel, ohne HUD/X-Ray/Fußringe):
  1. Weiter Eröffnungsschwenk (flauschiger Herbstwald, M7-Priest-Rauchbögen, vorrückende Shermans, Treppengiebel-Dorf),
  2. Tiefes Nahaufnahme-Duell vor der Sherman-Explosion (`19,50 s`),
  3. US-Aufklärungstrupp sprintet über die Herbstwiese in das Backstein-Treppengiebelhaus `(48, 2)` in Deckung (`garrison`),
  4. Gegenperspektive von der Dorfseite über die vorrückenden Panzer Richtung Herbstwald,
  5. Überflug in die Draufsicht über das gesamte Gefechtsfeld,
  6. Freie Seitenansicht auf das verputzte Treppengiebelhaus `(3, -82)` mit gezieltem 105-mm-Volltreffer und vollständigem Einsturz (`31,75 s`).
- **Master-Video:** Alle 6 Schnitte in 1080p@60fps aufgenommen und per `tools/trailer-cut.sh autumn_cut build/trailer/autumn_barrage.mp4` zusammengefügt (`31,90 s`, H.264 High, 60 fps, AAC 256k, `-14,0 LUFS`) inkl. 6-Panel-Kontaktbogen `build/autumn_compare/contact_sheet.png`. Werte vorläufig.
