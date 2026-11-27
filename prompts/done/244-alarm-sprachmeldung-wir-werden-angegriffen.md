# Prompt 244 – Sprachmeldung „Wir werden angegriffen!“ statt Alarmton (je Fraktion, je Frontabschnitt)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Abschnitte zu Ton: `ui_sound.gd`, Notices, Alarme
> außerhalb des Bildes), `docs/AUDIO_LIBRARY.md` (Zeilen „Repository
> visibility“ und „ElevenLabs“), `docs/SOURCES.md` (Abschnitt zu privaten
> Sounds), `../mud-and-steel-sfx/README.md` (Abschnitt „Voice lines“) sowie
> die erledigten Prompts
> `prompts/done/101-alarm-ausserhalb-des-bildes-und-auswahltasten.md`,
> `prompts/done/192-sirene-nur-einmal.md`, `prompts/done/053-sound-stufe2.md`
> und `prompts/done/069-sound-verbesserungen.md`.
> Im Code:
> `godot/scenes/hud/alerts.gd` (`report`, `spots`, `SPOT_RADIUS`,
> `SPOT_COOLDOWN`),
> `godot/scenes/world/supply_view.gd` (`_alarm`, `ATTACK_COOLDOWN`),
> `godot/scenes/world/air_view.gd` (`_warn`),
> `godot/scenes/audio/battle_sound.gd` (`notice`, `NOTICE_COOLDOWN`,
> `PLACE_GAP`),
> `godot/scenes/audio/ui_sound.gd` (`play`, `play_signal`),
> `godot/scenes/audio/sound_classes.gd` (`SIGNAL_CLASSES`),
> `godot/scenes/audio/sound_library.gd` (Schlüsselnamen `<class>@<key>`,
> `priv_*`),
> `godot/scenes/audio/battle_voices.gd` (`VOICE_FALLBACK`, `faction_of`),
> `godot/scenes/hud/subtitles.gd` (`NOTICES`),
> `godot/scenes/hud/unit_panel.gd` (`_faction_of_player`),
> `godot/assets/audio/sounds.json`,
> `tools/audio/synth.py` (`KEYED`, `prepare_keyed`, `PRIVATE_DIR`),
> `godot/scenes/tools/hud_self_test.gd` (Prompt-101-Teil),
> `godot/scenes/tools/sound_self_test.gd` (`_signals`),
> `web/sounds/usage.toml`.
>
> **Reine Darstellung:** `crates/sim` bleibt unverändert, die Golden Hashes
> ändern sich nicht.
> **Lizenz:** Die Sprachdateien sind ElevenLabs-Ausgaben und dürfen **nicht**
> ins öffentliche Repo (weder als `.mp3` noch als `.wav`). Sie bleiben im
> privaten Repo `../mud-and-steel-sfx` und landen nur als gitignorierte
> `priv_*.wav` unter `godot/assets/audio/`.

## 1. Problem

Wird eine eigene Einheit, ein eigenes Gebäude oder ein eigener
Versorgungspunkt außerhalb des Bildes angegriffen, spielt das Spiel heute den
Alarmton `ui_under_attack`:

- `alerts.gd report()` für Treffer auf Einheiten („fire“) und Gebäude
  („building“): je Ort (`SPOT_RADIUS = 60 m`) alle `SPOT_COOLDOWN = 10 s`,
  verschiedene Orte nur `PLACE_GAP = 1,5 s` auseinander.
- `supply_view.gd` für angegriffene Versorgungspunkte (je Punkt alle 20 s).
- `air_view.gd _warn()` für gesichtete Feindflieger.

In einem laufenden Gefecht ertönt der Ton dadurch ständig, er sagt nichts
über die Lage und klingt für jede Fraktion gleich. Der Nutzer hat stattdessen
drei Sprachmeldungen aufgenommen (Text laut
`../mud-and-steel-sfx/elevenlabs/we are under attack/info.txt`):

