# Prompt 079 – Große Karte mit Großstadt im Zentrum

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `docs/MAPS.md` ganz (Format, Objekte, Brücken, „Generator“ mit „Small town (prompt
> 041)“, „Desert village (prompt 059)“ und „Landscape maps (prompt 056)“,
> Karteneditor), in `docs/ARCHITECTURE.md` die Abschnitte zu Wegfindung, „Vision and
> fog of war“, Zerstörung, Brand und „Map objects and decoration“, dazu
> `docs/PERFORMANCE.md`, `docs/AI.md` und `docs/GAME_DESIGN.md` (Versorgungspunkte,
> Basis-Modus). Dann den Code der Generatoren: `crates/cli/src/gen_town.rs`,
> `gen_land.rs` und `gen_village.rs`, und das Ergebnis von Prompt 078 (Wohnblöcke) in
> `data/catalog/neutral.toml` und `data/objects.toml`.

**Voraussetzung:** Prompt 078 (mehrgeschossige Wohnblöcke) ist erledigt. Ohne die
Blöcke gibt es keine Großstadt. Fehlen sie, halte an und sag es; bau keinen Ersatz
aus Stadthäusern.

**Vorlage:** `.temp/map_grossstadt1.png` (nicht im Repo, `.temp/` ist ignoriert). Ein
Bildschirmfoto aus einem anderen Strategiespiel, 1679 × 849 Pixel: eine verschneite
Stadt von schräg oben. Liegt die Datei nicht vor, reicht die Beschreibung in
Abschnitt 2.

## 1. Ziel

Eine neue, große Gefechtskarte mit einer Großstadt in der Mitte: ein dichter Kern aus
Wohnblöcken, darum ein Gürtel aus Einfamilienhäusern, darum offenes Land, aus dem die
Spieler kommen. Um die Stadt und in ihren Straßen wird gekämpft.

Fertig ist der Prompt, wenn

- die Karte aus einem Generator kommt (`just gen-city`), bei jedem Lauf Bit für Bit
  gleich, und `just map-check` besteht,
- eine Ansicht der Stadt neben der Vorlage liegt und in Dichte, Gliederung und
  Übergang von der Stadt ins Land dazu passt,
- sie im Menü wählbar ist und ein Gefecht Mensch gegen KI und KI gegen KI darauf
  bis zum Ende läuft, ohne dass Einheiten hängen bleiben,
- Bildrate, Tickzeit, Wegfindung und Sicht auf der Karte gemessen sind und in den
  Grenzen aus `docs/PERFORMANCE.md` liegen,
- `just check`, `just selftest` und `just editor-selftest` grün sind.

## 2. Die Vorlage

Das Bild ist **Vorbild für Aufbau und Wirkung**, kein Material: keine Bildausschnitte,
kein Nachbau des gezeigten Ortes, nichts aus anderen Spielen (`docs/SOURCES.md`).
Keine echten Ortsnamen.

Was es zeigt:

- **Kern aus Wohnblöcken.** Vier- und fünfgeschossige Blöcke mit Flachdach, meist
  hell verputzt, einer aus rotem Ziegel. Sie stehen in lockeren Reihen und Winkeln
  und bilden Höfe, nicht geschlossene Straßenfronten. Zwischen ihnen breite Straßen
  im Raster und freie Flächen mit einzelnen Bäumen.
- **Gürtel aus Einfamilienhäusern.** Am Rand des Kerns beginnen ohne Übergang
  kleine Häuser mit steilen Dächern, jedes auf seinem Grundstück, entlang
  schmalerer Straßen, mit Gärten und Bäumen.
- **Offenes Land.** Hinter den letzten Häusern Felder und Wege, eine
  Fahrzeugkolonne auf einer Landstraße, eine ringförmige Erdstellung.
- **Kriegsschäden.** Mehrere Blöcke sind oben aufgerissen oder ausgebrannt.
- **Winter.** Schnee auf Dächern, Straßen und Feldern, kahle Bäume, lange Schatten.
- **Maßstab.** Die Blöcke sind etwa 30 bis 40 m lang, zwischen ihnen liegen 20 bis
  40 m. Die Stadt wirkt weiträumig, nicht eng.

## 3. Festlegungen

Diese Fragen sind entschieden, damit die Session nicht daran hängen bleibt. Spricht
beim Lesen des Codes etwas Gewichtiges dagegen, halte an und frag.

