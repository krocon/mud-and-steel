# Prompt 070 – YouTube-Demo-Video: Panzer fahren, schießen, explodieren

> In einer frischen Session im Repo-Ordner ausführen, mit dem zweiten Ordner
> `/Users/marckronberg/WebstormProjects/videoserien` als zusätzlichem Arbeitsordner.
> Lies zuerst hier `CLAUDE.md`, `docs/CONTROLS.md`, `docs/MISSIONS.md`,
> `docs/PERFORMANCE.md`, `godot/scenes/tools/demo_reel.gd` und im `justfile` die Rezepte
> mit `--write-movie`. Lies dann im Video-Projekt `CLAUDE.md` (Abschnitt `npm run promo`),
> `series/comiccommander/SERIES.md` (Aufbau eines Promos) und
> `series/app-iconic-promo/` (jüngstes Promo: `promo.json`, `build.mjs`, `promo.js`,
> `tools/`, `youtube.md`).

## 1. Ziel

Ein Demo-Video des Spiels für YouTube, **1 bis 4 Minuten** lang (Richtwert 90 bis
120 Sekunden), in 16:9 und 1920 × 1080 bei 60 Bildern pro Sekunde. Es zeigt echte
Spielszenen aus wechselnden Blickwinkeln: Panzer fahren durch das Gelände, schießen,
werden getroffen und explodieren, Infanterie und Geschütze kämpfen mit, dazu Rauch,
Krater und der Spielton.

Das Video entsteht in zwei Teilen:

1. **Aufnahmen** aus dem Spiel: hier im Repo, mit einem geskripteten Ablauf und Godots
   Movie Maker (`--write-movie`), jede Einstellung als eigene Datei.
2. **Schnitt** im Video-Projekt: ein neues Promo `series/mud-and-steel-promo/` nach dem
   Muster der vorhandenen Promos (HyperFrames, `npm run promo`), mit Titel,
   Bildschirmtexten, Übergängen und Abspann.

Am Ende liegt `series/mud-and-steel-promo/dist/mud-and-steel_16x9.mp4` vor, dazu ein
Thumbnail und `youtube.md` mit Titel, Beschreibung, Kapiteln und Tags (EN und DE).
Hochgeladen wird nicht: das macht der Nutzer selbst.

## 2. Ausgangslage

- `godot/scenes/tools/demo_reel.gd` (Start mit `-- --demo`) ist ein alter geskripteter
  Ablauf für die Aufnahme: Kameraflug, Auswahlrahmen, Fahrbefehl, Drehung. Er stammt
  aus der Zeit vor Infanterie, Effekten, Ton und Karten und reicht für das Video nicht.
  Er zeigt aber, wie ein Skript die Kamera führt und Eingaben wie ein Spieler erzeugt.
- `main.gd` kennt schon `--focus`, `--yaw`, `--clip` und `--map`; die Rezepte
  `terrain-shots` und `objects-shots` nehmen mit `--write-movie` und `--fixed-fps` auf.
- Karten liegen in `data/maps/` (unter anderem `farms`, `river_land`, `town_small`,
  `desert_village`, `winter_village`, `mountain_pass`).
- Im Video-Projekt sind Promos Ordner mit `promo.json` statt `series.json`. Die
  Pipeline für Lernvideos (TTS, `sync`, `status`) fasst sie nicht an. `build.mjs` ist
  die Quelle für Szenen, Zeiten und Texte; `promo.json → durationSeconds` legt die
  Länge fest. Aufnahmen liegen in `assets/rec/<format>/` und sind wegen `*.mp4` in der
  `.gitignore` nicht im Git.
- Für das Video-Projekt gelten dessen Regeln: Kanal-Logo oben rechts, Begrüßung
  „Hi nerds!“ als Bildschirmtext, Lautheit −14 LUFS, **Freigabe-Gate** vor dem Rendern.

## 3. Regeln

- Die Simulation bleibt unverändert. Der Ablauf entsteht nur aus `Command`s, wie sie
  ein Spieler oder die KI gibt; nichts wird für das Video gestellt, was das Spiel nicht
  kann (keine unverwundbaren Einheiten, keine Treffer auf Bestellung). Der goldene Hash
  in `crates/sim/tests/determinism.rs` ändert sich nicht.
- Fester Seed und feste Karte je Einstellung, damit jede Aufnahme wiederholbar ist.
- Die Kamera darf für das Video mehr als im Spiel (tiefer, näher, flacher Winkel,
  Verfolgerfahrt). Das ist reine Darstellung in Godot und nur im Aufnahme-Modus aktiv.
