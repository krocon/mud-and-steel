# Prompt 042 – Infanterie: Texturen, Tarnmuster und mehr Animationen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/ASSETS.md`, `docs/SOURCES.md`.

## 1. Ziel

Soldaten haben flache Slot-Farben ohne Texturen und Tarnmuster („No textures yet“), und
manche Trupp-Mitglieder haben keine eigene Waffe (Karabiner, Pistole). Nach diesem
Prompt sehen die drei Fraktionen aus der Spielkamera unterscheidbar und glaubwürdig aus.

## 2. Ausgangslage (geprüft 2026-11-15)

- **Modelle:** ein Körper, ein Skelett, 25 prozedurale Clips für alle drei Fraktionen
  (`tools/blender/soldier/`, `just soldiers`, 57 Rollen-Kombinationen, ~3,5 s pro
  Soldat). Materialien sind nur die Slots `Cloth`, `Canvas`, `Leather`, `Paint`, `Skin`,
  `Metal`, `Wood`, `Rubber`, `Glass`, `Insignia` mit Vertex-Tönung; UV ist eine
  Frontprojektion ohne Nutzen.
- **Godot:** `SoldierLook` (`godot/scenes/tools/soldier_look.gd`) setzt pro Fraktion,
  Biom, Slot und Hautton ein gecachtes `StandardMaterial3D` aus
  `uniform_palettes.json`. Spiel (`infantry_view.gd`, auch die fernen MultiMesh-
  Soldaten), Galerie und Infanterie-Vorschau benutzen es.
- **Fahrzeuge/Umwelt** haben keine Bilddateien: Muster sind prozedural im Shader
  (`unit_camo.gdshader`, `env.gdshader`).
- **Animationen schon vorhanden:** `reload`, `throw_grenade`, `crawl`, `hit`,
  `death_a`, `death_b`, `mortar_operate`, `mg_setup`, `mg_carry`, `medic_treat`.
- **Waffen-Lücken in den Trupp-Dateien:** `de_inf_medic` zeigt den zweiten Mann mit
  Kar98k statt P 38; `us_inf_command` zeigt den Funker mit M1 statt Karabiner und die
  Pistole des Offiziers nicht; `su/officer` hat nur das Holster. Pistolen gibt es als
  Waffe gar nicht. Panzerfaust ist kein Katalogwaffe (nur Ausrüstung des
  `at_assistant`), Bazooka und PTRD gibt es.

## 3. Entscheidungen

