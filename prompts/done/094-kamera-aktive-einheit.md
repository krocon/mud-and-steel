# Prompt 094 – Kamera: aktive Einheit zentrieren und ihr automatisch folgen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md` (Abschnitt „Camera“), `godot/scenes/camera/rts_camera.gd`
> (`center_on`, `snap_to`, `panned_by_hand`, `_process`),
> `godot/scenes/selection/selection.gd` (`selected`, `_set_selection`, `_click`,
> `set_follow`, `_process`, `_center_camera_on_selection`, `recall_group`),
> `godot/scenes/hud/unit_panel.gd` (Einzelansicht, Kacheln, Knopf „Values“),
> `godot/scenes/hud/command_bar.gd`, `godot/scenes/menu/settings.gd` (belegbare
> Tasten, gespeicherte Einstellungen) und in `godot/scenes/self_test.gd` den
> Abschnitt „2b. Map controls“.

## 1. Ausgangslage

Der Nutzer steuert gern einen einzelnen Panzer (Bild: M4A3(75)W Sherman zwischen
Lehmhöfen auf der Wüstenkarte, Einheitenpanel unten rechts). Fährt der Panzer aus
dem Bild, muss er die Karte von Hand nachziehen. Er wünscht zwei Dinge:

1. **Zentrieren per Knopf.** Ein Mausklick auf einen Knopf schiebt die Karte so, dass
   die aktive Einheit (die zuletzt ausgewählte) in der Bildmitte steht.
2. **Automatisch folgen.** Eine Option (Umschaltknopf oder Häkchen), mit der die
   aktive Einheit ständig in der Bildmitte bleibt, die Karte scrollt also mit.

Vorhanden ist schon (Commit 0f24458):

- `rts_camera.gd`: `center_on(point)` gleitet weich hin, `snap_to(...)` springt,
  `panned_by_hand` meldet Verschieben von Hand (Tasten, Bildrand, Ziehen, Trackpad).
- `selection.gd`: **Folgemodus mit Taste C** (`set_follow`, `follow`). Er folgt der
  **Mitte der ganzen Auswahl**, endet bei Verschieben von Hand, bei leerer Auswahl
  und wenn keine Einheit mehr lebt. Einen Knopf dafür gibt es nicht.
- Gruppe zweimal schnell aufrufen (1…9) springt mit `_center_camera_on_selection`
  zur Mitte der Gruppe.
- Es gibt **keine „aktive Einheit“**: `selected` ist nach ID sortiert, die Reihenfolge
  der Auswahl geht verloren. Die Einheitenkarte (Shift + I) nimmt „die erste von
  mehreren“.

Zuerst im Spiel (`just run --skip-menu`) prüfen, wie sich C heute mit einem
fahrenden Panzer anfühlt (Nachziehen, Ruckeln, Verhalten beim Zoomen und Drehen),
und das Ergebnis kurz festhalten.

## 2. Ziel

### 2.1 Aktive Einheit

- `selection.gd` merkt sich eine **aktive Einheit** (`active_unit`, ID oder −1):
  - Klick auf eine Einheit, Klick auf eine Kachel im Einheitenpanel, Shift + Klick
    zum Hinzufügen: die angeklickte Einheit wird aktiv.
  - Auswahlrahmen, Doppelklick (alle dieses Typs), Gruppe aufrufen: die aktive
    Einheit bleibt, wenn sie in der neuen Auswahl ist; sonst die Einheit, die der
    Rahmenmitte bzw. Bildmitte am nächsten liegt (bei Gruppen: die erste der Gruppe).
  - Fällt die aktive Einheit aus der Auswahl (abgewählt, zerstört, eingestiegen),
    wird die nächstgelegene ausgewählte Einheit aktiv; ohne Auswahl −1.
- Das Einheitenpanel hebt die Kachel der aktiven Einheit sichtbar hervor (Rahmen),
  die Einzelansicht zeigt ohnehin die aktive Einheit. Die Einheitenkarte (Shift + I)
  nimmt künftig die aktive Einheit statt der ersten.
- Reine Darstellung: die aktive Einheit ist nur lokaler Zustand in Godot, kein
  `Command`, nichts in `crates/sim`.

### 2.2 Knopf „Zentrieren“

- Ein kleiner Knopf mit Symbol (Fadenkreuz o. Ä.) und Tooltip „Aktive Einheit
  zentrieren“ im Einheitenpanel, neben dem Knopf „Values“ an der Ecke des Bildes.
  Ein Klick lässt die Kamera mit `center_on` weich zur aktiven Einheit gleiten;
  Zoom und Drehung bleiben.
- Dazu eine belegbare Taste (in `settings.gd` eintragen). Vorher prüfen, welche
  Tasten frei sind (`docs/CONTROLS.md`, InputMap); Vorschlag: `X`. Der Knopf zeigt
  die Taste im Tooltip wie die Befehlsleiste.
- Ohne aktive Einheit ist der Knopf ausgegraut.

### 2.3 Umschalter „Folgen“

- Ein Umschaltknopf (eingerastet = an) neben dem Zentrieren-Knopf, Tooltip
  „Kamera folgt der aktiven Einheit“. Er ist der bestehende Folgemodus: Taste C und
  Knopf schalten dasselbe `follow`, der Knopf zeigt den Zustand immer richtig an
  (auch wenn C oder ein Ereignis ihn ändert).
- **Ziel des Folgens ist künftig die aktive Einheit**, nicht mehr die Mitte der
  Auswahl. Die Mitte der Auswahl als Ziel entfällt (sonst zwei fast gleiche Modi).
