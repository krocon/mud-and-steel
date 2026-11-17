# Prompt 072 – Explosionen und Rauch nach Videovorlage

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ARCHITECTURE.md` die Abschnitte „Effects (presentation only)“ (mit „Drawing“,
> „Budget and LOD“, „Fog of war“, „Decisions“ und „Big blasts (prompt 054)“) und
> „Indirect fire and smoke“, dazu `docs/SOURCES.md` und `docs/PERFORMANCE.md`. Dann den
> Code: `godot/scenes/world/effects.gd`, `effect_library.gd`, `effect_textures.gd`,
> `blast_effects.gd`, `graphics_quality.gd` (Schlüssel `fx_*`) und die Werkzeuge
> `godot/scenes/tools/effects_bench.gd` und `blast_shots.gd`.

**Vorlage:** `.temp/rts-explosionen.mp4` (nicht im Repo, `.temp/` ist ignoriert). Ein
Werbe-Kurzvideo des Spiels „Defilade“: Hochformat 2160 × 3840, 30 Bilder/s, AV1,
42,3 s. `.temp/rts-explosionen.m4a` ist seine Tonspur (gleiche Länge). `ffmpeg` und
`ffprobe` sind auf dem Rechner und lesen die Datei.

Liegt die Datei dort nicht, such nicht lange und rate nicht: halte an und frag nach dem
Pfad. Ohne das Video wird nichts umgesetzt, weil alle Zielwerte dieses Prompts aus ihm
gemessen werden.

## 1. Ziel

Die Explosionen im Spiel erreichen die Wirkung der Explosionen im Video. Vorbild sind
der Ablauf vom Blitz über den Feuerball bis zum Rauch, die Zeiten, die
Größenverhältnisse und die Art, wie der Rauch aufsteigt, sich ausbreitet, die Farbe
wechselt und vergeht. Der Rauch ist dabei der wichtigste Teil. Die Bilder selbst
entstehen mit eigenen, erzeugten Texturen und passen zur Optik dieses Spiels; es ist
kein Abbild der Grafik des anderen Spiels.

Fertig ist der Prompt, wenn

- für jede Explosionsart des Spiels (Abschnitt 4) eine Bildreihe aus dem Spiel neben
  der Bildreihe aus dem Video liegt, mit denselben Zeitpunkten, und beide im Ablauf,
  in den Größenverhältnissen und im Verhalten des Rauchs übereinstimmen,
- die gemessenen Werte aus dem Video und die erreichten Werte im Spiel in einer
  Tabelle nebeneinander stehen,
- die Bildrate mit 50 Explosionen zugleich in jeder Grafikstufe über 60 FPS bleibt
  (`just effects-bench`),
- `just check` und `just selftest` grün sind und der goldene Hash unverändert ist.

## 2. Regeln für die Vorlage

Das Video ist eine **Vorlage für Aussehen und Ablauf**, kein Material. Aus ihm werden
nur Messwerte übernommen: Zeiten, Größen, Farben, Formen, Bewegungen.

- Keine Bilder, Bildausschnitte, Texturen oder Töne aus dem Video ins Repo oder ins
  Spiel. Es stammt aus einem anderen Spiel, also gilt `docs/SOURCES.md` ohne Ausnahme:
  nichts aus anderen Spielen, auch kein Name und keine erkennbare Gestaltung.
- Einzelbilder und Bildreihen der Analyse liegen in `build/explosion-ref/` (ignoriert)
  und bleiben dort.
- Texturen werden weiter beim Start aus Rauschen erzeugt (`effect_textures.gd`).
  Reicht das für die Rauchform nicht, zuerst das Erzeugen verbessern (Abschnitt 6).
  Fertige Bilddateien nur, wenn sie CC0 sind, mit Eintrag in `docs/SOURCES.md` und
  `CREDITS.md`, und vorher fragen.

## 3. Schritt 1: Video analysieren

Nichts ändern, bevor diese Analyse steht.

1. **Überblick.** Eine Übersicht mit vier Bildern je Sekunde als Kachelbilder
   erzeugen und ansehen, zum Beispiel:

   ```bash
   ffmpeg -i .temp/rts-explosionen.mp4 -vf "fps=4,scale=240:-1,tile=8x4" build/explosion-ref/overview_%02d.png
   ```

   Daraus eine Liste aller Explosionen mit Startzeit, Art (Einschlag am Boden,
   Fahrzeug zerstört, große Detonation, Gebäude …) und geschätzter Größe. Die
   Lautstärkespitzen der Tonspur helfen, die Startzeiten auf das Bild genau zu finden.

   Ein erster Durchgang mit einem Bild je Sekunde (16.11.2026) ergab diese Szenen.
   Die Zeiten sind auf eine Sekunde genau und die Deutungen vorläufig; beides in der
   Session nachprüfen:

   | Zeit (s) | Szene | Zu sehen |
   |---|---|---|
   | 0–1 | Einblendung | nichts |
   | 1–5 | Wiese, Fahrzeug | weiße Rauchspuren quer durchs Bild, weiße ballige Nebelwolken, dahinter ein dunkler graubrauner Rauchballen; bei 4–5 an einem Feldweg ein dunkler Rauchballen und oranges Mündungsfeuer |
   | 6–9 | Explosion neben einem Haus | 6: Erdfontäne aus dunklen, strahlenförmigen Streifen; 7: gelbweißer Feuerausbruch; 8: orangebrauner, fleckiger Feuerball, der pilzförmig aufrollt; 9: schwarze Rauchsäule, daneben eine graue Staubwolke. **Die vollständigste Explosion des Videos** |
   | 10–11 | dieselbe Stelle danach | Trümmer, Rauch abgezogen (Schnitt dazwischen prüfen) |
   | 12–13 | Nahaufnahme | dichter graubrauner Rauch aus einzelnen Ballen, fliegende Trümmer |
   | 14 | Einschlag bei einem Panzer | heller, blassgelber Strahlenkranz im Augenblick des Einschlags |
   | 15–18 | Nahaufnahme | hellgraue bis weiße Wolke (Nebel) |
   | 19–24 | Wüstenstadt | graubraune Staub- und Rauchwolke nach einer Explosion an einer Mauer, steht und zieht über mehrere Sekunden ab; Rauchspuren |
   | 25–30 | Wüste, Ruinen | 27: großes gelbweißes Mündungsfeuer mit weißem Rauch; 29: Einschlag mit dunklem, strahlenförmigem Ausbruch |
   | 31–32 | hinter einer Mauer | schwarzbraune Rauchsäule mit glühenden Stellen (Brand) |
   | 33–35 | Infanterie auf Gras | Mündungsfeuer, braune Staubwölkchen von Einschlägen |
   | 36–42 | Abspann | nichts |

   Was das Messen erschwert: Titel oben und Untertitel in der Bildmitte liegen über
   dem Bild, die Kamera bewegt sich, es wird alle 3 bis 6 Sekunden geschnitten,
   Vordergrund und Hintergrund sind unscharf gezeichnet (Schärfentiefe), und
   Rauchspuren ziehen vor den Explosionen durch. Bei 30 Bildern/s ist eine Zeit auf
   33 ms genau. Wegen der Schnitte ist die Standzeit des Rauchs oft nur als
   Mindestwert zu messen; dann so notieren und die Standzeit aus der Szene 19–24 und
   aus dem Verlauf von Deckkraft und Größe ableiten.

   Die weißen Nebelwolken (1–5 und 15–18) sind Nebelmittel, keine Explosionen. Sie
   gehören nicht zu diesem Prompt; nur notieren, was an ihnen für die Nebelwände des
   Spiels brauchbar wäre.
2. **Auswahl.** Vier bis sechs Explosionen wählen, die gut zu sehen sind (frei im
   Bild, ruhige Kamera, nicht von anderen überdeckt) und die Arten aus Abschnitt 4
   abdecken. Fehlt im Video eine Art, das festhalten und sie aus der ähnlichsten
   ableiten.
3. **Bildreihen.** Je gewählter Explosion jedes Einzelbild der ersten Sekunde und
   danach vier Bilder je Sekunde bis zum Verschwinden des Rauchs, als Kachelbilder mit
   Zeitangabe (`drawtext` oder Dateiname), ausgeschnitten auf die Explosion (`crop`),
   dazu einige Bilder in voller Auflösung. Kachelbilder höchstens 2000 Pixel breit,
   damit sie sich ansehen lassen.
4. **Maßstab.** Aus bekannten Größen im Bild (Soldat 1,8 m, Fahrzeuge, Türen, Mauern)
   Meter je Pixel für jede gewählte Explosion schätzen. Das Video zeigt moderne
   Fahrzeuge, deren Maße nicht bekannt sind; Soldaten sind der verlässlichste Maßstab.
   Die Unsicherheit dazuschreiben.
5. **Messen, Phase für Phase.** Für jede gewählte Explosion eine Tabelle, Zeiten in
   Millisekunden ab dem ersten Bild der Explosion:

   | Phase | Zu messen |
   |---|---|
   | Blitz | Dauer, Durchmesser, Farbe, ob er die Umgebung aufhellt |
   | Feuerball | Zeit bis zur größten Ausdehnung, größter Durchmesser und Höhe, Form (Kugel, Halbkugel, Fontäne, mehrere Ballen), Farbverlauf vom Kern zum Rand und über die Zeit, wann die letzte Flamme erlischt |
   | Funken und Trümmer | Anzahl, Abwurfwinkel, Reichweite, Flugzeit, ob sie Rauchfahnen ziehen, ob sie aufschlagen und liegen bleiben |
   | Erde und Staub | Fontäne (Höhe, Dauer), Staubring am Boden (Reichweite, Geschwindigkeit, Dauer) |
   | Druckwelle | sichtbar oder nicht, Reichweite, Dauer |
   | Rauch | wann er aus dem Feuerball entsteht, Farbe und Deckkraft über die Zeit, Steiggeschwindigkeit, wie schnell er breiter wird, Form (Säule, Pilz, flache Wolke, einzelne Ballen), Größe und Anzahl der einzelnen Ballen, innere Bewegung (Wirbeln, Rollen), Schattierung (helle Sonnenseite, dunkle Unterseite, noch glühender Kern), Abdrift mit dem Wind, Zeit bis halb durchsichtig, Zeit bis ganz verschwunden |
   | Nachbrennen | Feuer und Rauchsäule am Wrack: Höhe, Neigung, Farbe, Dauer |
   | Boden | Brandfleck oder Krater: Durchmesser, ob er bleibt |
   | Kamera | Erschütterung: Stärke, Dauer |

   Farben nicht schätzen, sondern messen: Bildausschnitt auf einen Pixel mitteln
   (`crop`, `scale=1:1`) oder mit einem kleinen Python-Skript, und als sRGB-Wert
   notieren, an drei bis fünf Zeitpunkten je Phase.
6. **Ergebnis.** Je Art ein Ablaufprofil: die Tabelle aus 5. und drei Sätze, was diese
   Explosion ausmacht und worin sie sich von der heutigen im Spiel unterscheidet.
   Dieses Profil ist die Zielvorgabe für alles Weitere. Dem Nutzer die Auswahl und die
   Profile kurz zeigen (Kachelbilder mitschicken) und weiterarbeiten.

## 4. Explosionsarten im Spiel

Auslöser und heutige Effekte stehen in der Tabelle in `docs/ARCHITECTURE.md`,
„Effects“. Für diese Arten braucht es ein Profil:

| Art | Auslöser heute |
|---|---|
| Einschlag klein | `hit` am Boden, 37 bis unter 75 mm |
| Einschlag mittel | `hit` am Boden, 75 bis unter 110 mm |
| Große Detonation | `hit` ab 110 mm, Bomben, schwere Raketen, explodierende Lager (`blast_effects.gd`) |
| Fahrzeug zerstört | `destroyed`, Fahrzeug: Feuerball, dann brennendes und schwelendes Wrack |
| Munitionsexplosion | `StrikeEvent::AmmoExplosion` |

Zwischen den Arten wird wie bisher nach Kaliber skaliert (Knoten-Maßstab Kaliber /
75 mm, Fahrzeuggröße). Die Größen aus dem Video an dieser Skala festmachen: welches
Kaliber entspricht der gemessenen Explosion, und passt die Skala dann auch für die
anderen Arten? Größen sind im Spiel 1:1 in Metern; eine Explosion darf nicht größer
werden als im Video im Verhältnis zu den Fahrzeugen daneben.

Mündungsfeuer, Handwaffentreffer, Staubfahnen und Abgas sind nicht Teil dieses
Prompts, außer das Video zeigt etwas, das sich ohne Mehraufwand mitnehmen lässt.

## 5. Schritt 2: Ist-Zustand als Bildreihe

Die vorhandenen Werkzeuge machen Standbilder zu festen Zeitpunkten (`just
effects-shots`, `just blast-shots`). Für den Vergleich braucht es Bildreihen über die
ganze Dauer.

- Ein Werkzeug ergänzen (in `effects_bench.gd` oder `blast_shots.gd`, mit eigenem
  Rezept im `justfile`, zum Beispiel `just explosion-series`), das jede Art aus
  Abschnitt 4 einzeln auslöst, auf ebenem, ruhigem Boden, mit festem Bildtakt
  (`--fixed-fps`), aus einem Blickwinkel und Abstand wie im Video, und die Bilder zu
  denselben Zeitpunkten wie die Bildreihen aus Schritt 1 speichert.
- Daraus dieselben Kachelbilder bauen und je Art Video und Spiel untereinander
  stellen (`ffmpeg`, `vstack`). Das ist das Vergleichsbild; es wird nach jeder
  Änderungsrunde neu erzeugt.
- Den Stand vor der ersten Änderung aufheben (`build/explosion-ref/before/`), ebenso
  die Zahlen von `just effects-bench` vor der Änderung.

## 6. Schritt 3: Umsetzen

Arbeite eine Art nach der anderen ab, zuerst „Fahrzeug zerstört“ und „Einschlag
mittel“, und vergleiche nach jeder Runde mit dem Vergleichsbild. Was genau zu ändern
ist, ergibt sich aus den Profilen; die folgenden Punkte sind Erfahrungswerte, wo der
Unterschied zu einer guten Vorlage üblicherweise liegt, keine Vorgabe.

- **Zeitverlauf.** Blitz, Feuerball und Rauch überlappen und gehen ineinander über.
  Der Feuerball wird nicht ausgeblendet, sondern färbt sich zu Rauch: Farbverlauf über
  die Lebenszeit von fast weiß über gelb und orange zu dunkelrot und schwarzbraun,
  die Leuchtkraft fällt dabei auf null. Wachstum schnell am Anfang, dann langsam
  (Kurven statt fester Werte).
- **Rauchform.** Rauch aus wenigen großen, weichen Scheiben wirkt flach. Ballen
  entstehen aus mehr Teilchen mit unterschiedlicher Größe, Drehung und eigener
  Lebenszeit, mit Verwirbelung (`ParticleProcessMaterial`, Turbulenz), abnehmender
  Steiggeschwindigkeit und wachsender Größe. Mehrere Rauschtexturen oder eine
  erzeugte Bildfolge geben den Ballen innere Bewegung.
- **Schattierung.** Die beleuchteten Teilchen haben schon eine gewölbte Schein-Normale.
  Prüfen, ob Unterseite und Kern dunkler sein müssen, ob der Kern nachglüht und ob der
  Rauch zur Tageszeit und zum Wetter passt (Nacht: nur vom Feuer beleuchtet).
- **Aufbau aus Schichten.** Eine Explosion besteht aus mehreren Aussendern mit eigenem
  Zeitversatz: Blitz, Kern, Feuerballen, aufsteigender Rauch, Rauch am Boden,
  Erdfontäne, Staubring, Funken, Trümmer mit Rauchfahnen. Fehlende Schichten ergänzen,
  vorhandene abstimmen.
- **Dauer.** Die Zeiten aus dem Video sind Bildschirmzeit und gelten so auch im Spiel.
  Das Spiel läuft mit Zeitfaktor 2 (`docs/SCALE.md`); prüfen, ob der Rauch bei diesem
  Tempo zu lange steht, und die Entscheidung begründen.
- **Wind und Boden.** Rauch driftet mit dem Wind des Bioms, Staub und Erde nehmen die
  Bodenfarbe an (`tint`). Beides bleibt erhalten.
- **Licht.** Blitzlicht und Feuerschein auf Boden und Gebäuden nach dem Video
  abstimmen, innerhalb der Lichter-Budgets.

Die Werte je Art stehen als benannte Konstanten oder als eine Tabelle im Code, mit
Kommentar, aus welcher Messung sie stammen, nicht verstreut als Zahlen.

## 7. Grenzen

- **Nur Darstellung.** Kein Rust-Code in `crates/sim`, keine neuen Ereignisse, nichts
  wird zurückgelesen. Der goldene Hash ändert sich nicht. Godot-Zufall ist hier
  erlaubt.
- **Budget.** Aussender bleiben gepoolt, die Budgets je Grafikstufe (`fx_*`) und die
  LOD-Regeln bleiben bestehen und werden bei Bedarf erweitert, nicht umgangen. Mehr
  Teilchen und längerer Rauch kosten Füllrate: `just effects-bench` vorher und nachher
  in allen drei Stufen, alle Zeilen der Tabelle in `docs/ARCHITECTURE.md`. Jede Stufe
  bleibt mit 50 Explosionen über 60 FPS; „low“ darf einfacher aussehen als das Video.
- **Nebel des Krieges.** Neue Effekte starten nur an sichtbaren Punkten
  (`effects.visible_at`), auch neue Schichten.
- **Lesbarkeit.** Explosionsrauch verdeckt in den Spielregeln keine Sicht, Nebelwände
  schon (Abschnitt „Indirect fire and smoke“). Explosionsrauch darf Einheiten nicht
  so lange und so dicht verdecken, dass man ihn für eine Nebelwand hält oder die
  eigenen Einheiten nicht mehr findet, und er muss sich von einer Nebelwand
  unterscheiden.
- **Bekannte Falle.** Ein Mesh als Teilchen mit `StandardMaterial3D` hat in Godot
  4.7.2 beim Beenden seinen Shader verloren (Tabelle „Decisions“). Bei jedem neuen
  Material prüfen, dass das Spiel ohne Leck-Meldung endet. Godot und godot-rust
  werden nicht aktualisiert.
- Alle Biome (Wald, Wüste, Winter) und alle Tageszeiten ansehen, nicht nur den
  Standardfall.

## 8. Prüfen

1. Vergleichsbilder Video gegen Spiel für jede Art, vorher und nachher.
2. Tabelle Zielwert gegen erreichten Wert für die Messgrößen aus Abschnitt 3: Zeiten
   der Phasen, größte Ausdehnung, Steiggeschwindigkeit und Standzeit des Rauchs.
   Abweichungen über 20 % begründen oder beheben. Die gemessenen Farben sind
   Orientierung für Helligkeit und Farbverlauf, kein Zielwert auf den Farbton genau.
3. `just effects-shots build/effects`, `just blast-shots` und `just strike-shots`
   laufen durch; die Bilder ansehen, auch Wüste, Winter und Nacht.
4. `just effects-bench` in allen Stufen, Zahlen vorher und nachher.
5. Eine Bewegtaufnahme: eine Einstellung mit Gefecht aus `just trailer-shots
   --only=<shot>` neu aufnehmen und ansehen, ob die Explosionen im Zusammenhang
   stimmen (viele zugleich, über Fahrzeugen, an Gebäuden).
6. `just selftest` und `just check`.

Die Vergleichsbilder und die Tabelle dem Nutzer zeigen. Was nicht erreicht wurde,
steht im Abschlussbericht mit Grund, nicht als erledigt.

## 9. Abschluss

- `docs/ARCHITECTURE.md`: Abschnitt „Effects“ anpassen (Auslöser-Tabelle, „Drawing“,
  Budget- und Messtabelle) und einen Unterabschnitt „Explosions after a video
  reference (prompt 072)“ mit den Ablaufprofilen als Tabelle, dem neuen Werkzeug und
  den offenen Punkten. Auf Englisch, ohne Bilder aus dem Video.
- `docs/PERFORMANCE.md` anpassen, falls sich Messwerte dort ändern.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten (Werkzeug, dann je Art). Kein
  `git push`.
- Offen notieren: Das Demo-Video (`prompts/periodic/070-youtube-demo-video.md`) zeigt
  danach die alten Explosionen und müsste neu aufgenommen werden.

## Entscheidungen (autonom, Nachtlauf 17.11.2026)

Der Prompt hatte keine offenen Fragen; diese Entscheidungen hat der Nachtlauf selbst
getroffen. Alle vorläufig.

| Frage | Entscheidung | Warum |
|---|---|---|
| Welche Explosionen aus dem Video | A1 Einschlag 6,233 s, A2 Fahrzeug 6,933 s, C Einschlag am Panzer 14,417 s, D große Detonation an der Mauer 19,633 s, E kleiner Einschlag 29,45 s, F Brand 31,2 s, G stehender Rauchballen 1–6,2 s | Frei im Bild, alle Arten außer Munitionsexplosion abgedeckt |
| Munitionsexplosion | Aus A2 (Fahrzeug) × 1,5 plus Staubdom aus D abgeleitet | Im Video nicht zu sehen |
| Kaliber der Video-Einschläge | A1 ≈ 15-cm-Klasse (Maßstab 2,0), E ≈ 5 cm; Skala Kaliber / 75 mm bleibt | Fächer 11 m breit bei A1, halb so groß passt zu 7,5 cm; die lineare Skala trägt alle Arten |
| Dauer bei Zeitfaktor 2 | Bildschirmzeit wie im Video; schwarzer Rauch weg nach ~9,5 s, Erdwolke nach ~8 s | Länger stehender Rauch läse sich als Nebelwand; die Wrack-Rauchsäule läuft ohnehin weiter |
| Weißer Schockdom (054) | Bleibt, aber schwächer | Im Video ist nur der Staubdom zu sehen |
| Glühen des Feuerballs als Licht | Blitzlicht bleibt 0,4 s statt 1,4 s | 1,4 s kosteten auf „high“ ~4 FPS mit 50 Explosionen |
| Kleine Details aus der Ferne | Glut, Funken, Kernblitz erst ab 50 px Explosionsgröße | Je Explosion weniger Aussender; aus der Ferne unsichtbar |
