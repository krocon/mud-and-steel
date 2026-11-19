# Prompt 103 – Weite Übersicht, HUD ausblenden, Fotomodus

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `docs/PERFORMANCE.md`, `prompts/done/002-*.md`,
> `prompts/done/090-*.md` (Schattendistanz und Gelände-LOD in Stufen),
> `godot/scenes/camera/rts_camera.gd` (`min_distance` 15, `max_distance` 600,
> `pitch_curve`, `zoom_level`, `_zoom_by`), `godot/scenes/editor/map_editor.gd`
> (hebt `max_distance` an), `godot/scenes/menu/settings.gd` (`DEFAULTS`,
> `HEALTH_BAR_MODES`), `godot/scenes/menu/settings_panel.gd`,
> `godot/scenes/tools/trailer_shots.gd` (Zeilen um 846–858: was dort alles
> ausgeblendet wird) und `prompts/096-kamera-gizmo-drehen-neigen.md`. Wenn Prompt
> 102 (taktische Pause) schon umgesetzt ist, dessen Pause mitbenutzen.

## 1. Ausgangslage

Im Test zum Referenzvideo (`docs/.vorlagen/vorlage.mp4`, nur als Anregung) wünscht
sich der Tester drei Dinge: eine **noch fernere Zoomstufe** für die Übersicht,
Markierungen (dort die Sektorgrenzen), die sich **abschalten oder abschwächen**
lassen, und ein **komplett abschaltbares HUD bzw. einen Fotomodus**, weil die
Modellbau-Optik sonst von Linien überdeckt wird. Gelobt werden die vielen
Einzelschalter für HUD-Symbole.

Bei uns heute:

- Zoom 15–600 m, kein Übersichtsmodus.
- Schalter gibt es für Lebensbalken (beschädigt / ausgewählt / nie) und das
  Einklappen von Befehls- und Luftleiste. Keine Schalter für Auswahlringe,
  Wegpunkte, Versorgungs- und Eroberungsringe, Einschlagmarken.
- Kein „HUD aus“, kein Fotomodus. Nur das Entwicklerwerkzeug `trailer_shots.gd`
  blendet HUD, Nebel, Seitenfärbung, Einschlagmarken und Flaggen aus; die Liste
  dort ist das Rezept.

## 2. Ziel

### 2.1 Übersicht

- Eine Taste (und der Ansichtsknopf aus Prompt 095, falls vorhanden) schaltet auf
  eine **Übersicht**: Kamera gleitet so weit hinaus, dass ein großer Teil der
  Karte im Bild ist, steiler Blick. Nochmal drücken oder hineinzoomen bringt
  die alte Ansicht zurück (Ort unter der Maus als Ziel, damit man mit der
  Übersicht schnell springen kann).
- In der Übersicht sind Einheiten nur noch Punkte. Deshalb dort **Symbole statt
  Modelle** betonen: kleine, seitenfarbige Marken je Einheit bzw. Gruppe,
  Versorgungspunkte mit Besitz, Missionsziele. Vorhandene Marken wiederverwenden.
- Prüfen, ob stattdessen einfach `max_distance` wachsen kann (der Editor geht bis
  1500 m). Messen, was das an Bildrate, Schatten, Gelände-LOD, Vegetation und
  Nebel kostet (`just bench-view`), und die einfachste Lösung nehmen, die flüssig
  bleibt. Schatten, Gras und Kleinteile dürfen in der Übersicht aus sein.

### 2.2 Einzelschalter

In den Einstellungen (Reiter Interface) je ein Schalter, Standard wie heute:

- Auswahlringe, Wegpunkte und Routen, Versorgungs- und Reparaturringe, Fläche und
  Ring der Versorgungspunkte, Einschlagmarken, Missionsringe, Seitenfärbung der
  Einheiten, Randpfeile (Prompt 101), Reichweitenkreise (Prompt 099, falls da).
- Für Linien am Boden zusätzlich eine **Stärke** (kräftig / dezent), damit man sie
  abschwächen kann, statt sie ganz zu verlieren.

### 2.3 HUD aus und Fotomodus

- **HUD aus:** eine Taste blendet das ganze HUD und alle Marken in der Welt aus und
  wieder ein. Befehle bleiben möglich. Ein kurzer Hinweis beim Ausblenden nennt
  die Taste, damit niemand „ohne HUD“ festsitzt.
- **Fotomodus** (nur Einzelspiel und Wiederholung): hält das Spiel an, blendet HUD
  und Marken aus, gibt die Kamera frei (freies Drehen und Neigen, tiefer hinunter
  als im Spiel, Brennweite), optional Nebel des Krieges aus (nur in der
  Wiederholung bzw. nach Spielende, sonst wäre es Schummeln), Tiefenschärfe
  (es gibt `depth_of_field` mit „miniature“), und speichert ein Bild als PNG in
  einen Benutzerordner. Verlassen stellt Kamera und Einstellungen exakt wieder
  her.
- Im Mehrspieler: HUD aus ja, Fotomodus nein.

## 3. Regeln

- Reine Darstellung; `crates/sim` und der Golden Hash bleiben.
- Die Kameragrenzen des normalen Spiels ändern sich nur, wenn die Messung es
  trägt. Prompt 096 (Neigen) nicht doppelt bauen: ist 096 umgesetzt, dessen
  Neigung im Fotomodus weiterverwenden.
- Keinen Test abschwächen. Batch-Sessions teilen sich das Arbeitsverzeichnis.

## 4. Vorgehen

1. Bestand lesen; messen, was 800 m, 1000 m und 1500 m Abstand auf der größten
   Karte kosten.
2. Übersicht bauen (Taste, Rücksprung, Marken).
3. Einzelschalter und Stärke.
4. HUD aus, Fotomodus, Bild speichern.
5. Selftest: Übersicht an/aus stellt die Kamera wieder her; jeder Schalter
   blendet sein Element aus; HUD aus und wieder an; Fotomodus hält die Ticks an
   und stellt danach alles wieder her; im LAN kein Fotomodus.
6. `just check`, `just selftest`, `just lan-selftest`, `just bench-view`
   vorher/nachher.
7. Doku: `docs/CONTROLS.md`, `docs/PERFORMANCE.md`, `docs/ARCHITECTURE.md`.
8. Für den Nutzer: je ein Bild Übersicht, „HUD aus“ und Fotomodus mit
   Tiefenschärfe; die Messwerte.

## 5. Nicht Teil dieses Prompts

- Befehle auf einer eigenen strategischen Kartenansicht (Vollbildkarte).
- Filmkamera mit Kamerafahrten (dafür gibt es `trailer_shots.gd`).
