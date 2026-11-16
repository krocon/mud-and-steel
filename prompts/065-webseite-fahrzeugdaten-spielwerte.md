# Prompt 065 – Webseite: Fahrzeugdaten und Spielwerte

> In einer frischen Session im Repo-Ordner ausführen, nach Prompt 064. Lies zuerst
> `CLAUDE.md`, `web/units/README.md`, `data/vehicles/SCHEMA.md`, `docs/SCALE.md` und in
> `docs/ARCHITECTURE.md` die Abschnitte „Combat“, „Destruction“ und „Indirect fire and
> smoke“.

## 1. Ziel

Die Detailansicht der Einheiten-Webseite (`just units-web`) zeigt zu jeder Einheit zwei
neue Blöcke:

1. **Technische Daten** aus der Fahrzeug-Datenbank (`data/vehicles/*.json`, Prompt 064).
2. **Spielwerte:** was die Einheit im Spiel tatsächlich kann – Höchstgeschwindigkeit,
   Empfindlichkeit, Stärke gegen Fahrzeuge, gegen Infanterie, gegen Mauern und Gebäude,
   Reichweite, Sicht, Verbrauch –, berechnet von der Simulation selbst.

Dazu fünf Kennzahlen je Einheit (Tempo, Schutz, gegen Fahrzeuge, gegen Infanterie, gegen
Bauten), die Prompt 066 im Spiel wiederverwendet.

## 2. Ausgangslage

- `web/units/app.js` (`renderSheet`) zeigt die realen Katalog-Werte, eine Panzerungs-
  Tabelle (Wanne und Turm, Front/Seite/Heck), die Bewaffnung, Quellen und die Kosten im
  Spiel. Die Kosten kommen aus `cli economy-table`, das `tools/units_web.py` nach
  `build/units-web/economy.md` schreibt. Felder, die die Seite nicht kennt, erscheinen
  unter ihrem Katalog-Namen.
- Die Seite hat keinen Build-Schritt und keine Datenkopie: Sie liest Katalog, Wirtschaft
  und Modelle zur Laufzeit aus dem Repo.
- Spielwerte gibt es bisher nur in den Regeln und im Code, nirgends als Übersicht:
  - `data/combat.toml`: Durchschlag aus Kaliber und Mündungsgeschwindigkeit
    (`muzzle_penetration_mm`, `penetration_at` in `crates/sim/src/combat.rs`), Verlust
    je km, Hohlladung, wirksame Plattendicke aus Neigung, Trefferpunkte (`max_hp`),
    Schaden je Treffer gegen Fahrzeuge und Infanterie, Streuung, Turmdrehung je Kategorie.
  - `data/destruction.toml`: Trefferpunkte von Mauern, Bäumen und Gebäuden,
    Mindestkaliber, ab welcher Masse ein Fahrzeug etwas überrollt.
  - `data/indirect.toml`: Splitter und Druck, Streuung, Mindestreichweite.
  - `data/scale.toml`: Umrechnung realer Werte in Spielwerte (Zeitfaktor 2,
    gestauchte Reichweiten).
  - `data/vision.toml`, `data/logistics.toml`, `data/terrain.toml`, `data/transport.toml`,
    `data/economy.toml`: Sicht, Verbrauch, Geländefaktoren, Sitzplätze, Kosten.
- Zuerst prüfen, was davon schon irgendwo ausgegeben wird (`cli`-Unterbefehle,
  `catalog_doc`, `SimHost.catalog_card`, Tests), und nur ergänzen, was fehlt.

## 3. Grundsatz

**Keine zweite Rechnung.** Jeder Spielwert kommt aus den Funktionen und Regeln der
Simulation. JavaScript rechnet nichts nach, es formatiert nur. Ändert sich eine Regel in
`data/*.toml`, stimmt die Webseite nach dem nächsten Start von selbst. Es entstehen
keine neuen Spielregeln; fehlt der Sim die Grundlage für einen gewünschten Wert, wird er
nicht erfunden, sondern im Bericht gemeldet (Abschnitt 4, Schritt 6).

## 4. Umfang

