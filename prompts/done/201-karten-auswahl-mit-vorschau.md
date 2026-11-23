# Prompt 201 – Kartenauswahl mit Vorschau (Map-Picker)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> in `docs/ARCHITECTURE.md` die Teile zu Menü, Lobby und Map-Editor,
> `docs/MAPS.md`, `docs/PERFORMANCE.md`. Im Code:
> `godot/scenes/menu/main_menu.gd` (`map_option`, `_update_map_info`,
> Startaufstellung, Spielerzahl), `menu/match_setup.gd` (`available_maps`,
> `map_title`, `describe_map`, `map_starts`, `variant_choices`, `is_large`),
> `menu/menu_theme.gd`, `menu/unit_model_view.gd` (3D-Ansicht im Menü),
> `lan/lan_lobby.gd`, `online/steam_lobby.gd` (`_map`, `_variant`),
> `editor/editor_panel.gd` (Dialog „open“, `open_option`),
> `menu/missions_screen.gd`, `menu/campaign_screen.gd`, `hud/minimap.gd`,
> `crates/bridge/src/hud_funcs.rs` (`minimap_image`).

## 1. Wunsch des Nutzers (2026-11-23)

Die Kartenauswahl im Einzelspiel ist eine Auswahlliste mit Namen
(„skirmish“) und einer Textzeile („2,4 km × 2,4 km, Wald. Große Karte …“).
Man sieht nicht, wie die Karte aussieht. Gewünscht:

- **a)** Die gewählte Karte wird in der rechten Fensterhälfte gezeigt (das
  Formular nimmt heute nur die rechte Hälfte ein, links ist es leer): als
  Draufsicht oder sogar als kleines 3D-Modell.
- **b)** Statt der Auswahlliste ein **Dialog (Map-Picker)**: links eine
  Tabelle der Karten, rechts die Detailansicht der markierten Karte
  (Draufsicht oder 3D-Ansicht und weitere Angaben).
- Der Picker soll **an mehreren Stellen** dienen: Einzelspiel, LAN- und
  Steam-Lobby und **im Map-Editor** (Dialog „Karte öffnen“, heute ebenfalls
  nur eine Auswahlliste mit Ordnernamen).

## 2. Ziel

Ein wiederverwendbarer Baustein `menu/map_picker.gd` (Dialog) plus eine
Vorschau `menu/map_preview.gd` (Draufsicht und 3D-Ansicht, auch einzeln
verwendbar). Reine Darstellung, keine Regeländerung, kein Hash-Einfluss.

### 2.1 Vorschaubild (Draufsicht)

- Aus den Kartendaten (`height.png`, `terrain.png`, `objects.csv`, Straßen,
  Wasser, Gebäude, Wald) im Stil der Minimap: Geländefarbe, Hangschattierung,
  Straßen, Wald, Gebäude, Wasser. Eine Funktion in der Bridge, die eine Karte
  **ohne Spielstart** liest und ein Bild liefert (z. B.
  `SimHost.map_preview_image(name, size)` oder eine eigene statische
  Funktion); vorhandenen Minimap-Code teilen statt doppeln.
- Darüber: **Startpunkte** je Startaufstellung (Nummer/Farbe je Spieler bzw.
  Team), **Versorgungspunkte**, Brücken; Kilometerangabe/Maßstab und Nordpfeil.
  Wechsel der Startaufstellung aktualisiert die Marken.
- **Zwischenspeicher:** große Karten (city_large, harbor_city …) nicht bei
  jedem Öffnen neu rechnen. Bild in `user://map_previews/<name>.png` mit
  Prüfsumme/Änderungszeit der Kartendateien; erst messen, wie lange das
  Erzeugen je Karte dauert (Tabelle im Abschluss). Für ausgelieferte Karten
  ggf. vorab erzeugte Bilder im Export (`just`-Befehl), wenn das Rechnen
  spürbar ist. Erzeugen im Hintergrund/verzögert, damit die Liste nicht
  hängt; bis dahin Platzhalter.

### 2.2 3D-Ansicht (Stufe 2, nach der Draufsicht)

- Kleines Höhenrelief als Mesh aus `height.png` (stark vereinfacht, z. B.
  128×128 Stützpunkte), Draufsicht-Bild als Textur, Wasser als Fläche,
  langsam drehbar mit der Maus (wie `unit_model_view.gd`), Höhe leicht
  überhöht und als Schalter. Keine Objekte/Bäume als Modelle nötig.