- Gezeigt wird nur, was im Repo liegt und dessen Lizenz die Veröffentlichung erlaubt.
  Sounds aus `assets-src/audio-private/` (Pixabay, nur lokal) sind im Video hörbar und
  damit veröffentlicht: vor der Aufnahme in `docs/SOURCES.md` und
  `docs/AUDIO_LIBRARY.md` prüfen, ob das erlaubt ist, sonst für die Aufnahme abschalten.
  Namensnennungen für CC BY gehören in die YouTube-Beschreibung.
- Keine Hakenkreuze, SS-Runen oder Roten Kreuze (gilt ohnehin). Keine Verherrlichung:
  sachlicher Ton in den Texten, keine Parolen.
- Behauptungen im Video nur mit Beleg aus dem Repo. Kein Erscheinungsdatum, kein Preis,
  kein „jetzt erhältlich“, solange der Nutzer das nicht vorgibt. Der Stand heißt
  „in Entwicklung“.
- Nichts installieren, nichts pushen, nichts hochladen ohne Freigabe. Im Video-Projekt
  nur den Ordner `series/mud-and-steel-promo/` und, falls nötig, ein Startskript
  `run-mud-and-steel-promo.command` anlegen.

## 4. Schritte

### Schritt 1: Konzept und Drehbuch (kostet nichts)

Schreibe das Drehbuch als Tabelle in `series/mud-and-steel-promo/SERIES.md`: Zeit,
Einstellung, Karte, Blickwinkel, was passiert, Bildschirmtext. Vorschlag für den Aufbau,
anpassen an das, was im Spiel gut aussieht:

| Abschnitt | Inhalt |
|---|---|
| Einstieg (5–8 s) | Ruhiges Bild, dann erster Schuss; Titel „Mud and Steel“, „Hi nerds!“ |
| Anmarsch | Panzerkolonne im Gelände: Übersicht von oben, dann Verfolgerfahrt tief neben den Ketten, Staub und Spuren |
| Erster Kontakt | Schusswechsel auf Entfernung: Mündungsfeuer, Leuchtspur, Abpraller, Durchschlag |
| Gefecht | Mehrere Blickwinkel auf dieselbe Szene: Spieleransicht mit HUD, flacher Winkel am Boden, Schwenk um ein brennendes Wrack; Explosionen, Rauch, Krater |
| Vielfalt | Kurze Schnitte: Infanterie und Geschütze, Dorf, Winter, Wüste, Fluss, Tarnungen |
| Unter der Haube | Drei bis vier Fakten als Text: Rust-Simulation mit 20 Hz, deterministischer Lockstep-Mehrspieler, Maße 1:1 mit Quellen, Karteneditor |
| Abspann | Titel, „in Entwicklung“, Link oder QR-Code, Kanal-Logo |

Mindestens sechs deutlich verschiedene Blickwinkel, mindestens zwei Karten, mindestens
eine Einstellung mit HUD (so sieht es der Spieler) und mehrere ohne. Kein Schnitt länger
als etwa 8 Sekunden ohne Bewegung im Bild.

Lege zu jeder Einstellung ein Standbild vor (einzelnes Bild über den Aufnahme-Modus aus
Schritt 2 oder ein vorhandenes Bildschirmfoto-Rezept), damit der Nutzer das Drehbuch
beurteilen kann.

### Freigabe-Gate

Zeige dem Nutzer Drehbuch und Standbilder und warte auf sein „Go“. Kläre dabei:

- Länge (Vorschlag nennen) und ob zusätzlich ein 60-Sekunden-Short in 9:16 entstehen soll.
- Ton: nur Spielton, oder Spielton mit Musik darunter. Die Kanalmusik des Video-Projekts
  ist ein Erklärvideo-Loop und passt vermutlich nicht; ohne andere Vorgabe nur Spielton.
- Wohin Link und QR-Code im Abspann zeigen (Repository, Webseite, nichts).
- Sprache der Bildschirmtexte (Vorschlag: Englisch, Beschreibung EN und DE).

### Schritt 2: Aufnahme-Modus im Spiel

Baue einen Aufnahme-Modus, der eine benannte Einstellung abspielt und beendet:

- Ein Skript in `godot/scenes/tools/` (das alte `demo_reel.gd` ersetzen oder ablösen,
  nicht zwei Wege pflegen) mit einer Liste von Einstellungen: Karte, Seed, Aufstellung,
  Befehle mit Zeitpunkt, Kamerafahrt (Wegpunkte mit Position, Höhe, Drehung, Neigung,
  weich interpoliert), Dauer, HUD an oder aus.
- Aufstellung und Befehle über vorhandene Wege: Szenario oder Missionsskript
  (`docs/MISSIONS.md`) und normale `Command`s. Wenn eine Seite von der KI geführt wird,
  mit festem Seed.
- Vorlauf ohne Aufnahme, damit das Gefecht zum gewünschten Zeitpunkt läuft: die Sim bis
  zum Starttick vorspulen, dann erst Bilder schreiben. Den passenden Seed und Starttick
  je Einstellung suchen, bis Treffer und Explosion im Bild liegen.