1. **Technische Daten anzeigen** (nur Einheiten mit Datenbank-Eintrag):
   - Gruppen wie in der Datenbank: Allgemein, Feuerkraft, Beweglichkeit, Panzerung.
   - Panzerung als Tabelle je Baugruppe (Wanne, Aufbau, Turm, Blende, Schild) mit
     Front/Seite/Heck/Dach-Boden, je Zelle Dicke und Winkel. Sie ersetzt die bisherige
     kleine Tabelle, wenn es den Eintrag gibt; sonst bleibt die alte.
   - Werte mit Originaltext (`text`) zeigen Zahl und Text; Quelle mit Link und
     Abrufdatum unter dem Block.
   - Wo Katalog und Datenbank denselben Wert führen, nur einmal zeigen (Katalog gewinnt)
     und eine Abweichung kennzeichnen, mit dem Grund aus `note`.
   - Deutsche Beschriftungen an einer Stelle; unbekannte Felder erscheinen unter ihrem
     Schlüssel, damit neue Felder nicht verloren gehen.
2. **Spielwerte berechnen:** ein Modul in `crates/sim` (Vorschlag `stats.rs`), reine
   Funktionen ohne Zustand, nur `Fixed` und Ganzzahlen, die aus Katalog-Eintrag und
   Regeln eine Wertekarte bauen. Es ändert weder den Spielzustand noch den Hash. Je
   Einheit, soweit die Kategorie es hergibt:
   - **Bewegung:** Höchstgeschwindigkeit Straße und Gelände als Spielwert (m/s im Spiel
     und km/h real), im Wasser, Wende- und Turmdrehgeschwindigkeit, Faktor in Schlamm
     und am Hang, überrollt Zäune/Draht/Mauern ja oder nein.
   - **Empfindlichkeit:** Trefferpunkte; wirksame Panzerung Front/Seite/Heck für Wanne
     und Turm (mit Neigung); gegen eine feste Reihe von Vergleichswaffen aus dem Katalog
     (Gewehr, MG, 3,7 cm, 5,7 cm, 7,5 cm, 8,8 cm, Panzerfaust-Klasse) je Seite: bis zu
     welcher realen Entfernung sie durchschlägt und wie viele Treffer bis zum Ausfall
     nötig sind; Verhalten gegen Splitter, Feuer und Minen, soweit es Regeln gibt.
   - **Gegen Fahrzeuge:** Durchschlag auf 100, 500, 1000 und 1500 m real, Schaden je
     durchschlagendem Treffer, Schuss je Minute, Treffer bis zum Ausfall eines
     Fahrzeugs, welche Vergleichsziele (leichter, mittlerer, schwerer Panzer der
     Gegenseiten) frontal und seitlich auf 500 m zu knacken sind.
   - **Gegen Infanterie:** Schaden je Treffer, Splitterradius der Sprenggranate, Zahl
     und Feuerrate der MGs, Niederhalte-Wirkung, soweit geregelt.
   - **Gegen Mauern und Gebäude:** Treffer bis zum Einsturz für feste Vergleichsobjekte
     aus `data/destruction.toml` (Trockenmauer, Bauernhaus, Steinhaus, Baum), oder
     „wirkungslos“ unter dem Mindestkaliber.
   - **Sonstiges:** Reichweite im Spiel (direkt und indirekt, mit Mindestreichweite),
     Sichtweite, Munitionsvorrat in Schuss und Gefechtsminuten, Kraftstoffverbrauch,
     Sitzplätze, Kosten und Bauzeit.
3. **Kennzahlen 0–100:** Tempo, Schutz, gegen Fahrzeuge, gegen Infanterie, gegen Bauten.
   Jede aus den Werten von Schritt 2 mit einer einfachen, dokumentierten Formel und
   festen Bezugswerten (nicht relativ zum stärksten Fahrzeug im Katalog, damit sich die
   Zahlen nicht verschieben, wenn Einheiten dazukommen). Formeln und Bezugswerte in
   einer Datendatei oder einem klar markierten Block, nicht über den Code verstreut.
   Sie sind Anzeige, keine Regel: Die KI und die Sim benutzen sie nicht.
4. **Ausgabe:** `cli unit-stats` schreibt alle Wertekarten als JSON (Ganzzahlen, Einheit
   im Feldnamen, stabil sortiert); `tools/units_web.py` legt sie beim Start nach
   `build/units-web/stats.json`, wie die Preistabelle. Braucht die Ausgabe `serde_json`,
   die Abhängigkeit vorher mit mir abstimmen (siehe Prompt 064) oder das JSON von Hand
   schreiben. `SCHEMA` der Ausgabe kurz in `web/units/README.md`.
