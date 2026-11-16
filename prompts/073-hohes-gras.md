# Prompt 073 – Hohes Gras nach Bildvorlage

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ARCHITECTURE.md` die Abschnitte „Terrain rendering“ (mit „Splatting shader“ und
> „Meadow (prompt 057)“), „Map objects and decoration“ und „Lighting, atmosphere and
> biomes“, dazu `docs/MAPS.md` (Geländearten, Karteneditor), `docs/PERFORMANCE.md` und
> `docs/SOURCES.md`. Dann den Code: `godot/scenes/world/meadow_tufts.gd`,
> `godot/assets/shaders/meadow_tuft.gdshader`, `meadow_palette.gdshaderinc`,
> `godot/assets/materials/terrain_meadow.gdshaderinc`, die Dekorationsschichten in
> `godot/scenes/world/map_objects.gd`, `scatter_deco` und `DecoLayer` in
> `crates/bridge/src/objects.rs`, `data/terrain.toml` und `crates/editor/src/brush.rs`.

**Vorlage:** `.temp/dschungel-grass.png` (nicht im Repo, `.temp/` ist ignoriert). Ein
Bildschirmfoto aus einem anderen Strategiespiel, 433 × 740 Pixel. Die untere Bildhälfte
zeigt das hohe Gras, um das es geht. Liegt die Datei nicht vor, reicht die Beschreibung
in Abschnitt 2.

## 1. Ziel

Karten können Flächen mit **hohem Gras** bekommen: ein dichter, üppiger, kniehoher bis
brusthoher Bewuchs wie in den Tropen, der aus der Spielkamera so wirkt wie in der
Vorlage. Er lässt sich im Karteneditor malen, sieht in jeder Kameraentfernung nach
hohem Gras aus, bewegt sich im Wind und wird von Fahrzeugen niedergedrückt.

Fertig ist der Prompt, wenn

- eine Fläche hohes Gras im Spiel neben dem Ausschnitt der Vorlage liegt und in
  Dichte, Aufbau, Helligkeitsverlauf und Artenmischung dazu passt,
- die Fläche bei 15, 35, 110 und 350 m Kameraabstand als hohes Gras zu erkennen ist
  und beim Zoomen nichts aufspringt,
- beim Schwenken nichts flimmert,
- die Bildrate über hohem Gras in jeder Grafikstufe über 60 FPS bleibt und die Kosten
  gegen dieselbe Ansicht mit normaler Wiese gemessen sind,
- `just check`, `just selftest` und `just editor-selftest` grün sind.

## 2. Die Vorlage

Die Vorlage ist ein **Vorbild für Aufbau und Wirkung**, kein Material: keine
Bildausschnitte und keine Texturen daraus, nichts aus anderen Spielen
(`docs/SOURCES.md`). Alles wird erzeugt oder selbst gebaut. Das Bild ist klein und
komprimiert; es taugt für Aufbau, Farben und Verhältnisse, nicht für Einzelheiten. Der
blaue Fleck in der Bildmitte ist eine Markierung und gehört nicht dazu.

Was das hohe Gras in der Vorlage ausmacht (untere Bildhälfte):

- **Geschlossener Bestand.** Zwischen den Pflanzen ist kein Boden zu sehen. Boden
  zeigt sich nur am Rand zur Schotterfläche.
- **Lange, schmale, überhängende Halme in Büscheln.** Die Halme stehen steil und
  biegen sich an der Spitze nach außen. Man sieht einzelne Halme, keine Flächen.
- **Starker Helligkeitsverlauf.** Die Spitzen sind hell gelbgrün und von der Sonne
  beschienen, zur Wurzel hin wird es dunkelgrün bis fast schwarz. Daraus entsteht die
  Tiefe: helle Halme vor dunklem Grund.
- **Mehrere Arten gemischt.** Grasbüschel überwiegen. Dazwischen stehen Kräuter mit
  breiteren Blättern, einzelne Sträucher mit ockergelbem bis braunem Laub oder
  Samenständen, die das Gras überragen, und Rosetten mit strahlenförmigen Wedeln
  (unten links). Die Sträucher stehen in lockeren Gruppen, nicht gleichmäßig.
- **Höhe.** Gemessen an den Panzern im Bild (etwa 2,7 m hoch) steht das Gras
  ungefähr 0,8 bis 1,2 m hoch, die Sträucher bis etwa 1,5 m. Das ist eine Schätzung
  aus einem kleinen Bild; in der Session nachmessen und die Unsicherheit notieren.
- **Rand.** Zur Schotterfläche endet der Bestand unregelmäßig, mit kürzerem,
  ausgefranstem Gras, nicht an einer geraden Linie.

Die obere Bildhälfte zeigt kurze Wiese mit kahlgefahrenen Spuren und jungen
Nadelbäumen. Sie ist nur der Gegensatz, vor dem das hohe Gras wirkt.

## 3. Festlegungen

Diese Fragen sind entschieden, damit die Session nicht daran hängen bleibt. Spricht
beim Lesen des Codes etwas Gewichtiges dagegen, halte an und frag.

| Frage | Festlegung | Grund |
|---|---|---|
| Woher weiß die Karte, wo hohes Gras steht? | Neue Geländeart `tall_grass` am Ende von `data/terrain.toml` (nächste freie id), mit denselben Bewegungswerten wie `grass` | Geländearten lassen sich schon malen, speichern und prüfen; keine neue Kartendatei, kein neues Format. |
| Spielregeln | Keine Wirkung in diesem Prompt: gleiche Geschwindigkeit wie `grass`, keine Tarnung, keine Deckung | Erst muss es gut aussehen. Tarnung im hohen Gras ist ein eigener Auftrag (Abschnitt 9). |
| Biome | Wald: üppig grün wie die Vorlage. Wüste: trockene, strohfarbene Fassung, lichter. Winter: braune, geknickte Halme, stark ausgedünnt, mit Schnee. Kein neues Biom „Tropen“ | Ein viertes Biom zieht Licht, Bäume, Häuser und Boden nach sich; das ist ein eigener Auftrag. |
| Bauweise der Pflanzen | Wie die Büschel aus Prompt 057: einzelne Halme als deckende Dreiecke, keine Karten mit Alpha-Test | Karten zeichnen aus der hohen Kamera Kästen und flimmern an den Rändern; das war der Befund von 057. |
| Texturen | Erzeugt, wie die Wiesenpalette. Bilddateien nur CC0, mit Eintrag in `docs/SOURCES.md` und `CREDITS.md`, und vorher fragen | Wie bisher. |

**Zur Simulation:** Die neue Geländeart ist eine beabsichtigte Regeländerung, auch
wenn sie nichts am Verhalten ändert. Der goldene Hash in
`crates/sim/tests/determinism.rs` sollte gleich bleiben, solange die Karte des Tests
kein hohes Gras hat. Ändert er sich, finde zuerst heraus, warum (zum Beispiel weil die
Regeltabelle in den Hash eingeht). Nur dann anpassen, und im Commit sagen, warum.
Kein Float in `crates/sim`. Karten, auf denen Tests oder Messreihen beruhen
(Determinismus, KI-Turnier, Balance, Benchmarks), bekommen kein hohes Gras.

**Bekannte Enge im Shader:** Der Terrain-Shader hat genau acht Gewichte (zwei
RGBA-Raster), und die acht vorhandenen Geländearten belegen sie. Deshalb wurde
aufgewühlte Erde früher nicht als eigene Art eingeführt (Tabelle bei „Terrain
deformation“). `tall_grass` bekommt kein neuntes Gewicht: beim Mischen der
Bodentexturen zählt es als `grass` (Umsetzung in `crates/bridge/src/terrain_layers.rs`),
und für den dunklen Grund und die Fassung der Ferne gibt es eine eigene Maske mit
einem Kanal (eigenes kleines Raster oder ein freier Kanal eines vorhandenen). Die
Büschel brauchen die Maske nicht: `scatter_deco` liest die Geländeart je Gitterpunkt
aus der Karte.

Alle Stellen, die Geländearten aufzählen, müssen die neue Art kennen: Kartenprüfung
(`just map-check`), Editor-Palette und Pinsel, weicher Boden bei Regen und Schnee
(Tabelle je Geländeart, „Weather, mud and time of day“), Staubfarben der Effekte
(`GROUND_COLORS` in `effects.gd`), Brand (`data/fire.toml`), Dekorationsdichten,
Minikarte, `docs/MAPS.md`. Such nach den Stellen, die `grass` oder die id 0 besonders
behandeln oder genau acht Arten annehmen, und entscheide für jede, ob hohes Gras sich
wie Gras verhält. Im Zweifel: wie Gras.

Zeigt sich dabei, dass die Geländeart einen größeren Umbau verlangt (viele Stellen,
die acht Arten fest annehmen, oder ein Eingriff ins Kartenformat), halte an, bevor du
umbaust, und schlag eine der beiden Alternativen vor: Flächen als Kartenobjekt
(`data/objects.toml`, wie `env_bush`, hält niemanden auf) oder eine eigene
Bewuchs-Schicht in der Karte.

## 4. Aufbau der Darstellung

Was genau zu bauen ist, entscheidet der Vergleich mit der Vorlage. Die folgenden
Punkte sind der erwartete Aufbau, keine Vorschrift.

- **Pflanzen.** Neue Varianten in `meadow_tufts.gd` und eine eigene
  Dekorationsschicht für `tall_grass`: hohes Büschel (viele überhängende Halme aus
  mehreren Abschnitten), breitblättriges Kraut, Strauch mit ockerfarbenem Laub oder
  Samenständen, Rosette mit Wedeln. Die Büschel überlappen, damit der Bestand
  geschlossen ist. Die Mischung kommt aus Gewichten und einem Fleckenrauschen, damit
  Sträucher in Gruppen stehen.
- **Farbe und Tiefe.** Helle Spitzen, dunkle Wurzeln, stärker als bei der Wiese.
  Abdunkelung zum Boden hin und im Inneren des Bestands. Farbwechsel in Flecken
  (frisch, gelblich, dunkel) aus der Palette, damit die Fläche nicht einfarbig ist.
- **Boden darunter.** Der Terrain-Shader zeichnet unter hohem Gras einen dunklen,
  grünen Grund, damit durch Lücken keine helle Erde scheint.
- **Ferne.** Jenseits der Sichtweite der Büschel (heute 55 m für die Wiese) zeichnet
  der Terrain-Shader das hohe Gras selbst: dunkler, rauer und satter als die Wiese,
  mit hellen Halmspitzen als Muster und einem Schattensaum am Rand der Fläche, damit
  sie auch bei 110 und 350 m höher wirkt als die Wiese daneben. Der Übergang zwischen
  Büscheln und Shader-Fassung darf nicht aufspringen (überblenden, Büschel am Rand
  der Sichtweite kleiner oder lichter).
- **Wind.** Hohes Gras lebt von Wellen, die über die Fläche laufen. Zusätzlich zum
  Zittern der einzelnen Halme laufen langsame Böen als helle und dunkle Bänder
  darüber, in Windrichtung des Bioms. Die Shader-Fassung der Ferne macht die Bänder
  mit.
- **Rand.** Die Geländearten liegen auf einem 2-m-Raster. Der Rand des Bestands
  folgt nicht den Rasterkanten: ausgefranst durch Rauschen, mit kürzerem Gras als
  Saum zu Weg, Straße und Wiese.
- **Fahrzeuge und Treffer.** Wo Fahrzeuge gefahren sind, liegt das Gras flach und
  bleibt flach. Ein Raster für Fahrspuren gibt es noch nicht (Reifenspuren im Schlamm
  sind Decals; bei „Meadow“ steht eine Spur-Textur als offen). Baue dafür ein
  Spur-Raster nur in der Darstellung, in das Fahrzeuge ihre Spur stempeln und das
  Büschel und Shader-Fassung lesen. Es wird nie zurückgelesen und geht nicht in die
  Simulation. Wird das zu groß, bleibt es ein offener Punkt, und das Gras weicht nur
  unter dem Fahrzeug aus. Über Kratern und Brandflecken steht nichts. Brennt Gras
  nach `data/fire.toml`, brennt hohes Gras ebenso und hinterlässt schwarze Stoppeln.
- **Lesbarkeit.** Soldaten (1,80 m) und Fahrzeuge bleiben aus der Spielkamera
  erkennbar und anwählbar. Liegende Soldaten dürfen im Gras fast verschwinden, ihre
  Auswahlmarken und Anzeigen nicht. Wenn nötig, weicht das Gras im Umkreis eines
  halben Meters um Einheiten zur Seite.
- **Schatten.** Nur die nahen, hohen Pflanzen werfen Schatten; die Schattenkosten
  aus 057 und „Map objects“ gelten auch hier.

## 5. Flimmern

Schmale Halme, die dünner als ein Pixel werden, flimmern bei jeder Kamerabewegung.
Prompt 068 und 071 haben das für die Wiese untersucht; lies deren Ergebnis in
`docs/ARCHITECTURE.md` und nutze `just ground-motion` und `tools/ground_flicker.py`.

- Halme werden mit der Entfernung breiter oder weniger, sodass keiner dünner als
  etwa ein Pixel gezeichnet wird, und gehen dann in die Shader-Fassung über.
- Jede Textur-Koordinate der Shader-Fassung hängt nur von der Weltposition ab, nie
  vom Abstand zur Kamera (die Regel aus 071).
- Eine Kamerafahrt über hohes Gras mit dem Werkzeug aus 071 messen und mit der
  Wiese vergleichen.

## 6. Karten und Editor

- Der Geländepinsel des Editors malt `tall_grass`, die Palette zeigt es mit eigener
  Farbe, `just map-check` und `just editor-selftest` kennen es.
- Eine Testfläche für Bilder und Messungen: mehrere hundert Meter hohes Gras mit
  Rändern zu Wiese, Feldweg, Straße, Wasser und Wald, auf einer Karte, an der keine
  Tests oder Messreihen hängen, oder auf einer neuen kleinen Testkarte.
- Auf ein oder zwei Spielkarten hohes Gras dort malen, wo es hingehört
  (Flussufer, Feldränder, Brachen, Bahndämme), sparsam. Vorher und nachher als Bild
  zeigen. Welche Karten sich eignen, entscheidet die Session nach der Regel in
  Abschnitt 3.

## 7. Prüfen

1. **Vergleichsbild.** Ein Ausschnitt aus dem Spiel bei ähnlichem Blickwinkel und
   Maßstab neben dem Ausschnitt der Vorlage (untere Bildhälfte). Nach jeder
   Änderungsrunde neu.
2. **Bildreihe.** Eine neue Auswahl für `just terrain-shots` (wie `SHOT_SET=meadow`),
   die die Testfläche bei 15, 35, 110 und 350 m zeigt, dazu den Rand zur Wiese und
   eine Fahrspur, in allen drei Biomen und bei Tag, Morgen und Nacht.
3. **Bewegung.** Eine kurze Aufnahme mit Kamerafahrt und Wind; das Flimmermaß aus
   Abschnitt 5.
4. **Bildrate.** `just terrain-bench` über der Testfläche in allen Grafikstufen,
   einmal mit hohem Gras, einmal dieselbe Ansicht als Wiese (A/B in einem Lauf wie
   `MEADOW_AB`). Über 60 FPS in jeder Stufe; die Dichte und Sichtweite je Stufe
   stehen in `graphics_quality.gd`. Dreiecke und Instanzen notieren.
5. `just check`, `just selftest`, `just editor-selftest`, `just map-check` für jede
   geänderte Karte.

Die Bilder dem Nutzer zeigen. Was nicht erreicht wurde, steht im Abschlussbericht mit
Grund, nicht als erledigt.

## 8. Abschluss

- `docs/ARCHITECTURE.md`: Unterabschnitt „Tall grass (prompt 073)“ nach „Meadow“
  (Aufbau, Tabelle der Varianten, Messwerte, offene Punkte), „Decoration“ anpassen.
  `docs/MAPS.md`: Tabelle der Geländearten. `docs/PERFORMANCE.md`, falls sich
  Messwerte dort ändern. Auf Englisch, ohne das Bild der Vorlage.
- Entsteht ein neues Modell mit Katalog- oder Objekteintrag, gelten die Regeln aus
  `CLAUDE.md` für Vorschaubilder und `just units-web-sync`. Für erzeugte Büschel
  braucht es das nicht.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten (Geländeart, Pflanzen, Ferne, Karten).
  Kein `git push`.

## 9. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Tarnung im hohen Gras.** Infanterie, die im hohen Gras steht oder liegt und nicht
  schießt, könnte wie im Wald verborgen sein (Hinterhalt-Regel, `data/vision.toml`
  `[ambush]`). Das ist eine Spielregel mit Folgen für Balance und KI.
- **Bewegung.** Langsamere Infanterie oder Radfahrzeuge im hohen Gras.
- **Biom „Tropen“** mit eigenem Licht, Palmen, Bambus und Häusern.
- **Schilf und Getreidefelder**, die dieselbe Technik nutzen könnten.
