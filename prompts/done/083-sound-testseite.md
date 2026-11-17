# Prompt 083 – Webseite zum Testen aller Klänge

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ARCHITECTURE.md` den Abschnitt „Audio (presentation only)“ mit allen
> Unterabschnitten (auch Stufe 2, Prompt 053, und Musik/Ambiente, Prompt 030), in
> `docs/ASSETS.md` den Abschnitt „Audio (prompt 029)“, in `docs/AUDIO_LIBRARY.md` die
> „Key decisions“ (private Klänge), dann `godot/assets/audio/sounds.json` und die
> Skripte in `godot/scenes/audio/` ganz. Als Muster für eine Seite ohne Build-Schritt:
> `web/units/README.md` und `tools/units_web.py`.
>
> Prompt 082 kann parallel im selben Checkout laufen und hat `justfile` und
> `docs/ARCHITECTURE.md` in Arbeit: dort nur ergänzen, nichts umsortieren.

## 1. Ziel

Eine Seite im Projekt, auf der ich jeden Klang des Spiels im Browser anhören kann.
Neben jedem Klangnamen steht eine kurze Bemerkung, bei welcher Gelegenheit das Spiel
ihn spielt.

Fertig ist der Prompt, wenn

- `just sounds-web` die Seite unter `http://localhost:<port>/web/sounds/` öffnet,
- dort jede Audiodatei unter `godot/assets/audio/` genau einmal erscheint und sich
  abspielen lässt (heute 165 Dateien im Repo, dazu lokal 6 private `priv_*`),
- jede Klasse, jeder Waffen-Ordner, jede Ambiente-Schicht und jede Musik-Stimmung
  eine Bemerkung hat, die aus dem Code belegt ist (Abschnitt 4),
- `just sounds-web --check` fehlende und verwaiste Bemerkungen meldet und in
  `just check` läuft,
- `just check` grün ist. Die Simulation und das Spiel werden nicht angefasst
  (Golden Hash unverändert, kein `just selftest` nötig).

## 2. Was es heute gibt

- `godot/assets/audio/sounds.json`: 46 Klassen mit Gruppe, Bus, Pegel, Reichweite,
  Priorität, `loop`. Jede Klasse hat einen Ordner `godot/assets/audio/<klasse>/`.
- Nicht im Manifest: `weapon/<slug>/` (Aufnahmen einzelner Waffen, gespielt als
  `shot_…@<slug>`), `ambience/<schicht>/` (12 Schichten, `ambience.gd`), `music/`
  (7 Stücke, Stimmung im Dateinamen, `music.gd` / `match_music.gd`), später
  `voice/<fraktion>/<ruf>/` (heute leer, `battle_voices.gd`).
- Herkunft am Dateinamen: `<klasse>_NN` = synthetisiert, `ext_*` = freie Aufnahme,
  `priv_*` = aus dem privaten Repository (gitignored). Liegt in einem Ordner eine
  `priv_*`-Datei, spielt das Spiel **nur** diese (`sound_library.gd`).
- `just sound-gallery` spielt jede Klasse im Spiel ab, braucht aber Godot und zeigt
  weder einzelne Dateien noch die Verwendung. Es bleibt, wie es ist.

## 3. Die Seite

Ort `web/sounds/` (`index.html`, `style.css`, `app.js`, `README.md`), ohne
Build-Schritt und ohne neue Abhängigkeiten. Texte auf Deutsch wie die
Einheiten-Seite; Schrift und Farben von dort übernehmen (Dateien aus
`web/units/vendor/` einbinden, nicht kopieren). Es ist ein Werkzeug: schlicht, dicht,
schnell zu überfliegen, kein Hero, keine Animation.

**Aufbau.** Abschnitte in dieser Reihenfolge, jeweils eine Zeile je Klang:
Schüsse · Waffenaufnahmen · Überschall und Nachhall · Einschläge und Treffer ·
Explosionen und Feuer · Fahrzeuge und Flugzeuge · Gebäude und Objekte ·
Meldungen und Oberfläche · Stimmen (nur wenn Dateien da sind) · Ambiente · Musik.
Die Zuordnung der Klassen zu den Abschnitten legst du nach Gruppe und Name fest.

**Eine Zeile zeigt**

- den Namen (`shot_cannon_medium`, bei Waffen `weapon/mg_42`) und daneben die
  Bemerkung,
- je Datei einen Knopf zum Abspielen mit Länge in Sekunden und Herkunft
  (synthetisiert / Aufnahme / privat); Dateien, die das Spiel wegen einer
  `priv_*`-Datei im selben Ordner nicht spielt, sind als „im Spiel verdrängt“
  markiert, aber abspielbar,
- klein die Werte aus dem Manifest: Gruppe, Bus, Pegel, Reichweite, Priorität,
- „alle nacheinander“ für die Varianten; Schleifen-Klassen (`loop`) laufen als
  Schleife, bis man sie stoppt.

**Bedienung.** Suchfeld (filtert nach Name und Bemerkung), „Alles stoppen“,
Lautstärkeregler, Schalter „wie im Spiel“ (Pegel `volume_db` der Klasse und zufällige
Tonhöhe `pitch_spread` anwenden; aus = Datei unverändert). `#<name>` in der Adresse
springt zur Zeile. Was gerade spielt, ist sichtbar hervorgehoben. Auswahl von
Lautstärke und Schalter in `localStorage`.

Nicht nachbauen: Entfernung, Tiefpass, Stimmenbudget, Nebel des Krieges. Dafür gibt
es `just sound-gallery` und das Spiel; die Seite verweist darauf in einem Satz.

## 4. Die Bemerkungen

