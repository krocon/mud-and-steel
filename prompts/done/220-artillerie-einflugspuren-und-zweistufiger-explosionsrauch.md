# Prompt 220 – Einflugspuren steiler Geschosse, zweistufiger Explosionsrauch und Mündungsrauch-Ketten

> In einer frischen Session im Repo-Ordner ausführen. Baut auf Prompt 199 (Granaten als
> Lichtpunkte) und Prompt 200 (Rauch als Wolke) auf; empfohlen nach Prompt 208
> (Partikel-Material-Sharing).
> Lies zuerst `CLAUDE.md`, `docs/ARCHITECTURE.md` („Indirect fire and smoke“, „Effects“),
> `docs/PERFORMANCE.md` sowie die Ergebnisse von `prompts/done/199-granaten-als-lichtpunkte.md`
> und `prompts/done/200-rauch-als-wolke.md`. Im Code:
> `godot/scenes/world/artillery_view.gd`,
> `godot/scenes/world/rocket_effects.gd`,
> `godot/scenes/world/effects.gd`,
> `godot/scenes/world/blast_effects.gd`,
> `godot/scenes/world/effect_library.gd`,
> `godot/scenes/world/effect_textures.gd`,
> `godot/scenes/world/kill_sequence.gd`.
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `00:00–00:06`.
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hash unverändert.

## 1. Problem

Die Referenz (Sek. 00:00–00:06) zeigt vier extrem gut lesbare, dramatische Gefechtseffekte,
die sich klar von unserem aktuellen Stand abheben:

1. **Hereinfallende Projektile über dem Ziel (oben Mitte/Rechts):**
   Bei einem Artillerie-/Raketenschlag aufs Dorf sind steil (~60–75°) vom oberen Bildrand
   herabfallende **gestrichelte Ketten aus kleinen hellweißen Rauch-/Kondenswölkchen**
   zu sehen, die direkt auf den hellen orange-gelben Einschlagblitz zulaufen. So liest der
   Spieler sofort, *woher* und *wie schwer* der Schlag einfällt.
   - Bei uns zeichnet `artillery_view.gd:56-60` (Prompt 199) nur eine sehr feine, 0,3 s
     kurze blasse Linie (`TRAIL_SECONDS = 0.3`, `TRAIL_ENERGY = 0.24`), die über hellen
     Feldern oder aus 60–120 m kaum sichtbar ist.
2. **Zweistufiger Explosions- & Wrack-Rauchpilz (Vordergrund & Dorf):**
   - Bei einem Fahrzeug-Volltreffer (Sek. 00:02–00:05 auf dem linken Panzer) entsteht zuerst
     ein gleißender **gelb-oranger Feuerball + flacher, heller Boden-Druckwellen-/Staubring**
     rund um die Wanne.
   - Sowohl über frischen Explosionen als auch über dem brennenden Wrack (Mitte) steht ein
     markanter **Zwei-Ton-Rauchpilz**: Unten am Brandherd eine **dichte, helle, warm vom
     Feuer angestrahlte graue Dampf-/Staubwolke**, die nach oben rasch in eine **massive,
     samtig-tiefschwarze, hohe Ölrauch-Säule** übergeht (zusammenhängend, voluminös, keine
     kleinen verstreuten Kleckse).
3. **Flacher Bodendunst-Schleier:**
   Jede schwere Explosion legt einen **breiten, niedrigen, halbtransparenten beige-grauen
   Staub-/Rauchschleier** dicht über Straße und Wiese, der einige Sekunden schwebt und die
   Szene atmosphärisch verbindet.
4. **Mündungsrauch-Wölkchenkette bei Panzerschüssen:**
   Wenn die Panzer im Vordergrund feuern (Sek. 00:02–00:06), treibt aus jedem Rohr eine
   kurze Kette aus **4–6 weichen, runden, hellgrauen Rauchwölkchen** schräg weg und löst
   sich sanft auf.

## 2. Ziel

Reine Darstellung (alle Positionen/Treffer/Zeiten unverändert aus `SimHost`):

