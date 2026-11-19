# Prompt 095 – HUD neu anordnen und gestalten: Karte unten links, Befehle als Kachelraster

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `godot/scenes/hud/hud.gd` ganz (Anordnung: `_place_unit_card`,
> `_keep_events_above_command_bar`, `_keep_unit_panel_beside_command_bar`, `covers`),
> `godot/scenes/hud/minimap.gd`, `command_bar.gd`, `air_bar.gd`, `unit_panel.gd`,
> `event_feed.gd`, `picker.gd`, in `godot/scenes/world/base_hud.gd` Ressourcenleiste
> und Bau-/Produktionsleiste, `godot/scenes/menu/menu_theme.gd`, in
> `godot/scenes/menu/settings.gd` `ui_scale`, `font_size`, `ui_scale_for()`,
> `godot/scenes/tools/hud_self_test.gd`, `godot/locale/hud.csv`, `docs/CONTROLS.md`
> und die HUD-Abschnitte in `docs/ARCHITECTURE.md`. Prüfe, ob die Prompts **092**
> (größere Befehlsleiste) und **094** (Knöpfe Zentrieren/Folgen im Einheitenpanel)
> schon umgesetzt sind (`git log`, `prompts/done/`), und baue auf ihnen auf statt
> gegen sie.

## 1. Ausgangslage

Das HUD ist über Prompts gewachsen: Befehlsleiste und Luftleiste unten links als
Textknöpfe, Einheitenpanel unten in der Mitte (weicht nach rechts aus), Minimap unten
rechts, Ressourcenleiste oben in der Mitte, Ereignisse am linken Rand, Einheitenkarte
oben rechts. Viel ist dauernd sichtbar, auch wenn nichts ausgewählt ist, und die
Teile schieben sich gegenseitig herum.

Der Nutzer hat drei Bilder aus einem kommerziellen WW2-RTS gezeigt, die ihm gefallen
(abgelegt in `docs/.vorlagen/hud/`, nicht im Repo: `gesamt.webp`,
`oben-rechts.png`, `unten-rechts.png`). Sie sind **nur Vorbild für Anordnung,
Verhalten und Stimmung**. Keine Symbole, Texturen, Schriften, Namen oder Grafiken
daraus übernehmen (`docs/SOURCES.md`, „Stil-Referenz“ in `prompts/README.md`).

Was die Bilder zeigen:

- **Unten links: Minimap**, quadratisch mit dunklem Rahmen, oben ein „N“ als
  Nordmarke. Rechts daneben eine schmale senkrechte Spalte mit drei kleinen
  quadratischen Knöpfen (Zoom/Lupe, Kamera nach Norden drehen, Pause). Eigene
  Einheiten und Gebäude als kleine Symbole in Teamfarbe, Sichtfeld der Kamera als
  Rahmen.
- **Unten Mitte: Infofeld der Auswahl**, nur wenn etwas ausgewählt ist. Breites,
  flaches, dunkles, leicht durchscheinendes Feld; oben links der Name in einer
  Serifenschrift, oben rechts die Kategorie („Gebäude“) mit kleinem Symbol; darunter
  links ein Bild der Einheit, rechts daneben Werte mit Symbol (z. B. Zustand
  9915/10000). Im Hintergrund sehr blass das Hoheitszeichen der Nation.
- **Unten rechts: Befehlsraster** 5 × 3 gleich große, dunkle, quadratische Kacheln
  mit dünnem hellerem Rand. Belegte Kacheln zeigen nur ein Symbol (kein Text), ein
  kleiner Punkt an der Ecke markiert einen Zustand (z. B. Dauerbefehl an). Leere
  Kacheln bleiben als dunkle Fläche stehen, das Raster springt also nicht.
  Rechts unten überlappend ein großes rundes Nationsabzeichen.
- **Oben rechts: Ressourcen** in einer kompakten dunklen Leiste (Truppenstärke
  „200/200“, rot am Limit; Punkte mit Münzsymbol), daneben „?“ (Hilfe) und ein
  Menüknopf. **Darunter eine senkrechte Spalte Unterstützung** (Abwurf, Aufklärer,
  Jäger, Bomber, Fallschirmjäger …): quadratische Symbolkacheln, darunter ein kleines
  Feld mit Kosten (gold) bzw. verbleibenden Einsätzen (blau).
- **Oben Mitte: Gruppen** als kleine Reiter „1“, „2“ – nur die belegten Gruppen.
- Auf der Karte: die Grundfläche des ausgewählten Gebäudes als Umriss in Teamfarbe,
  Reichweiten als gestrichelte Kreise.

Der Nutzer vermutet, dass die übrigen Menüs nur bei Bedarf erscheinen. Das ist auch
das Ziel hier: **Minimap und Ressourcen immer, alles andere nur, wenn es etwas zu
tun gibt.**

## 2. Ziel

### 2.1 Anordnung

