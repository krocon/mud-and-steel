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
`ambush` ergänzen, diesen Prompt mit `git mv` nach `prompts/periodic/` verschieben.

## Ergebnis Stufe 1 (17.11.2026): Plan und Standbilder, wartet auf „Go“

Standbilder: `build/trailer/stills/sheet_ambush_1.jpg` bis `_3.jpg` (alle 53
Standbilder der 17 Shots, einzeln als `ambush_<shot>_<Sekunde>.png` daneben),
Lichtvergleich `compare_ambush_light.jpg`. Neu aufnehmen:
`just trailer-shots --stills --only=ambush_…`.

### Was gebaut ist

- **Karte `ambush_meadow`** (`just gen-land ambush_meadow`, Seed 82, 1 km):
  weder `tall_grass_test` noch `twin_meadow` passen (Wald nur in Flecken, keine
  durchgehende Waldkante; `twin_meadow` ohne hohes Gras). Wald im Norden (Kante
  bei y ≈ −120) und an der ganzen Westflanke (x ≈ −140), Wiese mit hohem Gras
  im Südosten, Waldweg von einer Lichtung zur Kante. `just map-check` nimmt sie
  an.
- **Szene `data/trailer/ambush.toml`**, Seed 14. Drei neue Skriptaktionen, die
  nur normale `Command`s absetzen (`docs/MISSIONS.md`): `barrage`
  (`Command::Barrage`), `air` (`CallAir` / `CallAirLine`), `camouflage`
  (`SetCamouflage`). Golden Hash unverändert, `just check` und `just selftest`
  grün.
- **Spielkamera:** ein Shot mit `"game_camera": true`, dessen Schlüssel nur
  Blickpunkt, Abstand und Drehung nennen, wird von der Kamera des Spiels selbst
  gestellt (`rts_camera.gd` `snap_to`): Neigung aus der Zoomstufe, Abstand 15
  bis 600 m, Sichtfeld, Bodenabstand. Alle 17 Shots sind so angelegt.
- **Scout:** `just trailer-scout <shot> <sekunden> [seed …]` zeigt Schüsse,
  Treffer (durchschlagen oder abgeprallt), Einschläge, fliegende Türme und was
  übrig ist; mit Seeds eine Zeile je Seed. 30 Seeds durchsucht.

### Ablauf (Seed 14, Spielsekunden)

| Zeit | Ereignis |
|---|---|
| 0 | Tiger legen Tarnung an (30 s); die sowjetische Infanterie marschiert neben den stehenden Fahrzeugen los |
| 34,0 | Panzer und Werfer fahren ab; die Werfer halten 400 m vor dem Wald und richten sich ein |
| 50,0 | Spitzenpanzer 100 m vor der Waldkante: beide Tiger feuern, T-34 Nr. 1 zerstört bei (−1, −25), **Turm fliegt** |
| 51,0 | MG aus dem Westwald vernichten einen Schützentrupp bei (−49, 377) |
| 51,25 / 51,75 | T-34 Nr. 3 durchschlägt Tiger 2, T-34 Nr. 4 durchschlägt Tiger 1 (je 58 % Schaden) |
| 55,0 | zweite Salve der Tiger: T-34 Nr. 2 und Nr. 3 auf 23 % |
| 56,5–60,75 | beide BM-13 feuern ihre Salve (32 Raketen) |
| 57,75 | **Tiger 1 zerstört** bei (−35, −114) durch T-34 Nr. 4, Turm fliegt |
| 60,0 | T-34 Nr. 3 zerstört bei (69, −41) |
| 64,75 | T-34 Nr. 2 zerstört bei (−69, −31) |
| 65 | die Deutschen fordern Bomber an (Linie entlang der Kolonne) |
| 69,25–74,25 | 32 Raketen schlagen im Wald ein (x −40…48, y −87…−200) |
| 74,5 | letzter T-34 (Nr. 4) zerstört bei (0, 23), Turm fliegt |
| 82,25–86,0 | 24 Bomben (3 He 111 H-16) von y 127 bis 351, drei Reihen |
| 100 | übrig: Tiger 2 (42 %), deutsche Infanterie fast vollständig (ein Schützentrupp 78 %), beide BM-13 (43 %, 50 %), sowjetische Infanterie 56–100 % (ein Trupp vernichtet) |

### Shotliste (Reihenfolge des Schnitts, zusammen 74,1 s)

Abstand → Neigung nach der Spielkamera (45° nah bis 65° weit); „Standpunkt“ ist
die Seite, auf der die Kamera steht. Bis zum ersten Schuss steht sie im Osten
oder Süden: die Kolonne fährt im Bild immer nach rechts oben.

