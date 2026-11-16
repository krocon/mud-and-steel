# Prompt 066 – Einheitenwerte im Spiel anzeigen

> In einer frischen Session im Repo-Ordner ausführen, nach Prompt 065. Lies zuerst
> `CLAUDE.md`, `docs/ARCHITECTURE.md` (HUD aus 031, Picker aus 058, „Unit stats“ aus
> 065), `docs/CONTROLS.md` und die Konkretisierung in
> `prompts/done/043-sprache-barrierefreiheit.md`.

## 1. Ziel

Wer im Spiel eine Einheit bauen will oder ausgewählt hat, sieht, was sie kann: die fünf
Kennzahlen aus Prompt 065 (Tempo, Schutz, gegen Fahrzeuge, gegen Infanterie, gegen
Bauten) auf einen Blick und auf Wunsch eine Einheitenkarte mit allen Spielwerten und den
technischen Daten. Dieselben Zahlen wie auf der Webseite, aus derselben Rechnung.

## 2. Ausgangslage

- Der Picker (`godot/scenes/hud/picker.gd`, Prompt 058) zeigt Kacheln mit Bild, Name,
  Kosten, Bauzeit und Tastenkürzel. Der Tooltip listet wenige Katalog-Werte aus
  `SimHost.catalog_card` (`crates/bridge/src/catalog_funcs.rs`): Masse, Geschwindigkeit,
  Panzerung, Waffen mit Kaliber und Reichweite.
- Die Einheitenanzeige (`unit_panel.gd`, `unit_portrait.gd`) zeigt zur Auswahl Porträt,
  Zustand, Munition, Kraftstoff, Rang.
- Spielwerte und Kennzahlen rechnet seit 065 `sim::stats`; `cli unit-stats` gibt sie
  für die Webseite aus. Die technischen Daten liefert seit 064
  `SimHost.vehicle_record(type_id)`.
- Texte laufen über Übersetzungsschlüssel (Deutsch/Englisch, `tools/l10n_check.py`),
  die Oberfläche skaliert, Farben folgen der gewählten Palette (043).

## 3. Umfang

1. **Bridge:** `SimHost.unit_stats(type_id)` liefert die Wertekarte aus `sim::stats` als
   Dictionary (hier dürfen Floats entstehen), einmal je Typ berechnet und gepuffert.
   Keine eigene Rechnung in der Bridge oder in GDScript.
2. **Kennzahlen im Picker:** jede Kachel bekommt fünf kleine Balken oder ein kompaktes
   Zeichen je Kennzahl, lesbar bei UI-Skalierung 1 und in allen Farbpaletten (Form oder
   Beschriftung, nicht nur Farbe). Gebäude und Stellungen ohne Kampfwerte zeigen nur,
   was sie haben.
3. **Tooltip der Kachel:** ersetzt den bisherigen. Name, Bezeichnung, Baujahr, die fünf
   Kennzahlen mit Zahl, dazu höchstens acht Zeilen: Geschwindigkeit Straße/Gelände,
   Panzerung Front/Seite/Heck, Hauptwaffe mit Durchschlag auf 500 m und Reichweite im
   Spiel, Wirkung gegen Infanterie und Bauten in einem Wort (stark, mittel, schwach,
   keine), Besatzung oder Truppstärke, Sitzplätze. Eine Zeile „gut gegen … / verwundbar
   durch …“, abgeleitet aus der Vergleichsmatrix von 065.
4. **Einheitenkarte:** ein Fenster, das über die Kachel (Rechtsklick oder Taste, siehe
   „Offene Fragen“) und über einen Knopf in der Einheitenanzeige aufgeht:
   - Kopf: Bild aus `godot/assets/thumbnails/`, Name, Bezeichnung, Dienstjahre, Kosten.
   - Reiter „Spielwerte“: alle Gruppen aus 065 (Bewegung, Empfindlichkeit, gegen
     Fahrzeuge, gegen Infanterie, gegen Bauten, Sonstiges), Durchschlag über Entfernung,
     Matrix der Vergleichswaffen und Vergleichsziele.
   - Reiter „Technische Daten“: die Datenbank-Werte, wenn es einen Eintrag gibt
     (Panzerung je Baugruppe, Motor, Fahrwerk, Stückzahl, Bauzeitraum), mit Quelle.
   - Das Spiel läuft weiter, solange die Karte offen ist (Mehrspieler: keine Pause);
     Escape schließt sie; sie verdeckt weder Minikarte noch Befehlsleiste.