| Ort | Teil | Sichtbar |
|-----|------|----------|
| unten links | Minimap mit Nordmarke und Knopfspalte (Zoom/Ansicht, Norden, Pause) | immer |
| unten Mitte | Infofeld der Auswahl (heutiges Einheitenpanel, neu gestaltet) | nur mit Auswahl |
| unten rechts | Befehlsraster 5 × 3 (heutige Befehlsleiste) | nur mit eigener Auswahl, die Befehle hat |
| oben rechts | Ressourcenleiste, Hilfe, Menü | immer |
| rechts, unter den Ressourcen | Unterstützungsspalte (heutige Luftleiste) | wenn der Spieler Unterstützung hat |
| oben Mitte | Gruppenreiter | nur belegte Gruppen |
| links | Ereignisse | wie heute, rückt nicht mehr in die Minimap |

- Bau- und Produktionsleiste (`base_hud.gd`, Picker aus Prompt 058/066) erscheinen
  bei Auswahl eines Gebäudes **im Befehlsraster bzw. direkt darüber**, nicht mehr als
  eigener Block links. Prüfen, ob das Raster dafür eine zweite Seite oder ein
  aufklappendes Feld braucht, und eine Lösung wählen.
- Die Einheitenkarte (Shift + I) bleibt ein großes Fenster, liegt aber nicht über
  Unterstützungsspalte und Ressourcen.
- Missions-HUD und Untertitel dürfen nichts davon verdecken.
- Kein Teil überlappt einen anderen, auch nicht bei 1280 × 720, großer UI-Skala oder
  großer Schrift. Die Ausweichlogik in `hud.gd` wird dadurch einfacher (feste Ecken),
  nicht komplizierter. Was danach unnötig ist, entfernen.

### 2.2 Befehlsraster

- 5 Spalten × 3 Zeilen fester Kacheln; die Befehle der Auswahl liegen auf **festen
  Plätzen** (z. B. obere Zeile Bewegung, mittlere Zeile Kampf, untere Zeile Haltung
  und Sonderbefehle), damit die Hand sie blind findet. Nicht verfügbare Befehle:
  Kachel leer oder ausgegraut, nach einer klaren Regel.
- Heute gibt es mehr als 15 Befehle (Formation, Move, Attack, Ground, Patrol,
  Advance, Secure, Stop, Hold fire, Occupy, Barrage, Smoke, Dismount, Unlimber,
  Stand/Kneel/Prone, Auto, Gebäude beschießen aus 093 …). Lösung finden: nur die zur
  Auswahl passenden zeigen, Haltung als eine Kachel mit Untermenü, oder eine zweite
  Seite. Begründen und in `docs/CONTROLS.md` festhalten.
- Jede Kachel hat ein **eigenes Symbol** (selbst gezeichnet als SVG oder per Skript
  erzeugt, einheitlicher Strich, hell auf dunkel, Akzentfarbe für Zustände). Ablage
  z. B. `godot/assets/ui/icons/`. Keine Symbole aus anderen Spielen oder
  unfreien Sammlungen.
- Tooltip mit Name, Kurzbeschreibung und Taste (wie heute). Die Taste klein in einer
  Ecke der Kachel. Dauerzustände (Feuer halten, Auto, Haltung) als Punkt/Rahmen.
- Alle Tastenkürzel wirken weiter, unabhängig davon, ob das Raster sichtbar ist.

### 2.3 Infofeld der Auswahl

- Eine Einheit oder ein Gebäude: Name (Serifenschrift aus dem Spiel, falls
  vorhanden, sonst eine freie, z. B. OFL), Kategorie mit Symbol, Vorschaubild
  (`godot/assets/thumbnails/<id>.png`), Zustand, Munition/Besatzung/Rang wie heute im
  Einheitenpanel, kompakt mit Symbolen statt langer Texte. Knöpfe aus 094
  (Zentrieren, Folgen) und „Werte“ (Einheitenkarte) bleiben.
- Mehrere: Kacheln wie heute, aktive Einheit hervorgehoben (094).
- Hintergrund: sehr blass das Hoheitszeichen der eigenen Nation (Balkenkreuz,
  Roundel, Stern, roter Stern – nur erlaubte Zeichen, `docs/SOURCES.md`).
- Feindliche oder neutrale Auswahl: Infofeld ja, Befehlsraster nein.

### 2.4 Minimap unten links

- Nordmarke „N“ am Rand, die sich mitdreht, wenn die Kamera gedreht ist (oder die
  Minimap dreht mit – entscheiden, was zur Kamera passt, und begründen).
- Knopfspalte rechts daneben: Kamera nach Norden ausrichten (Taste N), Ansicht/
  Zoomstufe der Minimap oder Kamera-Zoom zurücksetzen, Pause (nur wo Pause erlaubt
  ist: Einzelspieler, Replay). Je ein Symbol, Tooltip mit Taste.
- Klick, Ziehen, Rechtsklick (Befehl auf der Minimap) wie heute.

### 2.5 Ressourcen und Unterstützung oben rechts

- Ressourcenleiste mit Symbol + Zahl je Ressource (heute Treibstoff, Munition,
  Truppenstärke, Einkommen – Inhalt aus `base_hud.gd` übernehmen, nicht ändern), rot
  bei Limit/Mangel, Einkommen als Tooltip oder kleine Zahl. Daneben „?“ (öffnet die
  Steuerungshilfe, falls es eine gibt; sonst nur Menü) und der Menüknopf (Esc).