- Umschalter Draufsicht / 3D in der Detailansicht. Wenn die 3D-Ansicht
  zu teuer oder hässlich wird: Bilder zeigen, Nutzer entscheiden lassen.

### 2.3 Map-Picker-Dialog

- **Links Tabelle**: Name (übersetzter Titel), Größe, Biom, Spielerzahl
  (Starts), Startaufstellungen, Missionen/Schlachten auf der Karte (wenn
  einfach zu ermitteln), Herkunft (Geodaten/Skizze/handgebaut, falls in
  `map.toml` erkennbar). Sortierbar nach Spalte; Filter/Suche per Textfeld;
  optional Filter Biom und Spielerzahl.
- **Rechts Detailansicht**: großes Vorschaubild bzw. 3D, Titel,
  Beschreibung (falls die Karte eine hat; sonst `describe_map`), Größe,
  Biom, Wetter/Tageszeit-Vorgabe, Startaufstellungen mit Marken, Hinweis
  „große Karte“.
- Bedienung: Pfeiltasten in der Tabelle, Enter/Doppelklick wählt,
  Esc bricht ab; Maus und Controller-taugliche Fokusführung wie im übrigen
  Menü (`MenuTheme`). Fenstergrößen von klein bis 4K prüfen.
- Aufruf als Funktion mit Rückgabe über Signal, z. B.
  `MapPicker.open(parent, current, filter)` → `map_chosen(name)`; `filter`
  erlaubt z. B. nur Karten mit genug Starts für die gewählte Spielerzahl
  oder (Editor) auch Testkarten.

### 2.4 Einsatzorte

1. **Einzelspiel** (`main_menu.gd`): Feld „Karte“ wird ein Knopf mit
   Kartenname, der den Picker öffnet; in der freien linken Fensterhälfte
   (bzw. neben dem Formular) die Vorschau der gewählten Karte mit
   Startmarken der gewählten Startaufstellung (Wunsch a). Bei schmalen
   Fenstern Vorschau über/unter dem Formular oder ausblenden.
2. **LAN- und Steam-Lobby**: Host wählt über den Picker; alle Spieler sehen
   die Vorschau der gewählten Karte (nur der Kartenname geht über das Netz,
   jeder rechnet das Bild lokal; fehlt die Karte beim Client, wie bisher
   melden). Lobby-Protokoll nur ändern, wenn nötig.
3. **Map-Editor**: Dialog „Karte öffnen“ nutzt den Picker (mit Vorschau,
   Größe, Startaufstellungen); nach dem Speichern wird das zwischengespeicherte
   Vorschaubild der Karte ungültig. Optional auch beim Speichern unter neuem
   Namen die vorhandenen Karten zeigen (Überschreiben sichtbar machen).
4. Prüfen, ob Missions-/Kampagnenbildschirm die Vorschau sinnvoll zeigen
   kann (Karte der Mission mit Startgebiet); wenn klein, mitnehmen, sonst
   als offen notieren.

## 3. Prüfen

- `just check`; Golden Hash unverändert (reine Darstellung).
- `just selftest`, `just lan-selftest`, `just editor-selftest` laufen und
  wählen ihre Karte jetzt über den Picker (bzw. dessen Funktion); Selbsttest
  prüft, dass Vorschaubild und Startmarken für jede Karte entstehen
  (keine leeren Bilder, keine Fehler bei Karten ohne Varianten).
- Zeiten: Öffnen des Pickers, Erzeugen jedes Vorschaubilds kalt/warm
  (Tabelle), Speicherbedarf des Caches.
- Übersetzungen de/en für alle neuen Texte (`godot/locale/menu.csv`).
- Bilder im Fenster ansehen (Einzelspiel mit Vorschau, Picker, Editor-Dialog,
  Lobby) und dem Nutzer zeigen; vorher/nachher.

## 4. Abschluss

`docs/ARCHITECTURE.md` (Menü/Picker/Vorschau-Cache), ggf. `docs/MAPS.md`
(Vorschaubild, Cache, Exportbefehl), README-Zeile auf „erledigt“ mit
Zahlen und Offenem, Prompt nach `prompts/done/` verschieben, Commit.
