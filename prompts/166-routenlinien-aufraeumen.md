# Prompt 166 – Routenlinien der Auswahl aufräumen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md` (Wegpunkte, Shift + Rechtsklick, Abschnitt zu den Linien),
> `godot/scenes/hud/waypoints.gd` (zeichnet die Linien), `godot/scenes/hud/hud.gd`
> (`waypoints`, Zeitmessung `usec`), `godot/scenes/selection/selection.gd`
> (Auswahl, Befehle), `godot/scenes/menu/settings.gd` und
> `godot/scenes/menu/settings_panel.gd`, `godot/scenes/hud/relation_colors.gd`,
> `godot/scenes/debug/path_debug.gd` (F5 zeigt alle Pfade weiterhin für die
> Fehlersuche) sowie die offenen Prompts `099` (Zielvorschau), `103`
> (HUD ausblenden) und `164` (Hervorheben unter der Maus), damit nichts doppelt
> gebaut wird.

## 1. Ausgangslage

Der Nutzer (Bildschirmfoto 2026-11-20, 12 Einheiten ausgewählt: 2 Opel Blitz,
10 Pz IV H, Wiese von oben): **„Die vielen grünen Linien sind störend.“**

Auf dem Bild ziehen sich ein gutes Dutzend hellgrüne Linien quer über den
ganzen Bildschirm: Jede ausgewählte Einheit zeigt ihre volle restliche Route
(`waypoints.gd`, `ROUTE_COLOR`, 2 px, Alpha 0,55) von der Einheit über jeden
Pfadknick bis zum Ziel. Die Routen laufen erst auseinander, bündeln sich im
Bild rechts zu einem Fächer, knicken mehrfach ab und reichen über den
Bildrand hinaus; Linien von Einheiten außerhalb des Bildes kreuzen das Bild
ebenfalls. Bis zu `MAX_UNITS = 48` Routen werden so gezeichnet, jeden Frame,
solange die Auswahl besteht.

Was die Linien eigentlich zeigen sollen: wohin die Auswahl fährt und welche
Wegpunkte in der Kette stehen (Prompt 031). Der genaue Pfad jeder einzelnen
Einheit ist dafür nicht nötig.

## 2. Ziel

Nach einem Befehl sieht der Spieler auf einen Blick, wohin seine Auswahl
fährt (Ziel und Wegpunktkette), ohne dass Linien das Bild durchziehen. Die
volle Route einzelner Einheiten gibt es nur, wenn er sie sehen will.

## 3. Umsetzen (Vorschlag, Reihenfolge = Wichtigkeit)

1. **Gruppe statt Einzelrouten**: Einheiten mit demselben Befehl (gleiche
   Ziel- bzw. Wegpunktkette, Formationsziele zählen als ein Ziel) bekommen
   **eine** gemeinsame Darstellung: Zielmarke(n) und Wegpunktkette einmal,
   dazu eine Linie vom Schwerpunkt der Gruppe (oder vom vordersten Fahrzeug)
   zum ersten Wegpunkt bzw. Ziel. Formationsziele als kleine Punkte am Ziel,
   nicht als eigene Linien. Gruppieren nach Befehl, nicht nach Nähe.
2. **Nur kurz nach dem Befehl voll zeigen**: Routen erscheinen nach einem
   Befehl für etwa 2–3 s deutlich und blenden dann auf eine sehr schwache
   Linie oder nur die Zielmarke herunter. Wieder voll bei: neuem Befehl,
   gehaltener Shift-Taste (Wegpunkte setzen), Maus über einer ausgewählten
   Einheit (nur deren Route) oder einer Taste „Routen zeigen“ (vorher
   `docs/CONTROLS.md` auf freie Tasten prüfen).
3. **Route nicht als Pfad mit allen Knicken**: Für die Übersicht reicht die
   Gerade bzw. die Wegpunktkette; die genaue Pfadlinie mit allen Knicken nur
   für eine einzelne ausgewählte Einheit oder unter der Maus. Dünner (1–1,5 px),
   zum Ziel hin ausblendend oder gestrichelt, damit sie nicht wie Gelände
   (Straßen, Grenzen) wirkt.
4. **Bildrand**: Linien von Einheiten außerhalb des Bildes nicht quer über den
   Schirm ziehen; höchstens ein kurzer Ansatz am Rand oder gar nichts.
   Abschnitte hinter der Kamera werden heute schon verworfen, das bleibt.
5. **Einstellung** im Menü „Routenlinien“: Aus / Nur Ziele / Kurz nach Befehl
   (Standard) / Immer (heutiges Verhalten). Farben (Route, Angriff in
   Bewegung, Patrouille, gesicherter Bereich, Warteschlange) bleiben in ihrer
   Bedeutung; Paletten für Farbsehschwächen beachten, falls `relation_colors.gd`
   dafür etwas anbietet.

Die Debug-Ansicht F5 (`path_debug.gd`) bleibt unverändert und zeigt weiter
alle Pfade.

Erst ansehen, dann entscheiden: Bilder von heute und von zwei bis drei
Varianten an derselben Stelle (12 Einheiten wie im Bildschirmfoto, dazu
ein Trupp mit Wegpunktkette, eine Patrouille, ein Angriff in Bewegung) in
`build/route-lines/` legen und im Abschlussbericht die gewählte Variante
begründen.

## 4. Regeln

- Reine Darstellung: nichts in `crates/sim` ändern, kein Einfluss auf Hashes.
  Gebraucht werden nur vorhandene Abfragen (`unit_waypoints`,
  `unit_queue_length`, `unit_patrols`, `unit_drill`); fehlt etwas
  (z. B. eine Befehlskennung zum Gruppieren), dann in `crates/bridge` aus
  vorhandenen Sim-Daten lesen, nicht neu rechnen.
- Zeichnen bleibt billig: Zeit von `waypoints.usec` vorher/nachher bei 48
  ausgewählten Einheiten messen, nicht langsamer als heute.
- Nichts doppelt zu 099, 103 und 164; HUD-Ausblenden (103) blendet auch die
  Routen aus.

## 5. Abschluss

- Selbsttest anpassen bzw. erweitern (`markers`, `secured` bleiben prüfbar):
  12 Einheiten mit einem Befehl → eine Gruppenlinie statt 12; Einstellung
  „Aus“ → keine Linien; Wegpunktkette und Patrouille weiter sichtbar.
  `just check`, `just selftest`.
- Bilder vorher/nachher in `build/route-lines/` für den Nutzer, dazu die
  Messwerte.
- `docs/CONTROLS.md` (Linien, neue Taste, Einstellung) aktualisieren.
- Prompt nach `prompts/done/` verschieben und `prompts/README.md` nachtragen.

## 6. Nicht Teil dieses Prompts

- Zielvorschau und Reichweitenkreise (Prompt 099).
- Änderungen an Wegfindung oder Formationen in der Simulation.