- Ein Rezept `just trailer-shots [--only=<name>,…]`, das je Einstellung mit
  `--write-movie` und `--fixed-fps 60` in `build/trailer/<name>.avi` schreibt (Bild und
  Ton zusammen), höchste Grafikeinstellungen, 1920 × 1080. Prüfe an der ersten
  Einstellung, dass Bild und Ton synchron und ohne Aussetzer sind; sonst als PNG-Folge
  plus WAV aufnehmen und mit ffmpeg zusammenfügen.
- Im Aufnahme-Modus aus: Randscrollen, Mauszeiger, Debug-Anzeigen, Menüs.
- Das Rezept in `CLAUDE.md` (Commands) und kurz in `docs/ARCHITECTURE.md` eintragen.
  `build/trailer/` bleibt außerhalb von Git.

`just check` und `just selftest` bleiben grün.

### Schritt 3: Aufnahmen ansehen und auswählen

Nimm alle Einstellungen auf, wandle sie mit ffmpeg in H.264-MP4 um und lege sie nach
`series/mud-and-steel-promo/assets/rec/16x9/`. Sieh dir aus jeder Aufnahme Standbilder
an (Anfang, Mitte, Höhepunkt) und prüfe: Explosion im Bild, keine Einheit läuft durch
die Kamera, kein Flimmern (siehe Prompt 068: betroffene Winkel meiden), Schatten und
Rauch vorhanden, Ton nicht übersteuert. Schwache Einstellungen mit anderem Seed,
Starttick oder Winkel neu aufnehmen.

### Schritt 4: Schnitt als Promo

Lege `series/mud-and-steel-promo/` nach dem Muster von `app-iconic-promo` an:
`promo.json` (Dauer, Formate, Lautheit −14 LUFS, Dateipräfix `mud-and-steel_`, Liste der
Aufnahmen), `build.mjs`, `promo.css`, `promo.js`, `SERIES.md`, `youtube.md`,
`tools/thumbs.mjs`.

- Look passend zum Spiel statt zum Kanal: gedeckte Erdfarben, Schablonenschrift oder
  schmale Grotesk für Titel, sparsame Texte. Schriften lokal mit freier Lizenz.
- Der Spielton der Aufnahmen trägt das Video; harte Schnitte auf Schuss und Explosion,
  weiche Übergänge nur zwischen Abschnitten. Musik nach der Entscheidung am Gate.
- Texte kurz und unten oder seitlich, nie über dem Geschehen. Kanal-Logo oben rechts.
- Bauen mit `npm run promo -- mud-and-steel-promo 16x9`. Vor dem Rendern Standbilder
  mit `npx hyperframes snapshot` prüfen (Lesbarkeit, Überlappungen).
- Thumbnail 1280 × 720 aus dem stärksten Standbild mit Titel, in `thumbnails/`.
- `youtube.md`: Titel, Beschreibung EN und DE, Kapitel aus dem Drehbuch, Tags,
  Namensnennungen für CC-BY-Sounds, Hinweis „in Entwicklung“.

Falls am Gate ein Short gewünscht wurde: eigener 60-Sekunden-Schnitt in 9:16 mit eigens
aufgenommenen Einstellungen im Hochformat (nicht das Querformat beschneiden).

### Schritt 5: Abnahme

Sieh das fertige Video in Standbildern alle 5 Sekunden durch und prüfe Dauer, Lautheit
und Bildrate mit ffprobe. Zeige dem Nutzer die Datei und warte auf Rückmeldung;
Änderungen an Text und Schnitt kosten nur einen neuen Render, neue Einstellungen einen
Lauf von `just trailer-shots --only=…`.

## 5. Fertig, wenn

- `just trailer-shots` jede Einstellung wiederholbar aufnimmt und `just check` sowie
  `just selftest` grün sind; der goldene Hash ist unverändert.
- `series/mud-and-steel-promo/dist/mud-and-steel_16x9.mp4` zwischen 1 und 4 Minuten
  lang ist, 1920 × 1080, 60 fps, −14 LUFS, und fahrende, schießende und explodierende
  Panzer aus mindestens sechs Blickwinkeln auf mindestens zwei Karten zeigt.
- Thumbnail, `youtube.md` und `SERIES.md` (Drehbuch, Aufnahme-Anleitung, Lizenzen der
  hörbaren Sounds) vorliegen.
- Der Nutzer das Video abgenommen hat. Hochladen und Veröffentlichen bleibt bei ihm.
- Beide Repositories getrennte Commits haben (kein Push); dieser Prompt liegt mit
  `git mv` in `prompts/done/`, die Zeile in `prompts/README.md` steht auf „erledigt“.