5. **Spielwerte anzeigen:**
   - Oben in der Detailansicht die fünf Kennzahlen als Balken mit Zahl.
   - Darunter der Block „Spielwerte“ in den Gruppen aus Schritt 2; reale und
     Spiel-Werte nebeneinander, wo es beide gibt (38 km/h real, 21 m/s im Spiel).
   - Durchschlag über Entfernung als kleine Tabelle, die Vergleichswaffen und
     Vergleichsziele als Matrix (durchschlägt ja/nein, Treffer bis zum Ausfall).
   - Ein Hinweis, dass Regeln und damit Spielwerte vorläufig sind, solange Prompt 045
     (Balancing) offen ist.
   - Liste links: Sortieren nach jeder Kennzahl; Filter und Suche bleiben, wie sie sind.
   - Fehlt `stats.json`, zeigt die Seite einen Hinweis wie heute bei den Preisen.
   - Für Einheiten ohne Kampfwerte (LKW, Sanitäter, Umweltobjekte) nur die passenden
     Gruppen; nichts mit Null füllen.
6. **Lückenliste im Abschlussbericht:** Tabelle der Datenbank-Felder, die eine Spielregel
   stützen könnten, die es heute nur pauschal oder gar nicht gibt (Turmdrehzeit je
   Fahrzeug statt je Kategorie, Bodendruck für Schlamm, Watfähigkeit, Steigfähigkeit,
   Grabenüberschreitung, Panzerungswinkel aller Seiten, Rückwärtsgang), mit Aufwand und
   Wirkung auf den Golden-Hash. Nur auflisten, nichts davon umsetzen.
7. **Impressum:** so klein und unauffällig, wie es rechtlich geht (§ 5 DDG; die DSGVO
   regelt die Datenschutzerklärung, nicht das Impressum):
   - Ein kleiner Link „Impressum“ in der Fußzeile jeder Ansicht, gedämpfte Farbe, kleine
     Schrift. Er muss von jeder Seite aus mit einem Klick erreichbar und klar so benannt
     sein; nicht hinter Menüs, Hover oder Scroll-Tricks verstecken.
   - Der Klick öffnet einen schlichten Block (Dialog oder eigene Ansicht) mit genau
     diesen Angaben, als Text (nicht als Bild), E-Mail als `mailto:`-Link:
     ```
     Marc Kronberg
     Herrnwaldstr. 11
     61462 Königstein im Taunus
     E-Mail: krocon@gmail.com
     ```
   - Die Angaben stehen an genau einer Stelle (z. B. `web/units/impressum.html` oder ein
     Block in `index.html`), nicht in mehreren Dateien.
   - Keine weiteren Rechtstexte außer der Datenschutzerklärung (Schritt 8).
8. **Datenschutzerklärung** (Art. 13 DSGVO), ebenso schlicht wie das Impressum:
   - Daneben in der Fußzeile ein zweiter kleiner Link „Datenschutz“, gleiche Optik und
     gleiche Regeln (jede Ansicht, ein Klick, klar benannt). Text an genau einer Stelle.
   - **Erst prüfen, dann schreiben:** Alle externen Anfragen der Seite auflisten
     (Schriften, CDNs, Bilder, Skripte, Analyse), dazu Cookies und `localStorage`.
     Ziel ist eine Seite, die nichts an Dritte schickt: externe Schriften und
     Bibliotheken in `web/units/` selbst ablegen (Lizenz prüfen und nennen), kein
     Tracking, keine Cookies. Was sich nicht vermeiden lässt, vor dem Ändern mit mir
     abstimmen.
   - Der Text beschreibt nur, was die Seite danach wirklich tut, kurz und auf Deutsch:
     1. Verantwortlicher: die Angaben aus dem Impressum.
     2. Hosting und Server-Logs (IP-Adresse, Zeitpunkt, abgerufene Datei, Browser):
        Zweck Betrieb und Sicherheit, Rechtsgrundlage Art. 6 Abs. 1 lit. f DSGVO,
        Speicherdauer laut Anbieter. Der Anbieter steht noch nicht fest: Platzhalter
        `[Hosting-Anbieter, Anschrift, Speicherdauer der Logs]` setzen und mich im
        Bericht danach fragen.
     3. Keine Cookies, kein Tracking, keine Analyse; `localStorage` nur, falls die Seite
        es nutzt, mit Zweck (z. B. gewählter Filter) und dem Hinweis, dass es den
        Rechner nicht verlässt.
     4. Kontakt per E-Mail: Verarbeitung nur zur Beantwortung, Art. 6 Abs. 1 lit. b
        bzw. f DSGVO, Löschung nach Erledigung.
     5. Externe Dienste: keine, oder die nach der Prüfung verbliebenen mit Anbieter,
        Zweck, Rechtsgrundlage und ggf. Drittlandübermittlung.
     6. Rechte: Auskunft, Berichtigung, Löschung, Einschränkung, Datenübertragbarkeit,
        Widerspruch (Art. 15–21 DSGVO) und Beschwerde bei einer Aufsichtsbehörde, z. B.
        dem Hessischen Beauftragten für Datenschutz und Informationsfreiheit.
     7. Stand (Datum).
   - Keine Textbausteine aus Generatoren oder fremden Seiten kopieren; nichts
     aufnehmen, was die Seite nicht tut. Im Bericht vermerken, dass der Text kein
     Rechtsrat ist und vor dem Veröffentlichen geprüft werden sollte.
