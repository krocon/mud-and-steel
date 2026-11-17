# Prompt 082 – YouTube-Video „Hinterhalt am Waldrand“

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `godot/scenes/tools/trailer_shots.gd` ganz (Aufbau eines Shots, `game_camera`,
> `--trailer-scout`, `--trailer-stills`), `tools/trailer-shots.sh`, `docs/MISSIONS.md`
> (Szenenskripte, Aktionen `order` und `fire`), die Szenen in `data/trailer/` als
> Muster (vor allem `column.toml`), in `docs/ARCHITECTURE.md` die Abschnitte zu den
> Trailer-Aufnahmen, „Air support and anti-aircraft fire“, hohem Gras (Prompt 073),
> Explosionen (Prompt 072) und Raketenwerfern (Prompts 062 und 076), dazu in
> `docs/GAME_DESIGN.md` Luftunterstützung, Sicht/Tarnung und Feuermodi.
>
> **Zwei Stufen.** Stufe 1 endet mit Standbildern und dem Ablaufplan; zeig sie und
> warte auf mein „Go“. Erst dann Stufe 2 (Aufnahme und Schnitt).

## 1. Ziel

Ein Video für YouTube (1920 × 1080, 60 fps, rund 75 Sekunden, nur Spielton, keine
Musik, kein Sprecher, kein HUD), das ein einziges Gefecht aus dem Spiel erzählt.
Du bist der Regisseur: Du entscheidest, wann geschnitten wird, wie lange ein Bild
steht und von wo man schaut. Aber du hast nur die Kamera, die auch der Spieler hat.

Fertig ist der Prompt, wenn

- `build/trailer/ambush_final.mp4` existiert und die Geschichte aus Abschnitt 3
  ohne Erklärung verständlich zeigt,
- jeder Shot mit `"game_camera": true` aufgenommen ist (Abschnitt 2),
- alles, was passiert, aus normalen Befehlen des Szenenskripts kommt und die
  Simulation unangetastet bleibt (Golden Hash unverändert),
- `just trailer-shots --only=ambush_…` die Shots Bild für Bild gleich neu aufnimmt,
- `just check` und `just selftest` grün sind.

## 2. Kamera: nur die Sicht des Spielers

- Nur die Spielkamera (`rts_camera.gd`): Neigung so, wie das Spiel sie aus der
  Zoomstufe ableitet (45° nah bis 65° weit), Abstand innerhalb `min_distance` bis
  `max_distance`, Sichtfeld des Spiels. Keine flachen Winkel, keine Verfolgerfahrt
  auf Höhe der Ketten, kein Blick unter die Baumkronen, den das Spiel nicht hergibt.
- Erlaubt und erwünscht: verschiedene Zoomstufen (weit für die Lage, mittel für die
  Bewegung, nah für den Treffer), verschiedene Blickrichtungen (Drehung um die
  Hochachse) und verschiedene Standpunkte; ruhiges Schwenken, Drehen und Zoomen, wie
  ein Spieler es mit Maus und Tasten könnte. Harte Schnitte zwischen den Shots.
- Regie: Jeder Shot hat einen Grund. Erst Ruhe und Weite, dann der Schock nah, dann
  das Chaos im Wechsel beider Seiten, am Ende wieder weit. Dieselbe Sekunde darf aus
  zwei Richtungen gezeigt werden (gleiche Szene, gleicher Seed, anderer Shot). Die
  Achse beachten: Die Kolonne fährt im Bild immer in dieselbe Richtung, bis das
  Gefecht beginnt.
- Kein HUD, kein Nebel des Krieges, keine Auswahlkreise (Standard der Shots).

## 3. Die Geschichte

Ort: grüne Landschaft im Sommer, klares Wetter, tiefer Sonnenstand mit langen
Schatten. Eine Hälfte der Fläche ist Wiese mit hohem Gras, die andere dichter Wald;
die Waldkante läuft quer durchs Bild.

| Akt | Was geschieht | Bildidee |
|---|---|---|
| 1 Marsch | Eine sowjetische Kolonne fährt langsam durch das hohe Gras auf den Wald zu: vorn 4 Panzer (`su_t34_76_1943`), dahinter 2 Raketenwerfer (`su_bm13`), dahinter viel Infanterie zu Fuß. | Weit: die Kolonne als Linie im Gras. Mittel von der Seite: Gras teilt sich. Blick über die Kolonne auf den stillen Wald. |
| 2 Erster Schuss | Kurz vor der Waldkante fällt ein Kanonenschuss aus dem Wald. Der vorderste Panzer explodiert, sein Turm fliegt weg. | Nah am Führungspanzer, der Schuss kommt von außerhalb des Bildes. Danach derselbe Moment weit, mit dem Mündungsfeuer im Wald. |
| 3 Feuerüberfall | MG-Salven aus dem Wald, die Infanterie wirft sich hin. Die drei übrigen Panzer fahren auseinander und schießen zurück. | Mittel, wechselnd zwischen Wiese und Waldkante; Leuchtspur quer durchs Bild. |
| 4 Enthüllung | Im Wald stehen 2 Tiger (`de_tiger1_e`) und viel Infanterie mit MG-Trupps (`de_inf_mg42_team`). | Erster Blick von der deutschen Seite aus dem Wald auf die Wiese. |
| 5 Raketen | Die beiden BM-13 feuern ihre Salven in den Wald. Die Einschläge setzen den Deutschen schwer zu. | Abschuss nah, dann weit mit den Rauchspuren, dann die Einschläge im Wald. |
| 6 Höhepunkt | Alle Panzer schießen, so gut sie können. Die Deutschen sind fast geschlagen: ein Tiger zerstört oder schwer beschädigt, kaum noch Infanterie. | Schnelle Wechsel, kürzere Shots. |
| 7 Wende | Deutsche Bomber (Luftunterstützung „Bombers“, He 111 H-16; ersatzweise „Ground attack“, Ju 87 D-5) bombardieren die sowjetische Stellung auf der Wiese. | Schatten der Flugzeuge über dem Gras, dann die Bombenreihe. |
| 8 Schluss | Die ganze Wiese brennt, Rauch zieht über den Wald. | Langsam herauszoomen auf die weiteste Stufe, lange stehen lassen. |