- Unterstützungsspalte: je Luftunterstützung eine quadratische Symbolkachel,
  darunter Kosten bzw. Einsätze; ausgegraut, wenn zu teuer oder in Abklingzeit
  (Abklingzeit als dunkler Kreissektor oder Balken über der Kachel). Kürzel
  Shift + Ziffer bleiben. Der Hinweis „Air support: left click the target …“
  erscheint nur, während ein Ziel gewählt wird.

### 2.6 Gestaltung

- Eine gemeinsame Gestaltung für alle HUD-Teile (`HudTheme` oder Erweiterung von
  `menu_theme.gd`): dunkle, leicht durchscheinende Flächen (Kartenbild scheint
  schwach durch), dünner heller Rand, kleine Innenabstände, Gold für Kosten und
  Akzente, Rot für Mangel, Teamfarbe für Auswahl. Keine bunten Standard-Godot-Knöpfe.
- Alles wächst mit UI-Skala / HUD-Größe (092) und Schriftgröße; Mindestgrößen so, dass
  es auf Retina und 4K lesbar bleibt.
- Einblenden/Ausblenden von Infofeld und Befehlsraster kurz und weich (≤ 0,15 s),
  ohne dass sich andere Teile verschieben.

## 3. Regeln und Technik

- Reine Darstellung und Bedienung: nichts in `crates/sim`, Golden Hash unverändert,
  keine neuen `Command`s. Befehle laufen weiter über die bestehenden Funktionen der
  Befehlsleiste.
- `covers()` in `hud.gd` muss alle neuen Teile kennen; Klicks auf HUD dürfen keine
  Auswahl oder Befehle auf der Karte auslösen. Unsichtbare Teile fangen keine Klicks.
- Knöpfe mit `focus_mode = FOCUS_NONE`.
- Texte in `godot/locale/hud.csv` (de/en); bestehende Schlüssel weiterverwenden.
- Karteneditor, Replay, Zuschauen, LAN/Online und Missionen prüfen: dort fehlende
  Teile (keine Ressourcen, keine Befehle) einfach ausblenden.
- Die Screenshot-Werkzeuge (`l10n_shots.gd`, `base_shots.gd`, Trailer) nicht kaputt
  machen; ggf. anpassen.

## 4. Vorgehen

1. **Bestand aufnehmen:** Vorher-Bilder (1920 × 1080, 1280 × 720, Retina) mit und
   ohne Auswahl, mit Gebäude-Auswahl, mit Luftunterstützung. Liste aller HUD-Teile,
   wann sie heute sichtbar sind und wer wen verschiebt.
2. **Plan:** Skizze (Text oder Bild) der neuen Anordnung und der Belegung des
   Befehlsrasters; Entscheidungen zu Haltung, Bau-/Produktionsleiste und Minimap-
   Drehung. Kurz mit dem Nutzer abstimmen, bevor Symbole gezeichnet werden.
3. Gemeinsame Gestaltung (Theme) bauen.
4. Minimap nach unten links mit Knopfspalte; Ressourcen nach oben rechts;
   Unterstützungsspalte darunter.
5. Befehlsraster mit Symbolen unten rechts; Bau-/Produktionsleiste einhängen.
6. Infofeld unten Mitte; Gruppenreiter oben Mitte.
7. Sichtbarkeitsregeln, Ein-/Ausblenden, Aufräumen der alten Ausweichlogik.
8. Tests in `hud_self_test.gd` / `self_test.gd` ergänzen, nichts abschwächen:
   - ohne Auswahl: nur Minimap, Ressourcen (und Unterstützung, falls vorhanden) sichtbar;
   - eigene Einheit ausgewählt: Infofeld und Raster sichtbar, Kacheln auf ihren festen
     Plätzen, Klick auf eine Kachel löst denselben Befehl aus wie die Taste;
   - feindliche Einheit: Infofeld ja, Raster nein;
   - Gebäude: Bau-/Produktionsangebot erreichbar;
   - keine zwei sichtbaren HUD-Teile überlappen sich bei 1280 × 720 und 3840 × 2160,
     UI-Skala min/max, Schrift max;
   - Klick auf HUD-Teile erzeugt keine Befehle (`covers`).
   `just selftest`, `just lan-selftest`, `just editor-selftest` und `just check` grün.
9. Doku: `docs/CONTROLS.md` (wo was liegt, Rasterbelegung), `docs/ARCHITECTURE.md`
   (HUD-Aufbau), `docs/ASSETS.md` (UI-Symbole: Herkunft, Format, Größe).
10. Nachher-Bilder in denselben Lagen wie in Schritt 1 für den Nutzer; er probiert es
    danach selbst mit `just run` aus.

## 5. Nicht Teil dieses Prompts

- Neue Ressourcen, neue Unterstützungsarten oder geänderte Kosten.
- Änderungen an Befehlen, Auswahlregeln oder der Simulation.
- Umriss der Gebäudegrundfläche und Reichweitenkreise auf der Karte (eigener Prompt,
  falls gewünscht).
- Hauptmenü und Einstellungsfenster.