9. **Doku:** `web/units/README.md` (neue Quellen, Grenzen), `docs/ARCHITECTURE.md`
   (Abschnitt „Unit stats“: woher jeder Wert kommt, Formeln der Kennzahlen), `CLAUDE.md`
   nur, wenn ein neuer `just`-Befehl dazukommt.

## 5. Akzeptanzkriterien

1. `just check` grün. Neue Tests für das Stats-Modul: bekannte Werte treffen die Regeln
   (Tiger-Front gegen 7,5 cm auf 500 m, 8,8 cm gegen T-34-Seite, 7,5 cm gegen die
   Trockenmauer: drei Treffer laut `data/destruction.toml`), jede Katalog-Einheit ergibt
   eine Wertekarte ohne Fehler, zwei Läufe ergeben dieselbe Ausgabe.
2. Golden-Hash unverändert; `just sim 1000 42` gibt denselben Hash wie vorher.
3. `just units-web --no-open` starten und die Seite im Browser prüfen: je eine Einheit
   mit Datenbank-Eintrag (Tiger I), ohne (Opel Blitz), Infanterie, Geschütz, Flugzeug,
   Umweltobjekt. Keine Fehler in der Konsole, keine leeren Blöcke, schmale Fenster
   brauchbar. Bildschirmfotos in den Abschlussbericht.
4. In `web/units/` steht keine Formel aus der Sim (kein Durchschlag, kein Schaden in
   JavaScript).
5. Jede Zahl im Block „Spielwerte“ lässt sich über die Doku auf eine Regeldatei oder
   einen Katalog-Wert zurückführen.
6. Die Links „Impressum“ und „Datenschutz“ sind in jeder Ansicht sichtbar, auch im
   schmalen Fenster, und zeigen die Texte aus Abschnitt 4, Schritt 7 und 8 vollständig.
7. Im Netzwerk-Tab des Browsers gehen beim Laden und Bedienen der Seite keine Anfragen
   an fremde Server (außer den mit mir abgestimmten); die Datenschutzerklärung nennt
   genau diese.

## 6. Offene Fragen

| Frage | Vorschlag |
|---|---|
| Kennzahlen als 0–100 oder als 1–5 Sterne? | 0–100 mit Balken; Sterne lassen sich daraus ableiten. |
| Vergleich zweier Einheiten nebeneinander? | Nur wenn nach dem Pflichtumfang Zeit bleibt: zweite Einheit anheften, Werte in zwei Spalten. |
| Spielwerte auch in `docs/UNIT_CATALOG.md`? | Nein, die Datei bleibt die Liste der realen Werte; Spielwerte ändern sich mit dem Balancing. |
| Erfahrung (050) und Wetter (040) einrechnen? | Grundwerte ohne Boni bei trockenem Boden; die Faktoren als eigene Zeilen nennen. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim/src/lib.rs`, `cli/src/main.rs`, `justfile`, Doku) nur klein
und additiv ändern. Am Ende nach `prompts/done/` verschieben und `prompts/README.md`
aktualisieren. Abschlussbericht: was gebaut wurde, wie ich es teste, Bildschirmfotos,
die Lückenliste aus Schritt 6, was offen ist.