| # | Shot | Start (s) | Länge (s) | Zoomstufe (Abstand, Neigung) | Standpunkt | Blickpunkt | Kamerabewegung | Warum |
|---|---|---|---|---|---|---|---|---|
| 1 | `ambush_march_wide` | 34.3 | 5.0 | weit (300 → 280 m, 63°) | von Südosten | (0, 325) | Schwenk, Zoom heran, Drehung +7° | Lage: die Kolonne als Linie im Gras, der Westwald dahinter, lange Schatten. |
| 2 | `ambush_march_side` | 37.5 | 4.5 | nah (70 → 55 m, 53 → 51°) | von Osten | folgt den T-34 | folgt, Zoom heran, Drehung -15° | Bewegung: die Panzer von der Seite, das Gras teilt sich, die Spur bleibt liegen. |
| 3 | `ambush_march_wood` | 46.8 | 3.0 | weit (235 → 225 m, 62°) | von Süden | (0, -48) | Schwenk, Zoom heran | Ruhe vor dem Schuss: der stille Wald, die Spitze fährt von unten ins Bild. |
| 4 | `ambush_shot_near` | 48.6 | 4.2 | nah (32 → 42 m, 47 → 49°) | von Osten | folgt T-34 Nr. 1 | folgt, Zoom heraus, Drehung -8° | Der Schock nah: der Schuss kommt von außerhalb, der Turm fliegt. |
| 5 | `ambush_shot_wide` | 49.2 | 4.3 | mittel (200 m, 61°) | von Süden | (0, -68) | steht | Dieselbe Sekunde weit: Mündungsfeuer beider Tiger an der Waldkante, die Explosion davor. |
| 6 | `ambush_mg` | 49.8 | 3.6 | nah (60 → 55 m, 51°) | von Osten | (-50, 376) | Zoom heran | Feuerüberfall: MG aus dem Westwald mähen einen Schützentrupp nieder. |
| 7 | `ambush_spread` | 52.0 | 4.5 | mittel (175 → 160 m, 60 → 59°) | von Osten | (0, -35) | Schwenk, Zoom heran, Drehung -15° | Die drei Panzer fahren auseinander (Spuren im Gras) und schießen zurück. |
| 8 | `ambush_reveal` | 53.4 | 4.0 | mittel (150 → 140 m, 59 → 58°) | von Osten | (0, -98) | Zoom heran | Enthüllung: beide Tiger an der Waldkante, die Wiese vor ihnen (55,0 s zweite Salve). |
| 9 | `ambush_rockets_near` | 56.2 | 4.0 | nah (50 → 58 m, 50 → 51°) | von Osten | (0, 300) | Schwenk, Zoom heraus | Raketen nah: die BM-13 feuern Schiene um Schiene. |
| 10 | `ambush_tiger_hit` | 56.9 | 2.6 | nah (45 m, 49°) | von Süden | (-36, -112) | steht | Höhepunkt: ein Tiger explodiert, sein Turm fliegt. |
| 11 | `ambush_rockets_trails` | 57.2 | 4.4 | mittel (200 m, 61°) | von Süden | (0, 255) | Schwenk, Drehung +8° | Die Salve von hinten: Rauchspuren Richtung Wald. |
| 12 | `ambush_t34_east` | 58.9 | 2.6 | nah (55 m, 51°) | von Südosten | folgt T-34 Nr. 3 | folgt | Der Panzer an der Ostflanke wird getroffen. |
| 13 | `ambush_duel` | 63.2 | 3.2 | mittel (185 m, 60°) | von Südosten | (-16, -74) | steht | Panzer gegen Panzer über die Waldkante, im Hintergrund der brennende Tiger. |
| 14 | `ambush_impacts` | 69.0 | 5.2 | mittel (170 → 160 m, 60 → 59°) | von Südosten | (5, -128) | Zoom heran, Drehung -13° | Die Raketen schlagen im Wald ein. |
| 15 | `ambush_last_t34` | 73.6 | 2.4 | nah (45 m, 49°) | von Osten | folgt T-34 Nr. 4 | folgt | Der letzte T-34 fällt, Turm fliegt: die Wiese gehört den Deutschen. |
| 16 | `ambush_bombers` | 80.0 | 6.6 | weit (270 m, 63°) | von Osten | (0, 235) | Schwenk | Die Wende: Bombenreihen laufen über Infanterie und Werfer. |
| 17 | `ambush_end` | 89.0 | 10.0 | weit (230 → 600 m, 62 → 65°) | von Südosten | (0, 150) | Schwenk, Zoom heraus, Drehung +15° | Schluss: langsam heraus auf die weiteste Stufe, lange stehen lassen. |

### Abweichungen vom Wunsch

