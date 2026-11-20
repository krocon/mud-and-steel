# Prompt 096 – Kamera-Gizmo: Karte mit der Maus drehen und neigen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md` (Abschnitt „Camera“), `godot/scenes/camera/rts_camera.gd` ganz
> (`yaw`/`target_yaw`, `pitch()`, `pitch_near_deg`/`pitch_far_deg`, `pitch_curve`,
> `_apply`, `min_clearance`, `building_clearance`, `shadow_distance_factor`,
> `far_factor`), `godot/scenes/camera/ground_pick.gd`, `godot/scenes/hud/minimap.gd`
> (Sichtfeld als Trapez), `godot/scenes/hud/hud.gd` (`covers`), in
> `godot/scenes/menu/settings.gd` belegbare Tasten und gespeicherte Einstellungen,
> `godot/scenes/self_test.gd` Abschnitt „2b. Map controls“ und
> `godot/scenes/tools/trailer_shots.gd` (setzt Kamerawerte). Prüfe, ob Prompt **095**
> (HUD neu anordnen, Minimap unten links mit Knopfspalte) schon umgesetzt ist, und
> füge das Gizmo dort ein.

## 1. Ausgangslage

Der Nutzer möchte ein Navigations-Gizmo, wie es 3D-Programme (z. B. SolidCraft,
Blender) in einer Bildecke haben (Bild: kleiner dunkler Kreis mit drei farbigen
Achsenkugeln X/Y/Z und einem Haus-Knopf daneben). Damit soll man **die Karte mit
der Maus leicht drehen** und **den Blickwinkel einstellen** können, also wie steil
(fast von oben) oder flach (mehr zum Horizont) man schaut.

Heute:

- **Drehen** nur in 45°-Schritten mit Q/E, zurück nach Norden mit N
  (`target_yaw`, weich nachgezogen mit `rotate_smoothing`). Stufenloses Drehen gibt
  es nicht.
- **Neigung** ist fest an den Zoom gekoppelt: `pitch()` mischt zwischen
  `pitch_near_deg` (45°, nah) und `pitch_far_deg` (65°, fern) über `pitch_curve`.
  Der Spieler kann sie nicht ändern.
- Mittlere Maustaste ziehen und Alt + Maus verschieben die Karte.

## 2. Ziel

### 2.1 Das Gizmo

- Ein rundes, dunkles, leicht durchscheinendes Bedienelement (~64–80 px bei
  UI-Skala 1, wächst mit UI-Skala/HUD-Größe), bei der Minimap (nach 095 unten links
  neben bzw. über der Knopfspalte; ohne 095 neben der Minimap). Eigene Gestaltung,
  passend zum restlichen HUD; nichts aus SolidCraft übernehmen.
- Statt X/Y/Z zeigt es, was im RTS zählt: eine **Kompassrose** mit „N“ (hervorgehoben)
  und kleinen Marken für O/S/W. Sie dreht sich mit der Kamera, „N“ zeigt immer zur
  Kartennord-Richtung, und sie wird je nach Neigung zur Ellipse gestaucht (flach =
  schmale Ellipse, steil = fast Kreis). So sieht man Drehung und Neigung auf einen
  Blick.
- Daneben ein kleiner **Haus-Knopf**: Norden und Standard-Neigung wiederherstellen
  (weich).

### 2.2 Bedienung mit der Maus

- **Ziehen auf dem Gizmo:** waagerecht = stufenlos drehen, senkrecht = neigen
  (nach oben ziehen = steiler, nach unten = flacher; Richtung prüfen, was sich
  natürlich anfühlt). Beim Loslassen kein Nachschwingen, kein Einrasten.
- **Klick auf „N“, „O“, „S“, „W“:** weich in diese Himmelsrichtung drehen.
- **Doppelklick auf das Gizmo:** wie Haus-Knopf.
- **Ohne Gizmo, direkt auf der Karte:** zusätzlich eine Zieh-Geste zum Drehen/Neigen
  um die Bildmitte (Vorschlag: rechte Maustaste ziehen mit gedrückter Strg-/Ctrl-Taste
  oder mittlere Maustaste + Shift; vorher prüfen, was frei ist und mit Befehlen,
  Auswahlrahmen und Alt-Verschieben nicht kollidiert). Auf dem Trackpad: Drehgeste
  (`InputEventMagnifyGesture` gibt es schon für Zoom; prüfen, ob Godot 4 / godot-rust
  0.5.5 eine Rotationsgeste liefert, sonst weglassen).
- Mausrad über dem Gizmo: neigen statt zoomen.
- Q/E (45°) und N bleiben wie sie sind. Zusätzlich zwei belegbare Tasten für
  „steiler“/„flacher“ in Schritten (Vorschlag: R/F oder Bild↑/Bild↓, vorher Belegung
  prüfen), in `settings.gd` eintragen.
- Klicks und Ziehen auf dem Gizmo lösen keine Auswahl oder Befehle aus (`covers`),
  Knöpfe mit `focus_mode = FOCUS_NONE`.

### 2.3 Neigung in der Kamera