| Sprache | Text | Datei unter `../mud-and-steel-sfx/elevenlabs/we are under attack/` |
|---|---|---|
| de | „Wir werden angegriffen!“ | `de/ElevenLabs_2026-11-26T19_53_29_Marc_ivc_s50_sim75_v4.mp3` |
| en | „We're under attack!“ | `en/ElevenLabs_2026-11-26T19_52_07_Greg - Ghostly, Gritty, Brave and Raspy_pvc_s50_sim75_v4.mp3` |
| ru | „Нас атакуют!“ | `ru/ElevenLabs_2026-11-26T19_58_05_Greg - Ghostly, Gritty, Brave and Raspy_pvc_s50_sim75_v4.mp3` |

## 2. Ziel

Statt des Alarmtons spricht ein Melder **in der Sprache der gespielten
Fraktion** „Wir werden angegriffen!“ – selten und nur, wenn es wirklich etwas
Neues zu melden gibt: **höchstens einmal pro Minute je Frontabschnitt und nur
bei einem neuen Angriff.**

## 3. Umsetzung

### 3.1 Dateien und Aufbereitung

- Zuordnung Sprache → Fraktion: `de` → `de`, `ru` → `su`, `en` → `us` und
  `uk` (Großbritannien spricht wie bei den Gefechtsrufen mit der US-Stimme,
  vgl. `VOICE_FALLBACK`).
- Die Dateien werden wie alle privaten Sounds von `just audio` aufbereitet
  (Stille am Anfang weg, gleiche Lautheit für alle drei Sprachen) und als
  `priv_*.wav` abgelegt, Vorschlag:
  `godot/assets/audio/announce/<faction>/under_attack/priv_*.wav`.
- Vorschlag für die Quelle: `synth.py` liest die Meldungen **direkt** aus dem
  Ausgabeordner von `tools/voice_lines.py`
  (`<PRIVATE_DIR>/elevenlabs/<englische Zeile>/<de|en|ru>/*.mp3`) über eine
  kleine Tabelle Meldung → Ordnername (`under_attack` → `we are under
  attack`). So muss nichts kopiert werden, und jede weitere Aufnahme im
  selben Ordner wird automatisch eine Variante (zufällige Wahl wie bei jeder
  Klasse). Im privaten Repo liegt daneben schon `fall back/` – die Lösung
  soll weitere Meldungen ohne neuen Code aufnehmen können (nur ein
  Tabelleneintrag).
- Eigener Eintrag in `sounds.json` für Meldungen: **nicht ortsgebunden**,
  Bus `UI`, kein `pitch_spread`, Lautstärke so, dass die Meldung im Gefecht
  klar verständlich ist. Abspielen über `ui_sound.gd`, nicht über die
  ortsgebundenen Gefechtsrufe von `battle_voices.gd`.
- Die Meldung ist **kein Signal** im Sinn von Prompt 192 (kein Dauerton, der
  mit der Raketensirene schwebt): sie darf über der Sirene gesprochen
  werden. Es läuft aber nie mehr als eine Meldung gleichzeitig.
- `../mud-and-steel-sfx/SOURCES.md`: prüfen, ob die drei Dateien dort schon
  eine Zeile haben; sonst ergänzen (ElevenLabs, Stimme laut Dateiname, Text
  laut `info.txt`, Datum 2026-11-26, erzeugt mit `tools/voice_lines.py`).
  Im privaten Repo sonst nichts verschieben oder löschen; dort nur nach
  Rückfrage committen, nie pushen.
- `docs/SOURCES.md` / `docs/AUDIO_LIBRARY.md` im öffentlichen Repo: die
  Meldungen als private ElevenLabs-Sprachaufnahmen nennen (ohne Dateien).

### 3.2 Fraktion des Spielers

Die Fraktion des lokalen Spielers bestimmt die Sprache der Meldung – nicht
die Sprache der Oberfläche. `unit_panel.gd _faction_of_player()` ermittelt
sie bereits; an eine gemeinsame Stelle ziehen statt zu kopieren und das
Ergebnis je Gefecht merken.

### 3.3 Wann die Meldung spricht

Der Begriff „Frontabschnitt“ existiert im Code noch nicht. Festlegung (alles
in der Darstellung, Konstanten benannt und als `[provisional]` markiert):