- Eine Datei `web/sounds/usage.toml`, von Hand gepflegt, Deutsch: ein Eintrag je
  Klasse, je Waffen-Ordner, je Ambiente-Schicht, je Musik-Stimmung (und je Ruf,
  sobald es Stimmen gibt). Ein Satz, höchstens etwa 120 Zeichen, der den Anlass
  nennt, nicht den Klang beschreibt. Beispiel für `ui_under_attack`: „Eigene
  Einheit wird getroffen, höchstens alle 20 Sekunden.“
- **Aus dem Code, nicht geraten.** Quelle sind `sound_classes.gd` (Schuss nach
  Kategorie, Kaliber, Kadenz; Motor nach Masse; Treffer nach Oberfläche),
  `battle_sound.gd`, `ui_sound.gd`, `battle_voices.gd`, `ambience.gd`, `music.gd`,
  `match_music.gd` und die Stellen, die Luft- und Artillerie-Ereignisse vertonen.
  Die Tabelle „Events and sounds“ in `docs/ARCHITECTURE.md` hilft, ist aber älter
  als manche Klasse; bei Widerspruch gilt der Code, und die Tabelle wird berichtigt.
- Schwellen gehören in die Bemerkung, wenn sie den Klang auswählen („Kanonen ab
  75 mm“, „Kettenfahrzeuge ab 20 t“, „Kadenz ab 300 Schuss/min“).
- Bei Waffenaufnahmen steht, welche Waffe des Katalogs sie spielt und welche Klasse
  sie ersetzt.
- Spielt der Code eine Klasse nirgends, lautet die Bemerkung „Derzeit ungenutzt“,
  und die Klasse steht im Abschlussbericht. Nichts löschen.

## 5. Werkzeug

`tools/sounds_web.py`, nur Python-Standardbibliothek, gebaut wie
`tools/units_web.py`:

- Schreibt beim Start `web/sounds/index.json` (gitignored): alle Dateien mit Pfad,
  Länge (WAV über `wave`; MP3-Länge darf fehlen, dann liest sie die Seite beim
  Laden), Herkunft, „verdrängt“, dazu die Manifestwerte und die Bemerkungen. Die
  Seite lädt nur diese eine Datei und die Audiodateien direkt aus
  `godot/assets/audio/`.
- Dient das Repo-Wurzelverzeichnis aus und öffnet den Browser. `--port N` (Standard
  8138, damit es neben `just units-web` läuft), `--no-open`.
- `--check`: Fehler mit Rückgabewert 1, wenn ein Ordner mit Audiodateien keine
  Bemerkung hat, eine Bemerkung keinen Ordner, oder eine Manifest-Klasse keinen
  Ordner. Der Check hängt nur an Ordner- und Dateinamen, damit er auch ohne Git LFS
  und ohne das private Repository läuft; `priv_*` zählt nicht mit.
- `justfile`: Rezept `sounds-web *args` in der Gruppe `sound`; `--check` in
  `just check` und in die CI aufnehmen wie `gen-geo-check`.

## 6. Grenzen

- **Nur lokal.** Die Seite wird nicht hochgeladen und bekommt kein Gegenstück zu
  `just units-web-sync`: Es werden keine Audiodateien nach `web/` kopiert. Die
  `priv_*`-Klänge dürfen nicht als einzelne Dateien weitergegeben werden
  (`docs/AUDIO_LIBRARY.md`); eine Kopie unter `web/` wäre ein Schritt dorthin. Das
  steht so in `web/sounds/README.md`.
- Keine Änderung an Klängen, am Manifest, an der Zuordnung im Spiel oder an
  `tools/audio/`. Fällt beim Lesen ein Fehler auf (Klasse ohne Aufrufer, falsche
  Schwelle in der Doku), wird er berichtet, nicht nebenbei behoben; nur die
  Doku-Tabelle wird berichtigt.
- Keine neuen Pakete, kein npm, kein Framework.

## 7. Prüfen

- Seite im Browser öffnen (eingebauter Browser der Session): keine Fehler in der
  Konsole, keine fehlgeschlagene Anfrage. Per Skript in der Seite jede Datei aus
  `index.json` einmal laden und dekodieren; die Zahl der Zeilen und Knöpfe mit dem
  Dateibestand vergleichen.
- Von Hand je Abschnitt einen Klang abspielen, dazu eine Schleife starten und
  stoppen, „alle nacheinander“, Suche, Schalter „wie im Spiel“, Sprung per `#name`.
- Schmales Fenster (Telefonbreite) und dunkles Farbschema ansehen.
- `just sounds-web --check` einmal mit einer absichtlich entfernten Bemerkung
  laufen lassen (muss fehlschlagen), dann wiederherstellen.
- Ein Bildschirmfoto der fertigen Seite zeigen.

## 8. Doku und Abschluss

- `web/sounds/README.md` (Englisch): Zweck, Start, Dateien, wie man eine Bemerkung
  ergänzt, warum die Seite lokal bleibt.
- `docs/ARCHITECTURE.md`, Abschnitt Audio → „Tools“: zwei Sätze zur Seite.
  `docs/ASSETS.md`, Abschnitt Audio, Schritt „neue Klasse“: Bemerkung in
  `usage.toml` ergänzen. `CLAUDE.md`: `just sounds-web` in die Befehlsliste und
  `web/sounds/` ins Layout.
- Abschlussbericht: Zahl der Klänge je Abschnitt, ungenutzte Klassen, berichtigte
  Stellen der Doku, was offen bleibt.
- Commit erst nach `just check`; dieser Prompt per `git mv` nach `prompts/done/`,
  Zeile in `prompts/README.md` auf „erledigt“. Kein `git push`.