1. **Sichtbare Einflug-Rauchpunkt-Spuren für steile Artillerie- & Raketengeschosse (`artillery_view.gd`):**
   - Im steilen Endanflug schwerer indirekter Geschosse (z. B. Kaliber ≥ 75 mm oder
     Raketen, letzte ~1,2–2,2 s vor Einschlag) entlang der Flugbahn eine **gepunktete/gepuffte
     Kette aus kleinen, weichen, hellgrau-weißen Rauchwölkchen** (Pool / MultiMesh oder
     leichtgewichtige Partikel-Puffs, deterministisch aus Geschoss-ID und Flugzeit, kein
     `randf()` in Sim-Code).
   - Oben kleiner und transparenter, nahe dem Ziel etwas dichter und vom Einschlagblitz
     leicht warm angeleuchtet; bei Nebel des Krieges wie bisher verborgen; auf Qualitätsstufe
     `Low` vereinfacht oder aus.
   - Die warm leuchtende Geschoss-Spitze aus Prompt 199 bleibt als heller Kopf der Spur.
2. **Zweistufiger Explosions- & Wrack-Rauchpilz (`blast_effects.gd`, `effect_library.gd`, `kill_sequence.gd`):**
   - **Aufschlag:** heller gelb-oranger Kernblitz + flacher, sich ausbreitender
     **Boden-Staub-/Druckwellenring** (auf weichem Boden/Acker/Straße sandig-beige).
   - **Zwei-Ton-Säule (Aufbauend auf Prompt 200 Cloud-Rauch):**
     - Unterer Kragen am Feuer/Einschlag: voluminöse, **hellgraue Wolkenballen mit warmem
       orangefarbenem Unterlicht** (Fire-Glow aus dem Brandkern).
     - Oberer Pilz/Säule: **sehr dunkler, dichter, samtig-schwarzer Ölrauch** (stark
       überlappende große Cloud-Puffs mit klarer Sonnenseite/Schattenseite, sodass eine
       geschlossene, mächtige Silhouette entsteht).
   - **Bodendunst-Schleier:** pro großem Einschlag (≥ 100 mm, Panzer-Kill, Salve) ein breiter,
     flacher, langsam abdriftender Staub-/Dunst-Schleier (1–3 m über dem Boden, 4–7 s Lebensdauer).
3. **Mündungsrauch-Puffkette bei Panzer-/Kanonenschüssen (`effects.gd`):**
   - Zusätzlich zum vorhandenen Mündungsfeuer (Prompt 077) eine kleine, kompakte Reihe aus
     4–6 weichen hellgrauen Rauchwölkchen, die sich mit abnehmender Geschwindigkeit vor dem
     Rohr ausbreiten und mit dem Wind verziehen (auf `Low` aus).

## 3. Prüfen und zeigen

- **Vorher/Nachher-Bilder und kurzer Clip** (aus Spielhöhe und aus der Weitwinkel-Perspektive
  von Prompt 217):
  - Schwerer Artillerie-/Raketenschlag auf ein Dorf (Einflugspuren über dem Ziel deutlich
    erkennbar!).
  - Panzer-Volltreffer + brennendes Wrack (Feuerblitz, Boden-Staubring, heller Rauchfuß +
    mächtige schwarze Säule, flacher Bodendunst).
  - Feuernde Panzer-Gruppe (Mündungsrauch-Puffkette).
  - Tag, Abend und Nacht prüfen (bei Nacht leuchtet der Feuerkern den unteren Rauch warm an,
    Einflug-Puffs bleiben dezent dunkel).
- **Performance:**
  - `just effects-bench` (50 Explosionen / 40 Schüsse/s) und `just bench-view --perf-kills`
    vorher/nachher direkt hintereinander messen und in `docs/PERFORMANCE.md` eintragen.
- `just check`, `just selftest` grün; Golden Hash unverändert.

## 4. Nicht Teil dieses Prompts

