# Prompt 165 – Verdeckte eigene Einheiten als farbiger Umriss sichtbar

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `docs/PERFORMANCE.md`, `docs/SCALE.md` („Sizes 1:1“),
> `prompts/164-eigene-einheiten-besser-sichtbar.md` bzw.
> `prompts/done/164-…` und `godot/scenes/world/unit_highlight.gd` (Hervorheben
> unter der Maus, Bodenmarken, Taste „alle zeigen“, Einstellung
> `unit_markers`; Punkt 5 dort war genau dieser Wunsch als „optional“),
> `godot/scenes/hud/relation_colors.gd` (Farben und Paletten für
> Farbsehschwächen), `godot/scenes/main.gd` (Einheitenknoten, `_tint`),
> `godot/scenes/world/infantry_view.gd` und `soldier_motion.gd` (wie Soldaten
> gezeichnet werden), `godot/scenes/world/map_objects.gd` (Häuser, Bäume),
> `godot/scenes/world/fog_of_war.gd`, `godot/scenes/selection/selection.gd`
> (Picking), `godot/scenes/world/graphics_quality.gd` und
> `godot/scenes/menu/settings.gd`.

## 1. Ausgangslage

Der Nutzer (2026-11-20) zeigt ein Bildschirmfoto aus einem anderen Spiel der
Gattung (nur als Beispiel für die Wirkung, nichts davon übernehmen) und
schreibt: **„Wie auf dem Bild eigene verdeckte Einheiten anzeigen (hier
blau).“** Auf dem Bild sind eigene Soldaten, Fahrzeuge und ein Geschütz, die
hinter Bäumen, Büschen und Hausdächern stehen, als **flache, kräftig blaue
Silhouette** durch das Hindernis hindurch zu sehen; die sichtbaren Teile der
Einheit sehen normal aus, nur der verdeckte Teil ist eingefärbt. Fremde
Einheiten bekommen das nicht.

Bei uns verschwinden eigene Einheiten bei der hohen Kamera hinter Häusern,
Baumkronen und Hügeln ganz; der Spieler sucht sie oder klickt ins Leere.
Prompt 164 hat Umrisse unter der Maus und Bodenmarken gebracht, aber auch die
sind hinter einem Haus verdeckt.

## 2. Ziel

Jeder Teil einer eigenen Einheit, der von etwas anderem verdeckt wird, ist als
gleichmäßige, halb deckende Silhouette in der eigenen Farbe zu sehen; der
sichtbare Teil bleibt unverändert. Der Spieler sieht auf einen Blick, wo seine
Leute hinter Häusern, im Wald und hinter Hängen stehen, und kann sie dort
auswählen.

## 3. Umsetzen (Vorschlag, Reihenfolge = Wichtigkeit)

1. **Technik klären und messen, bevor gebaut wird.** Gängige Wege in Godot 4
   (für die gepinnte Version nachsehen, `docs/VERSIONS.md`, und den Renderer
   in `project.godot` prüfen):
   - zweiter Material-Durchgang (`next_pass`) auf den Einheitenmodellen mit
     `depth_test_disabled`, `unshaded`, der die Tiefe der Szene
     (`hint_depth_texture`) mit der eigenen Tiefe vergleicht und nur dort
     zeichnet, wo etwas davor liegt;
   - oder eine eigene Ebene (Render-Layer) plus Nachbearbeitung
     (Compositor-Effekt / Bildschirm-Shader), die eine Einheiten-Maske mit
     der Szenentiefe vergleicht.
   Den einfacheren nehmen, der mit den Soldaten (Skelette bzw. MultiMesh,
   siehe `infantry_view.gd`), Fahrzeugen, Geschützen und Flugzeugen
   funktioniert. Ein Material pro Modellart, Farbe als
   Instanz-Shader-Parameter, keine neuen Materialien pro Bild.
2. **Was verdeckt:** Häuser, Ruinen, Bäume und Büsche, Gelände (Hänge),
   Brücken, andere Fahrzeuge. **Was nicht:** hohes Gras, Rauch, Staub,
   Wasseroberfläche und andere Effekte (sonst leuchtet ständig jeder Soldat im
   Gras); prüfen, ob das über die Tiefe (Durchsichtiges schreibt keine Tiefe)
   von selbst stimmt. Soldaten, die sich selbst oder ihr eigenes Fahrzeug
   verdecken, dürfen nicht flackern oder doppelt leuchten (kleiner
   Tiefenabstand, eigene Teile ausnehmen).