- Solange der Modus an ist:
  - Wechselt die Auswahl, folgt die Kamera der neuen aktiven Einheit (weich, kein
    Sprung). Ohne Auswahl wartet der Modus (Knopf bleibt an, Kamera steht) und
    greift wieder, sobald etwas ausgewählt wird.
  - Zoomen (Rad, Pinch) und Drehen (Q/E, N) bleiben möglich und beenden ihn nicht.
    Zoom zum Mauszeiger darf den Mittelpunkt dabei nicht wegschieben, solange gefolgt
    wird (zur Bildmitte zoomen).
  - Verschieben von Hand (Tasten, Bildrand, Ziehen, Trackpad, Minimap-Klick,
    Doppelklick auf Boden, Sprung zu Ereignis mit Leertaste) **beendet** den Modus
    wie heute, und der Knopf springt sichtbar heraus. Begründung: sonst kämpft der
    Spieler gegen die Kamera. Der Randscroll darf den Modus nicht aus Versehen
    beenden, wenn die Maus beim Klick auf den Knopf am Bildrand ruht: prüfen und ggf.
    erst nach einer kurzen Bewegung am Rand werten.
  - Stirbt die aktive Einheit und es bleibt keine Auswahl, endet der Modus.
- **Glatt folgen:** Die Kamera soll den fahrenden Panzer ohne Ruckeln und ohne
  sichtbares Hinterherhängen halten. Heute ruft `_process` jedes Bild `center_on`
  auf, das mit `glide_smoothing` nachzieht, also immer etwas hinterher ist. Beim
  Einschalten weich hingleiten, danach die interpolierte Position der Einheit
  (`current_positions()`) direkt als Fokus nehmen bzw. nur leicht glätten, so dass
  der Panzer höchstens ~0,5 m neben der Mitte steht. Kein Zittern durch den
  20-Hz-Takt der Simulation (die Darstellung interpoliert schon; prüfen).
- Der Zustand des Umschalters wird in den Einstellungen gespeichert und gilt im
  nächsten Gefecht wieder (Standard: aus). In Wiederholungen (Replay) und beim
  Zuschauen funktionieren Knopf und Modus genauso, soweit dort ausgewählt werden kann.

## 3. Regeln und Technik

- Reine Darstellung und Bedienung: nichts in `crates/sim`, Golden Hash unverändert,
  keine neuen `Command`s.
- Keine Godot-Physik für irgendetwas; Position kommt aus `SimHost`.
- Neue Texte (Tooltips, Einstellungsname der Taste) in `godot/locale/hud.csv` bzw.
  `settings.csv`, deutsch und englisch. Die Knöpfe wachsen mit der HUD-Skala wie der
  Rest des Einheitenpanels (siehe Prompt 092, falls schon umgesetzt).
- Knöpfe mit `focus_mode = FOCUS_NONE`, damit Tastatursteuerung nicht hängen bleibt.
- Klicks auf die Knöpfe dürfen keine Auswahl oder Befehle auf der Karte darunter
  auslösen.

## 4. Vorgehen

1. Verhalten von C heute prüfen und festhalten (siehe Ausgangslage).
2. Aktive Einheit in `selection.gd` einführen, Hervorhebung im Einheitenpanel,
   Einheitenkarte umstellen.
3. Zentrieren: Funktion in `selection.gd`, Knopf im Einheitenpanel, belegbare Taste.
4. Folgen: Ziel auf aktive Einheit umstellen, glattes Folgen, Zoom zur Bildmitte beim
   Folgen, Umschaltknopf mit Zustand, Speichern in den Einstellungen.
5. Tests in `self_test.gd` (Abschnitt „Map controls“) erweitern, nichts abschwächen:
   - Klick auf eine von zwei ausgewählten Einheiten macht sie aktiv; Auswahlrahmen
     behält die aktive Einheit; Abwählen wechselt sie.
   - Zentrieren-Knopf und Taste: nach dem Gleiten steht die aktive Einheit < 1 m von
     der Mitte (wie der Test für Doppelklick).
   - Folgen per Knopf: Knopf an, eine Einheit fahren lassen, über mehrere Sekunden
     bleibt sie < 1 m von der Mitte; Zoom und Drehen beenden das nicht; Verschieben
     von Hand beendet es und der Knopf ist aus; C und Knopf zeigen denselben Zustand.
   - Auswahlwechsel bei laufendem Folgen: Kamera wandert zur neuen aktiven Einheit.
   - Gespeicherter Zustand wird beim nächsten Gefecht übernommen
     (`l10n_self_test.gd` / Einstellungstest für „Standard wiederherstellen“ ergänzen).
   `just selftest` und `just check` grün.
6. Doku: `docs/CONTROLS.md` (Abschnitt Camera: Knopf, neue Taste, C folgt jetzt der
   aktiven Einheit; Abschnitt Selecting: was die aktive Einheit ist),
   `docs/ARCHITECTURE.md` kurz, falls dort die Kamera beschrieben ist.
7. Ein Bild des Einheitenpanels mit den beiden Knöpfen (an/aus) für den Nutzer; er
   probiert es danach selbst mit `just run` aus.

## 5. Nicht Teil dieses Prompts

- Andere Kameraarten (Verfolgerkamera hinter dem Panzer, Ich-Perspektive,
  automatisches Drehen mit der Fahrtrichtung).
- Automatisches Zoomen beim Folgen.
- Änderungen an Befehlen, Auswahlregeln oder der Simulation.
