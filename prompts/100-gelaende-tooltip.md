# Prompt 100 – Gelände-Tooltip: was der Boden unter der Maus bewirkt

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `data/terrain.toml` ganz (12 Geländearten, `speed`, `speed_pct`, `passable`,
> `[slope]`), `data/infantry.toml` (`[cover]`, `[[cover.object]]`),
> `data/vision.toml`, `data/objects.toml`, `crates/bridge/src/hud_funcs.rs`
> (`minimap_image`, Sichtgitter Foliage/Solid), die Abschnitte zu Gelände, Deckung
> und Sicht in `docs/MAPS.md`, `docs/INFANTRY.md` und `docs/GAME_DESIGN.md`,
> `godot/scenes/selection/shell_orders.gd` (`hover`, einziges Hover auf Kartenobjekte
> heute), `godot/scenes/hud/picker.gd` (Tooltips der Kacheln) und
> `godot/scenes/debug/path_debug.gd` (F4-Kostenansicht).

## 1. Ausgangslage

Im Referenzvideo `docs/.vorlagen/vorlage.mp4` (nicht im Repo, nur Vorbild für die
**Wirkung**) steht unten rechts ein kleiner Kasten, sobald die Maus über dem Boden
ruht: Name des Geländes und ein Satz, was es bewirkt, etwa „verlangsamt
geringfügig die Bewegung und bietet wenig Deckung“ oder „beeinträchtigt stark die
Sicht, Bodenfahrzeuge können nicht einfahren“. Der Tester betont außerdem den
Unterschied zwischen **Sichtschutz** und **Deckung**; dort gibt es nur Sichtschutz,
und das hat ihn gestört.

Bei uns heute:

- Über dem Boden zeigt die Maus nichts. `terrain_at` nutzen nur Effekte und Ton.
- Die Regeln sind da, aber unsichtbar: Tempo je Gelände und Steigung
  (`terrain.toml`), Deckung je Kartenobjekt (`infantry.toml`), Sichtblocker im
  Sichtgitter (Bewuchs, fest).
- Geländearten haben keine übersetzten Anzeigenamen.

## 2. Ziel

Ruht die Maus kurz (Richtwert 0,4 s) über der Karte, erscheint ein kleiner Kasten:

- **Name** des Geländes oder des Kartenobjekts unter der Maus (Wald, Feld, Sumpf,
  Furt, Mauer, Haus, Graben …), übersetzt.
- **Bewegung:** getrennt für Fahrzeuge und Infanterie, als Wort und Zahl
  („Fahrzeuge langsam, 60 %“, „Fahrzeuge: nicht befahrbar“). Steigung dazu, wenn
  sie das Tempo merklich drückt.
- **Sichtschutz:** keiner / leicht / stark (aus dem Sichtgitter).
- **Deckung:** keine / leicht / schwer (aus `[cover]`), getrennt vom Sichtschutz.

Die Angaben kommen **aus denselben Daten wie die Regeln**, nicht aus einer
handgepflegten Texttabelle. Ändert jemand `terrain.toml`, stimmt der Tooltip
weiter.

## 3. Umsetzung

Reine Darstellung plus eine Lesefunktion in der Bridge. Golden Hash bleibt.

- **Bridge:** eine Funktion „Geländeauskunft an Weltpunkt“, die Geländeart,
  Tempo-Prozent für Fahrzeug und Infanterie, Befahrbarkeit, Steigungsanteil,
  Sichtzustand der Zelle und das Deckung gebende Objekt (falls eines dort liegt)
  liefert. Werte aus den Sim-Daten lesen, nichts nachrechnen.
- **Wetter:** Wenn Regen oder Schnee das Tempo ändert (`weather.toml`), den
  aktuellen Wert zeigen.
- **Ausgewählte Einheit:** Ist genau eine Art ausgewählt, die Bewegungszeile für
  diese Art schreiben (Kette, Rad, Fuß), sonst beide Zeilen.
- **Namen:** Anzeigenamen für alle Geländearten und Deckungsobjekte in die
  Übersetzungen aufnehmen (alle vorhandenen Sprachen, `l10n_shots.gd` beachten).
- **Kasten:** im gemeinsamen HUD-Stil. Platz so wählen, dass er zu Prompt 095
  passt (dort ist unten rechts das Befehlsraster): prüfen, ob 095 schon umgesetzt
  ist, und den Kasten neben die Maus oder an eine freie Stelle legen. Er darf
  nichts verdecken, worauf man gerade klicken will, und nicht flackern, wenn die
  Maus über eine Geländegrenze fährt.
- **Nebel:** Geländeauskunft gibt es überall (die Landschaft ist nach Prompt 098
  immer sichtbar). Vom Gegner gebaute Stellungen außerhalb der Sicht dürfen über
  den Tooltip **nicht** verraten werden: nur zeigen, was auch gezeichnet wird.
- **Schalter** in den Einstellungen (Interface): Gelände-Tooltip an / nur mit
  gedrückter Taste / aus. Standard an.
- Kein Tooltip, während ein Rahmen gezogen, die Kamera verschoben oder ein
  Bauplatz gewählt wird.

## 4. Vorgehen

1. Bestand lesen; Tabelle „Geländeart → Tempo Fahrzeug/Fuß, Sichtschutz, Deckung“
   aus den Daten erzeugen und auf Lücken und Widersprüche ansehen. Auffälligkeiten
   (z. B. Wald ohne Sichtschutz, Mauer ohne Deckung) als Befund melden, nicht in
   diesem Prompt umbalancieren.
2. Bridge-Funktion mit Test (bekannte Punkte einer Testkarte).
3. Kasten, Verzögerung, Schalter, Übersetzungen.
4. Selftest: Maus über Straße, Wald, Wasser, Haus → erwarteter Name und
   erwartete Stufen; kein Kasten während Rahmenziehen.
5. `just check`, `just selftest`, `just export-selftest`.
6. Doku: `docs/CONTROLS.md`, kurzer Absatz in `docs/MAPS.md`.
7. Für den Nutzer: Bilder des Kastens über fünf Geländearten und die Befundliste
   aus Schritt 1.

## 5. Nicht Teil dieses Prompts

- Änderungen an Tempo, Sicht oder Deckung.
- Eine eingefärbte Karten-Überlagerung „wo ist Deckung“ (wäre ein eigener Prompt).
- Tooltips für Einheiten (die hat das Einheitenfeld).
