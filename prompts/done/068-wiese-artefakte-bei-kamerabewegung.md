# Prompt 068 – Wiese: Artefakte bei Kamerabewegung analysieren

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ARCHITECTURE.md` die Abschnitte „Terrain rendering“, „Meadow (prompt 057)“,
> „Map objects and decoration“ und „Lighting, atmosphere and biomes“, dazu
> `docs/PERFORMANCE.md` und `prompts/done/057-bodenoptik-wiese.md`.

## 1. Ziel

Beim Bewegen des Bildausschnitts (Schwenken, Zoomen, Drehen) zeigt der Rasen störende
Artefakte: Flimmern, wandernde Muster, vermutlich Moiré. Im Standbild fällt es nicht
auf. Nach diesem Prompt steht fest, **welcher Teil der Wiese** das Artefakt erzeugt und
**warum**, mit Bildern als Beleg, und es liegt ein begründeter Vorschlag zur Behebung
vor. Kleine, eindeutige Ursachen werden gleich behoben.

Erst messen, dann ändern: keine Korrektur ohne Beleg, dass sie die Ursache trifft.

## 2. Ausgangslage

Die Wiese besteht seit Prompt 057 aus drei Teilen mit gemeinsamer Palette
(`godot/assets/shaders/meadow_palette.gdshaderinc`):

- **Boden** (`godot/assets/materials/terrain_meadow.gdshaderinc`, aufgerufen in
  `terrain.gdshader`): prozedurale Muster in mehreren Größen (Flecken 3,7/6,1 m,
  Sprenkel 0,3 m, Büschel 45 cm, Halmstriche in 3,6-cm-Zellen, Blumen), jeweils
  abhängig von der Pixelgröße ein- und ausgeblendet und über den Pixel-Fußabdruck
  geglättet.
- **Büschel** (`godot/scenes/world/meadow_tufts.gd`,
  `godot/assets/shaders/meadow_tuft.gdshader`): Instanzen auf einem 0,55-m-Raster bis
  55 m um die Kamera, 14–18 Halme je Büschel als deckende Dreiecke ohne Alpha-Test,
  Wind in der Vertex-Stufe. Wüste und Winter nutzen weiter die Büschel aus Prompt 013
  (`map_objects.gd`, Raster 0,7 m, 100 m Sichtweite).
- **Licht** (`godot/scenes/world/atmosphere.gd`, `forest.tres`): Sonnenschatten mit
  Weichzeichnung, Tilt-Shift-Schärfentiefe bei nahem Zoom.

Unbekannt ist, bei welcher Zoomstufe, welcher Bewegung und in welchem Biom das
Artefakt am stärksten ist. Der Nutzer hat es beim Bewegen des Ausschnitts auf dem
Rasen gesehen.

## 3. Verdächtige (jede einzeln prüfen, nichts vorab ausschließen)

| Nr. | Verdacht | Woran man ihn erkennt |
|---|---|---|
| 1 | Unterabtastung feiner Bodenmuster (Halmstriche, Sprenkel): die Glättung über den Pixel-Fußabdruck reicht bei flachem Blickwinkel oder in Bewegung nicht | Flimmern auch ohne Büschel; stärker bei bestimmter Zoomstufe; verschwindet, wenn die feinste Musterstufe aus ist |
| 2 | Umschalten der Musterstufen nach Pixelgröße (2,5 cm, 1,5–8 cm, 6 cm) | Sichtbare Bänder oder „Atmen“ beim Zoomen, im Schwenk ruhig |
| 3 | Dünne deckende Halm-Dreiecke schmaler als ein Pixel, ohne Kantenglättung | Funkeln an den Büscheln, nur innerhalb von 55 m; hängt an MSAA/TAA/FXAA |
| 4 | Regelmäßiges Raster der Büschel (0,55 m) gegen das Pixelraster | Echtes Moiré: großflächige wandernde Streifen in mittlerer Entfernung |
| 5 | Büschel-Raster oder Ein-/Ausblendgrenze wandert mit der Kamera statt mit der Welt | Büschel „schwimmen“, springen oder tauchen am Rand auf, nur im Schwenk |
| 6 | Kachelung oder Mipmaps der Grastextur (fehlende Mipmaps, kein anisotroper Filter, zwei gedrehte Abtastungen) | Moiré auch bei ausgeschalteter Prozedur-Schicht, stärker bei flachem Winkel |
| 7 | Schatten: Texel der Sonnenschatten kriechen beim Schwenk über die Halme | Flimmern nur im Schatten oder nur mit Schatten an |
| 8 | Wind- oder Zeit-Animation, deren Phase von der Kameraposition abhängt | Muster läuft mit der Bewegung mit, steht bei ruhiger Kamera |
| 9 | Nachbearbeitung: Schärfentiefe, Auflösungs-Skalierung, zeitliche Glättung (Nachziehen) | Verschwindet, wenn der Effekt aus ist; abhängig von den Grafik-Einstellungen |
| 10 | Rundungsfehler bei großen Weltkoordinaten in den Rauschfunktionen | Stärker weit weg vom Kartenursprung, auf `test_small` schwach, auf 4-km-Karten stark |

## 4. Umfang

1. **Nachstellen.** Das Artefakt gezielt erzeugen und festhalten: Biom, Karte,
   Zoomstufe, Blickwinkel, Art der Bewegung (Schwenk, Zoom, Drehung), Tempo,
   Fenstergröße, Grafik-Einstellungen. Zuerst die Einstellungen prüfen, mit denen der
   Nutzer spielt (`godot/scenes/menu/settings.gd`, gespeicherte Einstellungen, Kanten-
   glättung in `project.godot`).
2. **Bewegte Aufnahme als Werkzeug.** Die vorhandenen Aufnahmen
   (`SHOT_SET=meadow just terrain-shots <dir>`) zeigen Standbilder; sie reichen hier
   nicht. Ein Werkzeug ergänzen (Vorschlag `just meadow-motion <dir>`), das feste
   Kamerafahrten bei festem Zeitschritt als Bildfolge aufnimmt: langsamer und schneller
   Schwenk, Zoomfahrt, Drehung, je in naher, mittlerer und weiter Zoomstufe. Ausgabe
   nach `build/` (nicht ins Repo). Dazu ein Differenzbild aufeinanderfolgender Bilder
   nach Ausgleich der Kamerabewegung oder ein einfaches Maß (mittlere Helligkeits-
   änderung je Pixel auf einer Wiesenfläche), damit „flimmert weniger“ eine Zahl ist.
3. **Eingrenzen durch Abschalten.** Schalter nur für die Analyse (Debug-Parameter am
   Shader oder Kommandozeile, nicht in den Spieleinstellungen): Büschel aus,
   Bodenmuster stufenweise aus, Grastextur einfarbig, Schatten aus, Wind aus,
   Schärfentiefe aus, jede Kantenglättung einzeln, Raster der Büschel verwürfelt.
   Für jeden Schalter die Bildfolge und das Maß aus Schritt 2. Ergebnis als Tabelle:
   Verdacht, Schalter, Maß vorher/nachher, Urteil.
4. **Ursache benennen.** Für jeden bestätigten Verdacht: was genau passiert (welche
   Frequenz trifft welches Raster, welche Schwelle schaltet), in zwei bis drei Sätzen,
   mit Bildpaar.
5. **Beheben, wo es klein und eindeutig ist.** Zum Beispiel: feinste Musterstufe früher
   ausblenden, Glättungsbreite an den Blickwinkel anpassen, Büschel im Raster
   verwürfeln, Raster an Weltkoordinaten statt an die Kamera binden, Mipmaps oder
   anisotropen Filter einschalten. Nach jeder Änderung die Messung wiederholen.
   Größere Eingriffe (andere Kantenglättung als Standard, Umbau der Büschel, anderes
   Verfahren für den Boden) nur vorschlagen: mit Aufwand, Wirkung auf Optik und
   Bildrate, und mir zur Entscheidung vorlegen.
6. **Nichts verschlimmern.** Die Standbilder aus Prompt 057 mit
   `SHOT_SET=meadow just terrain-shots` vorher und nachher vergleichen; die Wiese darf
   in Ruhe nicht flacher oder matschiger werden. `just bench-view` und
   `just terrain-bench` halten das Budget aus `docs/PERFORMANCE.md`.
7. **Doku:** Abschnitt „Meadow“ in `docs/ARCHITECTURE.md` (Ursache, was geändert wurde,
   wie man die bewegte Aufnahme benutzt), `docs/PERFORMANCE.md` bei geänderten
   Messwerten, `CLAUDE.md` nur, wenn der neue Befehl dort hingehört.

## 5. Nicht in diesem Prompt

- Kein neues Aussehen der Wiese, keine neuen Pflanzen, keine Jahreszeiten.
- Keine Änderung an `crates/sim`: das ist reine Darstellung. Der Golden-Hash bleibt.
- Andere Flächen (Wasser, Felder, Wald) nur, wenn dieselbe Ursache sie trifft; dann
  melden und in der Liste führen, nicht nebenbei umbauen.

## 6. Akzeptanzkriterien

1. Das Artefakt ist mit einem Befehl reproduzierbar; Bildfolge und Maß liegen vor.
2. Die Tabelle aus Schritt 3 deckt alle zehn Verdächtigen ab, jeder mit Urteil
   (bestätigt, ausgeschlossen, nicht prüfbar mit Grund).
3. Für jede vorgenommene Korrektur zeigt die Messung eine Verbesserung; die Standbilder
   sind nicht schlechter.
4. `just check`, `just selftest` grün; `just bench-view` im Budget.
5. Abschlussbericht mit Bildpaaren vorher/nachher (Pfade unter `build/`), der Tabelle,
   der Liste der vorgeschlagenen größeren Eingriffe und dem, was offen ist.

## 7. Offene Fragen

| Frage | Vorschlag |
|---|---|
| Bei welcher Zoomstufe und in welchem Biom tritt es auf? | Zuerst Wald-Biom auf `twin_meadow` in allen drei Zoomstufen prüfen; finde ich es dort nicht, frage ich nach einem Bildschirmfoto oder einer kurzen Aufnahme. |
| Kantenglättung als Standard ändern (MSAA, TAA, FXAA)? | Nur vorschlagen, mit Messung der Bildrate; die Entscheidung liegt bei dir. |
| Büschel verwürfeln, auch wenn das die Optik leicht ändert? | Ja, wenn die Messung es stützt und die Standbilder gleichwertig bleiben. |

### Entscheidungen (autonom, Nachtlauf 17.11.2026)

| Frage | Entscheidung (vorläufig) |
|---|---|
| Zoomstufe und Biom | Wald-Biom auf `test_large` (offene Wiese bei (-30…-20, 70…85)), Kameraabstand 15, 35 und 55 m mit der Neigung des Spiels; die Hauptursache (Gleiten der zweiten Texturabtastung, 40–250 m) ist in Prompt 071 behoben. Keine Rückfrage nach Bildschirmfoto. |
| Kantenglättung als Standard ändern? | Nein, nur gemessen und vorgeschlagen (siehe Abschlussbericht/ARCHITECTURE.md). |
| Büschel verwürfeln? | Nicht nötig: das Raster ist schon um eine ganze Zelle verwürfelt und an Weltkoordinaten gebunden (`objects.rs::scatter`). |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`terrain.gdshader`, `main.gd`, `justfile`, Doku) nur klein und
additiv ändern. Am Ende nach `prompts/done/` verschieben und `prompts/README.md`
aktualisieren. Abschlussbericht: Ursache, was geändert wurde, wie ich es teste,
Messwerte und Bilder, was offen ist.
