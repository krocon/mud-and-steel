# Prompt 060 – Berge und Täler

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/MAPS.md` (Höhenfeld, Hangneigung, Generator, Editor), `docs/ARCHITECTURE.md`
> (Terrain rendering, Wegfindung, Nebel), `docs/PERFORMANCE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Das Höhenfeld kann schon beliebige Höhen (i32 Millimeter, 2-m-Raster), Hänge über 60 %
sind unbefahrbar, die Sicht folgt dem Gelände (018), und der Editor hat Heben/Senken.
Der Generator baut aber nur sanfte Hügel (bis etwa 60 m). Nach diesem Prompt gibt es
Karten mit echten Bergen, Kämmen, Pässen und Tälern: Höhen beherrschen das Feld, Pässe
und Talstraßen werden zu Engstellen, Hänge geben Deckung vor Sicht und Feuer.

## 2. Umfang

1. **Generator:** Gelände-Profile (`--relief gentle|hilly|mountains`): Bergkämme mit
   Grat-Rauschen, Täler mit Fluss in der Sohle (014), Pässe als befahrbare Sättel,
   Serpentinen-Straßen mit Hangneigung unter der Grenze, Felsabbrüche als
   unpassierbare Zonen. Starts und Versorgungspunkte erreichbar (map-check prüft).
2. **Regeln** prüfen und ggf. ergänzen: Steigfähigkeit pro Fahrzeug (Katalog), Rad vs.
   Kette am Hang, Infanterie langsamer bergauf, Sichtvorteil von Höhen (größere
   Sichtweite ab Höhenunterschied), Hangdeckung; mit Begründung, im Hash.
3. **Darstellung:** Fels und Geröll nach Hangneigung im Splatting (011), Schnee über
   einer Höhengrenze im Winterbiom, Nadelwald an Hängen, Nebel in Tälern (015/040),
   Höhenlinien optional auf der Minikarte (031); Kamera bleibt über hohem Gelände
   bedienbar (Zoomgrenzen relativ zum Boden).
4. **Editor (028):** Pinsel Kamm/Tal/Glätten/Terrasse, Straße am Hang mit automatischer
   Begrenzung der Steigung.
5. **Karte:** eine Gebirgskarte (z. B. Talkessel mit Pass) für 2 und 4 Spieler.
6. **Höhenbereich:** `height_step_mm` so wählen, dass 300–500 m Relief möglich sind;
   Speicher- und Ladezeit messen (`docs/MAPS.md` „If 8 km maps ever come“).

## 3. Akzeptanzkriterien

1. `just check` grün; Tests für neue Hangregeln und Sichtvorteil; Golden-Hash nur
   geändert, wenn Regeln es verlangen, mit Begründung.
2. `just map-check` der Gebirgskarte, `just editor-selftest`, `just selftest` grün;
   Wegfindungs- und Terrain-Benchmark auf der Gebirgskarte im Budget von 016.
3. Doku: `docs/MAPS.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Concretized (decisions, prompt 060 run)

Starting point checked in the code: heights are `i32` mm with a 16-bit file
encoding whose step the generator and the editor already pick from the
range (65 m per mm of step), so 300–500 m of relief need no format change
(step 5–8 mm). Slope rules: one grade limit (60 %) and one uphill penalty
for all units; passability per size class only. Vision 018 already lets
heights look over cover and hides reverse slopes. The camera keeps its
focus on the ground and lifts above hillsides. The shader already blends
earth and rock by grade.

| Question | Decision | Why |
|---|---|---|
| Generator interface | `--relief gentle\|hilly\|mountains`, default `gentle`; own random stream; layout and objects unchanged | Test maps regenerate bit-identically (checked); one generator for all maps. |
| Mountain shape | Ridged value noise (4 octaves, squared) under a massif noise, up to 600 m; valleys along every track, road and the river at the meadow level; basins 30 m lower around distant starts; cliff bands as 24 m benches | Tracks become valleys and passes without a separate route planner; starts and supply points stay reachable by construction (map-check proves it). |
| Serpentine | One fixed serpentine west of the central meadow (6 legs, hairpins, 11 %) up to a lookout plateau with a supply point | Shows the feature with simple geometry; a general road planner on arbitrary terrain is much more work [provisional]. |
| Cliffs | Steep steps (> 60 %), no new terrain id | A rock terrain id would change `terrain.toml` and the hash of every map; steepness is already impassable and the shader draws it as rock. |
| Per-vehicle climbing, wheels vs. tracks | Not done | No sourced climbing values in the catalog (SOURCES.md), and a grade limit per vehicle needs another passability/component grid per limit [provisional]. |
| Infantry uphill | Unchanged (same penalty as vehicles) | Already slower uphill. |
| Height advantage | Downhill sight bonus: ground > 20 m below the eye is seen up to 5 % farther per 10 m, at most 25 % [provisional] | The line of sight already gives cover and overview; this adds range only where heights matter. Golden hash changes only by the hashed rules (test_small has no such drops). |
| Slope cover | Nothing new | Reverse slopes already hide through the line of sight; a fire modifier would be a new combat rule without need. |
| Look | Scree, rock strata, grass on steeper slopes and a snow line on maps with > 120–220 m relief (from the height range, no new map field) [provisional]; conifers above 25 m, tree line 200 m | Presentation only, no format change; existing maps keep their look. |
| Fog in valleys, minimap contours | Open | Optional in the prompt; weather fog (040) is global. |
| Editor | Ridge/Valley (linear falloff), Terrace (5 m) [provisional], Road on slope (12 % max) [provisional] with 50 % cuttings | Each a few lines on top of the existing brush machinery; the road works piece by piece so the level follows the ground ahead. |
| Map | `mountain_pass`: 2 km, 4 starts (2 or 4 players), starts in basins, pass in the middle | One map for both team sizes, like `twin_meadow`. |
