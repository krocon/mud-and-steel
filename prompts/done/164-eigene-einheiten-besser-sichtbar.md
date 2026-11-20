# Prompt 164 – Eigene Einheiten besser sichtbar: Hervorheben unter der Maus

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `docs/ASSETS.md`, `docs/SCALE.md` („Sizes 1:1“: Lesbarkeit
> soll aus Markierungen, Symbolen und Umrissen kommen, nicht aus vergrößerten
> Modellen), `godot/scenes/hud/relation_colors.gd` (Farben eigen / verbündet /
> feindlich, Paletten für Farbsehschwächen), `godot/scenes/main.gd` (Einheiten-
> knoten, `SelectionRing`, `_tint` für fremde Einheiten),
> `godot/scenes/selection/selection.gd` (Auswahl, Picking, `_selectable_index`),
> `godot/scenes/world/combat_view.gd` (`HealthBars`), die Soldaten-Darstellung
> in `godot/scenes/world/` (Infanterie), `godot/scenes/menu/settings.gd` und
> die offenen Prompts `099` (Zielvorschau, Reichweitenkreise), `101` (Alarm,
> Auswahltasten) und `103` (HUD ausblenden, Fotomodus), damit nichts doppelt
> gebaut wird.

## 1. Ausgangslage

Der Nutzer (Bildschirmfoto 2026-11-20, mittlere Zoomstufe von oben, Wiese mit
hellen und dunklen Flecken): **„Meine eigenen Soldaten sind schlecht zu
sehen. Was kann man da machen? Highlighten, wenn ich mit der Maus in der Nähe
bin?“** Auf dem Bild sind die Soldaten kleine dunkelgraue Figuren auf grünem
Gras; ohne Auswahl ist nicht zu erkennen, dass es eigene sind. Fremde Einheiten
werden heute eingefärbt (`_tint`), eigene nicht; der Auswahlring erscheint
nur bei Auswahl.

Größen bleiben 1:1 (Soldat 1,80 m). Lösungen nur in der Darstellung.

## 2. Ziel

Eigene Einheiten, vor allem Infanterie, sind auf jedem Untergrund und jeder
üblichen Zoomstufe auf einen Blick zu finden und als eigene zu erkennen, ohne
das Bild zuzukleistern.

## 3. Umsetzen (Vorschlag, Reihenfolge = Wichtigkeit)

1. **Hervorheben unter der Maus** (Wunsch des Nutzers): Einheiten in einem
   Radius um den Mauszeiger auf dem Boden (Startwert 25 m, mit der Zoomstufe
   wachsend) bekommen einen leichten Umriss bzw. Aufhellung in der
   Beziehungsfarbe und ihren Ring, sanft ein- und ausgeblendet; die Einheit
   direkt unter dem Zeiger stärker. Gilt für eigene, verbündete und feindliche
   Einheiten (jeweils in ihrer Farbe). Ein Trupp wird als Ganzes hervorgehoben,
   nicht Mann für Mann.
2. **Dauerhafte, dezente Bodenmarke für eigene Einheiten**: ein schwacher,
   flacher Ring oder Fleck in der eigenen Farbe unter jedem Soldaten bzw. eine
   gemeinsame Marke unter dem Trupp, nur ab einer bestimmten Kameradistanz
   oder immer (beides ausprobieren, Bildvergleich). Muss auf hellem Gras,
   Schnee, Sand, Schlamm und bei Nacht lesbar sein.
3. **Taste „alle zeigen“** (z. B. gedrückt halten, vorher `docs/CONTROLS.md`
   auf freie Tasten prüfen): alle eigenen Einheiten im Bild mit Umriss und
   Symbol über dem Kopf, solange die Taste gehalten wird.
4. **Einstellung** im Menü (Aus / Nur unter der Maus / Immer), Standard:
   unter der Maus plus dezente Bodenmarke. Paletten für Farbsehschwächen
   aus `relation_colors.gd` verwenden.
5. Optional, wenn billig: Umriss durch Hindernisse hindurch (Einheiten hinter
   Häusern und Bäumen) nur für eigene Einheiten.

Technik: Umriss per zweitem Material-Durchgang oder Shader-Parameter pro
Instanz (`set_instance_shader_parameter`), keine neuen Materialien pro Frame.
Bildrate mit `just bench-view` vorher/nachher messen (Ziel: weniger als 0,5 ms
pro Bild mehr im Gefecht, `docs/PERFORMANCE.md`).

## 4. Regeln

- Reine Darstellung: nichts in `crates/sim` ändern, kein Einfluss auf Hashes.
- Keine vergrößerten Modelle. Farben nur aus `relation_colors.gd`.
- Nichts doppelt zu den Prompts 099, 101 und 103 bauen; wenn einer davon
  schon eingecheckt ist, darauf aufbauen (z. B. HUD-Ausblenden im Fotomodus
  blendet auch die neuen Markierungen aus).

## 5. Abschluss

- Selbsttest erweitern: Maus über einen eigenen Trupp → Hervorhebung an, weg
  → aus; Einstellung „Aus“ → keine Bodenmarken. `just check`, `just selftest`.
- Bilder vorher/nachher an derselben Stelle (Wiese, Schnee, Nacht) in
  `build/unit-visibility/` für den Nutzer, dazu die Messwerte der Bildrate.
- `docs/CONTROLS.md` (neue Taste, Einstellung) aktualisieren.

## 6. Nicht Teil dieses Prompts

- Reichweitenkreise und Zielvorschau (Prompt 099).
- Größere Modelle oder andere Uniformfarben.
