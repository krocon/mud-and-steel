# Prompt 058 – Bilder in der Bau- und Produktionsauswahl

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (HUD aus 031, Basisbau-HUD aus 023), `docs/ASSETS.md` (Galerie).
>
> **Konkretisiert** (siehe unten); ursprünglich: vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Bau- und Produktionsmenü zeigen bisher Textschaltflächen. Nach diesem Prompt sieht man in
der Auswahl rechts ein Bild jeder Einheit und jedes Gebäudes als Kachel mit Name, Kosten,
Personal und Bauzeit; nicht verfügbare Kacheln sind abgedunkelt mit Grund, die Auswahl
ist in Reiter gegliedert (Basis, Verteidigung, Infanterie, Panzer, Geschütze, Luft).

## 2. Umfang

1. **Bilder vorab rendern** (statt zur Laufzeit wie die Porträts aus 031): ein
   Werkzeug (`just thumbnails`) rendert jede Katalog-Einheit und jedes baubare Objekt
   einheitlich (Licht, Winkel von schräg vorn, Hintergrund nach Fraktion/Tarnung) als
   PNG nach `godot/assets/thumbnails/` (Git LFS), aus der Modellgalerie oder Blender
   headless. Die Porträts der Einheitenanzeige (031) nutzen dieselben Bilder, wenn
   vorhanden.
2. **Kacheln:** Bild, Name, Ressourcenkosten, Personal, Bauzeit, Tastenkürzel,
   Warteschlangen-Zähler, Fortschritt als Überlagerung; Tooltip mit Werten aus dem
   Katalog (Panzerung, Waffe, Reichweite).
3. **Reiter** nach Kategorie, scrollbar, Größe passend zu UI-Skalierung (043).
4. **Prüfung:** `just gallery-check` (oder eigener Check) meldet fehlende Bilder.

## 3. Akzeptanzkriterien