3. **Wer:** eigene Einheiten immer; verbündete in ihrer Farbe als Einstellung
   (Standard an), **feindliche nie** (auch nicht, wenn sie im Sichtbereich
   sind; kein Vorteil durch Wände). Einheiten im Nebel des Krieges oder
   unsichtbar für den Spieler bleiben unsichtbar. Soldaten **in** einem
   besetzten Gebäude oder Graben: nicht als Silhouette durch die Wände
   (die Besatzung zeigt das Gebäude-Symbol), nur wer dahinter oder daneben
   steht. Mitfahrende auf der Ladefläche wie das Fahrzeug.
4. **Aussehen:** flache Farbe aus `relation_colors.gd` (Paletten für
   Farbsehschwächen beachten), etwa 50–70 % deckend, ohne Licht und Schatten,
   optional ein etwas kräftigerer Rand. Ausgewählte Einheiten und die unter
   der Maus (164) etwas heller. Bodenmarken und Auswahlringe aus 164 hinter
   Häusern ebenfalls sichtbar machen, wenn das mit demselben Verfahren billig
   geht. Muss bei Tag, Nacht, Schnee und Regen lesbar sein und darf bei
   naher Kamera nicht stören (prüfen, ob es ab einer Zoomstufe schwächer
   werden soll; Bildvergleich).
5. **Auswählen:** Klick und Rahmen auf eine Silhouette wählen die Einheit
   aus, auch wenn ein Dach davor ist. Prüfen, wie `selection.gd` heute pickt
   (Bildschirmposition oder Strahl gegen Geometrie) und das Haus/den Baum
   nicht vor die Einheit stellen.
6. **Einstellung** im Menü (Aus / Nur eigene / Eigene und Verbündete),
   Standard „Eigene und Verbündete“; mit der Einstellung `unit_markers` aus
   164 abstimmen statt eine zweite, widersprüchliche anzulegen. Im Fotomodus
   bzw. bei ausgeblendetem HUD (Prompt 103, falls schon da) aus.
7. **Qualitätsstufen:** auf der niedrigsten Grafikstufe darf es einfacher
   sein (z. B. nur Fahrzeuge und Trupp-Marke statt jedes Soldaten), aber
   nicht ganz fehlen.

## 4. Regeln

- Reine Darstellung: nichts in `crates/sim` ändern, kein Einfluss auf Hashes.
- Keine vergrößerten Modelle, keine anderen Uniformfarben.
- Nichts übernehmen aus dem Vorbild außer der Idee; Farben nur aus
  `relation_colors.gd`.
- Bildrate mit `just bench-view` vorher/nachher messen; Ziel: höchstens
  0,5 ms pro Bild mehr in einem großen Gefecht (`docs/PERFORMANCE.md`). Wenn
  ein Weg teurer ist, den anderen nehmen oder auf Trupp-Ebene vereinfachen.
- Nichts doppelt zu 164 (Umriss unter der Maus, Bodenmarken, „alle zeigen“)
  bauen, sondern darauf aufbauen.

## 5. Abschluss

- Selbsttest erweitern: eigener Trupp hinter einem Haus → Silhouette an (z. B.
  Instanz-Parameter bzw. Masken-Pixel prüfen), feindlicher Trupp dahinter →
  keine; Einstellung „Aus“ → keine. `just check`, `just selftest`.
- Bilder vorher/nachher an denselben Stellen (Trupp hinter Haus, im Wald,
  hinter Hang, Fahrzeug hinter Baum; Tag und Nacht) in
  `build/occluded-units/` für den Nutzer, dazu die Messwerte der Bildrate.
- `docs/CONTROLS.md` (Einstellung) aktualisieren; kurzer Abschnitt in
  `docs/ARCHITECTURE.md`, wie die Silhouette gezeichnet wird.

## 6. Nicht Teil dieses Prompts

- Dächer oder Baumkronen ausblenden / durchsichtig machen (Ausschnitt-Ansicht).
- Gebietsgrenzen und Frontlinien (die blaue Linie auf dem Vorbild).
- Feindliche Einheiten durch Wände zeigen.
