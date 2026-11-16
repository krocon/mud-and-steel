# Prompt 057 – Bodenoptik: Wiesen und Rasen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` („Terrain rendering“, Vegetation, Licht/Biome), `docs/PERFORMANCE.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Aus der hohen Spielkamera soll eine Wiese wie eine echte Wiese aussehen: dichtes, feines
Gras mit sichtbarer Halmstruktur, Farbwechsel zwischen sattem Grün, trockenen gelblichen
Flecken und dunkleren feuchten Stellen, Mähstreifen und Trampelpfade, weicher Übergang
zu Feldwegen, Grasbüschel und Blumen als Akzente. Referenz ist ein Screenshot des
Nutzers (nicht im Repo): nur die Wirkung zählt, nichts wird übernommen.

## 2. Ausgangslage

- Eigener Terrain-Renderer (011): Splatting von 8 Geländearten, `terrain_albedo.png`,
  `terrain_normal.png`, `terrain_macro.png`; Spuren-Richtung für Feldwege.
- Grasbüschel und Blumen als Instanzen (013) bis 100 m, bis zu 70 000 nahe der Kamera.
- Budgets aus 016 (`docs/PERFORMANCE.md`).

## 3. Umfang

1. **Vergleich zuerst:** Screenshots der aktuellen Wiese aus 3–4 Zoomstufen (Movie-Writer
   oder Screenshot-Tool) in `build/` ablegen und Unterschiede zur Zielwirkung notieren.
2. **Mikrodetail im Shader:** prozedurale Halm-/Sprenkelstruktur in hoher Frequenz
   (stochastisches Tiling gegen Kachelmuster), Detail-Normalen, Ausblenden nach Distanz.
3. **Farbvariation in mehreren Skalen:** große Flecken (trocken/feucht), mittlere
   Wolken, kleine Sprenkel; nach Biom und Jahreszeit (015) parametrisiert;
   Mähstreifen/Wiesenstruktur optional als Kartenebene oder Rauschen.
4. **Gras-Schicht nah:** prüfen, ob Shell-Texturing oder dichtere, kleinere Halm-Instanzen
   (GPU-Partikel/MultiMesh mit Wind) die Wiese nah besser machen als die Büschel aus 013.
5. **Übergänge:** Wege mit ausgefransten Rändern, Fahrspuren mit plattgedrücktem Gras,
   Krater (021) mit Erdauswurf.
6. **Nahaufnahme** (zweiter Screenshot des Nutzers, nächste Zoomstufe): Dort trägt die
   Wirkung vor allem
   - **einzelne Halmbüschel** in unterschiedlicher Höhe und Neigung, dicht und ohne
     sichtbares Raster, mit hellen Spitzen und dunklem Grund dazwischen;
   - **verstreute kleine Blüten** (weiße Gänseblümchen, gelber Löwenzahn) als winzige
     flache Punkte über die ganze Fläche, unregelmäßig in lockeren Gruppen; nach Biom
     und Jahreszeit (keine Blüten im Winter und in der Wüste);
   - **dunkle Erdflecken** und kahle Stellen, Fußspuren/Schleifspuren, wo Soldaten
     gelaufen oder gekrochen sind (Darstellung, verblassen langsam);
   - **lange, weiche Schatten** von Gebäuden, Mauern und Bäumen, die das Gras deutlich
     abdunkeln, und Gras, das selbst Schatten wirft bzw. Kontaktschatten an Objekten;
   - **Tiefenunschärfe** an den Bildrändern bei naher Zoomstufe (Tilt-Shift-Wirkung),
     abschaltbar in den Einstellungen.
   Halme, Blüten und Spuren müssen sich mit Wind (013) und mit den Budgets aus 016
   vertragen; Blüten als Teil des Bodenshaders oder als billige Instanzen, nicht als
   eigene Nodes.
7. **Vorher/Nachher-Bilder** aus allen Zoomstufen in den Abschlussbericht.

## 4. Akzeptanzkriterien

