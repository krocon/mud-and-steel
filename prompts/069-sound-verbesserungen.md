# Prompt 069 – Sound-Verbesserungen: Kettenfahrzeuge, MG, Panzerschüsse, Treffer, Explosionen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/SOURCES.md` (Abschnitt „Sounds“), `docs/AUDIO_LIBRARY.md` (vor allem „Key
> decisions“), `docs/ASSETS.md` (Audio), `godot/assets/audio/sounds.json`,
> `tools/audio/synth.py` und `prompts/done/053-sound-stufe2.md`.

## 1. Ziel

Fünf Klanggruppen überzeugen noch nicht und werden besser:

- **Kettenfahrzeuge:** Fahrgeräusch (Motor unter Last) und Kettenrasseln. Heute je eine
  synthetische Schleife in `engine_track_light`, `engine_track_heavy`,
  `engine_halftrack` und `tracks`.
- **MG-Feuer:** die Klasse `shot_mg` (vier synthetische Varianten) und die Waffenordner
  `godot/assets/audio/weapon/mg_*` (heute Lubini 338242, laut Kommentar dort mit zu
  niedriger Feuerrate).
- **Panzerschüsse:** `shot_cannon_small`, `shot_cannon_medium`, `shot_cannon_large`
  (synthetisch). Es fehlen Druck, der scharfe Mündungsknall und der Nachhall im
  Gelände; die Kaliber unterscheiden sich zu wenig.
- **Treffer:** `hit_shell`, `hit_shell_small`, `hit_penetration`, `hit_ricochet`,
  `hit_bullet_armor` (synthetisch). Ein Durchschlag, ein Abpraller und ein Einschlag
  im Boden sollen sich auf Anhieb unterscheiden lassen.
- **Explosionen:** `explosion_vehicle`, `explosion_tail`, `crater`, dazu der Einschlag
  von Sprenggranaten. Es fehlen Wucht im Bass, Trümmer und ein langer Ausklang.

Für diese drei Gruppen hat der Nutzer keine Kandidaten genannt: die Session sucht
selbst, zuerst in `docs/AUDIO_LIBRARY.md`, dann bei Freesound (CC0, CC BY), zuletzt
über ElevenLabs.

Am Ende klingen ein fahrender Panzer, ein feuerndes MG, ein Panzerschuss, sein Treffer
und eine Explosion im Spiel glaubwürdig, jede
neue Datei hat eine geklärte Lizenz und eine Zeile in `docs/SOURCES.md`, und es steht
fest, ob ElevenLabs als Quelle für weitere Sounds taugt.

Erst prüfen und anhören, dann einbauen: keine Datei kommt ins Repo, deren Lizenz nicht
auf der Seite selbst nachgelesen wurde.

## 2. Ausgangslage

- Regeln für fremde Dateien stehen in `docs/SOURCES.md`: Ablage in
  `assets-src/audio/<class>/` bzw. `assets-src/audio/weapon/<slug>/`, Zeile in der
  Sounds-Tabelle vor dem Commit; für KI-Sounds Werkzeug und Version, vollständiger
  Prompt, Datum und Lizenz der Ausgabe (kommerzielle Nutzung muss erlaubt sein).
  Klassen und Waffen mit Dateien in `assets-src/audio/` spielen nur diese Aufnahmen.
- `docs/AUDIO_LIBRARY.md` hat für **Pixabay** schon entschieden: kommerzielle Nutzung
  erlaubt, eigenständige Weitergabe verboten; deshalb nicht in ein Repository, das
  öffentlich ist oder werden kann (lokal in `assets-src/audio-private/`, gitignored),
  niedrige Priorität, Herkunft der Uploads unsicher. Diese Entscheidung gilt weiter,
  solange die Prüfung in Schritt 1 nichts anderes ergibt.
- Für Ketten liegen dort geprüfte Alternativen: Zeilen 10 (Sherman, CC BY-SA),
  11 (T-26, CC0), 12 und 13 (YleArkisto, CC BY 4.0).
- Für Schüsse, Treffer und Explosionen liegen dort: Zeile 21 (Geschütz, CC0, schon in
  `shot_howitzer`), 23 (Artillerie), 25 (EOD-Detonation), 26 und 27 (ferne
  Explosionen, Ausklang), 28 (Trümmer), 29 (Beton), 30 (Metall-Einschläge, Foley).
- Wie Schüsse, Treffer und Explosionen heute geschichtet werden (Knall, Ausklang,
  Schallverzögerung, Hall, Zoom-Akustik), steht in `prompts/done/053-sound-stufe2.md`
  und `prompts/done/054-explosionen-stufe2.md`.
