# Prompt 125 – Karte nach Bildvorlage: Front an der Flussgabel (unsymmetrisch)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 124
> (Grabenlinie) ist erledigt (`prompts/done/124-*`): Er bringt die Skizzen-Bausteine
> `[[trench]]`, `[[crater_field]]`, `[[wire]]`, Pfützen und den Schlamm-Look. Lies
> zuerst `CLAUDE.md`, dann `prompts/done/124-karte-grabenlinie.md` und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, das Ergebnis von Prompt 123
> (Reichweitentabelle), falls erledigt, in `docs/MAPS.md` „Maps from picture
> sketches (prompt 088)“ ganz mit der Karte `trench_line`, „Bridges“, in
> `docs/ARCHITECTURE.md` Sicht („Seeing downhill“), indirektes Feuer und
> Wegfindung, `data/map_sketches/trench_line.toml` und `river_fork.toml` als
> Muster, `crates/cli/src/gen_sketch.rs`, `data/scale.toml` (`[range]`),
> `docs/AI.md`.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/flussgabel/flussgabel.webp`.
> Eine mit KI gezeichnete, frei erfundene Lagekarte im Stil der Westfront; die
> Beschriftungen sind verstümmelt und werden **nicht** übernommen (keine Orts- oder
> Flussnamen in Karte, Markern, Menüs). Verwendet wird nur die Landschaft.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach jeder Stufe Bilder aus dem Spiel neben
> der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `river_fork_front` (im Spiel „Front an der Flussgabel“ /
„River fork front“) nach der Vorlage, mit den Bausteinen aus Prompt 124. Sie ist
**bewusst unsymmetrisch**: Eine Seite hält Hügel mit Wald, die andere liegt in der
Ebene mit Dörfern. Das Gleichgewicht kommt aus Ausgleich, nicht aus Spiegelung.

Die Vorlage zeigt:

- **drei Flüsse, die sich in der Mitte treffen:** der Hauptfluss schräg von
  Nordwest nach Südost, ein Nebenfluss von Nordosten, einer von Süden; mehrere
  Brücken, einige mitten im Grabengebiet;
- **zwei Fronten** quer über die Karte von Südwest nach Nordost, die die Flüsse
  mehrfach überqueren: tiefe Grabensysteme mit mehreren Linien, dazwischen
  Niemandsland mit Trichtern;
- **Hügelseite (Nordost):** wellige Höhen mit Höhenlinien, Wald am Rand, eine
  Batterie mit mehreren Geschützen am Waldrand, Dörfer und Gehöfte hinter der Front;
- **Ebene (Südwest):** flach, Dörfer mit Feldern, eine Batterie in einem Dorf,
  Straßen in alle Richtungen;
- verstreute Trichter auch hinter den Linien, Wald im Nordwesten.

## 2. Unsymmetrisch und trotzdem ausgeglichen

- **Hügelseite (Verteidiger):** Höhe gibt Sicht nach unten (Sichtregeln mit
  Geländehöhe) und gute Feuerstellungen am Hinterhang. Dafür: weniger Straßen, die
  Grabenlinien liegen am Vorderhang (von unten einsehbar), nur zwei Brücken auf
  ihrer Seite der Flüsse, Versorgungspunkte weiter auseinander.
- **Ebene (Angreifer):** mehr Brücken und Übergänge (Furten), Dörfer als Deckung
  bis nah an die Front, mehr Versorgungspunkte in Reichweite, kürzere Wege für
  Nachschub; aber jede Bewegung im Blick der Höhen.
- **Fronten über die Flüsse:** Jede Seite hat Abschnitte auf beiden Ufern; die
  Brücken in den Gräben sind Schlüsselstellen. Brücken achsparallel (Nord-Süd oder
  West-Ost), der Fluss wird an jeder Brücke kurz gerade geführt wie bisher im
  Generator.
- **Abstände nach Reichweite** (indirekt 1/8, Sicht 1/4; Tabelle wie in 123/124 in
  `docs/MAPS.md`): Mörser über das Niemandsland, leichte Haubitzen aus den Batterien
  bis zur gegnerischen Front, nicht bis zur gegnerischen Batterie. Die
  Batteriestellungen der Vorlage (Waldrand Nordost, Dorf Südwest) als vorbereitete
  Feuerstellungen.
- **Starts:** 1 gegen 1 und 2 gegen 2; Team Nordost auf den Hügeln hinter dem Wald,
  Team Südwest hinter den Dörfern; außerhalb der Reichweite leichter gegnerischer
  Artillerie.
- **Versorgungspunkte:** Flussgabel in der Mitte (umkämpft), Brücken in den Gräben,
  ein Dorf auf jeder Seite der Front, die Höhe über der Gabel, das Wäldchen im
  Nordwesten.
- **Ausgleich messen:** KI-Turnier über viele Seeds mit vertauschten Seiten (jede
  KI-Stufe spielt beide Seiten gleich oft). Ziel: keine Seite gewinnt deutlich über
  60 %. Liegt es daneben, an den Stellschrauben drehen (Brücken, Furten,
  Versorgungspunkte, Lage der Gräben, Höhe der Hügel), nicht an Regeln oder
  Katalogwerten. Ergebnis und Stellschrauben in `docs/AI.md` und `docs/MAPS.md`.

## 3. Stufen

**Stufe 1 – Skizze und Karte:** `data/map_sketches/river_fork_front.toml` nach der
Vorlage, etwa 3,6 km × 2,0 km (Seitenverhältnis der Vorlage, höchstens 4 km), Biom
`forest`, Schlamm-Look aus 124. Flüsse, Hügel (Höhe so, dass die Sicht nach unten
wirkt, Vorschlag 30–60 m; der Generator muss das tragen), Grabensysteme mit
mehreren Linien und Laufgräben, Draht mit Gassen, Trichterfelder dicht im
Niemandsland und lose dahinter, Dörfer, Felder in der Ebene, Wald, Wege,
Batteriestellungen. `just gen-sketch river_fork_front`, `just map-check`,
`just map-reach` (alle Starts, Stellungen und Punkte für Fahrzeuge mit gezogenem
Geschütz erreichbar), Sichtfeld-Bilder von Hügeln, Batterien und vordersten
Gräben, Reichweitentabelle.

**Stufe 2 – Ausgleich und Abschluss:** KI-Turnier mit vertauschten Seiten, Nachjustieren,
Messungen (Bildrate, Wegsuche im Trichterfeld) in `docs/PERFORMANCE.md`,
Menüname und Übersetzung, Tabelle der Skizzenkarten in `docs/MAPS.md` (mit „bewusst
unsymmetrisch“ und den Stellschrauben), Test wie bei den anderen Skizzenkarten
(zweimal erzeugt, gleiche Dateien). `just check`, `just selftest`. Der Golden Hash
ändert sich nicht (neue Karte); wenn doch, Ursache finden.

## 4. Nicht Teil dieses Prompts

- Eine Mission „Angriff auf die Höhen“ auf dieser Karte (sinnvoller Folgeauftrag:
  Spieler greift aus der Ebene an).
- Neue Skizzen-Bausteine; was fehlt, als offener Punkt für 124 bzw. einen Folgeauftrag.
- KI-Verbesserungen für Angriffe über Flüsse und gegen Höhen.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
