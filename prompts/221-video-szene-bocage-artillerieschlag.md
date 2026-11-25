# Prompt 221 – Video-Szene „Bocage-Artillerieschlag“ (Karte, Skript-Gefecht und 1080p-Aufnahme)

> In einer frischen Session im Repo-Ordner ausführen. **Empfohlen nach Prompts 217–220**
> (Weitwinkel-Perspektive, Bocage-Felder/-Hecken, Normandie-Häuser/Kirche,
> Einflugspuren & zweistufiger Explosionsrauch).
> Lies zuerst `CLAUDE.md`, `docs/MAPS.md`, `docs/MISSIONS.md`, `docs/ARCHITECTURE.md`
> („Trailer shots“) sowie als Vorbild `prompts/periodic/082-video-hinterhalt-am-waldrand.md`.
> Im Code:
> `data/map_sketches/`,
> `data/trailer/ambush.toml`,
> `data/trailer/shells.toml`,
> `godot/scenes/tools/trailer_shots.gd`,
> `tools/trailer-shots.sh`,
> `tools/trailer-scout.sh`,
> `tools/trailer-cut.sh`.
>
> **Stil-Referenz (nur lokal ansehen, nichts ins Repo übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:00–00:06`.
> **Keine Bilder, Namen oder Assets aus dem Video ins Repo**; Vergleichsbilder nur lokal
> in `build/bocage_compare/` ablegen (`build/` ist gitignored).
> Alle Bewegungen, Schüsse und Treffer müssen echte deterministische Simulations-Befehle
> sein – **keine Cheats, keine Godot-Physik, Golden Hash unverändert**.

## 1. Ziel

Ein deterministisch reproduzierbares, direkt in Godot aufgezeichnetes **1080p-Video**
(6–15 Sekunden Haupt-Einstellung, `1920 × 1080 @ 60 fps` mit Original-Spielton), das die
Stimmung, Bildkomposition, Perspektive und Gefechtswirkung der ersten 6 Sekunden der
Referenz in *Mud and Steel* erreicht – rein mit unseren eigenen WW2-Assets, eigener
Skizzenkarte und echter Simulation:

- **Vordergrund (links/Mitte):** Ein US-Panzerzug (z. B. 4–5× `us_m4` / `us_m4a3_75` +
  1× `us_m3_halftrack` oder Lkw) rollt entlang eines leicht geschwungenen Feldwegs und über
  ein helles, rechteckiges Stoppelfeld zwischen dicken Bocage-Hecken.
  - Das Führungsfahrzeug auf dem Feldweg brennt bereits stark mit hellem Rauchfuß und
    hoher, tiefschwarzer Rauchfahne; ein nachfolgender Panzer weicht seitlich aus.
  - Ein Panzer auf dem hellen Stoppelfeld erhält im laufenden Bild einen Volltreffer
    (gleißender Feuerblitz, Boden-Staubring, aufsteigender Zwei-Ton-Rauchpilz).
  - Die übrigen Panzer im Vordergrund feuern über die Hecken ins Dorf (sichtbare
    Mündungsrauch-Puffketten).
- **Mittelgrund:** Scharf parzelliertes Bocage-Patchwork aus hellem Strohfeld, dunklen
  gepflügten Äckern, grünen Reihenfeldern, olivgrünen Wiesen, dichten Heckenwällen mit
  Durchfahrten und einzelnen runden Laubbäumen.
- **Hintergrund (oben Mitte/Rechts):** Ein kleines Normandie-Dorf an der Wegkreuzung mit
  steinerner Dorfkirche und zwei bis drei ländlichen Steinhäusern (dunkle Schiefer- und
  rote Ziegeldächer), das von einer **schweren Artillerie- oder Raketensalve** getroffen
  wird (steil herabfallende helle Projektil-Rauchpunkt-Ketten, mehrfache Einschläge mit
  Feuerblitzen, schwarzen Rauchpilzen und breitem Bodendunst). Dahinter schließt eine
  dichte Wald-Wand den Horizont vollständig ab.
- **Kamera:** Ruhige (oder minimal schleichende) Weitwinkel-Perspektive nach Prompt 217
  (`fov ≈ 50–54°`, `pitch ≈ 31–35°`, `dist ≈ 60–75 m`), sodass der Vordergrund steil von
  oben und der Hintergrund flach in die Tiefe wirkt.

## 2. Umsetzung in zwei Stufen

### Stufe 1 – Karte und Szene aufsetzen & scouten
1. **Skizzenkarte `data/map_sketches/bocage_village.toml` anlegen** und mit
   `just gen-sketch bocage_village` erzeugen (und per `just map-check bocage_village` und
   `just map-reach bocage_village` prüfen; falls sinnvoll auch als spielbare Skirmish-Karte
   nutzbar, oder reine Trailer-Karte – wie `ambush_meadow` aus Prompt 082 entscheiden):
   - Leicht hügeliges oder sanft ansteigendes Gelände Richtung Dorf/Wald (unterstützt die
     Tiefenwirkung!).
   - Exaktes Parzellen-Raster im Kamerablickfeld: großes helles Strohfeld links vorne,
     dunkler gepflügter Acker links außen, grünes Reihenfeld in der Mitte, weitere Äcker
     dahinter, geschwungener Feldweg von Südwest nach Nordost, durchgehende Bocage-Hecken
     mit Lücken, Einzelbäume, kleines Dorf mit Kirche + Gehöften an der Kreuzung, dichter
     Nadel-/Mischwald entlang des gesamten Nordrands.
   - Gedämpfte, leicht bedeckte Westeuropa-Lichtstimmung (Prompt 218).
2. **Szenen-Skript `data/trailer/bocage_barrage.toml` schreiben** (Format: `docs/MISSIONS.md`,
   wie `data/trailer/ambush.toml`):
   - Alle Aktionen über reguläre Simulationsbefehle (`order`, `fire`, `barrage` / Artillerie
     bzw. Raketenwerfer außerhalb des Bildes, versteckte Pak/Panzer im Dorf als Schütze
     gegen den linken Feld-Panzer und das Führungsfahrzeug).
   - Mit `just trailer-scout` einen deterministischen Seed + exaktes `start_s`-Timing
     ermitteln, sodass:
     - Bei `t = 0 s` des Shots das Führungsfahrzeug auf dem Weg bereits brennt (volle
       Rauchsäule durch `pre_roll_s`),
     - Zwischen `t = 1–3 s` der Panzer auf dem Strohfeld getroffen wird und hochgeht,
     - Über die gesamte Dauer (`t = 0–6 s`) die Shermans feuern und die schwere
       Artillerie-/Raketensalve steil auf Kirche und Ortseingang niedergeht!
3. **Standbilder ziehen & iterieren:**
   - Shot `bocage_barrage` in `godot/scenes/tools/trailer_shots.gd` eintragen
     (`stills = [0.5, 1.5, 2.5, 3.5, 4.5, 5.5]`).
   - `just trailer-shots --only=bocage_barrage --stills` laufen lassen und die 6 Standbilder
     neben die 6 Referenz-Frames (Sek. 1–6 aus `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`,
     nur lokal in `build/bocage_compare/`) vergleichen; Kamera-Winkel, Feld-Kontraste,
     Hecken-Platzierung und Rauchdichte feinjustieren, bis Bildkomposition und Stimmung sitzen.

### Stufe 2 – Video aufnehmen
1. **Vollständige 1080p-60fps-Aufnahme** über `just trailer-shots --only=bocage_barrage`
   (plus ggf. `just trailer-cut`, Ausgabedatei in `build/trailer/bocage_barrage.mp4` –
   bleibt in `build/`, nicht committen!).
2. Aus der fertigen MP4-Datei erneut 6 Einzelbilder (`t = 1..6 s`) ziehen und visuell auf
   Artefakte prüfen (kein Flimmern, keine abgeschnittenen Schatten, keine stehenden
   Partikel-Kanten, sauberer Ton ohne Übersteuerung).

## 3. Prüfen und zeigen

- **Vergleichs-Kontaktbogen** lokal in `build/bocage_compare/` (6 Frames unserer Aufnahme
  bei `t = 1..6 s`) und Video-Pfad `build/trailer/bocage_barrage.mp4` dem Nutzer zeigen.
- **Reproduzierbarkeit & Tests:**
  - `just map-check bocage_village` grün.
  - `just check`, `just selftest` grün; Golden Hashes unverändert.
  - Zweiter Lauf mit demselben Seed erzeugt deterministisch denselben Ablauf.

## 4. Nicht Teil dieses Prompts

- Änderungen an Spielregeln in `crates/sim`.
- Committen von großen Video-Dateien oder Referenzbildern (bleiben in `build/` bzw. `.temp/`).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/221-video-szene-bocage-artillerieschlag.md prompts/done/`
verschieben, Ergebnisabschnitt (Seed, Kamera-Werte `fov`/`pitch`/`dist`, Bildrate,
Vergleichsbeobachtungen) ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.