- **Frontabschnitt:** Angriffsereignisse, die höchstens `SECTOR_RADIUS`
  (Vorschlag 250 m) vom ersten Ereignis des Abschnitts entfernt liegen,
  gehören zum selben Abschnitt – wie die `spots` in `alerts.gd`, nur
  größer. Ein Abschnitt verfällt, wenn dort `SECTOR_QUIET` lang nichts
  passiert ist.
- **Angriffsereignis:** Treffer auf eine eigene Einheit oder ein eigenes
  Gebäude, Verlust einer eigenen Einheit, Angriff auf einen eigenen
  Versorgungspunkt. **Jedes** Ereignis hält seinen Abschnitt „heiß“ – auch
  eines im Bild und auch eines, das selbst keinen Alarm auslöst.
- **Neuer Angriff:** ein Angriffsereignis in einem Abschnitt, in dem
  `SECTOR_QUIET = 60 s` lang keines war (oder der noch nicht existiert).
  Nur dieses erste Ereignis kann die Meldung auslösen; ein andauerndes
  Gefecht meldet sich nie ein zweites Mal, egal wie lange es dauert. Damit
  ist auch „höchstens einmal pro Minute je Abschnitt“ erfüllt.
- **Nur außerhalb des Bildes** (wie heute): Beginnt der Angriff im Bild,
  bleibt es still; der Abschnitt gilt trotzdem als heiß, sodass späteres
  Wegscrollen keine Meldung nachholt.
- **Über alle Abschnitte:** zwei Meldungen liegen mindestens
  `ANNOUNCE_GAP` (Vorschlag 8 s) auseinander. Eine Meldung, die in diese
  Lücke fällt, entfällt (nicht nachholen, keine Warteschlange).
- Verluste spielen weiter `ui_unit_lost` nach den bisherigen Regeln; sie
  lösen die Meldung nicht selbst aus, halten den Abschnitt aber heiß.

Alles Sichtbare bleibt, wie es ist: Feed-Zeilen, Minimap-Ping, Randpfeile,
`alarm_jump` und deren Abklingzeiten (`SPOT_COOLDOWN` usw.) ändern sich
nicht. Geändert wird nur, **was zu hören ist und wie oft**.

### 3.4 Was aus dem Alarmton wird

- Treffer auf Einheiten/Gebäude (`alerts.gd`) und angegriffene
  Versorgungspunkte (`supply_view.gd`): der Alarmton entfällt, es gilt
  allein die Regel aus 3.3.
- Hat die Fraktion keine Meldungsdateien (Klon ohne privates Repo, künftige
  Fraktion), spielt an derselben Stelle und **nach derselben Regel** der
  bisherige Ton `ui_under_attack` – also ebenfalls nur bei einem neuen
  Angriff je Abschnitt.
- Feindflieger (`air_view.gd _warn`) behalten den Alarmton: ein gesichtetes
  Flugzeug ist kein Angriff auf einen Frontabschnitt, und für „Feindflieger“
  gibt es noch keine Aufnahme.
- Untertitel (`SUB_UNDER_ATTACK`) erscheinen genau dann, wenn Meldung oder
  Ton tatsächlich spielt, weiterhin in der Sprache der Oberfläche.
- Wiederholungen und Zuschauer bekommen wie bisher keine Alarme.

## 4. Prüfen und zeigen

- **Selbsttests anpassen, nicht abschwächen:** Der Prompt-101-Teil von
  `hud_self_test.gd` und `_signals` in `sound_self_test.gd` prüfen heute
  den Alarmton je Ort. Sie auf die neue, gewollte Regel umstellen und im
  Commit benennen, was sich geändert hat. Die Prüfungen für Feed, Ping,
  Pfeile und Sprungtaste bleiben unverändert streng.
