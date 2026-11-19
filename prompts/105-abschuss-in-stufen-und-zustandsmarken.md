# Prompt 105 – Abschuss in Stufen und Zustandsmarken über Einheiten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/DETERMINISM.md`, `prompts/done/017-*.md` (Durchschlag, Panzerseiten),
> `prompts/done/035-*.md` (Kette ab), `prompts/done/036-*.md` (fahrunfähig),
> `prompts/done/041-*.md` (Feuer), `prompts/done/047-*.md` (Niederhalten),
> `prompts/done/048-*.md` und `049-*.md` (Munitionsbrand, Aussteigen),
> `godot/scenes/world/effects.gd` (`destroyed`, `explosion`, `_start_wreck`,
> `_update_wrecks`, `BURN_SECONDS`, `SMOLDER_SECONDS`, `unit_hit`),
> `godot/scenes/world/combat_view.gd` (`_make_wreck`, `WRECK_TINT`, `HealthBars`),
> `godot/scenes/world/works_view.gd` (`_draw_tracks`),
> `godot/scenes/hud/unit_panel.gd` (Warntexte fahrunfähig, niedergehalten),
> `data/logistics.toml` (`[damage] immobile_pct`), `data/fire.toml` (`[wreck]`),
> `crates/sim/src/sim/combat.rs` (Stelle, an der eine Einheit zerstört wird) und
> `godot/scenes/audio/sound_classes.gd` (`hit_ricochet`, `hit_penetration`).

## 1. Ausgangslage

Im Referenzvideo `docs/.vorlagen/vorlage.mp4` (nicht im Repo, nur Vorbild für die
**Wirkung**) stirbt ein Panzer nicht in einem Augenblick. Der Tester beschreibt es
so: erst Rauch, dann eine Feuerwelle, schließlich die Explosion. Über beschädigten
Fahrzeugen sitzt eine kleine Marke, an der man den Zustand ohne Auswahl erkennt.

Bei uns heute:

- **Abschuss:** `destroyed` löst sofort eine Explosion aus, danach brennt das Wrack
  60 s und schwelt 90 s. Es gibt keine Stufen davor.
- **Zustände** gibt es in den Regeln, aber fast nur als Text im Einheitenfeld:
  fahrunfähig (≤ 35 % Trefferpunkte), Kette ab (als Kästchen neben dem Fahrzeug),
  niedergehalten (Infanterie), ohne Munition, ohne Sprit. Über der Einheit steht
  nur der Lebensbalken.
- Abpraller geben Funken und einen eigenen Ton; ein wegfliegendes Geschoss sieht
  man nicht.

## 2. Ziel

### 2.1 Abschuss in Stufen (Darstellung)

- Wird ein Fahrzeug zerstört, läuft eine kurze Folge ab statt eines Knalls:
  **Treffer mit Stichflamme → Rauch quillt aus Luken und Motor → Feuer greift um
  sich → Munition geht hoch** (bei Fahrzeugen mit Munition; Turm oder Luken
  fliegen nur, wenn das Modell es hergibt). Gesamtdauer Richtwert 2–5 s, mit
  etwas Streuung, damit zehn Abschüsse nicht gleich aussehen.
- Die Art hängt vom Fahrzeug und vom Treffer ab: Lkw und leichte Fahrzeuge brennen
  eher aus, als zu explodieren; ein schwerer Treffer überspringt Stufen.
- **Die Regeln ändern sich nicht:** Die Einheit ist mit dem zerstörenden Treffer
  tot, schießt und fährt nicht mehr, zählt sofort als Verlust. Die Folge ist nur
  Bild und Ton. Die Besatzung steigt wie heute aus (Prompt 049); die Folge muss
  dazu passen (erst Rauch, dann raus, dann Feuer).
- Streuung über einen Wert, der auf allen Rechnern gleich ist (Einheiten-ID,
  Tick), nicht über Godot-Zufall, damit Wiederholungen gleich aussehen.