- Vier vom Nutzer mit ElevenLabs erzeugte MG-Dateien liegen in `docs/.sfx/MG-Sound/`
  (`Firing_sound_of_the__#1…#4-….wav`, je 5 s, 48 kHz, 16 bit, Stereo). Der Ordner
  `docs/.sfx/` ist gitignored und dient als Eingang für Rohmaterial.

## 3. Kandidaten des Nutzers

| Gruppe | Seite | Hinweis |
|---|---|---|
| Kettenfahrzeug, Fahrgeräusch | https://pixabay.com/de/sound-effects/film-spezialeffekte-tank-moving-143104/ | war in der Liste doppelt genannt |
| Kettenfahrzeug, Fahrgeräusch | https://pixabay.com/de/sound-effects/film-spezialeffekte-tank-moving-sfx-319878/ | |
| Kettenrasseln | https://pixabay.com/de/sound-effects/film-spezialeffekte-tank-track-ratteling-197409/ | |
| MG-Schüsse | https://pixabay.com/de/sound-effects/film-spezialeffekte-semi-auto-rifle-simulation-74566/ | Titel sagt halbautomatisches Gewehr und „Simulation“: prüfen, ob es als MG taugt |
| Militärfahrzeuge (Kategorie) | https://www.epidemicsound.com/de/sound-effects/categories/vehicles/military/ | Abo-Dienst |
| MG-Schüsse | https://www.epidemicsound.com/sound-effects/tracks/14994f5a-4588-4769-9e1c-6a75d69c1eb8/ | Abo-Dienst |
| MG-Schüsse (ElevenLabs, schon erzeugt) | `docs/.sfx/MG-Sound/` | Prompt, Modell und Tarif beim Nutzer erfragen |

## 4. Umfang

1. **Lizenzen prüfen.** Jede Seite aus Abschnitt 3 im Browser öffnen und dort
   nachlesen: Lizenztext, Urheber, Format, Länge, Beschreibung, Hinweise auf die
   Herkunft. Besonders:
   - Pixabay: aktuelle „Content License“; ob der Upload nach einem Film- oder
     Spielmitschnitt aussieht (Kategorie „Film & Spezialeffekte“, Musik oder Stimmen im
     Hintergrund, Name des Uploaders). Nichts aus Spielen oder Filmen
     (`docs/SOURCES.md`).
   - Epidemic Sound: ob die Lizenz das Einbetten in ein verkauftes Spiel überhaupt
     abdeckt, mit welchem Tarif, und was nach Ende des Abos gilt. Der Nutzer hat dort
     kein Konto genannt; ohne klare Erlaubnis für Spiele scheidet die Quelle aus.
   - ElevenLabs: Nutzungsbedingungen für Sound Effects; kommerzielle Nutzung nur mit
     bezahltem Tarif (den Tarif des Nutzers erfragen), Pflicht zur Nennung, Verbote.
   Ergebnis als Tabelle in `docs/AUDIO_LIBRARY.md` (neuer Abschnitt, gleiche Spalten wie
   „Best individual sounds“), mit Urteil je Zeile: ins Repo, nur lokal, ausgeschlossen.
2. **Anhören und vergleichen.** Vor jedem Download fragen (Dateiname, Quelle, Größe).
   Rohdateien nach `docs/.sfx/<gruppe>/`. Je Gruppe die Kandidaten gegen den heutigen
   Stand und gegen die geprüften Alternativen aus `docs/AUDIO_LIBRARY.md` (Zeilen 3,
   10–15, 36) stellen: Spektrum, Lautheit, Störgeräusche, Schleifenfähigkeit,
   Feuerrate (MG 42 etwa 20 Schuss/s, MG 34 etwa 15, Maxim/Vickers/Browning 8–10).
   Kurzer Bericht mit Empfehlung je Klasse; die Auswahl treffe ich.
3. **Kettenfahrzeuge einbauen.** Aus den gewählten Aufnahmen nahtlose Schleifen
   schneiden (Überblendung wie bei `fire`), Motor und Kette getrennt halten, damit
   `tracks` weiter mit der Geschwindigkeit und der Motor mit der Last laufen kann.
   Mindestens leicht/schwer unterscheiden. Mono, 44,1 kHz, 16 bit; Pegel an die
   übrigen Klassen angleichen. Prüfen, wie Tonhöhe und Lautstärke heute an der
   Geschwindigkeit hängen (`godot/` Audio-Skripte), und nachstellen, dass Anfahren,
   Fahrt und Halt ohne Sprung klingen, auch mit zehn Panzern gleichzeitig.
