# Prompt 064 – Fahrzeug-Datenbank (JSON)

> In einer frischen Session im Repo-Ordner ausführen, nach der Vorab-Session von 063
> (sie liefert `data/vehicles/index.json` und `tools/vehicle_refs.py`); die
> Modell-Sessions von 063 dürfen parallel laufen. Lies zuerst `CLAUDE.md`,
> `docs/SOURCES.md`, `data/catalog/SCHEMA.md` und `docs/VEHICLE_REFS.md`.

## 1. Ziel

Eine einfache JSON-Datenbank mit den technischen Daten aller Fahrzeuge aus
`docs/.vorlagen/fahrzeuge/` liegt im Repo, mit festem Schema, Quelle je Eintrag und einer
Prüfung gegen den Katalog. Spiel und Einheiten-Webseite lesen sie; Prompt 065 zeigt sie
auf der Webseite, Prompt 066 im Spiel.

## 2. Ausgangslage

- Die Vorlagen (`<NATION>/<Familie>/<Variante>/data.json`, 141 Dateien) sind nicht im
  Repo. Ihre Werte sind Text mit uneinheitlichen Schlüsseln; Einzelheiten in Prompt 063,
  Abschnitt 2.
- `tools/vehicle_refs.py` (aus 063) liest und vereinheitlicht sie bereits;
  `data/vehicles/index.json` ordnet jeden Vorlagen-Ordner einer Katalog-Id zu.
- Der Katalog (`data/catalog/*.toml`) ist die Eingabe der Simulation: streng gelesen,
  nur Ganzzahlen, in die Sim einkompiliert (`include_str!`). Er hält etwa 25 Werte je
  Fahrzeug. Die Vorlagen haben deutlich mehr: Panzerung je Seite mit Winkel für Wanne,
  Aufbau, Turm und Blende, Richtbereich, Turmdrehzeit, Munition, Motor, Getriebe,
  Bodendruck, Watfähigkeit, Grabenüberschreitung, Steigfähigkeit, Wendekreis,
  Stückzahl, Bauzeitraum, Hersteller, Funkgerät.
- Im Workspace gibt es `serde` und `toml`, aber kein `serde_json`.

## 3. Entscheidung zur Rollenverteilung

Der Katalog bleibt die einzige Quelle der Simulation. Die Datenbank ist ein
Nachschlagewerk daneben: mehr Felder, nur Fahrzeuge aus den Vorlagen, nie Eingabe für
eine Spielregel. Braucht eine Regel später einen Wert daraus (z. B. Turmdrehzeit oder
Bodendruck), wandert er als eigenes Feld mit Quelle in den Katalog. So gibt es keine
zweite Wahrheit und keinen Einfluss auf den Golden-Hash.

Abweichungen von dieser Entscheidung vorher mit mir klären.

## 4. Umfang

1. **Dateien:** `data/vehicles/de.json`, `su.json`, `us.json` (Fraktionskürzel wie im
   Katalog) und `data/vehicles/SCHEMA.md` (Englisch). Ein Eintrag je Katalog-Id, sortiert
   nach Id, stabil formatiert (zwei Leerzeichen, feste Schlüsselreihenfolge), damit
   Diffs lesbar bleiben.
2. **Schema** je Eintrag, Vorschlag:
   - `id` (Katalog-Id), `name`, `type` (Typ laut Quelle), `family`.
   - `source`: `name`, `url`, `fetched`, `references` (Literaturkürzel der Quelle).
   - `general`: Bezeichnungen, Hersteller, Stückzahl, Bauzeitraum, Besatzung, Maße,
     Rohrüberstand, Gefechtsgewicht, Funkgerät.
   - `firepower`: Hauptbewaffnung (Name, Kaliber, Kaliberlänge), Munition,
     Seitenrichtbereich, Höhenrichtbereich, Turmdrehzeit für 360°, Zieloptik,
     Nebenbewaffnung mit Munition.
   - `mobility`: Motor, Hubraum, Leistung mit Drehzahl, Leistungsgewicht, Getriebe,
     Kraftstoff, Tankinhalt, Verbrauch, Reichweite Straße/Gelände, Geschwindigkeit
     Straße/Gelände/Wasser, Kettenbreite, Kettenauflage, Kettenglieder, Bodendruck,
     Bodenfreiheit, Wendekreis, Steigfähigkeit, Kletterfähigkeit, Watfähigkeit,
     Grabenüberschreitung.
   - `armor`: je Baugruppe (`hull`, `superstructure`, `turret`, `mantlet`, `gun_shield`,
     Sonderfälle der Mehrturmpanzer als eigene Namen) die Seiten `front`, `side`, `rear`,
     `top_bottom` mit `mm` und `deg`.
   - `notes`: Text der Quelle, falls vorhanden.
3. **Werte:** Zahlen als Ganzzahlen mit Einheit im Feldnamen wie im Katalog (`_mm`,
   `_kg`, `_kmh_x10`, `_deg`); psi in kPa umrechnen und beide ablegen oder die
   Umrechnung im Schema festhalten. Bereiche als `_min`/`_max`. Jeder Wert, der sich
   nicht eindeutig wandeln lässt („6.12 or 5.64*“, „80mm @60°&90°“, „floats“,
   „Manual“), bleibt zusätzlich als Originaltext in einem Feld `text` neben der Zahl
   erhalten. Nichts raten, nichts still weglassen; „n.a.“ und „-“ heißen: Feld fehlt.