- **Neue Prüfungen** (mit Zählern wie `notices` / `played`, ohne echte
  Wartezeit – die Zeit im Test vorstellen):
  - zwei Treffer im selben Abschnitt kurz nacheinander → eine Meldung;
  - Dauergefecht (alle 5 s ein Treffer über 3 Minuten) → eine Meldung;
  - 60 s Ruhe, dann neuer Treffer im selben Abschnitt → zweite Meldung;
  - gleichzeitiger Angriff in einem weit entfernten Abschnitt → eigene
    Meldung, aber erst nach `ANNOUNCE_GAP`, sonst keine;
  - Angriff beginnt im Bild, Kamera schwenkt weg, Gefecht dauert an →
    keine Meldung;
  - Fraktionen `de`, `su`, `us`, `uk` wählen die Ordner de/su/us/us;
  - ohne Meldungsdateien spielt der Ton nach derselben Regel (der
    Selbsttest muss in einem Klon ohne privates Repo grün bleiben);
  - Feindflieger spielen weiter den Ton.
- **Hörprobe:** je Fraktion ein Testgefecht (`just run --skip-menu` bzw.
  `just battle …`), Angriff außerhalb des Bildes auslösen; Lautheit der drei
  Sprachen gegeneinander und gegen Gefechtslärm prüfen, dem Nutzer die drei
  aufbereiteten Dateien nennen.
- `just audio` (bzw. nur die neue Art) und `just import`; prüfen, dass
  `git status` keine der Sprachdateien zeigt (`priv_*` gitignoriert).
- `web/sounds/usage.toml`: Bemerkung für den neuen Ordner, Text von
  `ui_under_attack` an die neue Regel anpassen (`just check` prüft das).
- `just export-selftest`: die Meldungen sind im exportierten Spiel
  enthalten.
- `just check`, `just selftest` grün.
- `docs/ARCHITECTURE.md` (Ton-Tabellen, Alarme) und der Kopfkommentar von
  `alerts.gd` beschreiben die neue Regel.

## 5. Nicht Teil dieses Prompts

- Weitere Meldungen (z. B. „Rückzug!“ aus `fall back/`, Feindflieger,
  Einheit verloren, Punkt verloren).
- Eigene britische Stimme.
- Frontabschnitte als Spielregel oder in der KI (`crates/sim`, `crates/ai`).
- Änderungen an Feed, Minimap-Ping, Randpfeilen oder der Sprungtaste.

## Abschluss

