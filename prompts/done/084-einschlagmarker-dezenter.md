# Prompt 084 – Einschlagmarker der Artillerie dezenter und kürzer

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `godot/scenes/world/artillery_view.gd` ganz (vor allem `_draw_shells`, `_flat_material`,
> die Konstanten `MARKER_*` und `WHISTLE_SECONDS`), in `godot/scenes/world/strike_view.gd`
> den Flächenring des Luft- und Raketenschlags, in `docs/ARCHITECTURE.md` den Abschnitt zu
> indirektem Feuer (Prompt 032) und „Air support and anti-aircraft fire“, in
> `docs/GAME_DESIGN.md` Artillerie und Feuermodi, dazu `hud/relation_colors.gd`
> (Farbpaletten für Farbsehschwäche).

## 1. Was die Ringe heute sind

Jede Granate, die der **lokale Spieler selbst** verschossen hat, bekommt einen orangen
Ring (`TorusMesh`) an ihrem Zielpunkt am Boden – vom Abschuss bis zum Einschlag, also
die ganze Flugzeit lang. Es ist eine Vorschau „hier wird gleich eine eigene Granate
landen“, keine Warnung vor feindlichem Feuer (gegnerische Granaten bekommen keinen Ring).
Nebelgranaten bekommen einen grauen Ring.

Heute sind sie zu laut:

- `MARKER_COLOR = Color(1.0, 0.35, 0.2, 0.8)`, unschattiert und **emittierend**: sie
  leuchten auch in Dämmerung und Nacht voll und liegen über Gras, Rauch und Bäumen.
- Radius `max(2, Kaliber × 0,1)` m, Ringstärke 15 % des Radius: bei 105 mm ein Ring von
  10,5 m mit 1,5 m breitem Band; bei 150 mm 15 m.
- Sichtbar über die **ganze Flugzeit** (bei Haubitzen viele Sekunden), dazu ein Pulsieren
  um ±15 %.
- Eine Salve aus mehreren Geschützen und Schüssen ergibt einen Teppich aus dutzenden sich
  überlagernden Ringen, der das Gefecht verdeckt (siehe Bild der Aufgabe: Wiese am
  Waldrand, ein großer Teil des Bildes ist Orange).

## 2. Ziel

Die Marker bleiben als Hilfe für den Spieler erhalten (er soll sehen, wohin sein Feuer
geht), aber man nimmt sie nur noch nebenbei wahr. Fertig ist der Prompt, wenn

- ein Marker erst kurz vor dem Einschlag erscheint (Richtwert: in den letzten
  `WHISTLE_SECONDS` = 1,5 s, also zusammen mit dem Pfeifen; entscheide selbst und
  begründe), sanft einblendet, beim Einschlag nicht hart abreißt und nicht mehr pulsiert,
- er deutlich schwächer ist: dünne Linie statt breitem Band (Richtwert unter 0,3 m),
  Deckkraft um 0,3–0,4, keine Emission (oder nur so viel, dass er nachts nicht grell
  wird), eine Farbe, die zur Szene passt statt Signalorange; die Farbe kommt aus
  `relation_colors.gd`, damit die Paletten für Farbsehschwäche weiter greifen,
- der Radius zur tatsächlichen Wirkung passt statt zum Kaliber × 0,1 (falls die Simulation
  einen Splitter-/Wirkradius kennt, diesen über SimHost nehmen; sonst einen kleineren,
  festen Richtwert je Kaliberklasse) und nie größer ist als heute,
- eine Salve nicht mehr als Ringteppich erscheint: überlagerte Marker addieren sich nicht
  zu kräftigem Orange (z. B. Deckkraft so wählen, dass Überlagerung kaum dunkler wird, oder
  nahe Zielpunkte derselben Salve zu einem Marker zusammenfassen – entscheide selbst),
- der Flächenring aus `strike_view.gd` zum neuen Stil passt, falls er dasselbe Problem hat
  (prüfen, nicht blind ändern),
- in den Trailer-Aufnahmen (`trailer_shots.gd`, „kein HUD“) keine Marker erscheinen
  (Prompt 082 arbeitet gerade an `trailer_shots.gd`; nur einen kleinen Schalter ergänzen
  und Konflikte vermeiden),
- `just check` und `just selftest` grün sind; der Golden Hash bleibt unverändert (reine
  Darstellung, nichts in `crates/sim`).

## 3. Vorgehen

1. Vorher-Standbilder: eine Szene mit einer Salve von mindestens zwei Batterien auf eine
   Wiese, bei Tag und in Dämmerung, mittlere und weite Zoomstufe (z. B. über
   `just trailer-shots --stills` mit einer kleinen Szene oder einem Werkzeug wie
   `blast_shots.gd`). Nach `build/marker_shots/before_*.png`.
2. Änderung in `artillery_view.gd` (Zeitfenster, Einblenden, Radius, Stärke, Farbe,
   Material, kein Pulsieren), Konstanten mit kurzem Kommentar zum Zweck.
3. Nachher-Standbilder mit denselben Kameras nach `build/marker_shots/after_*.png`; zeig
   beide nebeneinander. Der Marker muss im Nachher-Bild noch erkennbar sein, wenn man ihn
   sucht, darf aber nicht das Erste sein, was man sieht.
4. Self-Tests anpassen, die die Marker zählen oder ihre Sichtbarkeit prüfen (nicht
   abschwächen: prüfen, dass der Marker im Fenster vor dem Einschlag sichtbar ist und
   davor nicht).
5. `docs/ARCHITECTURE.md` (Abschnitt Prompt 032) und `docs/CONTROLS.md`, falls dort die
   Marker beschrieben sind, nachziehen.

## 4. Nicht Teil dieses Prompts

- Warnungen vor feindlichem Feuer (gibt es bewusst nicht; Sicht und Pfeifen genügen).
- Änderungen an Flugbahn, Streuung oder Wirkung der Granaten.
- Der Zielmarker beim Befehl (`main.gd`, `_show_target_marker`), außer er wirkt neben den
  neuen Markern deutlich zu grell.