4. **Import:** `tools/vehicle_refs.py` schreibt die Datenbank; `just vehicle-db` ruft es
   auf. Es braucht die Vorlagen auf dem Rechner und läuft deshalb nicht in der CI. Am
   Ende ein Bericht: Einträge je Fraktion, Zahl der Warnungen, Liste der Werte, die nur
   als Text vorliegen. Zwei Läufe hintereinander ergeben dieselbe Datei.
5. **Prüfung im Repo** (läuft ohne Vorlagen, Teil von `just check` oder eigenes
   `just vehicle-db-check`, das die CI aufruft):
   - Jede Datei entspricht dem Schema; unbekannte Felder sind Fehler.
   - Jede `id` gibt es im Katalog; jedes Katalog-Fahrzeug aus `index.json` hat einen
     Eintrag.
   - Felder, die es in beiden gibt (Länge, Breite, Höhe, Masse, Besatzung,
     Geschwindigkeit, Reichweite, Leistung, Panzerung vorn/seitlich/hinten, Kaliber,
     Munition), stimmen innerhalb einer Toleranz überein. Jede größere Abweichung ist
     entweder ein Fehler im Katalog (dann korrigieren, mit Quelle) oder eine bewusste
     Abweichung (dann steht das Feld in `uncertain` und der Grund in `note`).
   Die Prüfung dort bauen, wo sie am wenigsten kostet: als `cli`-Unterbefehl, wenn
   `serde_json` dazukommt, sonst als Python-Skript. Eine neue Abhängigkeit vorher mit
   mir abstimmen.
6. **Zugriff für das Spiel:** eine Funktion in der Bridge, die den Eintrag einer
   Katalog-Id als Dictionary liefert (Vorschlag `SimHost.vehicle_record(type_id)`), leer
   für Einheiten ohne Eintrag. Die Daten müssen im exportierten Spiel ankommen
   (`just export-selftest`): entweder in die Bridge einkompiliert wie der Katalog oder
   als Datei im Export; begründet entscheiden. Nicht in `crates/sim`: die Datenbank ist
   keine Spielregel.
7. **Zugriff für die Webseite:** nichts kopieren, `web/units/` liest `data/vehicles/*.json`
   zur Laufzeit wie den Katalog. Die Anzeige selbst ist Prompt 065; hier nur
   sicherstellen, dass `tools/units_web.py` den Ordner ausliefert.
8. **Doku:** `data/vehicles/SCHEMA.md`, Abschnitt in `docs/SOURCES.md` (Datenbank,
   Herkunft, Rang der Quelle), `docs/ARCHITECTURE.md` (Rolle neben dem Katalog),
   `CLAUDE.md` (Zeile bei „Layout“ und der neue Befehl).

## 5. Akzeptanzkriterien

1. `just check` grün, die neue Prüfung läuft in der CI ohne die Vorlagen.
2. Alle Fahrzeuge mit Status `existing` oder `new` aus `docs/VEHICLE_REFS.md` haben einen
   Eintrag; `skipped` hat keinen.
3. Keine Abweichung zwischen Datenbank und Katalog ohne Korrektur oder Begründung;
   die Liste der Korrekturen am Katalog steht im Abschlussbericht.
4. Im Repo liegen nur Fakten: keine Bilder, keine Bilddateinamen als Verweis auf
   fremdes Material, kein kopierter Fließtext außer den kurzen `notes` der Quelle.
5. Golden-Hash unverändert. Katalog-Korrekturen aus Schritt 5 können ihn ändern: dann
   ist das eine beabsichtigte Änderung mit Grund im Commit, je Korrektur einzeln
   nachvollziehbar.
6. `SimHost.vehicle_record("de_tiger1_e")` liefert im Selbsttest die erwarteten Werte,
   auch im exportierten Spiel.

## 6. Offene Fragen

| Frage | Vorschlag |
|---|---|
| Eine Datei oder eine je Fraktion? | Je Fraktion, wie der Katalog; zusammen etwa 300 kB. |
| Fahrzeuge ohne Vorlage (Halbketten, LKW, Spähwagen, Geschütze, Flugzeuge)? | Kein Eintrag; Webseite und Spiel zeigen dann nur die Katalog-Werte. Die Datenbank später aus anderen Quellen zu ergänzen, bleibt möglich: `source` steht je Eintrag. |
| Originaltexte der Quelle zusätzlich komplett ablegen? | Nein, nur dort, wo die Zahl allein nicht eindeutig ist (`text`). |
| Katalog aus der Datenbank erzeugen statt von Hand pflegen? | Nein. Der Katalog hat je Wert eine geprüfte Quelle und Waffenwerte, die in den Vorlagen fehlen; die Prüfung aus Schritt 5 hält beide zusammen. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`bridge/src/lib.rs`, `justfile`, Katalog, Doku) nur klein und
additiv ändern, weil die Modell-Sessions von 063 parallel laufen. Ändert sich der
Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/` verschieben und
`prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde, wie ich es
teste, Zahl der Einträge und Warnungen, Korrekturen am Katalog, was offen ist.