1. `just selftest`, `just gallery-check` grün; Golden-Hash unverändert.
2. Terrain-Benchmark aus 016: höchstens 1 ms pro Frame mehr auf `test_large`.
3. Doku: Abschnitt „Terrain rendering“ ergänzt.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (2026-11-16): Die Grasschicht ist eine prozedurale Textur
(`gen_textures.rs`, Halme auf 4-m-Kachel) mit Anti-Tiling (zwei gedrehte Samples) und
einer groben Trocken/Satt-Variation über `macro_far.g`. Darauf stehen die Grasbüschel
aus 013 (Kreuz-Karten mit alpha-getesteten Halmkämmen, 0,7-m-Gitter, 100 m Sicht,
~70 000 Instanzen); von oben sehen sie wie kleine Kisten mit dunklen Rückseiten aus.
Blüten gibt es nur als Variante „Flowers“ dieser Büschel. Biome (015) sind Wald, Wüste
und Winter; Jahreszeiten gibt es nicht. Sonne im Wald 42°, harte Kanten, Schatten der
Kronen im billigen Schattenpass als Schachbrett. Keine Tiefenunschärfe. Vergleichsbilder
vorher: `SHOT_SET=meadow just terrain-shots <dir>` (neu, 15/35/110/350 m am Dorf und
zwei offene Ansichten).

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Bodenshader statt Texturen:** neue Datei `terrain_meadow.gdshaderinc` mit einer Funktion `meadow_apply`, ein einziger Aufruf in `terrain.gdshader` nach dem Schichten-Blend und vor nassem Boden und Schnee. Alles prozedural (Makro-Rauschen, Hashes), keine neuen Texturen. | 040 (Wetter) ändert denselben Shader: eine Zeile plus Include hält die Änderung additiv; Nässe und Pfützen liegen danach weiter oben drauf. Keine Downloads. |
| 2 | **Farbe in vier Skalen** aus einer gemeinsamen Palette (`meadow_palette.gdshaderinc`: satt, trocken/strohig, feucht/dunkel): Makro (Hunderte m), Wolken 29/47 m, Flecken 3,7/6,1 m (je zwei gedrehte Samples gegen Wiederholung in der Übersicht), Sprenkel 0,3 m. Die Grastextur liefert nur noch ihr Hell/Dunkel-Muster (Kontrast 1,5). | Die Wiese war einfarbig oliv; die Referenz lebt von Farbwechseln. Erste Version mit je einem Sample zeigte in 350 m ein Raster. |
| 3 | **Halme und Büschel im Boden:** nah (Pixel < 2,5 cm) Halmstriche in 3,6-cm-Zellen mit hellen Spitzen und Klumpen von 16 cm mit dunklen Lücken; mittel (Pixel 1,5–8 cm) Büschelmuster von 45 cm. Antialiasing über die Pixelgröße, ausgeblendet, wo es nicht mehr auflöst. | Gibt die Halmstruktur ohne Geometrie; zwei Halmschichten kosteten nah ~0,3 ms mehr ohne sichtbaren Gewinn, daher eine. |
| 4 | **Grasbüschel neu (Punkt 4 der Aufgabe):** dichtere Halm-Instanzen statt Shell-Texturing. `meadow_tufts.gd` baut drei Büschel-Meshes aus 14–18 einzelnen, opaken Halmen (kein Alpha-Test), Normalen nach oben, Farbe aus der Palette am Standort; sie ersetzen im Wald die 013-Büschel der Ebene „grass“ (0,55-m-Gitter, 55 m Sicht). Kurze Büschel: gerade Halme (1 Dreieck), hohe: gebogen (3) und mit Schatten. Wüste und Winter behalten die 013-Büschel. **[vorläufig]** | Shells zeichnen den Boden 8–16-mal und zeigen am Bildrand Schichten; die Streuung aus 013 (Geländeart, Wald, Objekte, Krater) wird wiederverwendet. Gemessen sind die Büschel in 120 m sogar 2 ms billiger als die alten Karten (kürzere Sicht, kein Alpha-Test). |
| 5 | **Blüten im Bodenshader** (Gänseblümchen weiß mit gelber Mitte, Löwenzahn gelb, 1,6–3,5 cm, lockere Gruppen, weniger auf trockenen Flecken, Erde und Wegen), `flower_density` je Biom (Wald 1, Wüste und Winter 0). Jahreszeiten: es gibt keine; das Biom ist die Jahreszeit. **[vorläufig]** | Keine Nodes, keine Instanzen; Biome sind die einzige Jahreszeit in 015. |
| 6 | **Erdflecken und Übergänge:** kahle Stellen von einigen Metern, ausgefranste Erde am Rand von Feldwegen, Halme entlang der Fahrspuren plattgedrückt. Mähstreifen nicht umgesetzt. | Weiden der Zeit wurden beweidet, nicht gemäht; Mähstreifen wirken wie ein Sportplatz. |
| 7 | **Fußspuren/Schleifspuren von Soldaten und Erdauswurf an Kratern: offen.** | Spuren brauchen eine Laufzeit-Spurtextur; 040 bringt Spuren in denselben Shader, beides sollte eine Lösung teilen. |
| 8 | **Licht:** Waldsonne 37° statt 42° (längere Schatten) **[vorläufig]**, Schattenunschärfe 2 statt 1, runde Blatt-Kleckse im billigen Schattenmuster der Kronen statt Quadrate. 34° sah besser aus, kostete in der Übersicht aber ~0,6 ms (mehr Schattenwerfer). | Lange weiche Schatten aus der Referenz; Budget aus 016. |
| 9 | **Tiefenunschärfe (Tilt-Shift)** über `CameraAttributesPractical` in `atmosphere.gd`: scharfes Band 0,86–1,1 Kameradistanzen, Übergang 0,22, Stärke 0,09, blendet zwischen 30 und 60 m Kameradistanz aus; Box-Bokeh, niedrigste Qualität. Einstellung „Depth of field at close zoom“ (Standard an **[vorläufig]**), auf Qualität „low“ immer aus. | Wirkung wie in der Referenz; ein reines Fern-Blur (015) hatte den oberen Rand in jeder Zoomstufe verschmiert. |
| 10 | **Messen:** `MEADOW_AB=1 just terrain-bench` misst jede Ansicht pro Variante mit je einem abgeschalteten Teil (`no_ground`, `no_tufts`, `no_dof`, `old_sun`, `old_blur`, `none` = vorher) im selben Lauf. | Die Maschine war durch parallele Sessions stark belastet (Vorher-Läufe schwankten um 1–2 ms); nur der Vergleich im selben Lauf ist belastbar. |

