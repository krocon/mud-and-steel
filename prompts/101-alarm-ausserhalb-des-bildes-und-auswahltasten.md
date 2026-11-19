# Prompt 101 – Alarm außerhalb des Bildes und Tasten für die Auswahl

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `prompts/done/031-*.md`, `prompts/done/037-*.md`,
> `prompts/done/043-*.md`, `godot/scenes/hud/event_feed.gd` ganz (`COLORS`,
> `FIRE_SECONDS`, `PING_SECONDS`, `_add_ping`, `jump_to`, Aktion `event_jump`),
> `godot/scenes/hud/minimap.gd` (`add_ping`), `godot/scenes/hud/hud.gd`,
> `godot/scenes/audio/battle_sound.gd` (`notice`, `NOTICE_COOLDOWN`,
> `ui_under_attack`, `ui_unit_lost`), `godot/scenes/hud/subtitles.gd` (`NOTICES`),
> `godot/scenes/selection/selection.gd` (`_select_same_type`, `_on_key`,
> `recall_group`, `_own_indices`) und die Eingabe-Aktionen in `godot/project.godot`.

## 1. Ausgangslage

Im Test zum Referenzvideo (`docs/.vorlagen/vorlage.mp4`, nur als Anregung) nennt
der Tester zwei fehlende Standards: eine deutliche Meldung, wenn Truppen
**außerhalb des Blickfelds** Schaden nehmen oder fallen („Scharfschütze? Du hast
keinen Scharfschützen mehr.“), und eine Taste, die **die ganze Armee** auswählt.

Bei uns heute:

- Die Ereignisleiste meldet „unter Beschuss“ (höchstens alle 10 s), „verloren“,
  „gesichtet“, „verwundet“ und mehr; die Minimap pingt bei jedem Treffer; es gibt
  die Töne `ui_under_attack` und `ui_unit_lost`; Leertaste springt zum neuesten
  Ereignis.
- **Nichts davon weiß, ob das Ereignis im Bild ist.** Ein Treffer mitten im
  Gefecht vor der Kamera meldet genauso wie ein Überfall am anderen Kartenende.
  Dadurch gehen die wichtigen Meldungen unter. Es gibt keinen Hinweis am Bildrand.
- Auswahl: Doppelklick wählt gleiche Art im Bild, Gruppen 1–9. Es fehlen „ganze
  Armee“, „gleiche Art auf der ganzen Karte“ und „untätige Einheit“.

## 2. Ziel

### 2.1 Alarm nur, wenn man es nicht sieht

- Für jedes Ereignis „eigene Einheit getroffen“, „eigene Einheit verloren“,
  „eigenes Gebäude getroffen“ prüfen, ob der Ort im Kamerabild liegt (mit etwas
  Rand). **Im Bild:** kein Ton, keine Leistenmeldung für Treffer (Verluste melden
  weiter, aber ohne Alarmton). **Außerhalb:** volle Meldung.
- **Hinweis am Bildrand:** ein kleiner Pfeil am Rand in Richtung des Ereignisses,
  rot für Verlust, gelb für Beschuss, der einige Sekunden bleibt und beim Klick
  dorthin springt. Mehrere Ereignisse am selben Ort (Richtwert 60 m, wie
  `PING_RADIUS`) ergeben einen Pfeil, nicht zehn.
- **Verlust wichtiger Einheiten:** Verluste außerhalb des Bildes bleiben länger in
  der Leiste als andere Meldungen und werden nicht von „gesichtet“ oder „bereit“
  verdrängt.
- **Sprung:** Leertaste springt wie heute zum neuesten Ereignis. Zusätzlich eine
  belegbare Taste „zum letzten Alarm“, die nur Beschuss und Verluste durchgeht
  (mehrfach drücken = ältere).
- Gebäude unter Beschuss als eigene Meldung, getrennt von Einheiten.
- Abklingzeiten je **Ort** statt nur global: Ein zweiter Überfall an anderer Stelle
  meldet sofort, auch wenn der erste erst 3 s her ist.

### 2.2 Auswahltasten

- **Ganze Armee:** alle eigenen Kampfeinheiten auf der Karte. Ohne Gebäude, ohne
  unbewaffnete Versorgungs- und Zugfahrzeuge (Regel aus den vorhandenen Rollen
  ableiten und nennen).
- **Gleiche Art überall:** Strg/Cmd + Doppelklick wählt alle eigenen Einheiten
  dieser Art auf der ganzen Karte.
- **Untätige Einheit:** Taste springt zur nächsten eigenen Einheit ohne Befehl und
  wählt sie aus; mit Umschalt alle untätigen. Was „untätig“ heißt, an den
  vorhandenen Befehlszuständen festmachen (steht ohne Auftrag, feuert nicht, lädt
  nicht auf, baut nicht).
- Alle drei als belegbare Aktionen in `project.godot`, in den Einstellungen
  (Reiter Controls) und in `docs/CONTROLS.md`. Freie Tasten wählen, die mit nichts
  kollidieren; Vorschlag im Ergebnis nennen.

## 3. Regeln

- Reine Darstellung und Bedienung; `crates/sim` und der Golden Hash bleiben.
- Mehrspieler: Alarm und Auswahl betreffen nur eigene Einheiten; Verbündete
  höchstens über die vorhandenen Pings.
- Wiederholung und Zuschauer: keine Alarme.
- Farben über `relation_colors.gd` bzw. die vorhandenen Meldungsfarben, auch für
  die Farbschwäche-Paletten.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Bestand lesen; Tabelle der heutigen Meldungen mit „im Bild / außerhalb“ und dem
   neuen Verhalten.
2. Sichtbarkeitsprüfung (Kamerabild als Bodenviereck; `minimap.gd _update_view`
   rechnet es schon) einmal bauen und für Leiste, Ton, Ping und Pfeil benutzen.
3. Randpfeile, längere Verlustmeldungen, Sprungtaste.
4. Auswahltasten.
5. Selftest: Treffer im Bild → kein Alarmton, kein Pfeil; Treffer außerhalb →
   Pfeil in der richtigen Richtung, Klick springt hin; zwei Orte kurz
   nacheinander → zwei Alarme; „ganze Armee“ enthält kein Gebäude und keinen
   Lkw; „untätig“ findet eine stehende Einheit und keine fahrende.
6. `just check`, `just selftest`, `just lan-selftest`.
7. Doku: `docs/CONTROLS.md`, `docs/ARCHITECTURE.md`.
8. Für den Nutzer: kurzes Bild oder Clip mit Randpfeil, Liste der neuen Tasten.

## 5. Nicht Teil dieses Prompts

- Sprachmeldungen der Einheiten (die Stimmen fehlen noch, siehe
  `battle_voices.gd`).
- Gruppenreiter am Bildschirm (Prompt 095).
- Einzelne Mitfahrer auswählen oder absetzen (Prompt 109).