5. **Ausgewählte Einheit:** in der Einheitenanzeige die fünf Kennzahlen kompakt neben dem
   Porträt. Bei eigenen Einheiten mit Rang (050) zusätzlich die Boni der Stufe als
   eigene Zeile, nicht in die Grundwerte eingerechnet. Bei gegnerischen Einheiten nur
   Typwerte, nie der aktuelle Zustand jenseits dessen, was der Nebel des Krieges zeigt.
6. **Nachschlagewerk im Hauptmenü:** ein Eintrag „Einheiten“, der alle Katalog-Einheiten
   nach Fraktion und Kategorie listet und dieselbe Einheitenkarte öffnet, hier mit
   drehbarem 3D-Modell statt Bild (Darstellung aus `scenes/tools/model_gallery` wieder-
   verwenden) und Tarnungswahl. Zeigt auch Einheiten, die noch nicht baubar sind (die
   Fahrzeuge aus 063), als solche markiert.
7. **Texte:** alle Beschriftungen als Übersetzungsschlüssel in Deutsch und Englisch,
   Zahlenformat und Einheiten nach Sprache; `tools/l10n_check.py` grün. Keine festen
   Texte in GDScript.
8. **Selbsttests:** `hud_self_test.gd` erweitern: Tooltip und Karte gehen für jede
   baubare Id auf, zeigen Werte, keine fehlenden Schlüssel; die Karte schließt mit
   Escape und fängt keine Spielbefehle ab. Ein Werkzeug für Bildschirmfotos wie
   `scenes/tools/l10n_shots.gd` (Picker mit Kennzahlen, Tooltip, Karte je Reiter, in
   beiden Sprachen und zwei UI-Skalierungen).
9. **Doku:** `docs/CONTROLS.md` (neue Taste, Rechtsklick), `docs/ARCHITECTURE.md` (HUD),
   `docs/GAME_DESIGN.md` (was die Kennzahlen bedeuten, in Spielersprache).

## 4. Nicht in diesem Prompt

- Keine neue Regel, kein neuer `Command`, keine Änderung an KI oder Wirtschaft.
- Keine Trefferwahrscheinlichkeit oder Durchschlags-Vorhersage am Mauszeiger beim
  Zielen; das wäre eine eigene Entscheidung zur Spielhilfe (siehe „Offene Fragen“).

## 5. Akzeptanzkriterien

1. `just check` und `just selftest` grün, ebenso `just lan-selftest` (die Karte darf im
   Lockstep nichts verändern) und `just export-selftest` (Datenbank und Werte kommen im
   exportierten Spiel an).
2. Golden-Hash unverändert.
3. Für jede baubare Id stimmen die Zahlen im Spiel mit `cli unit-stats` überein
   (Selbsttest vergleicht Stichproben über die Bridge).
4. Der Picker bleibt bei 30 Kacheln flüssig: Wertekarten werden einmal je Typ geholt,
   nicht je Bild; `just bench-view` hält das Budget.
5. Alles lesbar bei UI-Skalierung 0,8 bis 1,5 und in jeder Farbpalette; nichts ragt aus
   dem Fenster bei 1280 × 720.
6. Bildschirmfotos aus Schritt 8 im Abschlussbericht.

## 6. Offene Fragen

| Frage | Vorschlag |
|---|---|
| Wie geht die Karte auf? | Rechtsklick auf die Kachel und eine Taste für die ausgewählte Einheit (Vorschlag `I`); vorher in `docs/CONTROLS.md` auf Kollisionen prüfen. |
| Fünf Balken auf jeder Kachel oder nur im Tooltip? | Auf der Kachel, wenn sie bei Skalierung 1 lesbar bleiben; sonst nur die stärkste und schwächste Kennzahl als Zeichen und der Rest im Tooltip. Mit Bildschirmfoto entscheiden. |
| Technische Daten im laufenden Spiel oder nur im Nachschlagewerk? | In beiden; im Spiel als zweiter Reiter, damit die Karte kurz bleibt. |
| Vorhersage beim Zielen (durchschlägt die Front ja/nein)? | Nicht hier. Als Vorschlag für später notieren, mit der Frage, ob das im Mehrspieler erwünscht ist. |
| Nachschlagewerk auch für Gebäude und Umweltobjekte? | Gebäude und Stellungen ja (Kosten, Trefferpunkte), Umweltobjekte nein. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`bridge/src/lib.rs`, `main.gd`, `hud.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Am Ende nach `prompts/done/` verschieben und `prompts/README.md` aktualisieren.
Abschlussbericht: was gebaut wurde, wie ich es teste, Bildschirmfotos, was offen ist.