Eine V1 gibt es im Katalog nicht; dafür wird hier kein neues Fahrzeug gebaut.

## 3a. Umsetzung im Spiel

- **Karte:** Prüfe zuerst `tall_grass_test` und `twin_meadow`. Passt keine (halb
  hohes Gras, halb dichter Wald, genug Platz für Anmarsch und Auseinanderziehen),
  erzeuge eine kleine eigene Karte `ambush_meadow` mit den vorhandenen Generatoren;
  `just map-check ambush_meadow` muss sie annehmen.
- **Szene:** `data/trailer/ambush.toml`, fester Seed. Die Deutschen stehen im Wald
  mit „Feuer halten“ und bekommen den Feuerbefehl, wenn der vorderste Panzer nahe
  genug ist; die BM-13 erhalten ihr Ziel per `fire`; der Luftangriff wird wie von
  einem Spieler angefordert. Kann ein Szenenskript das noch nicht, ergänze eine
  Aktion, die den normalen `Command` absetzt.
- **Der Zufall gehört der Simulation.** Ob der erste Treffer die Munition zündet
  (Turm fliegt), wer wann fällt und ob die Deutschen „fast geschlagen“ sind, wird
  nicht erzwungen. Such mit `--trailer-scout` über Seeds, Abstände, Aufstellung und
  Zeitpunkte nach einem Ablauf, der die Geschichte trägt, und halte Seed und
  Zeitpunkte im Skript fest. Keine Sonderregeln, keine Unverwundbarkeit, keine
  Eingriffe in `crates/sim` für das Video.
- **Feuer am Schluss:** Nutze, was es an Bränden, Wracks und Rauch gibt. Reicht das
  nicht für „das ganze Areal steht in Flammen“, sag es in Stufe 1 und schlag eine
  reine Darstellungslösung vor (kein Einfluss auf Regeln); bau sie erst nach „Go“.
- **Ton:** Buspegel wie bei den anderen Trailer-Shots, Musik stumm. Der erste
  Schuss soll aus der Stille kommen: davor nur Motoren, Ketten, Wind.

## 4. Stufe 1: Plan und Standbilder (dann Halt)

1. Karte und Szene anlegen, mit `--trailer-scout` den Ablauf finden.
2. Shotliste schreiben: Name (`ambush_…`), Akt, Startzeit in der Szene, Länge,
   Zoomstufe, Blickrichtung, Standpunkt, Kamerabewegung, ein Satz „warum“.
   Richtwert 12 bis 16 Shots, 3 bis 8 Sekunden, zusammen rund 75 Sekunden.
3. Von jedem Shot ein Standbild mit `--trailer-stills` aufnehmen und ansehen:
   Ist das Wichtige im Bild, nicht von Baumkronen verdeckt, groß genug?
4. Mir zeigen: Shotliste, Standbilder, die Zerstörungsliste aus dem Scout-Lauf und
   alles, was vom Wunsch abweicht (z. B. Turm fliegt nicht, Wald verdeckt die Tiger,
   Brand zu klein). **Dann warten.**

## 5. Stufe 2: Aufnahme und Schnitt (nach „Go“)

1. `just trailer-shots --only=<die ambush-Shots>` aufnehmen.
2. Jeden Shot ansehen (Einzelbilder aus Anfang, Mitte, Ende) und schlechte neu
   einstellen.
3. Mit ffmpeg in der Reihenfolge der Shotliste hart aneinanderschneiden, Ton an den
   Schnitten 2 bis 3 Bilder überblenden, damit es nicht knackt; Lautheit −14 LUFS;
   H.264, `yuv420p`, AAC → `build/trailer/ambush_final.mp4`. Der Schnitt steht als
   Skript in `tools/` (ein Aufruf baut das Video neu), nicht als Handarbeit.
4. Das fertige Video einmal ganz in Einzelbildern prüfen (alle 2 Sekunden eines).

## 6. Grenzen

- Kein `git push`. Nichts installieren, ohne zu fragen.
- Keine Tests abschwächen; ändert sich der Golden Hash, ist etwas falsch.
- Keine Hoheitszeichen außer denen, die das Spiel schon zeigt (`docs/SOURCES.md`).
- Titel, Texteinblendungen, Logo und Thumbnail gehören nicht zu diesem Prompt.

## 7. Abschluss

Kurzbericht: Pfad des Videos, Länge, Shotliste mit Seed, was vom Wunsch abweicht
und warum, neue Dateien. `docs/ARCHITECTURE.md` (Trailer-Abschnitt) um die Szene
`ambush` ergänzen, diesen Prompt mit `git mv` nach `prompts/done/` verschieben.