1. **„Fährt langsam“ geht nicht.** T-34 fahren im Gelände 20 m/s (36 km/h ×
   Zeitfaktor 2), Infanterie 2,2 m/s; gemischte Gruppen marschieren nicht im
   Gleichschritt. Die Infanterie läuft deshalb von Anfang an in zwei Reihen
   neben der Fahrzeugkolonne und fällt zurück, sobald die Fahrzeuge abfahren
   (beim ersten Schuss rund 350 m hinter der Spitze).
2. **Wald auch an der Westflanke** (L-Form statt halb/halb), damit MG-Trupps
   die zurückhängende Infanterie erreichen (MG 42: 200 m).
3. **Die Kolonne marschiert mit „Feuer halten“** und bekommt beim Überfall
   „Feuer frei“. Grund ist ein Fund in der Simulation (nicht in diesem Prompt
   angefasst): Die automatische Zielwahl (`choose_target`) prüft nur die eigene
   Sichtlinie (`sees_unit`), nicht die Sichtmaske des Spielers; getarnte
   Fahrzeuge (Prompt 055) und Trupps im Hinterhalt (Prompt 056) werden deshalb
   trotzdem automatisch beschossen, anders als `docs/ARCHITECTURE.md` und
   `docs/GAME_DESIGN.md` es beschreiben. Ohne „Feuer halten“ schossen die T-34
   ab Sekunde 2 auf die versteckten Trupps.
4. **Der erste Schuss sind zwei Schüsse:** ein 8,8-cm-Treffer kostet 774 von
   1000 Punkten; beide Tiger treffen im selben Tick. Im Bild ein Knall.
5. **Die Infanterie wirft sich nicht hin.** Getroffene fallen; Leuchtspur ist
   in den Standbildern nicht zu sehen (im bewegten Bild prüfen).
6. **Enthüllung von der Seite statt aus dem Wald:** von Norden verdecken die
   Baumkronen die Tiger; der Shot schaut von Osten die Waldkante entlang. Die
   deutsche Infanterie steht unter den Bäumen und ist nicht zu sehen (nur ihr
   Feuer).
7. **Raketen:** 13 s Flugzeit; Reihenfolge deshalb Abschuss → Tiger explodiert
   → Einschläge. Die Salve trifft den Wald, schadet der deutschen Infanterie
   aber kaum: „kaum noch Infanterie“ wird nicht erreicht. „Fast geschlagen“
   heißt hier: ein Tiger zerstört, der andere auf 42 %.
8. **Bomber:** fliegen in 500 m Höhe über der Kamera; man sieht die
   Bombenreihen, die Schatten sind in den Standbildern nicht zu erkennen.
9. **Feuer am Schluss reicht nicht:** einen Grasbrand gibt es nicht, der
   Bombenrauch ist nach rund 8 s weg, die Wracks qualmen dünn. Vorschlag unten.
10. **Licht:** Morgenlicht gibt lange Schatten, ist aber dunkel (mittlere
    Helligkeit 46 von 255 gegen 92 bei Tag, `compare_ambush_light.jpg`).
11. 17 statt 12–16 Shots; in den Totalen sind Fahrzeuge klein (das ist die
    Spielkamera).

### Offen für das „Go“

- **Licht:** Morgen lassen, oder Tag (hell, kurze Schatten)? Tag ändert die
  Sichtweiten (100 statt 80 %), der Ablauf wird neu gesucht.
- **Feuer (nur Darstellung, erst nach „Go“):** Flächenbrand im hohen Gras: von
  den Brandflecken, die Einschläge schon in das Spurenraster
  (`grass_trails.gd`, Kanal „verbrannt“) stempeln, frisst sich ein Feuersaum
  mit fester Geschwindigkeit durch das hohe Gras (Flammen am Saum, dichter
  dunkler Rauch, dahinter verkohlte Stoppeln); er endet an kurzem Gras, Weg und
  Wald, liest nichts zurück und ändert keine Regel. Dazu kräftigere Rauchsäulen
  über Wracks. Ohne das zeigt der Schluss qualmende Wracks und Krater.
- **Zielwahl-Fehler** (Punkt 3): eigener Prompt.

## Ergebnis Stufe 2 (17.11.2026): Aufnahme und Schnitt

**Video:** `build/trailer/ambush_final.mp4`, 76,1 s (4566 Bilder), 1920 × 1080,
60 fps, H.264 `yuv420p`, AAC, −14,2 LUFS (im MP4 gemessen), 271 MB. Einzelbilder alle 2 s:
`build/trailer/stills/sheet_ambush_final.jpg`. Neu bauen:

```bash
just trailer-shots --only=ambush_march_wide,ambush_march_side,ambush_march_wood,ambush_shot_near,ambush_shot_wide,ambush_mg,ambush_spread,ambush_reveal,ambush_rockets_near,ambush_tiger_hit,ambush_rockets_trails,ambush_t34_east,ambush_duel,ambush_impacts,ambush_last_t34,ambush_bombers,ambush_end
just trailer-cut ambush
```

**Shotliste:** wie in Stufe 1 (Seed 14, Karte `ambush_meadow`, Morgenlicht),
mit einer Änderung: `ambush_end` beginnt bei 110 s statt 89 s und dauert 12 s
(9 s Zoom von 230 auf 600 m, dann 3 s Stand), damit der Brand die Wiese
erfasst hat. Gesamtlänge damit 76,1 s.

### Neu in Stufe 2

- **Grasbrand** (`godot/scenes/world/grass_fire.gd`, nur Darstellung): Brand
  unter Einschlagflecken ab 4 m Breite (Raketen, Bomben, schwere Granaten,
  explodierende Fahrzeuge), Ausbreitung in Spielzeit über ein 4-m-Raster nur im
  hohen Gras (1,4 m/s, mit dem Wind schneller), 9 s Brenndauer je Zelle,
  verkohlte Stoppeln über das Spurenraster, Flammen und Rauch aus der
  Effektbibliothek, Brandton über die Feuerschleifen von `battle_sound.gd`.
  In der Szene brennen oder verkohlen bis Sekunde 110 rund 8,4 ha, bis
  Sekunde 120 rund 10,5 ha.
  Beschreibung: `docs/ARCHITECTURE.md` „Grass fire“. Nicht gemessen
  (Bildrate), alle Werte vorläufig.
- **Schnitt:** `just trailer-cut <prefix>` (`tools/trailer-cut.sh`): harte
  Schnitte, jeder Shot behält seine Bilder; Ton an jedem Schnitt je 1,5 Bilder
  aus- und eingeblendet (zusammen 3 Bilder, nichts überlappt, Bild und Ton
  bleiben synchron), `loudnorm` auf −14 LUFS mit Messdurchgang, der gemessene
  Rest als einfache Verstärkung mit Begrenzer bei −1,5 dB.
- **Trailer-Werkzeug:** Zielringe eigener Granaten sind ohne HUD aus;
  `pre_roll_s` je Shot (Effekte laufen in Echtzeit: was im schnellen Vorlauf
  startet, stand sonst zu lange im Bild); Partikelsysteme bekommen in
  Trailer-Aufnahmen feste Startwerte.

### Geprüft

- Jeder Shot in drei Einzelbildern (Anfang, Mitte, Ende), das fertige Video
  alle 2 s: kein HUD, keine Zielringe, keine schwarzen Bilder, Reihenfolge wie
  geplant.
- Vor dem ersten Schuss (Videosekunde 13,9) nur Motoren und Ketten; die
  Simulation feuert vor Spielsekunde 50,0 keinen Schuss.
- Wiederholbarkeit: zwei Aufnahmen von `ambush_march_side` sind Bild für Bild
  gleich (270 von 270); bei `ambush_tiger_hit` weichen 2 von 156 Bildern
  minimal ab (PSNR im Mittel 81 dB, schlechtestes Bild 59 dB). Der Ton ist nicht
  bitgleich. Ablauf, Kamera und Zeitpunkte sind immer dieselben.
- `just check` und `just selftest` grün, Golden Hash unverändert.

### Abweichungen (zusätzlich zu Stufe 1)

- Der Brand ist reine Darstellung: Infanterie und Werfer stehen im brennenden
  Gras, ohne Schaden zu nehmen.
- Das Schlussbild zeigt eine weitgehend abgebrannte, an den Rändern brennende
  Wiese mit Rauch; die Flammen sind in der weitesten Stufe klein.
- Wahrer Spitzenpegel −1,2 dBFS statt −1,5 und −14,2 statt −14,0 LUFS
  (vor der AAC-Kodierung −14,1 LUFS und −1,5 dB).
- Der Brandton stammt aus einer leisen Aufnahme und ist kräftig angehoben
  (bis +14 dB); im Schlussbild liegt er etwa auf dem Pegel der Motoren am
  Anfang.

### Nachtrag Prompt 085 (Bäume unter schwerem Feuer)

Seit Prompt 085 fällen und entlauben Explosionen ab 10 cm die Bäume ringsum.
Die Raketensalve in den Waldrand (Spielsekunde 69–74) hinterlässt jetzt eine
Schneise aus umgestürzten und kahlen Bäumen, und die Simulation verläuft
danach anders (andere Hashes). Die Aufnahmen ab `ambush_impacts` zeigen den
alten Stand; bei einer Neuaufnahme die Szene ab der Salve neu abnehmen
(`just trailer-scout`), die Zeitpunkte davor bleiben gleich.