## Ergebnis

Umgesetzt wie oben; Details in `docs/ARCHITECTURE.md` „Terrain rendering → Meadow“ und
„Light and atmosphere“.

- **Boden:** `assets/materials/terrain_meadow.gdshaderinc`, Palette
  `assets/shaders/meadow_palette.gdshaderinc`, Biom-Felder `meadow_strength`,
  `flower_density` (Selbsttest prüft sie pro Biom).
- **Büschel:** `scenes/world/meadow_tufts.gd`, `assets/shaders/meadow_tuft.gdshader`,
  `assets/materials/env/meadow_tuft.tres` (Wind über `atmosphere.gd`).
- **Licht:** Waldsonne 37°, Schattenunschärfe 2, runde Blatt-Kleckse im Schattenpass,
  Tilt-Shift-Unschärfe mit Einstellung im Grafik-Tab.
- **Messung** (`MEADOW_AB=1 just terrain-bench`, `test_large`, 1920 × 1080, M1 Max,
  Maschine durch parallele Sessions belastet; „vorher“ = Variante `none` im selben Lauf):

  | Ansicht | vorher | nachher | Differenz |
  |---|---|---|---|
  | near 15 m | 8,49 ms | 9,30 ms | +0,81 ms (Boden 0,8, Unschärfe 0,25) |
  | middle 120 m | 10,76 ms | 9,22 ms | −1,54 ms (Büschel billiger als die 013-Karten) |
  | overview 600 m | 9,83 ms | 10,56 ms | +0,73 ms (Sonne 37° 0,37, Boden 0,40) |
  | terrain 600 m | 9,24 ms | 9,96 ms | +0,72 ms |
  | river 90 m | 20,60 ms | 20,81 ms | +0,21 ms |
  | no_water 90 m | 19,67 ms | 19,88 ms | +0,21 ms |

- **Offen:** Fuß- und Schleifspuren (mit den Spuren aus 040 zusammen lösen), Erdauswurf
  an Kratern, Mähstreifen (bewusst nicht), Jahreszeiten.