1. `just selftest` mit Bauen und Produzieren über die Kacheln grün.
2. Jede baubare Einheit hat ein Bild; Check grün.
3. Doku: HUD-Abschnitt und `docs/ASSETS.md` (Thumbnails).

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach 023, 031, 033, 034): Das Baumenü und die Produktionsleiste
(`godot/scenes/world/base_hud.gd`) waren Textknöpfe „Name  Treibstoff / Munition /
Personal  Zeit“, angedockt über der Befehlsleiste unten links (`command_bar.gd`,
„Build“ öffnet das Baumenü). Die Optionen kommen aus `SimHost.build_options()` /
`production_options()` (Typ, Katalog-Id, Name, Kosten, Zeit, Sperrgrund, Struktur ja/nein);
eine Kategorie fehlte. Die Einheitenanzeige (031) rendert Porträts zur Laufzeit in einem
`SubViewport` (`unit_portrait.gd`). Kaufbar sind 4 Basisgebäude (geliehene Hausmodelle),
9 Stellungen (Bunker, leichte und schwere Flak) und je Fraktion 5 Infanterie- und
9 Fahrzeugwerk-Einträge (inkl. LKW und gezogener Geschütze aus 034) – zusammen 55 Ids.
Die Luftleiste (033) ruft Einsätze ab, sie kauft keine Einheiten. Eine UI-Skalierung (043)
gibt es noch nicht.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Vorab in Godot rendern**, nicht in Blender: `just thumbnails` startet `scenes/tools/thumbnail_render.tscn` mit Fenster und baut jedes Modell wie die Partie (Tarnung Wald, Markierungen, geliehene Häuser, Trupp-Szenen mit Soldatenlook). | Gleiche Shader und Materialien wie im Spiel (Tarnshader und EnvLook gibt es nur in Godot); Godot rendert headless nicht, daher mit Fenster wie die anderen `*-shots`. |
| 2 | **Welche Bilder:** neue statische Bridge-Funktion `SimHost.buildable_ids()` liest Gebäude, Stellungen und Produktionslisten aus `data/economy.toml` (alle Fraktionen). | Datengetrieben: Hindernisse/Minen (035) oder Nachschub-LKW (036) bekommen nach `just thumbnails` automatisch ein Bild, ohne Liste im Werkzeug. |
| 3 | **Bildaufbau [vorläufig]:** 240 × 160 px PNG, transparenter Hintergrund, orthografisch von schräg vorn links, 30° von oben, feste Haupt- und Fülllichter; dreifach gerendert, auf die Modellpixel zugeschnitten und mit 5 % Rand unten stehend eingepasst. Trupps zeigen die ersten fünf Soldaten in fester enger Gruppe. | Einheitlicher Look; Zuschnitt statt Begrenzungsbox, weil Antennen und Rohre sonst viel Leerraum lassen; echte Formationen sind für eine Kachel zu breit. |
| 4 | **Hintergrund nach Fraktion** zeichnet die Kachel (dunkles Grau / Rotbraun / Oliv), nicht das Bild; keine Bilder je Tarnung (nur Wald). | Ein Bild pro Id statt drei; Farbe ist später ohne Neurendern änderbar. |
| 5 | **Kacheln** in neuem `godot/scenes/hud/picker.gd`, von `base_hud.gd` zweimal benutzt (Baumenü, Produktionsleiste); Kachel 116 × 124 px, 4 Spalten, 2 Reihen sichtbar, scrollbar. Inhalt: Bild, Name, „F · A · M“ (Treibstoff, Munition, Personal), Bauzeit, Tastenkürzel, Zähler der Aufträge bzw. eigenen Baustellen, Fortschrittsbalken des ersten; gesperrt abgedunkelt mit Grund. | Gemeinsamer Code für beide Menüs; Aufbau nur bei geänderter Liste neu, Werte werden an Ort und Stelle aktualisiert (kein verlorener Klick). |
| 6 | **Ort:** weiter unten links über der Befehlsleiste angedockt (nicht rechts). Der Ereignis-Feed weicht nach oben aus, wenn die Leiste hoch wird (`hud.gd`). | Rechts unten sitzt die Minikarte, in der Mitte die Einheitenanzeige (031); kein Umbau des HUD-Layouts nötig. |
| 7 | **Reiter [vorläufig]:** Basis (Basisgebäude), Verteidigung (andere platzierbare Strukturen), dann nach Katalog-Gruppe Infanterie, Fahrzeuge, Geschütze, Luft, Sonstiges; leere Reiter verschwinden, ein einzelner Reiter blendet die Leiste aus. „Panzer“ heißt „Vehicles“, weil LKW, Halbketten und Spähwagen dazugehören. | Aus den Daten abgeleitet, keine Liste pro Id. Ein Luft-Reiter erscheint erst, wenn Flugzeuge produzierbar werden; die Luftleiste (033) bleibt Text, weil sie Einsätze abruft. |
| 8 | **Tastenkürzel [vorläufig]:** Alt + 1…9, 0 für die Kacheln des offenen Reiters, bei sichtbarer Produktionsleiste dort, sonst im Baumenü; nicht umbelegbar. | Ziffern sind Kontrollgruppen, Strg/Cmd + Ziffer speichert sie, Shift + 1…4 ist die Luftleiste; fast alle Buchstaben sind Befehle. |
| 9 | **Tooltip** aus neuer statischer `SimHost.catalog_card(id)`: Bezeichnung, Kosten, Besatzung/Truppgröße, Masse, Straßengeschwindigkeit, Sitze, Baujahr, Panzerung Wanne/Turm (vorn/Seite/hinten), Waffen mit Kaliber und **realer** wirksamer Reichweite; bei Basisgebäuden Trefferpunkte, Panzerung, Einkommen, Voraussetzungen. | „Werte aus dem Katalog“: echte Werte statt der umgerechneten Spielwerte (die zeigt die Einheitenanzeige). |
| 10 | **Porträts der Einheitenanzeige** nehmen das Vorschaubild, wenn es eines gibt; sonst wie bisher zur Laufzeit gerendert. | Gleiche Bilder überall, schneller; Szenario-Einheiten ohne Kaufoption behalten ihr Porträt. |
| 11 | **Prüfung** in `just gallery-check`: jede Id aus `buildable_ids()` braucht ein Bild und eine Katalogkarte. | Kein eigener Befehl; die Galerie prüft schon alle Modelle. |
| 12 | **Größe passend zur UI-Skalierung:** feste Pixelmaße als Konstanten in `picker.gd`, Bilder in doppelter Kachelgröße. | 043 gibt es noch nicht; dort skaliert das ganze HUD mit, die Bilder sind scharf genug. |
| 13 | Produktionsleiste für **jedes eigene Gebäude mit Produktionsoptionen**, statt fest Kaserne und Fahrzeugwerk. | Neue produzierende Gebäude (z. B. 036) brauchen keine Änderung. |

## Ergebnis

Umgesetzt wie oben; Details in `docs/ARCHITECTURE.md` (HUD, „Picker“),
`docs/ASSETS.md` („Thumbnails“), `docs/CONTROLS.md` (Reiter, Alt + Ziffer).
55 Bilder in `godot/assets/thumbnails/` (Git LFS, zusammen 1,5 MB).

Tests: `just selftest` öffnet das Baumenü über „Build“, prüft Bild, Kosten und Tooltip
jeder Kachel und Alt + 2, startet die Kaserne über ihre Kachel, produziert über eine
Kachel der Produktionsleiste und prüft den Auftragszähler; `just gallery-check` meldet
fehlende Bilder; `just base-shots <dir>` zeigt Baumenü und Produktionsleiste mit
zwei Aufträgen (Screenshots selbst begutachtet). Golden-Hash unverändert (nur
Darstellung und statische Lesefunktionen in der Bridge).

Offen: Bilder nur mit Waldtarnung; Luftleiste ohne Bilder; Tastenkürzel nicht
umbelegbar und kein Tastenkürzel für den Reiterwechsel; Kachelmaße warten auf die
UI-Skalierung (043); neue Kaufoptionen brauchen nach dem Modell `just thumbnails`
(die Galerie-Prüfung erinnert daran).