| Frage | Festlegung | Grund |
|---|---|---|
| Wie entsteht die Karte? | Neuer Generator `cli gen-city` (`crates/cli/src/gen_city.rs`, `just gen-city`), nach dem Muster von `gen_town` und `gen_land` | Wiederholbar, prüfbar, später änderbar; von Hand gesetzte Karten dieser Größe pflegt niemand. |
| Name | `city_large` (Ordner `data/maps/city_large`) | Kein echter Ort. |
| Größe | 3 km × 3 km | Groß genug für Land um die Stadt; kleiner als `test_large` (4 km), damit Dateien und Ladezeit im Rahmen bleiben. |
| Biom | `winter`, Standardwetter Schnee, wie die Vorlage; das Biom ist ein Parameter des Generators | Eine Sommerfassung kostet dann nur einen zweiten Lauf (offener Punkt). |
| Spieler | Vier Starts in zwei Teams, Team 0 im Süden, Team 1 im Norden; ein Spiel zu zweit nutzt die Spieler 0 und 1, diagonal gegenüber | Wie die Landschaftskarten aus Prompt 056. |
| Gerechtigkeit | Straßennetz, Plätze, Versorgungspunkte und Starts sind punktsymmetrisch zur Kartenmitte; Gebäudearten, Schäden und Bewuchs dürfen abweichen | Beide Teams haben dieselben Wege und Entfernungen, ohne dass die Stadt gespiegelt aussieht. |
| Fluss | Keiner | Ein Fluss durch die Stadt macht Brücken zu den einzigen Übergängen; das ist eine eigene Karte (offener Punkt). |

## 4. Aufbau der Karte

Von innen nach außen. Die Maße sind Anhaltswerte; der Generator legt sie fest und
`docs/MAPS.md` hält sie fest.

| Zone | Ausdehnung | Inhalt |
|---|---|---|
| **Zentrum** | bis etwa 150 m von der Mitte | Großer Platz (etwa 80 × 60 m) mit Rathaus und Kirche, darum die größten Blöcke. Zwei Hauptstraßen kreuzen sich hier und führen bis zum Kartenrand. |
| **Blockviertel** | bis etwa 450 m | Straßenraster mit Blöcken in Reihen und Winkeln um Höfe, wie in der Vorlage. Dazwischen Zeilen von Stadthäusern, ein Park, ein Lagerviertel mit Lagerhäusern, Halle und Depot. Höfe mit Bäumen, Schuppen, Mauern. |
| **Vorstadt** | bis etwa 700 m | Einzelne Häuser (Bauern-, Ziegel-, Stein- und Blockhäuser aus dem Bestand) auf Grundstücken mit Zäunen, Hecken, Obstbäumen, Holzstößen, an schmaleren Straßen. |
| **Land** | bis zum Rand | Felder, Feldwege mit Hecken, Gehöfte, Wälder, sanfte Hügel. Landstraßen aus der Stadt in alle vier Richtungen. Die Starts liegen hier. |

**Straßen und Wege**, das Wichtigste für das Spiel:

- Wegzellen sind 4 m groß. Hauptstraßen mindestens 16 m breit, Nebenstraßen
  mindestens 8 m, Achsen auf den Mittellinien der Wegzellen, wie `gen_land` es für
  Brücken und Dämme macht.
- **Mindestens drei unabhängige Fahrzeugrouten** durch die Stadt von Süd nach Nord
  und je eine Umgehung im Osten und Westen. Kein Engpass, an dem ein zerstörtes
  Fahrzeug oder eine Ruine alles sperrt.
- Keine Sackgassen für Fahrzeuge im Blockviertel. Infanterie erreicht jeden Hof.
- Abstände zwischen Blöcken wie in der Vorlage, 20 bis 40 m: Platz für Gefechte,
  und die Kamera sieht in die Straßen.
- Objekte stehen ohne Überlappung und nicht auf Straßen (Rastergitter wie in
  `gen_land`).

**Versorgungspunkte** stehen auf Orten, die man sieht, wie in Prompt 056: der Platz
im Zentrum als wertvollster, dazu je Hälfte punktsymmetrisch zwei bis drei weitere
(Lagerviertel, Park, eine Kreuzung in der Vorstadt, ein Gehöft vor der Stadt). Marker
für Werkzeuge und Aufnahmen wie bei den anderen Karten.

**Starts:** im Land, etwa 2,2 km auseinander, 60 m ringsum eben und frei, genug
offene Fläche für den Basis-Modus. Was die anderen Karten am Start haben
(Versorgungslager wie in `town_small`), nach deren Muster.

**Kriegsschäden:** ein Teil der Gebäude startet als `Damaged`, wenige als `Ruin`
(etwa jedes achte und jedes zwanzigste, gleich verteilt auf beide Hälften), dazu
einige Krater und Erdstellungen am Stadtrand, wenn die Karte das als Startzustand
tragen kann.

**Gelände:** in der Stadt eben, im Land sanfte Hügel und Rauschen wie in `gen_town`.
Straßen gepflastert, Höfe und Wege als Feldweg oder Gras; im Winter zeichnet das
Biom den Schnee.

## 5. Generator