- Änderungen an Schaden, Splitterradius, Flugbahnen oder Sichtblockade in `crates/sim`.
- Erstellung der Bocage-Karte und Video-Aufnahme (folgt in Prompt 221).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/220-artillerie-einflugspuren-und-zweistufiger-explosionsrauch.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md` auf „erledigt“ setzen.

## Ergebnis (2026-11-25)

Reine Darstellung ohne Änderungen an `crates/sim` (Golden Hashes unverändert):

- **Einflug-Rauchpunkt-Ketten (`godot/scenes/world/artillery_view.gd`):** Ein gemeinsamer `MultiMeshInstance3D` (`IncomingPuffs`, 1 Draw Call, bis 1024 Instanzen) mit eigenem Billboard-Shader (`INCOMING_PUFF_SHADER`) legt im Endanflug (`PUFF_WINDOW_S = 2.4 s`, `velocity.y <= -4.0 m/s`, `caliber >= 75 mm` oder Raketen) deterministische Ketten aus weichen hellgrau-weißen Rauchwölkchen (`PUFF_SPACING_M = 4.6 m`, `1.05–1.75 m` Durchmesser, leichte warme Einschlag-Aufhellung auf den letzten `0.45 s`) entlang der Flugbahn ab, die ~0,6–1,1 s in der Luft verweilen und mit dem Wind driften. Bei Nacht bleiben die Wölkchen dezent dunkel (`sky_lift = 0.0`, `alpha_scale = 0.22`), im Nebel des Krieges für gegnerische Geschosse verborgen, auf `Low` über `fx_shell_trails = false` aus. Die leuchtende Geschoss-Spitze aus Prompt 199 bleibt als heller Kopf erhalten. *(Schwellenwerte, Puff-Größen und Lebensdauern vorläufig.)*
- **Zweistufiger Explosions- & Wrack-Rauchpilz + Bodendunst (`effect_library.gd`, `effects.gd`, `blast_effects.gd`):**
  - `CLOUD_BODY` in `effect_library.gd` unterstützt optionales warmes Feuer-Unterlicht (`fire_glow`, default `0.0`), das untere Rauchkragen bei Tag warm anstrahlt und bei Abend/Nacht deutlich vom Brandkern aus beleuchtet.
  - Neue Wolken-Typen `smoke_collar` (Burst, hellgrau mit `fire_glow = 1.05`), `wreck_collar` (kontinuierlich über brennenden Wracks mit `fire_glow = 0.9`, auf `Low` über `fx_clouds` aus) und `ground_haze` (breiter, niedriger, halbtransparenter beige-grauer Bodendunst-Schleier über `5.5 s`, Pool `12`, auf `Low` aus).
  - `explosion_smoke` und `smoke_column` verdichtet und als samtig-tiefschwarze obere Ölrauch-Säule (`#030303`–`#050505` Kernfarbe, geschlossene Cloud-Silhouette) über dem hellen Rauchfuß abgestimmt; `fireball` leuchtender gelb-orange; Fahrzeug-Explosionen (`EXPLOSION_LAYERS["vehicle"]`) erhalten sandig-beigen Bodenstaub (`sand_dust`) plus flachen Druckwellenring (`blast_effects.ground_ring`); schwere Einschläge (`>= 100 mm`) auf Boden/Objekten legen ebenfalls Zwei-Ton-Rauch und Bodendunst. *(Farben, Größen, Schwellenwert `100 mm` und Pool-Größen vorläufig.)*
- **Mündungsrauch-Puffkette (`effect_library.gd`, `effects.gd`):** Neuer Burst `muzzle_puffs` (`6` weiche runde hellgraue Wölkchen, `2.6 s` Lebensdauer, außerhalb von `CLOUDS` damit die einzelnen Puffs als 4–6er-Kette lesbar bleiben) bei Kanonen-, schweren Kanonen- und Haubitzenschüssen (`kind in ["gun", "heavy", "howitzer"]`), auf `Low` über `fx_muzzle_detail = false` aus. *(Anzahl, Größe und Lebensdauer vorläufig.)*
- **Visuell geprüft & Performance gemessen:** Vorher/Nachher-Standbilder lokal in `build/fx_220/` gegen `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4` (Sek. 0–6) verglichen (Mörser Tag/Nacht, Haubitzen, Panzerduell, brennende Wracks Tag/Abend, Stadtbeschuss `battle_open`). Erste Nachher-Messung zeigte bei 40–50 Explosionen im 60-m-Nahzoom +7–11 ms Overdraw auf `high`; nach Straffung von Pool-Größen und Spät-Skalierung liegen `just effects-bench` und `just bench-view --perf-kills` auf Vorher-Niveau (alle Werte in `docs/PERFORMANCE.md` eingetragen). `just check` und `just selftest` grün.