4. **MG einbauen.** Einzelschüsse oder kurze Feuerstöße so schneiden, dass sie zur
   Feuerrate der Sim passen (die Sim bestimmt, wann ein Schuss fällt; der Ton darf
   keine eigene Salve vorspielen, die länger läuft als der Feuerstoß). Mindestens drei
   Varianten je Waffe, kein hörbares Wiederholungsmuster. Schnelle MG (MG 42) und
   langsame MG klingen verschieden. Überschallknall, Schallverzögerung und Hall aus
   Prompt 053 bleiben unverändert wirksam.
5. **Panzerschüsse einbauen.** Drei Kaliberstufen, die man blind auseinanderhält
   (etwa 20–50 mm, 75–88 mm, ab 100 mm). Jeder Schuss aus Schichten: harter
   Mündungsknall, Körper im Bass, Ausklang im Gelände; mindestens vier Varianten je
   Stufe. Nah und fern müssen beide stimmen (Zoom-Akustik und Entfernungsfilter aus
   053 anhören, nicht umbauen). Abgrenzen gegen `shot_howitzer` (dumpfer, länger).
6. **Treffer einbauen.** Durchschlag (Metallschlag plus dumpfer Innenknall),
   Abpraller (heller Schlag mit wegziehendem Sirren), Sprenggranate auf Panzerung,
   Einschlag im Boden, MG-Garbe auf Panzerung (kurzes Prasseln). Kurz halten, damit
   viele Treffer nacheinander nicht verschmieren; mindestens drei Varianten je Klasse.
7. **Explosionen einbauen.** Fahrzeugexplosion mit Bass, Trümmern und Ausklang
   (Munitionsbrand als eigene Schicht, wenn vorhanden), Granateinschlag nach
   Kalibergröße gestaffelt, `explosion_tail` als ferner Nachhall. Echte Detonationen
   bevorzugen (Zeilen 22, 25–28), mit Trümmer-Foley schichten. Pegel prüfen: zehn
   Explosionen gleichzeitig dürfen nicht übersteuern (Begrenzer und Prioritäten aus
   029/053 nutzen, nicht ändern).
8. **Zusammen anhören.** Ein Gefecht mit Panzern, MG und Artillerie nachstellen und
   prüfen, dass Schuss, Treffer und Explosion sich gegenseitig nicht zudecken und die
   Reihenfolge (Schuss, Flugzeit, Treffer) hörbar bleibt. Kurze Aufnahme oder
   Pegelmessung als Beleg im Abschlussbericht.
9. **ElevenLabs bewerten** (siehe Abschnitt 5) und, wenn freigegeben, gezielt Lücken
   füllen.
10. **Doku und Nachweise:** Zeilen in `docs/SOURCES.md` vor dem Commit, `CREDITS.md` bei
   Nennungspflicht, `docs/AUDIO_LIBRARY.md` (neue Tabelle, „Key decisions“ ergänzen),
   `docs/ASSETS.md` falls sich die Ablage ändert. Danach `just audio`, `just import`.

## 5. ElevenLabs

Der Nutzer hat ein Konto und fragt, ob es sinnvoll ist, Sounds über die API erzeugen zu
lassen. Einschätzung vorab: sinnvoll für Varianten und Lücken (Kettenrasseln,
Motorschleifen, Nachladen, entfernte Feuerstöße, Treffer auf Panzerung, Abpraller,
Trümmer), weil Länge und Schleife steuerbar
sind und die Rechte beim bezahlten Tarif klar sind; nicht als Ersatz für echte
Aufnahmen benannter Waffen, wo es welche gibt (Authentizität), und nur nach Hörprobe.

1. Zuerst die vier vorhandenen Dateien in `docs/.sfx/MG-Sound/` bewerten: klingen sie
   nach einem MG der Zeit, stimmt die Feuerrate, lassen sich Einzelschüsse schneiden?
2. Die aktuelle API-Dokumentation nachlesen (Sound-Effects-Endpunkt, Parameter für
   Länge, Prompt-Einfluss, Schleife, Ausgabeformat, Kosten je Aufruf), nicht raten.
3. Werkzeug `tools/audio/elevenlabs_sfx.py` mit `just sfx-generate <liste>`: liest eine
   Liste (Zielklasse, Prompt, Länge, Schleife, Anzahl Varianten) aus
   `tools/audio/sfx_prompts.toml`, schreibt nach `docs/.sfx/generated/` und legt neben
   jede Datei den Prompt, das Modell und das Datum ab (das ist die Zeile für
   `docs/SOURCES.md`). Der API-Schlüssel kommt nur aus der Umgebungsvariable
   `ELEVENLABS_API_KEY`, die der Nutzer selbst setzt: nie ins Repo, nie in den Chat,
   nie in Protokolle.
