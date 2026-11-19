# Prompt 104 – Fahrspuren auf jedem Boden, Schlamm und Wasser statt nur Staub

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/PERFORMANCE.md`, `docs/ARCHITECTURE.md` (Abschnitt Effekte; unter „Open“
> stehen „tire tracks in the mud (decals), splashes“), `prompts/done/019-*.md`,
> `prompts/done/040-*.md` (Wetter, Nässe, Pfützen), `prompts/done/073-*.md`
> (Spuren im hohen Gras, Entscheidungstabelle), `godot/scenes/world/grass_trails.gd`
> ganz (`MAX_TEXELS`, `STAMP_STEP_M`, `_collect_vehicles`, `press`, `burn`),
> `godot/scenes/world/effects.gd` (`_vehicle`, `_update_vehicles`, `_ground_kind`,
> `_trail_strength`, `GROUND_COLORS`, `_decal`), `effect_library.gd`
> (`"dust_trail"`, `"exhaust"`, Wasser-`"spray"` bei Einschlägen),
> `godot/scenes/terrain/terrain.gd` (`set_trails`) mit `terrain.gdshader` und
> `weather_look.gd` (`_apply_wetness`).

## 1. Ausgangslage

Im Referenzvideo `docs/.vorlagen/vorlage.mp4` (nicht im Repo, nur Vorbild für die
**Wirkung**) ziehen Fahrzeuge deutlich sichtbare **Kettenspuren** durch Sand und
Wiese; die Spuren bleiben liegen und erzählen, wo eine Kolonne gefahren ist. Der
Tester lobt das und bemängelt im selben Satz, dass überall nur Staubwolken
aufsteigen: Er wünscht sich **wechselnde Bodeneffekte bei Schlamm und Wasser**.
Das Spiel heißt „Mud and Steel“; hier können wir sichtbar besser sein.

Bei uns heute:

- Spuren gibt es nur im **hohen Gras** (Geländeart 8): ein Raster über die ganze
  Karte, 1 m je Texel, Fahrzeuge drücken alle 0,4 m ein Rechteck hinein; es wird
  nie gelöscht. Auf Sand, Schlamm, Schnee, kurzem Gras und Wegen gibt es keine
  Spur.
- Jedes Fahrzeug hat **einen** Staubstrahler, nach Untergrund nur eingefärbt
  (Erde, Sand, Schnee, Schlamm, Straße, flaches Wasser). Kein eigener
  Schlammwurf, kein Wasserschwall in der Furt, kein Schneestaub.
- **Wetter wirkt nicht** auf die Fahreffekte: Bei Regen staubt es weiter.

## 2. Ziel

### 2.1 Spuren

- Fahrzeuge hinterlassen auf **allen weichen Böden** Spuren: zwei schmale Bänder
  (Kette oder Räder, Spurweite aus der Fahrzeugbreite), nicht ein breites
  Rechteck. Tiefe und Deutlichkeit nach Boden und Masse: Schlamm und Schnee tief
  und dunkel bzw. hell, Sand deutlich mit weichem Rand, kurze Wiese als dunkler
  Streifen, fester Weg kaum, Straße und Fels gar nicht.
- Kurven und Wenden auf der Stelle sollen als aufgewühlter Fleck erkennbar sein.
- **Lebensdauer:** Spuren bleiben lange, verblassen aber (Richtwert Minuten;
  Schnee und Schlamm länger als Sand). Bei Regen verschwinden Spuren im Sand
  schneller, im Schlamm werden sie nasser. Werte als Konstanten.
- Mehrfach befahrene Strecken werden zu einem ausgefahrenen Weg (dunkler, breiter).
- Infanterie: höchstens im Schnee eine feine Trittspur, wenn es fast nichts kostet;
  sonst weglassen.

### 2.2 Fahreffekte je Boden

- Trocken: Staub wie heute.
- Schlamm und nasser Boden (auch trockene Erde **bei Regen**): kurze, schwere,
  dunkle Brocken hinter Ketten und Rädern, kein schwebender Staub.
- Furt und Flachwasser: Wasserschwall an Bug und Seiten, Kielwelle im Wasser.
- Schnee: heller, feiner Schneestaub.
- Stärke hängt weiter am Tempo. Nach Prompt 097 fahren die Fahrzeuge langsamer;
  die Effekte müssen bei niedrigem Tempo noch erkennbar sein.

## 3. Umsetzung

Reine Darstellung; `crates/sim` und der Golden Hash bleiben. Spuren haben **keine**
Wirkung auf Regeln (kein Tempo, keine Sicht, keine Aufklärung).

- Erst entscheiden und begründen: das vorhandene Raster aus `grass_trails.gd`
  erweitern (weiterer Kanal bzw. feinere Auflösung nahe der Kamera) oder ein
  zweites System. Ein Texel von 1 m ist für zwei getrennte Bänder zu grob;
  prüfen, was eine feinere Auflösung an Speicher und Bildrate kostet und ob ein
  Fenster um die Kamera reicht. Einfachste Lösung, die aus normaler Spielhöhe gut
  aussieht.
- Das Raster entsteht aus der Bewegung der Einheiten im Bild und ist nicht Teil
  des Spielstands. Es darf zwischen Rechnern verschieden sein.
- **Nebel des Krieges:** Gegnerische Fahrzeuge außerhalb der Sicht dürfen keine
  Spur hinterlassen, die man später sieht und die ihren Weg verrät (Regel aus
  Prompt 098: nichts über den Gegner im gedämpften Bereich). Einfachste Regel:
  nur stempeln, was gerade sichtbar ist.
- Grafikstufen (`graphics_quality.gd`): Spuren und Zusatzeffekte je Stufe
  abschaltbar bzw. gröber.
- Speichern/Laden und Wiederholung: Spuren gehen verloren wie heute im Gras; im
  Ergebnis nennen.

## 4. Vorgehen

1. Bestand lesen; Vorher-Bilder: Panzer und Lkw fahren je eine Acht auf Sand,
   Wiese, Schlamm, Schnee, durch eine Furt; trocken und bei Regen.
2. Spuren bauen, mit dem Video vergleichen (Breite der Spur im Verhältnis zum
   Fahrzeug, Deutlichkeit aus Spielhöhe).
3. Fahreffekte je Boden und Wetter.
4. Nachher-Bilder derselben Fahrten.
5. Selftest: nach einer Fahrt über Sand ist das Raster an der Strecke gesetzt, auf
   der Straße nicht; ein unsichtbares gegnerisches Fahrzeug stempelt nicht; bei
   Regen wird auf Erde der Schlamm-Effekt gewählt.
6. `just check`, `just selftest`, `just bench-view` vorher/nachher mit einer
   Kolonne von 30 Fahrzeugen. Die Bildrate darf nicht merklich fallen.
7. Doku: `docs/ARCHITECTURE.md` (Punkt aus „Open“ streichen),
   `docs/PERFORMANCE.md`.
8. Für den Nutzer: Vorher/Nachher nebeneinander und die Konstanten zum
   Nachstellen.

## 5. Nicht Teil dieses Prompts

- Spuren mit Spielwirkung (Aufklärung über Fährten, Festfahren im Schlamm).
- Verformung des Geländes durch Fahrzeuge.
- Bäume umfahren (Prompt 106).
