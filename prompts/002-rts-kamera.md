# Prompt 002 – RTS-Kamera für ein großes Spielfeld

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md` und
> `docs/ARCHITECTURE.md`. Deren Regeln gelten unverändert.

## 1. Ziel

Das Spielfeld soll später mehrere Kilometer groß werden. Der Spieler muss sich darüber
frei bewegen und stufenlos zwischen Nahansicht (einzelne Fahrzeuge und Soldaten gut
erkennbar) und Übersicht (ein ganzes Dorf samt Umgebung) zoomen können.

Ziel-Optik laut Vorlagen: hoher Blickwinkel, leicht perspektivisch, kein flaches
Iso-Raster. Beim Rauszoomen wird der Blick steiler, beim Reinzoomen etwas flacher.

Die Kamera ist **reine Darstellung**. Sie berührt `crates/sim` nicht.

## 2. Umfang

1. **Kamera-Rig** als eigene Szene `godot/scenes/camera/rts_camera.tscn` mit Skript:
   - Ein Fokuspunkt auf dem Boden, um den sich die Kamera bewegt (Drehpunkt, Abstand,
     Neigung).
   - Perspektivische `Camera3D` mit kleinem Sichtfeld (Startwert ca. 30°). Vergleiche
     kurz mit der orthografischen Variante und begründe die Wahl im Abschlussbericht.
2. **Schwenken:** WASD und Pfeiltasten, Bildschirmrand-Scrollen (abschaltbar),
   Ziehen mit gedrückter mittlerer Maustaste. Die Geschwindigkeit skaliert mit der
   Zoomstufe, damit sich Schwenken auf jeder Höhe gleich anfühlt.
3. **Zoomen:** Mausrad, weich interpoliert, zum Mauszeiger hin (der Punkt unter dem
   Cursor bleibt unter dem Cursor). Grenzen: nah ca. 15 m, fern ca. 600 m Abstand
   (Werte als exportierte Parameter, leicht anpassbar).
4. **Neigung abhängig vom Zoom:** z. B. 45° nah bis 65° fern, über eine Kurve.
5. **Drehen:** Q/E in 45°-Schritten, weich animiert; Taste zum Zurücksetzen nach Norden.
6. **Kartengrenzen:** Der Fokuspunkt bleibt innerhalb eines Rechtecks. Vorerst eine
   Testfläche von 2 km × 2 km. Die Größe kommt aus einer Konstante, die Prompt 004
   durch die echte Kartengröße ersetzt.
7. **Testfläche:** Boden auf 2 km × 2 km vergrößern, Raster-Shader mit zwei Stufen
   (5-m- und 100-m-Linien), dazu einige Orientierungs-Objekte (Platzhalter-Boxen in
   verschiedenen Entfernungen), damit Zoom und Tiefe beurteilbar sind.
8. **Schatten und Weitsicht:** Die Schattendistanz und `far` der Kamera an die
   Zoomstufe anpassen, damit es in der Nähe scharf und in der Ferne schattig bleibt.
9. **Mausauswahl:** Der Rechtsklick auf den Boden funktioniert weiterhin. Die
   Strahl-Ebene-Berechnung kommt in eine wiederverwendbare Hilfsfunktion, die Prompt
   005 später gegen das Höhenfeld austauscht.
10. **Einstellungen:** Tastenbelegung über die Godot-Input-Map (`project.godot`), nicht
    hart codiert.

## 3. Nicht-Ziele

Minimap, Kamerafahrten, Höhenfeld (kommt in 004/005), Auswahl von Einheiten (003).

## 4. Akzeptanzkriterien

1. `just check` ist grün.
2. `just selftest` besteht weiterhin. Der Selbsttest prüft zusätzlich, dass die Kamera
   nach simuliertem Zoom-Input innerhalb der Grenzen bleibt (Abstand, Fokuspunkt).
3. Ein kurzer Bildvergleich mit `--write-movie` (Nahansicht, mittlere Stufe,
   Übersicht) liegt als PNG im Scratchpad und ist im Abschlussbericht beschrieben.
   Screenshots nicht ins Repo legen.
4. Beschreibe mir, wie ich die Steuerung selbst teste (Tasten, Maus).
5. `docs/ARCHITECTURE.md` beschreibt das Kamera-Rig kurz.

## 5. Arbeitsweise

- Mit mir Deutsch, im Repo Englisch. Kein `git push` ohne meine Freigabe.
- Erst kurz prüfen und einen Plan zeigen, wenn eine größere Entscheidung ansteht
  (z. B. perspektivisch oder orthografisch). Sonst direkt umsetzen.
- Godot-APIs für 4.7 nachschlagen, nicht raten. Kein Test wird abgeschwächt.
- Pro sinnvollem Schritt ein Commit. Am Ende diese Datei mit `git mv` nach
  `prompts/done/` verschieben, Status in `prompts/README.md` auf „erledigt“ setzen und
  committen.
- Abschlussbericht: was gebaut wurde, wie ich es teste, was offen ist.
