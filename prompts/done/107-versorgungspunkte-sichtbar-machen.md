# Prompt 107 – Versorgungspunkte lesbar machen: Minimap, Grenzlinie, Ertrag

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/GAME_DESIGN.md` (Versorgungspunkte, Einkommen; „Open“: Punkte im Editor
> setzen), `data/economy.toml` (`[supply]`: Radius 25 m, `capture_s` 20,
> Einkommen), `crates/sim/src/sim/economy.rs` (`SupplyPoint`,
> `take_supply_points`, `supply_points()`), `godot/scenes/world/base_view.gd`
> (`_update_supply`, `_make_flag`, fester Ring mit fest eingetragenem Radius um
> Zeile 239), `godot/scenes/hud/minimap.gd` (`_draw_overlay`, `add_ping`),
> `godot/scenes/hud/mission_hud.gd` (gelbe Zielringe), `godot/scenes/hud/
> relation_colors.gd`, `godot/scenes/hud/event_feed.gd` und
> `prompts/095-hud-neu-anordnen.md` (Minimap unten links, Ressourcen oben rechts).

## 1. Ausgangslage

Im Referenzvideo `docs/.vorlagen/vorlage.mp4` (nicht im Repo, nur Vorbild für die
**Wirkung**; keine Linienform, Farbe, Flagge oder Symbolik übernehmen) erkennt man
jederzeit, wem ein umkämpfter Punkt gehört: Das Gebiet hat eine farbige
Grenzlinie am Boden (eigene Farbe bzw. Gegnerfarbe), auf der Minimap sitzt an
jedem Punkt eine Marke in Besitzerfarbe, ein Klick auf den Punkt zeigt Vorrat und
Ertrag pro Minute, und oben steht beim Kontostand der Zuwachs pro Minute. Der
Tester lobt, dass daraus ständige Kämpfe um die Punkte entstehen, und bemängelt,
dass man die Grenzlinien **nicht abschwächen** kann.

Bei uns heute:

- Versorgungspunkte haben eine Flagge in Besitzerfarbe, einen Ring, der mit dem
  Erobern wächst, und einen blassen weißen Ring für die Fläche. Dessen Radius ist
  im Skript fest eingetragen statt aus den Daten gelesen.
- **Auf der Minimap fehlen die Punkte ganz.**
- Was ein Punkt bringt, sieht man nirgends am Punkt.

## 2. Ziel

- **Minimap:** jeder Versorgungspunkt als klare Marke in Besitzerfarbe (neutral,
  eigen, verbündet, Gegner), mit Fortschritt beim Erobern. Missionsziele ebenfalls
  als Marke (in `docs/MISSIONS.md` als offen geführt). Besitz des Gegners nur so
  aktuell, wie man ihn gesehen hat (Regel wie im Hauptbild; prüfen, was der
  Snapshot liefert, und nichts verraten, was man nicht sieht).
- **Fläche am Boden:** Der Rand der Eroberungsfläche als ruhige Linie in
  Besitzerfarbe statt des weißen Rings; neutral blass. Radius aus
  `economy.toml`. Die Linie liegt über der Nebelschicht (Prompt 098) und folgt dem
  Gelände. Stärke „kräftig / dezent / aus“ in den Einstellungen (zusammen mit den
  Schaltern aus Prompt 103, falls vorhanden).
- **Umkämpft:** Stehen beide Seiten in der Fläche, ist das am Punkt und auf der
  Minimap erkennbar (z. B. Blinken oder zweifarbiger Rand).
- **Auskunft:** Klick auf den Punkt (oder Maus darüber) zeigt im Infofeld Besitzer,
  Ertrag pro Minute je Rohstoff und Fortschritt.
- **Ertrag oben:** neben jedem Kontostand der aktuelle Zuwachs pro Minute, so dass
  man sofort sieht, was ein verlorener Punkt kostet. Prüfen, ob es das schon gibt.
- **Meldung:** „Punkt erobert“, „Punkt verloren“, „Punkt wird angegriffen“ in der
  Ereignisleiste mit Sprung; mit Prompt 101 abstimmen, falls umgesetzt.

## 3. Regeln

- Reine Darstellung und Lesefunktionen; `crates/sim` und der Golden Hash bleiben.
- Farben nur über `relation_colors.gd` (auch Farbschwäche-Paletten); Formen
  unterscheiden sich zusätzlich, nicht nur die Farbe.
- Balkenkreuz statt Hakenkreuz, keine fremden Flaggen oder Symbole nachbauen.
- Keinen Test abschwächen.

## 4. Vorgehen

1. Bestand lesen; klären, was der Snapshot über Punkte des Gegners außerhalb der
   Sicht liefert.
2. Minimap-Marken, Grenzlinie, „umkämpft“.
3. Auskunft im Infofeld, Zuwachs oben, Meldungen.
4. Selftest: Punkt erscheint auf der Minimap in neutraler Farbe, nach dem Erobern
   in eigener; Radius der Linie = Wert aus den Daten; Schalter „aus“ blendet die
   Linie aus.
5. `just check`, `just selftest`, `just lan-selftest`.
6. Doku: `docs/CONTROLS.md`, `docs/ARCHITECTURE.md`, `docs/MISSIONS.md` (offenen
   Punkt streichen, falls erledigt).
7. Für den Nutzer: Bilder Hauptbild und Minimap in den vier Besitzzuständen.

## 5. Nicht Teil dieses Prompts (als Entwurf für später)

Diese Ideen aus dem Video ändern Spielregeln und brauchen erst eine Entscheidung
des Nutzers; hier nur als Vorschlag ins Ergebnis schreiben, nichts davon bauen:

- **Punkte mit Funktion:** Werkstatt (repariert), Flugplatz (schaltet
  Luftunterstützung frei), Funkstelle (deckt Stellungen auf), Bahnhof oder Hafen
  (Verstärkung trifft dort ein, siehe Prompt 108).
- **Zusammenhängende Gebiete und Frontlinie:** Karte in Gebiete teilen, Linie
  zwischen eigenem und gegnerischem Gebiet. Vorlagen dazu liegen in
  `docs/.vorlagen/frontline/`.
- Versorgungspunkte im Karteneditor setzen (`docs/GAME_DESIGN.md`, „Open“).
