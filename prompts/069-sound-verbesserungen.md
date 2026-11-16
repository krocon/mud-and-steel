# Prompt 069 – Sound-Verbesserungen: Kettenfahrzeuge und MG

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/SOURCES.md` (Abschnitt „Sounds“), `docs/AUDIO_LIBRARY.md` (vor allem „Key
> decisions“), `docs/ASSETS.md` (Audio), `godot/assets/audio/sounds.json`,
> `tools/audio/synth.py` und `prompts/done/053-sound-stufe2.md`.

## 1. Ziel

Zwei Klanggruppen überzeugen noch nicht und werden besser:

- **Kettenfahrzeuge:** Fahrgeräusch (Motor unter Last) und Kettenrasseln. Heute je eine
  synthetische Schleife in `engine_track_light`, `engine_track_heavy`,
  `engine_halftrack` und `tracks`.
- **MG-Feuer:** die Klasse `shot_mg` (vier synthetische Varianten) und die Waffenordner
  `godot/assets/audio/weapon/mg_*` (heute Lubini 338242, laut Kommentar dort mit zu
  niedriger Feuerrate).

Am Ende klingen ein fahrender Panzer und ein feuerndes MG im Spiel glaubwürdig, jede
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
5. **ElevenLabs bewerten** (siehe Abschnitt 5) und, wenn freigegeben, gezielt Lücken
   füllen.
6. **Doku und Nachweise:** Zeilen in `docs/SOURCES.md` vor dem Commit, `CREDITS.md` bei
   Nennungspflicht, `docs/AUDIO_LIBRARY.md` (neue Tabelle, „Key decisions“ ergänzen),
   `docs/ASSETS.md` falls sich die Ablage ändert. Danach `just audio`, `just import`.

## 5. ElevenLabs

Der Nutzer hat ein Konto und fragt, ob es sinnvoll ist, Sounds über die API erzeugen zu
lassen. Einschätzung vorab: sinnvoll für Varianten und Lücken (Kettenrasseln,
Motorschleifen, Nachladen, entfernte Feuerstöße), weil Länge und Schleife steuerbar
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
   Freigabe warten. Erster Lauf höchstens 12 Sounds.
5. Prompts beschreiben den Klang (Kaliber, Feuerrate, Entfernung, Umgebung,
   „trocken, ohne Hall, ohne Musik, ohne Stimmen“), nennen aber keine Spiele, Filme
   oder Marken.
6. Übernommen wird nur, was die Hörprobe gegen die echten Aufnahmen besteht.

## 6. Nicht in diesem Prompt

- Keine Änderung an `crates/sim`; der Golden-Hash bleibt.
- Keine anderen Klangklassen (Explosionen, Stimmen, Musik, Ambiente), außer ein Fund
  drängt sich auf: dann melden, nicht nebenbei einbauen.
- Kein Umbau der Audio-Architektur aus Prompt 029/053 (Busse, Prioritäten, Hall).
- Kein Abschluss eines Abos, kein Anlegen von Konten.

## 7. Akzeptanzkriterien

1. Jede Quelle aus Abschnitt 3 hat ein Urteil mit Beleg (Lizenzstelle, Datum der
   Prüfung) in `docs/AUDIO_LIBRARY.md`.
2. `engine_track_*`, `tracks` und die MG-Klassen bzw. MG-Waffenordner spielen die
   gewählten Dateien; jede davon hat ihre Zeile in `docs/SOURCES.md`, bei
   Nennungspflicht auch in `CREDITS.md`.
3. Schleifen laufen ohne Knackser; MG-Ton und Feuerrate der Sim passen zusammen.
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
| Epidemic Sound abonnieren? | Nein, solange freie Quellen und ElevenLabs reichen; nur vorlegen, falls die Lizenz Spiele ausdrücklich erlaubt. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen, Downloads und
kostenpflichtigen API-Aufrufen fragen. Gemeinsame Dateien (`sounds.json`, `synth.py`,
`justfile`, Doku) nur klein und additiv ändern. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren.