- Bit für Bit gleich auf jedem Rechner: keine Gleitkommafunktionen, deren Ergebnis
  von der Plattform abhängt (siehe den Hinweis zu `atan2` in „Landscape maps“). Ein
  Test erzeugt die Karte zweimal und vergleicht, zählt Blöcke, Häuser, Plätze und
  Versorgungspunkte und prüft die Symmetrie von Straßen und Starts.
- Teile wieder, was `gen_town`, `gen_land` und der Dorfpinsel des Editors schon
  können (Straßen, Zeilen, Grundstücke, Platzierung ohne Überlappung), statt es ein
  viertes Mal zu schreiben. Ziehe gemeinsame Helfer nur heraus, wenn es die anderen
  Generatoren nicht verändert: deren Karten müssen Bit für Bit gleich bleiben.
- Die erzeugte Karte liegt im Repo wie die anderen; `just gen-city` schreibt sie
  neu.
- Die Karte ist neu, an ihr hängt kein Test und keine Messreihe: der goldene Hash
  ändert sich nicht.

## 6. Die Stadt muss spielbar sein

Eine dichte Stadt belastet Teile des Spiels, die auf Dörfern nie gefordert waren.
Messen, vergleichen, beheben oder die Stadt lichter machen:

| Was | Wie messen | Grenze |
|---|---|---|
| Bildrate | `just bench-view` und eine Ansicht über dem Blockviertel und in einer Straße, jede Grafikstufe | über 60 FPS |
| Simulation | `just bench` auf der Karte mit einem großen Gefecht in der Stadt | Tickzeit im Rahmen aus `docs/PERFORMANCE.md` |
| Wegfindung | `just path-bench city_large`: lange Wege durch die Stadt, viele Einheiten zugleich | wie auf `test_large` |
| Sicht | `just vision-bench city_large`: viele Objekte, die die Sicht verdecken | wie auf `test_large` |
| Brand | Ein Brand im Blockviertel breitet sich nicht über die ganze Stadt aus; Straßen wirken als Schneisen (Prompt 041) | Bildrate und Tickzeit halten |
| KI | `just ai-tournament` auf der Karte, mehrere Startwerte; dazu ein Gefecht ansehen | jedes Spiel endet, keine Einheit steht minutenlang fest, die KI kämpft in der Stadt und nicht nur davor |
| Laden und Dateien | Ladezeit, Größe der Kartendateien, Zahl der Objekte | notieren, mit `test_large` vergleichen |

Dazu von Hand: Fahrzeuge und Infanterie durch jede Hauptstraße, durch ein paar Höfe
und über beide Umgehungen schicken; eine Straße mit einer Ruine sperren und sehen,
dass die Einheiten umfahren. Gilt für Einheiten hinter hohen Häusern, was Prompt 078
gelöst hat?

## 7. Prüfen und Abschluss

1. **Vergleichsbild.** Eine Ansicht des Blockviertels mit dem Rand zur Vorstadt bei
   ähnlichem Blickwinkel neben der Vorlage. Nach jeder Änderungsrunde neu.
2. **Bildreihe.** `SHOT_SET=land just terrain-shots <dir> city_large`: die ganze
   Karte, jede Zone, jeder Versorgungspunkt, ein Start; dazu Straßenansichten bei
   15 und 35 m.
3. Die Messungen aus Abschnitt 6, als Tabelle.
4. `just map-check city_large`, `just check`, `just selftest`,
   `just editor-selftest`; die Karte im Editor öffnen und speichern.
5. Sieh nach, was eine neue Karte braucht, um im Menü mit Namen, Größe und Vorschau
   zu erscheinen (wie die Karten aus Prompt 056), und ergänze es, auch die
   Übersetzungen.

Die Bilder und die Tabelle dem Nutzer zeigen. Was nicht erreicht wurde, steht im
Abschlussbericht mit Grund, nicht als erledigt.

- `docs/MAPS.md`: Abschnitt „Large city (prompt 079)“ beim Generator (Zonen, Maße,
  Straßenbreiten, Starts, Versorgungspunkte, Marker, offene Punkte).
  `docs/PERFORMANCE.md`: die Messwerte. `docs/AI.md`, falls die KI für die Stadt
  etwas lernen musste. `CLAUDE.md` und `justfile`: `just gen-city`. Auf Englisch,
  ohne das Bild der Vorlage.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten (Generator, Karte, Anpassungen an
  Wegfindung oder KI, Menü und Doku). Kein `git push`.

## 8. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Sommerfassung** der Karte (Biom als Parameter ist da) und eine **Wüstenstadt**
  mit den Flachdach-Blöcken.
- **Fluss mit Brücken** durch die Stadt, **Bahnhof und Gleise**, **Fabrikgelände**.
- **Neue Gebäude** über den Bestand und Prompt 078 hinaus (Schule, Kino,
  Wasserturm).
- **Straßenkampf als Regel:** Barrikaden, Kanalisation, Kampf im Haus,
  Stockwerke.
- **Einstellung im Demo-Video** auf dieser Karte.