- Danach brennt und schwelt das Wrack wie heute. Prüfen, ob die 60 + 90 s und das
  spurlose Verschwinden noch passen, wenn Gefechte nach Prompt 097 länger dauern;
  Vorschlag machen (Wrack bleibt als kalter Rumpf liegen?), aber nichts an den
  Regeln für Wracks (Deckung, Sperre, Feuer) ändern.
- Ton: jede Stufe mit eigenem Klang aus der vorhandenen Bibliothek
  (`docs/AUDIO_LIBRARY.md`); nichts Neues beschaffen ohne Rückfrage.

### 2.2 Zustandsmarken über Einheiten

- Über eigenen Einheiten (und sichtbaren gegnerischen, soweit man den Zustand von
  außen sehen kann) kleine Marken für: **fahrunfähig, Kette ab, brennt, ohne
  Munition, ohne Sprit, niedergehalten, wird repariert/versorgt**. Eigene
  Symbole im vorhandenen Stil, je eine klare Form, nicht nur Farbe.
- Sparsam: nur anzeigen, was der Spieler wissen muss, höchstens zwei Marken
  nebeneinander, Rest im Einheitenfeld. Beim weiten Herauszoomen nur die
  wichtigste.
- Gegner: „ohne Munition/Sprit“ nie zeigen (das sieht man von außen nicht);
  fahrunfähig, Kette ab, brennt und niedergehalten ja.
- Schalter in den Einstellungen wie bei den Lebensbalken (immer / nur ausgewählt /
  nie).

### 2.3 Abpraller sichtbar

- Prallt ein Geschoss ab, fliegt eine kurze Leuchtspur im Abprallwinkel weg
  (Richtung aus Schussrichtung und getroffener Seite, die die Simulation schon
  kennt). Reiner Effekt.

## 3. Regeln

- Darstellung in `godot/`, höchstens kleine Lesefunktionen in `crates/bridge`
  (z. B. Art des zerstörenden Treffers). Kein Eingriff in `crates/sim`; der Golden
  Hash darf sich nicht ändern.
- Kein Godot-Zufall für Dinge, die in Wiederholungen gleich aussehen sollen.
- Nebel des Krieges: keine Stufen, Marken oder Töne für Einheiten außerhalb der
  Sicht (heutige Regel in `main.gd`).
- Keinen Test abschwächen. `fx_fires` und die übrigen Obergrenzen der
  Grafikstufen gelten weiter.

## 4. Vorgehen

1. Bestand lesen; Vorher-Clip: je ein Panzer, Lkw, Geschütz und
   Halbkettenfahrzeug wird abgeschossen.
2. Stufenfolge je Fahrzeugklasse als kleine Tabelle festlegen und bauen.
3. Zustandsmarken, Schalter.
4. Abpraller-Leuchtspur.
5. Selftest: Nach `destroyed` läuft die Folge und endet im Wrackzustand; die
   Einheit ist ab dem ersten Tick nicht mehr auswählbar; fahrunfähige Einheit
   zeigt die Marke; gegnerische Einheit zeigt nie „ohne Munition“.
6. `just check`, `just selftest`, `just bench-view` mit einem Gefecht von 40
   Fahrzeugen vorher/nachher.
7. Doku: `docs/ARCHITECTURE.md`, `docs/CONTROLS.md` (Marken, Schalter).
8. Für den Nutzer: Vorher/Nachher-Clip, Tafel aller Marken, Vorschlag zur
   Lebensdauer der Wracks.

## 5. Nicht Teil dieses Prompts

- Bauteilschäden in den Regeln (Kanone, Motor, Turm einzeln beschädigt und
  reparierbar). Das wäre eine Regeländerung mit neuem Golden Hash und braucht
  eine eigene Entscheidung des Nutzers.
- Wracks bergen, abschleppen oder neu bemannen (`docs/GAME_DESIGN.md`, „Not in this
  version“).