4. Vor dem ersten Lauf Anzahl der Aufrufe und geschätzte Kosten nennen und auf
   Freigabe warten. Erster Lauf höchstens 12 Sounds, danach je
   Gruppe neu freigeben lassen.
5. Prompts beschreiben den Klang (Kaliber, Feuerrate, Entfernung, Umgebung,
   „trocken, ohne Hall, ohne Musik, ohne Stimmen“), nennen aber keine Spiele, Filme
   oder Marken.
6. Übernommen wird nur, was die Hörprobe gegen die echten Aufnahmen besteht.

## 6. Nicht in diesem Prompt

- Keine Änderung an `crates/sim`; der Golden-Hash bleibt.
- Keine anderen Klangklassen (Stimmen, Musik, Ambiente, Flugzeuge, Gebäude, Feuer,
  Oberfläche), außer ein Fund drängt sich auf: dann melden, nicht nebenbei einbauen.
- Keine Änderung an den sichtbaren Explosionen (Prompt 054), nur am Ton.
- Kein Umbau der Audio-Architektur aus Prompt 029/053 (Busse, Prioritäten, Hall).
- Kein Abschluss eines Abos, kein Anlegen von Konten.

## 7. Akzeptanzkriterien

1. Jede Quelle aus Abschnitt 3 hat ein Urteil mit Beleg (Lizenzstelle, Datum der
   Prüfung) in `docs/AUDIO_LIBRARY.md`.
2. `engine_track_*`, `tracks`, die MG-Klassen bzw. MG-Waffenordner, `shot_cannon_*`,
   die Treffer-Klassen und die Explosions-Klassen aus Abschnitt 1 spielen die
   gewählten Dateien; jede davon hat ihre Zeile in `docs/SOURCES.md`, bei
   Nennungspflicht auch in `CREDITS.md`.
3. Schleifen laufen ohne Knackser; MG-Ton und Feuerrate der Sim passen zusammen;
   die drei Kaliberstufen und die Trefferarten sind nach Gehör unterscheidbar; ein
   großes Gefecht übersteuert nicht.
4. Kein Material mit Weitergabeverbot liegt in git.
5. `just check` und `just selftest` grün.
6. Abschlussbericht: was ersetzt wurde, woher, mit welcher Lizenz, wie ich es anhöre
   (`just run --skip-menu`, welche Einheiten), was ElevenLabs gekostet hat, was offen
   ist.

## 8. Offene Fragen

| Frage | Vorschlag |
|---|---|
| Ist oder wird das Repository öffentlich? | Solange offen: Pixabay-Material nur lokal (`assets-src/audio-private/`), im Repo bleiben CC0/CC BY und eigene bzw. KI-Sounds mit klarer Lizenz. |
| Welchen ElevenLabs-Tarif hast du, und mit welchem Prompt sind die vier MG-Dateien entstanden? | Ohne bezahlten Tarif keine kommerzielle Nutzung: dann nur als Hörvorlage. |
| Echte Aufnahme oder KI, wenn beide gut klingen? | Echte Aufnahme für benannte Waffen und Fahrzeuge, KI für Varianten, Schleifen und Lücken. |
| Ein Kettenklang je Gewichtsklasse oder je Nation? | Zuerst leicht/schwer plus Halbkette; Nationen nur, wenn passende Aufnahmen vorliegen (Sherman, T-26). |
| Reihenfolge, wenn die Session zu groß wird? | Erst Panzerschüsse, Treffer und Explosionen (hört man ständig), dann MG, dann Ketten; jede Gruppe mit eigenem Commit, sodass ein Abbruch nichts Halbes hinterlässt. |
| Eigene Klänge je Geschütz (8,8 cm, 7,5 cm, 76 mm)? | Nein, drei Kaliberstufen; einzelne Geschütze nur, wenn eine echte Aufnahme vorliegt (Ablage wie bei `weapon/<slug>/`). |
| Epidemic Sound abonnieren? | Nein, solange freie Quellen und ElevenLabs reichen; nur vorlegen, falls die Lizenz Spiele ausdrücklich erlaubt. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen, Downloads und
kostenpflichtigen API-Aufrufen fragen. Gemeinsame Dateien (`sounds.json`, `synth.py`,
`justfile`, Doku) nur klein und additiv ändern. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren.
