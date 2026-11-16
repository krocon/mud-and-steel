# Prompt 074 – Felsen nach Bildvorlage

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ASSETS.md` den Abschnitt „Environment objects (prompt 007)“ ganz (Aufbau,
> Bauplan-Format, „Materials and shaders“, Budgets, Prüfungen) und „Thumbnails“, in
> `docs/ARCHITECTURE.md` „Map objects and decoration“ und „Terrain rendering“
> („Splatting shader“: Fels und Geröll an Steilhängen), in `docs/MAPS.md` die
> Abschnitte zu Objekten, Gebirge (Prompt 060) und zum Karteneditor, dazu
> `docs/SOURCES.md`. Dann den Bestand: `data/catalog/neutral.toml` und
> `data/objects.toml` (`env_rock`), `assets-src/blueprints/neutral/env_rock.toml`,
> `tools/blender/kit/env.py`, `godot/assets/shaders/env.gdshader` mit
> `env_common.gdshaderinc`, die Fels-Schicht in
> `godot/assets/materials/terrain.gdshader` und `data/vision.toml` (`[objects]`).

**Vorlagen:** `.temp/felsen.png`, `.temp/felsen2.png`, `.temp/felsen3.png` (nicht im
Repo, `.temp/` ist ignoriert). Drei Bildschirmfotos aus einem anderen Strategiespiel,
470 bis 980 Pixel breit. Liegen die Dateien nicht vor, reicht die Beschreibung in
Abschnitt 2.

## 1. Ziel

Das Spiel bekommt Felsen, wie die Vorlagen sie zeigen: flache Felsplatten, die aus dem
Boden treten, Gruppen mannshoher bis haushoher Blöcke und ganze Felsrücken, zwischen
deren Blöcken Bäume stehen. Heute gibt es nur `env_rock` mit drei kleinen Steinen
(höchstens 300 Dreiecke, bis 1,5 m Radius) und die Fels-Schicht des Terrain-Shaders an
Steilhängen.

Fertig ist der Prompt, wenn

- die drei Felsarten aus Abschnitt 3 als Umwelt-Objekte gebaut, geprüft und im Spiel
  sind, in allen drei Biomen,
- je Vorlage ein Bild aus dem Spiel bei ähnlichem Blickwinkel daneben liegt und in
  Form, Größe, Helligkeitsverlauf und Einbettung in den Boden dazu passt,
- sie sich im Karteneditor setzen lassen und auf mindestens einer Spielkarte stehen,
- Einheiten nicht durch Felsen laufen und nicht vor unsichtbaren Wänden stehen,
- die Bildrate in `just objects-bench` nicht merklich fällt und die Zahlen vorher und
  nachher notiert sind,
- `just check`, `just validate-assets`, `just selftest` und `just editor-selftest`
  grün sind.

## 2. Die Vorlagen

Die Bilder sind **Vorbild für Form und Wirkung**, kein Material: keine Bildausschnitte
und keine Texturen daraus, nichts aus anderen Spielen (`docs/SOURCES.md`). Alle Modelle
werden mit dem Baukasten erzeugt, alle Muster vom Shader. Die Bilder sind klein; sie
taugen für Aufbau, Verhältnisse und Farben, nicht für Einzelheiten. Oberflächen,
Symbole und Schrift in den Bildern gehören nicht dazu.

| Bild | Zeigt |
|---|---|
| `felsen.png` | Trockenes, gelbes Grasland mit **Felsplatten**: helle graue Platten, fast bündig mit dem Boden, rissig, in unregelmäßigen Feldern von einigen Metern bis über zehn Meter, dazwischen Erde und Gras. Rechts eine **Felsgruppe** aus großen Blöcken mit Zypressen dazwischen. Maßstab: das Haus in der Mitte (etwa 8 × 5 m), die Soldaten unten links. |
| `felsen2.png` | Links oben eine **Felsgruppe**: mehrere hohe Blöcke mit steilen, fast senkrechten Wänden, aneinandergelehnt, Büsche und junge Bäume am Fuß. Im Vordergrund flache Platten am Wegrand. Maßstab: Telegrafenmasten (etwa 7 m), Bäume, die Sandsackstellung. |
| `felsen3.png` | Draufsicht. Rechts ein **Felsrücken**: ein über hundert Meter langes Feld aus kantigen Blöcken mit dunklen Spalten, dazwischen Nadelbäume, unpassierbar. Links davon einzelne Platten im trockenen Boden entlang der Straße. Maßstab: Häuser, Lkw, Straße. |

Was die Felsen in allen drei Bildern ausmacht:

- **Hell oben, dunkel an der Seite.** Die Oberseiten sind hell, warm grau und von
  der Sonne beschienen, die Flanken kühler und dunkler, die Spalten und der Fuß fast
  schwarz. Dieser Verlauf macht die Form lesbar, auch von weit oben.
- **Kantig gebrochen, an den Kanten verwittert.** Blöcke mit ebenen Bruchflächen
  und klaren Kanten, keine runden Kiesel. Risse und Fugen teilen große Blöcke, an
  manchen Stellen liegen sie in Schichten.
- **Im Boden eingebettet.** Kein Fels liegt auf dem Boden wie abgestellt. Der Fuß
  steckt in Erde, Gras wächst an den Rand und in die Spalten, kleine Steine liegen
  ringsum.
- **Bewuchs.** Büsche, Grasbüschel und Bäume am Fuß und zwischen den Blöcken.
- **Harte Schatten.** Die Blöcke werfen klare Schlagschatten auf Boden und
  Nachbarblöcke.

Die Größen in Abschnitt 3 sind aus diesen kleinen Bildern geschätzt. In der Session an
den genannten Maßstäben nachmessen und die Unsicherheit notieren.

## 3. Die drei Felsarten

Neue Einträge im Katalog `neutral`, mit Bauplan, Modell, Regeln und Vorschaubild. Die
Namen sind Vorschläge; halte dich an die Benennung im Bestand.

| Art | Vorschlag id | Größe | Bewegung | Sicht | Varianten |
|---|---|---|---|---|---|
| **Felsplatte** | `env_rock_slab` | 2 bis 8 m breit, höchstens 0,3 m über dem Boden | hält niemanden auf | frei | 5 bis 6, auch längliche und zerbrochene |
| **Felsgruppe** | `env_rock_outcrop` | 3 bis 8 m breit, 2 bis 5 m hoch | hält alle auf (`blocks = "all"`) | verdeckt bis zur Kataloghöhe (`solid`) | 5 bis 6, vom einzelnen Block bis zur Gruppe aus vier Blöcken |
| **Felsrücken-Stück** | `env_rock_massif` | 12 bis 25 m lang, 4 bis 9 m hoch | hält alle auf | verdeckt | 4 bis 5, die sich aneinanderreihen und überlappen lassen |

Dazu `env_rock` (klein, mittel, groß) im selben Stil neu bauen, damit die losen
Steine am Fuß zu den großen Felsen passen. Seine Regeln bleiben, wie sie sind.

**Regeln, festgelegt:**

- **Grundriss.** Die Blockierform (Kreis oder Kasten in `data/objects.toml`) deckt
  den Fels, wie er gezeichnet ist: nirgends mehr als etwa einen halben Meter größer
  als das Modell und nirgends kleiner. Reicht eine Form je Variante nicht, das Modell
  dem Grundriss anpassen, nicht umgekehrt. Wegzellen sind 4 m groß; prüfe im Spiel,
  dass Einheiten dicht am Fels vorbeikommen und nicht hineinlaufen.
- **Felsplatten sind nur Bild.** Einheiten stehen auf Geländehöhe. Platten sind so
  flach, dass Ketten und Stiefel nicht sichtbar darin versinken.
- **Zerstörung.** Felsen gehören wie bisher in keine Zerstörungsklasse.
- **Deckung.** Prüfe im Abschnitt „Infantry cover“, wovon Deckung abhängt. Lässt sie
  sich an Objekte binden, geben Felsgruppen Deckung wie die Trockenmauer
  (`env_wall_stone`). Verlangt das neue Regeln, bleibt es ein offener Punkt.
- **Katalogwerte brauchen eine Quelle** (`docs/SOURCES.md`). Für Felsen sind das
  typische Maße natürlicher Aufschlüsse; halte dich daran, wie `env_rock` heute
  belegt ist.

**Zur Simulation:** Neue Objektarten sind eine beabsichtigte Regeländerung, wirken
aber nur dort, wo sie stehen. Der goldene Hash in `crates/sim/tests/determinism.rs`
bleibt gleich, solange die Karte des Tests keine neuen Felsen bekommt. Ändert er sich,
finde zuerst heraus, warum. Karten, an denen Tests oder Messreihen hängen
(Determinismus, KI-Turnier, Balance, Benchmarks), bekommen keine neuen Felsen.

## 4. Modelle bauen

Mit dem Umwelt-Baukasten (`just env`, `tools/blender/build_env.py`), als erzeugte
Formen aus einem Startwert je Variante, sodass ein Neubau dasselbe Modell ergibt.
Keine von Hand modellierten Einzelstücke, keine heruntergeladenen Modelle.

- **Form.** Ein neuer Baustein für Felsblöcke, zum Beispiel konvexe Körper, die von
  zufälligen Ebenen beschnitten werden, mit gebrochenen Kanten und leicht versetzten
  Flächen; mehrere Blöcke aneinandergelehnt und gestapelt ergeben Gruppen und
  Rücken. Platten sind flache, rissige Schollen mit Fugen. Was genau, entscheidet
  der Vergleich mit den Vorlagen.
- **Fuß.** Jeder Fels reicht unter den Boden (im Bauplan versenkt, wie die Bäume),
  damit er auch am Hang ohne Spalt sitzt. Die Prüfung kennt versenkte Objekte.
- **Schattierung im Modell.** Umgebungsverdunkelung in die Vertexfarbe (Kanal R)
  backen: dunkle Spalten, dunkler Fuß. Kantenabnutzung in G. So wie im Abschnitt
  „Materials and shaders“ beschrieben.
- **Budgets.** Die Zeile „rock ≤ 300“ reicht für große Felsen nicht. Vorschlag,
  in `docs/ASSETS.md` eintragen und in `validate_env.py` prüfen: Felsplatte ≤ 300,
  Felsgruppe ≤ 1 500, Felsrücken-Stück ≤ 3 000 Dreiecke, dazu für den Rücken ein
  Fern-Modell wie bei den Bäumen (≤ 300). Bleib darunter, wo es geht.
- **Größenprüfung.** `just env-lineup` stellt alle Varianten neben die 1,80-m-Figur
  und den Panzer IV. Dieses Bild mit den Vorlagen vergleichen, bevor es ins Spiel
  geht.

## 5. Oberfläche

Keine Texturdateien; das Muster kommt aus `env.gdshader`.

- Prüfe, wie `env_rock` heute gefärbt wird (Slot `Stone`, Art Feldstein). Für
  gewachsenen Fels braucht es voraussichtlich eine eigene Art oder einen eigenen
  Slot: Grundfarbe, Risse, Schichtlinien, Flechten und dunkle Wasserläufe, in
  Weltkoordinaten oder dreiseitig projiziert, damit Nachbarblöcke zusammenpassen
  und nichts verzerrt.
- **Oben hell, seitlich dunkel** nicht nur durch das Licht: die Oberseiten sind
  heller und wärmer, die Flanken dunkler, gesteuert über die Normale.
- **Passend zum Gelände.** Der Terrain-Shader zeichnet an Steilhängen eigenen Fels
  mit Schichtung und Geröll. Felsobjekte und Felshang haben je Biom dieselbe
  Grundfarbe, damit ein Felsrücken auf einem Steilhang wie ein Stück wirkt.
- **Biome** über `EnvLook`, wie bei allen Umwelt-Objekten: Wald grauer Fels mit
  Moos und Flechten auf den Oberseiten, Wüste ockerfarben und staubig, Winter mit
  Schnee auf allem, was nach oben zeigt.
- Jedes neue Material darauf prüfen, dass das Spiel ohne Leck-Meldung endet.

## 6. Einbettung in den Boden

Das ist der Unterschied zwischen einem abgestellten Stein und den Vorlagen.

- **Bewuchs am Fuß.** Die Dekoration kennt die Objekte schon (`DecoRasters` in
  `crates/bridge/src/objects.rs`: unter Kästen wächst nichts). Im Saum um Felsen
  mehr kleine Steine, Büsche und hohe Grasbüschel, unter dem Fels nichts.
- **Steiniger Boden.** Um Platten und Felsgruppen zeichnet der Terrain-Shader den
  Boden steiniger und kahler (ein Raster aus den Objekten, nur Darstellung). Wird
  das zu groß, sorgt das Modell selbst für einen flachen Saum aus Geröll, und der
  steinige Boden bleibt ein offener Punkt.
- **Bäume im Felsrücken.** Zwischen den Stücken eines Rückens stehen Nadelbäume
  oder Zypressen und Büsche. Das leistet die Platzierung (Abschnitt 7), nicht das
  Modell.

## 7. Karteneditor und Karten

- Der Objektpinsel bekommt Vorlagen: „Felsplatten“ (lockere Felder, verschiedene
  Varianten und Drehungen), „Felsgruppen“ (einzeln, mit Büschen) und „Felsrücken“
  (dichte, überlappende Reihe von Rücken-Stücken mit Bäumen dazwischen). Er hält
  Wege, Straßen und Wasser frei wie bisher.
- Der Kartengenerator für Gebirge (Prompt 060) setzt Felsrücken an Felswände und
  Felsgruppen an Hangfüße, wenn das mit wenig Aufwand geht. Sonst offener Punkt.
- Auf ein oder zwei Spielkarten Felsen setzen, wo sie hingehören (Gebirgspass,
  trockenes Hügelland), nach der Regel in Abschnitt 3. Vorher und nachher als Bild
  zeigen. `just map-check` für jede geänderte Karte; achte darauf, dass
  Felsrücken keine Kartenteile abschneiden, die erreichbar sein müssen.

## 8. Prüfen

1. **Vergleichsbilder.** Je Vorlage ein Bild aus dem Spiel bei ähnlichem Blickwinkel
   und Maßstab daneben. Nach jeder Änderungsrunde neu.
2. **Bildreihe.** Eine neue Auswahl für `just objects-shots`, die alle drei Arten
   bei 15, 35, 110 und 350 m zeigt, in allen drei Biomen, bei Tag und bei flacher
   Sonne, dazu einen Felsrücken auf einem Steilhang.
3. **Bewegung.** Im Spiel Infanterie und einen Panzer um eine Felsgruppe und an
   einem Felsrücken entlang schicken und über Felsplatten fahren lassen; als
   Bildreihe festhalten.
4. **Sicht.** Eine Einheit hinter einer Felsgruppe ist für den Gegner davor nicht
   zu sehen; ein Test in `crates/sim/tests/` neben den vorhandenen Sichttests.
5. **Bildrate.** `just objects-bench` vorher und nachher, dazu eine Ansicht über
   einem dichten Felsrücken. Dreiecke, Zeichenaufrufe und Sichtweiten der neuen
   Arten notieren.
6. `just validate-assets`, `just check`, `just selftest`, `just editor-selftest`.

Die Bilder dem Nutzer zeigen. Was nicht erreicht wurde, steht im Abschlussbericht mit
Grund, nicht als erledigt.

## 9. Abschluss

- **Vorschaubilder.** Jeder neue und jeder geänderte Katalogeintrag mit Modell
  braucht eines: `just thumbnails --only=<id>,<id>` (braucht ein Fenster), auch für
  das neu gebaute `env_rock`. Danach `just units-web-sync` und `just catalog-doc`.
- `docs/ASSETS.md` (Budgets, neuer Baustein, neue Shader-Art), `docs/MAPS.md`
  (Objekttabelle, Editor-Vorlagen), `docs/ARCHITECTURE.md` („Map objects and
  decoration“: Felsen, Einbettung, Messwerte, offene Punkte), `docs/SOURCES.md` für
  die Katalogwerte. Auf Englisch, ohne die Bilder der Vorlagen.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten (Baustein und Modelle, Regeln,
  Oberfläche, Einbettung, Editor und Karten). Kein `git push`.

## 10. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Klippen und Felswände als Gelände.** Senkrechte Wände aus dem Höhenraster
  bleiben Sache des Terrain-Shaders (Prompt 060).
- **Besteigbare Felsen** für Infanterie, Beobachter auf Felsen.
- **Abpraller und Splitter** an Felsen, Felsen als Hindernis für Geschosse über die
  Sichtregel hinaus.
- **Felsbrocken, die rollen oder gesprengt werden.**