Nach Abschluss diese Datei mit
`git mv prompts/244-alarm-sprachmeldung-wir-werden-angegriffen.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.

## Ergebnis

- **Aufbereitung:** `tools/audio/synth.py` hat die Art `announce`
  (`just audio announce`): Tabelle `ANNOUNCE` (Meldung → Ordner von
  `tools/voice_lines.py`, `under_attack` → `we are under attack`) und
  `ANNOUNCE_FACTIONS` (`de` → `de`, `en` → `us`, `ru` → `su`). Die Zeilen werden
  direkt aus `../mud-and-steel-sfx/elevenlabs/<Zeile>/<Sprache>/` gelesen,
  wie jeder Sound beschnitten und dann auf gleiche Lautheit gebracht (RMS der
  lauten 50-ms-Blöcke −19 dBFS, Spitze höchstens −1 dBFS; gemessen vorher
  −17,3 / −16,2 / −16,9 dBFS für de / en / ru). Ergebnis:
  `godot/assets/audio/announce/{de,us,su}/under_attack/priv_*.wav`
  (gitignoriert). Im privaten Repo wurde nichts geändert: die drei Dateien
  haben dort schon ihre Zeile in `SOURCES.md`.
- **Abspielen:** Manifest-Eintrag `announce` (Bus `UI`, nicht ortsgebunden,
  kein `pitch_spread`, −2 dB [provisional]); `ui_sound.gd speak` mit einem
  eigenen Spieler (nie zwei Meldungen gleichzeitig, kein Signal: spricht über
  der Raketensirene); `battle_sound.gd announce` wählt die Sprache der
  gespielten Fraktion (`HudTheme.player_faction`, aus `unit_panel.gd`
  herausgezogen; `uk` → `us`), ohne Aufnahme den Ton `ui_under_attack`, und
  zeigt den Untertitel nur, wenn etwas spielt.
- **Regel:** `alerts.gd attack` mit `SECTOR_RADIUS` 250 m, `SECTOR_QUIET` 60 s,
  `ANNOUNCE_GAP` 8 s (alle [provisional]). Treffer, Verluste und angegriffene
  Versorgungspunkte (`supply_view.gd`, jede Flanke, unabhängig von der
  Abklingzeit der Feed-Zeile) halten den Abschnitt heiß; nur das erste
  Ereignis eines Abschnitts außerhalb des Bildes meldet. Feed, Ping, Pfeile
  und Sprungtaste sind unverändert. Feindflieger behalten den Ton.
- **Selbsttests umgestellt:** Der Prompt-101-Teil von `hud_self_test.gd`
  prüfte „Treffer außerhalb des Bildes → Alarmton je Ort“; dieser Angriff
  beginnt im Bild, also prüft er jetzt, dass der ganze Abschnitt nichts
  meldet. `_signals` in `sound_self_test.gd` löst die drei Alarme über die
  Meldung einer Fraktion ohne Aufnahme aus (das ist der Ton) und prüft
  zusätzlich: Meldung über der Sirene, nie zwei Meldungen gleichzeitig. Neu:
  `_announce_test` (alle Fälle aus Abschnitt 4, einmal mit der Stimme der
  eigenen Fraktion, einmal als Fraktion ohne Aufnahme) und die
  Versorgungspunkt-Prüfung in `supply_point_self_test.gd`.
- **Geprüft:** `just selftest` grün mit den Aufnahmen und ohne den Ordner
  `announce/` (wie ein Klon ohne privates Repo); `just check` grün;
  `just export` und `just export-selftest` (pack) grün, der Selbsttest im
  exportierten Spiel findet die Aufnahmen für de, su, us und uk.
  **Nicht geprüft:** die
  Hörprobe im laufenden Gefecht (Lautheit gegen Gefechtslärm) – sie bleibt
  beim Nutzer; der Pegel steht in `sounds.json` (`announce`, `volume_db`).

### Nachtrag (2026-11-27): „Feindflieger gesichtet!“

Auf Wunsch des Nutzers bekommen auch Feindflieger eine Meldung statt des
Alarmtons (anders als in Abschnitt 3.4 geplant): Meldung `enemy_aircraft`,
erzeugt mit `tools/voice_lines.py "Feindflieger gesichtet!" --yes` im privaten
Repo (`elevenlabs/enemy aircraft spotted/`, en „Enemy aircraft spotted!“, ru
„Вижу вражеские самолёты!“). `air_view.gd _warn` ruft
`battle_sound.announce("enemy_aircraft")` (höchstens alle 15 s, `WARN_SECONDS`),
Untertitel `SUB_ENEMY_AIRCRAFT`. `ui_under_attack` ist damit nur noch der
Ersatzton für Fraktionen ohne Aufnahme. Wie man weitere Sätze erzeugt und
einbettet, steht in `CLAUDE.md` („Voice lines“).

### Nachtrag (2026-11-27): „Rückzug!“ und Release-Builds

- Meldung `retreat` („Rückzug!“ / „Fall back!“ / „Отступаем!“, Ordner
  `fall back/` des privaten Repos): Ein Rückzugsbefehl an Infanterie
  (`selection.retreat_ordered`) ruft `battle_sound.on_retreat_ordered`;
  gesprochen wird höchstens alle 4 s (`RETREAT_CALL_GAP`), dazwischen und
  ohne Aufnahme klickt der Befehl wie bisher (`ui_order`).
- Die alte Alarm-Aufnahme `priv_elevenlabs_under_attack_01.wav` ist auf
  Wunsch des Nutzers gelöscht (Spiel, privates Repo, Original-Download);
  `ui_under_attack` ist wieder der synthetische Ton.
- Release-Builds: `tools/release/check_private_sounds.py` bricht jeden
  Release-Export ohne die Sprachaufnahmen ab; der Release-Workflow holt das
  private Repo mit dem Secret `SFX_DEPLOY_KEY` (`docs/RELEASE.md`). Der
  Workflow selbst ist nicht gelaufen (das Secret legt der Nutzer an).