- Neigung wird ein **Versatz** zur bisherigen zoomabhängigen Kurve, kein Ersatz:
  `pitch() = Kurve(zoom) + tilt_offset`, geklemmt auf einen sinnvollen Bereich
  (Vorschlag gesamt 25°–85° über dem Horizont; flacher als ~25° zeigt zu viel
  Horizont und Himmel). So bleibt das bewährte Verhalten beim Zoomen erhalten, und
  Standard (Versatz 0) sieht aus wie heute.
- Weich nachziehen wie beim Drehen (`target_tilt`).
- Folgen auf das, was von der Neigung abhängt, prüfen und anpassen:
  - Bodenabstand (`min_clearance`, `building_clearance`): flach und nah darf die
    Kamera nicht in Gelände oder Gebäude tauchen.
  - Schattenreichweite und Sichtweite (`shadow_distance_factor`, `far_factor`): flach
    sieht man viel weiter; Ferne begrenzen bzw. Nebel nutzen, ohne dass Lücken am
    Kartenrand oder plötzlich auftauchende Objekte entstehen.
  - Minimap-Trapez, Bodenauswahl (`ground_pick.gd`), Randscrollen, Kartengrenzen
    (`bounds`), Nebel des Krieges, Gebäudedächer/Durchsicht, Höhenlinien des
    Geländes: mit flacher und steiler Neigung kontrollieren.
  - **Bildrate:** flach werden viel mehr Objekte sichtbar. Mit `just bench-view`
    (docs/PERFORMANCE.md) bei Standard, flachster und steilster Neigung auf der
    größten Karte messen; wenn es deutlich einbricht, den flachen Grenzwert anheben
    oder Sichtweite bei flacher Neigung begrenzen. Messwerte im Ergebnis nennen.
- Drehung und Neigung werden **nicht** in den Einstellungen gespeichert; jedes
  Gefecht startet nach Norden mit Standardneigung. Nur eine Einstellung
  „Gizmo anzeigen“ (Standard an) und ggf. „Neigung umkehren“ (Zieh-Richtung).
- Folgemodus (094) und Zentrieren funktionieren mit jeder Neigung; Drehen und Neigen
  beenden das Folgen nicht.
- Trailer-Aufnahmen (`trailer_shots.gd`) und Screenshot-Werkzeuge setzen den Versatz
  auf 0, damit sie sich nicht ändern.

## 3. Regeln und Technik

- Reine Darstellung und Bedienung: nichts in `crates/sim`, Golden Hash unverändert,
  keine neuen `Command`s. Godot-Floats sind hier erlaubt (nur Kamera/HUD).
- Das Gizmo zeichnet sich selbst (`_draw()` in einem `Control`), keine Bilddateien
  nötig. Texte (Tooltips, Einstellungsnamen, Himmelsrichtungen) in
  `godot/locale/hud.csv` bzw. `settings.csv`, deutsch und englisch (N/O/S/W bzw.
  N/E/S/W).
- Karteneditor (`just map-editor`) und Replay sollen das Gizmo ebenfalls haben, wenn
  sie dieselbe Kamera nutzen; prüfen.

## 4. Vorgehen

1. Kamera ansehen, im Spiel (`just run --skip-menu`) ausprobieren, welcher
   Neigungsbereich auf Wüsten-, Stadt- und Flusskarte gut aussieht und wo es bricht
   (Horizont, Kartenrand, Gelände). Ergebnis mit Bildern festhalten.
2. `tilt_offset`/`target_tilt` und stufenloses Drehen in `rts_camera.gd`, Klemmen,
   Bodenabstand, Sichtweite.
3. Gizmo-Control zeichnen und einhängen (Ziehen, Klick auf Richtungen, Haus-Knopf,
   Mausrad), Tasten, Zieh-Geste auf der Karte.
4. Minimap-Trapez, Bodenauswahl, Folgemodus, Trailer prüfen und ggf. anpassen.
5. Bildrate messen (Standard / flach / steil).
6. Tests in `self_test.gd` („Map controls“) ergänzen, nichts abschwächen:
   - Ziehen auf dem Gizmo ändert `yaw` stufenlos bzw. die Neigung; Grenzen werden
     eingehalten;
   - Klick auf „O“ dreht nach Osten, Haus-Knopf stellt Norden und Standardneigung her;
   - Tasten steiler/flacher wirken in Schritten und bleiben in den Grenzen;
   - nach Drehen/Neigen trifft die Bodenauswahl unter dem Mauszeiger weiter den
     richtigen Punkt (< 1 m);
   - Klick/Ziehen auf dem Gizmo erzeugt keinen Befehl und keine Auswahl;
   - Kamera bleibt bei flachster Neigung und kleinstem Zoom über Gelände und Dächern.
   `just selftest`, `just editor-selftest` und `just check` grün.
7. Doku: `docs/CONTROLS.md` (Gizmo, Zieh-Geste, neue Tasten),
   `docs/ARCHITECTURE.md` (Kamera: Neigung = Zoomkurve + Versatz),
   `docs/PERFORMANCE.md` (Messwerte, falls nennenswert).
8. Bilder für den Nutzer: Gizmo nah, dieselbe Szene steil, Standard und flach; er
   probiert es danach selbst mit `just run` aus.

## 5. Nicht Teil dieses Prompts

- Freie Kamera, Ich-Perspektive oder Verfolgerkamera.
- Rollen der Kamera (seitliches Kippen).
- Andere HUD-Umbauten (das macht 095).