| Frage | Entscheidung | Begründung |
|---|---|---|
| Texturen | **Prozedural im Shader, keine Bilddateien.** Neuer Shader `godot/assets/shaders/soldier.gdshader` mit Mustern je Slot: Stoff (Gewebe, Falten, Nähte), Segeltuch (grobes Gewebe), Leder (Narben, Glanz an Kanten), Metall (brüniert, abgegriffen), Holz (Maserung längs der Waffe), Haut (leichte Rötung, Bartschatten), Helmlack (rau, Kratzer). | Gleiche Regel wie bei Fahrzeugen und Umwelt; Texturspeicher 0 statt 1024² + 512² pro Soldat; Biome und Fraktionen kosten nichts. Aus 15–600 m zählen Farbe und grobe Struktur, nicht Pixel-Details. |
| Muster-Koordinaten | Der Blender-Build schreibt die **Ruhepose-Position** jedes Vertex in die UVs: `UV` = (x, z), `UV2` = (y, 0) in Metern. Der Shader rechnet damit statt mit `VERTEX`. | Muster kleben am Stoff, auch wenn der Körper animiert ist (VERTEX ist nach dem Skinning). Kein UV-Layout von Hand. |
| Material-Verteilung | `SoldierLook` bleibt die Stelle: ein gecachtes `ShaderMaterial` pro Fraktion × Biom × Slot × Hautton × Tarnung, **normale Uniforms** (keine Instanz-Uniforms). | Gleich viele Draw Calls wie vorher; die fernen MultiMesh-Soldaten (016) bekommen dieselben Materialien. |
| Tarnmuster | **de: Splittertarn** (Heer, 1931–45: Splitter in Braun und Grün auf Feldgrau-Beige, dunkelgrüne Regenstriche). **su: Amöben-Tarnanzug** (Scharfschützen und Aufklärer: dunkle Flecken auf Oliv-Khaki). **us: M1942 „Frogskin“** (fünffarbige Flecken auf Blassgrün). Prozedural (Voronoi-Splitter, Rausch-Flecken, Punkt-Flecken), Größen in echten Metern. | Ein belegtes Muster pro Fraktion reicht aus Spielkamera-Entfernung; Quellen in `docs/SOURCES.md`. |
| Erbsentarn, Sumpftarn | **Nicht umgesetzt.** [provisorisch] | Erbsentarn trug nur die Waffen-SS; das Projekt meidet SS-Verbände (`docs/SOURCES.md`). Sumpftarn (1943) wäre ein zweites Heer-Muster ohne Mehrwert auf Distanz. |
| Wer trägt Tarnung | Variante `camo` je Soldat. **Standard (`auto`):** de- und su-`sniper` und `scout` tragen sie im Wald-Biom; us trägt Frogskin nur als wählbare Variante, nicht standardmäßig. Schnitt: de **Tarnjacke + Helmbezug** (Oberkörper, Arme, Oberschenkel, Helm), su **ganzer Overall**, us **zweiteilige Uniform** (ganzer Körper). [provisorisch] | Historisch: Heer-Tarnjacken und Helmbezüge 1942–45, sowjetische Tarnanzüge für Scharfschützen/Aufklärer; Frogskin trug die US Army in Europa nur kurz (Sommer 1944, z. B. 41st Armored Infantry), dann zurückgezogen. |
| Wintertarn | Bestehender weißer Überzug (Palette `winter`), im Shader **verschmutztes Weiß** (Grauschleier, Dreck an Knien, Ellbogen, Saum). Gilt für alle Fraktionen, auch mit `camo`. | Wendeanzug (de), Schneehemd (su), behelfsmäßige Schneeanzüge (us) waren alle weiß; sauber weiß sah zu sehr nach Platzhalter aus. |
| Wüste | Keine Tarnung (helles Drillich wie bisher), nur die Stoffstruktur. | Tropenuniformen waren einfarbig. |
| Pistolen | Neue Waffenart `pistol`: **P 38, TT-33, M1911A1** mit echten Längen. Einhändig in `ready`, beidhändig in `aim`. | Katalog listet P 38, TT-33, M1911A1. |
| Fehlende Waffen | Neue Rolle **`de/stretcher_bearer`** (P 38 in der Hand, Armbinde, Sanitätstasche) für `de_inf_medic`; **`su/officer`** hält die TT-33; **`us/radio_operator`** trägt den M1-Karabiner (nur `us_inf_command` nutzt ihn; Katalog: Karabiner); **`us/officer`** bekommt das Holster der M1911A1. Abweichungen in den Trupp-Dateien entsprechend gestrichen. Bleibt: Bazooka-Schütze ohne eigenen Karabiner, MG-/Mörser-Gehilfen mit Pistole im Holster (Hände tragen Munition). | Jeder Mann trägt sichtbar die Katalogwaffe, soweit eine Hand frei ist. Panzerfaust ist keine Katalogwaffe, bleibt Ausrüstung. |
| Neue Clips | **`reload_kneel`, `reload_prone`, `throw_grenade_kneel`, `death_c`** (dreht weg, fällt auf die Seite), **`wounded`** (Schleife: liegt verwundet, krümmt sich), **`gun_load`** (Schleife: Granate vom Boden in den Verschluss auf ~1 m), **`gun_aim`** (Schleife: gebückt am Richtaufsatz, kurbelt), **`mg_feed`** (Schleife: Gehilfe liegt rechts neben dem MG und führt den Gurt), **`mount`** / **`dismount`** (auf ein 1,0 m hohes Deck steigen / abspringen, für 034). 25 → 35 Clips. | Laden, Werfen, Kriechen gab es schon im Stehen; fehlend waren Knien/Liegen, Verwundung, Geschütz- und MG-Bedienung, Aufsitzen. Gleiche prozedurale Pipeline. |
| Aufsitzen: Wurzelbewegung | `mount` endet mit den Hüften 1,0 m höher und 0,45 m weiter vorn, `dismount` umgekehrt; der Knoten bleibt stehen. Werte in `animations.json` (`rise`, `advance`). | 034 setzt den Soldaten nach dem Clip auf den Sitzplatz des Fahrzeugs; ohne Root-Motion bleibt die Sim-Position maßgeblich. |
| Leistung / VAT | Messen mit `just infantry-view-bench` vor und nach dem Shader. **VAT weiter nicht nötig**, solange die FPS nicht um mehr als 10 % fallen; Entscheidung in `docs/INFANTRY.md`. | Clips ändern nichts an der Laufzeit, der Shader ist der einzige neue Kostenfaktor. |
| Galerie-Bilder | `infantry_preview` bekommt `--camo=auto|none|camo`; Bilder je Fraktion × (Wald, Wald+Tarnung, Wüste, Winter) nach `build/renders/infantry/` (nicht im Repo). | Renders liegen nach Projektregel nicht im Repo. |

## 4. Umfang

1. Blender: Ruhepose-UVs in `mesh.py`; Pistolen in `weapons.py`; Rollen in
   `loadouts.py`; zehn Clips und Pistolen-Halte-Posen in `animation.py`; alle Soldaten
   und Trupps neu bauen.
2. Godot: `soldier.gdshader`, `SoldierLook` auf ShaderMaterial mit Tarnvarianten,
   Muster-Farben in `uniform_palettes.json`; Galerie- und Vorschau-Prüfungen anpassen;
   Vorschau mit `--camo`.
3. Doku: `docs/INFANTRY.md` (Texturen, Tarnung, Clips, Pistolen, Leistung/VAT),
   `docs/ASSETS.md` (Budget-Tabelle Soldat), `docs/SOURCES.md` (Tarnmuster-Quellen).

## 5. Akzeptanzkriterien

1. `just kit-test`, `just validate-assets`, `just gallery-check` grün.
2. `just selftest` grün; Infanterie-Benchmark nicht schlechter als 10 %.
3. Galerie-Bilder je Fraktion und Tarnung; Quellen in `docs/SOURCES.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.
