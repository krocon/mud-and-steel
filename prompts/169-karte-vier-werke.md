# Prompt 169 – Karte nach Bildvorlage: Vier Werke (Festungshügel, Bach mit drei Brücken, Sommer)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt
> 124 (Grabenlinie: Skizzen-Tabellen `[[trench]]`, `[[crater_field]]`,
> `[[wire]]`, Pfützen in Trichtern) ist erledigt (`prompts/done/124-*`).
> Empfohlen nach **168** (Westfront: gleiche Grundanordnung mit Bach, drei
> Brücken, Lagern und Grabenringen; deren Lösung für Bach, Brücken, Drehung und
> Prüfungen übernehmen), 131 (Batteriehöhen) und 122 (Look je Karte).
>
> Lies zuerst `CLAUDE.md`, dann `prompts/done/088-karten-nach-bildvorlagen.md`
> ganz und von `prompts/done/124-*` und, falls erledigt, `168-*` und `131-*`
> Ziel, Entscheidungen und Ergebnis. Ist 168 noch offen, lies
> `prompts/168-karte-westfront.md` trotzdem ganz: Abschnitte 2 bis 5 gelten hier
> sinngemäß, dieser Prompt nennt nur die Unterschiede. In `docs/MAPS.md` „Maps
> from picture sketches (prompt 088)“ ganz, „Bridges“, „Objects“ (`fort_*`) und
> den Symmetrie-Abschnitt von `city_large`; in `docs/ARCHITECTURE.md`
> Wegfindung (Größenklassen: Panzer brauchen 8 m, besser 10–12 m), Sicht,
> „Field works“, „Destruction“; `data/scale.toml` (`[range]`),
> `data/objects.toml`, `data/terrain.toml`, `docs/TEAMS.md`, `docs/SOURCES.md`,
> `crates/cli/src/gen_sketch.rs` und die Skizzen aus 124/131/168 als Muster.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/vierwerke/img.webp`: ein mit KI (ChatGPT) erzeugtes, frei
> erfundenes Bild, 1672 × 941 px (etwa 16 : 9). Es gibt **keine SVG und keine
> Beschreibung**: Maße und Regeln kommen aus diesem Prompt. Wolken an den
> Rändern sind Stimmung. Die **roten Kreuze** auf den Dächern der Lager werden
> **nicht** übernommen (`docs/SOURCES.md`: kein Rotes Kreuz).
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Skizzenkarte `four_forts` (im Spiel „Vier Werke“ / „Four forts“),
**4000 × 2250 m** (Seitenverhältnis des Bildes, etwa **2,4 m pro Pixel**),
Team West gegen Team Ost: grünes, sommerliches Hügelland mit Feldern und
Dörfern, in dem jede Seite **zwei Festungswerke** auf Hügeln hält. Zwischen den
Werken liegt ein Niemandsland mit Trichtern, kahlen Bäumen und Ruinen, das ein
gewundener **Bach** von Nord nach Süd teilt. **Drei Straßen** führen von Lager
zu Lager, jede über eine **Steinbrücke**.

Der Unterschied zu 168 (Westfront): Dort ist alles zerschossen und trocken,
und die Gräben sind Ringe um Hügel und Lager. Hier ist das Land **grün und
bewirtschaftet**, nur das Niemandsland ist zerwühlt, und die **vier Werke** sind
die Ankerpunkte der Karte: große, begrünte Erdhügel mit Betonfront, Kuppel und
eigenem Grabenring. Wer ein gegnerisches Werk nimmt, hat eine Flanke offen.

Die Vorlage zeigt (Koordinaten aus dem Bild umgerechnet, x Osten, y Süden,
Ursprung Mitte, ± 40 m):

- **Bach** von Nord nach Süd in engen Schleifen, etwa 15–25 m breit, mit
  Tümpeln und Schilf in der Mitte und einem kleinen Teich am Südrand;
- **drei Steinbrücken** bei etwa (−40, −720), (10, −100), (−15, +580), je auf
  einer Straße;
- **drei Straßen** West–Ost bei y ≈ −720, −100, +570, geschwungen; die mittlere
  ist die Hauptstraße und läuft fast gerade von Lager zu Lager, die äußeren
  biegen an den Enden zu den Lagern; an den Flanken verbinden **Querwege** die
  drei Straßen hinter den Werken;
- **Lager** im Westen bei etwa (−1850, −140) und im Osten bei (1830, −160):
  Hallen, Baracken, Zelte, Fahrzeuge, ein **Wachturm**, Zaun;
- **vier Festungswerke**: Nordwest bei etwa (−1170, −700), Südwest (−1210,
  +330), Nordost (1100, −720), Südost (1180, +300). Jedes: ein flacher,
  begrünter Hügel (Durchmesser etwa 250–350 m, Höhe 12–20 m) mit einem
  Betonbau an der Front zur Mitte, einer **Panzerkuppel** obenauf, einem
  **Zickzack-Grabenring** mit Holzverschalung rundum, **Höckerlinien**
  (Panzersperren) vor dem Graben, Rundbunkern an den Ecken und einem Laufgraben
  nach hinten;
- **vordere Bunkerlinie** je Seite bei x ≈ ±(600–900): einzelne Rundbunker an
  den Straßen und auf Kuppen, dazwischen Gräben mit Höckerlinien;
- **Niemandsland** etwa von x = −600 bis +600: Trichter (einige mit Wasser),
  kahle und abgebrochene Bäume, dazwischen aber noch Wiese, Büsche und einige
  **blühende** Bäume (weiß);
- **Ruinen**: große **Kirchenruine** mit Turm bei (460, −920) am Nordrand,
  Gehöft- und Hausruinen bei (−470, −230), (140, −300), (950, −230), (450,
  +110), (−370, +500), dazu Stapel mit Kisten und Fässern an einigen Ruinen;
- **Dörfer und Gehöfte** (heil): Dorf im Norden um (−450, −970), kleine Dörfer
  im Süden um (−750, +840), (430, +600), (1000, +770), einzelne Gehöfte an den
  Rändern;
- **Felder** an allen Rändern: Getreide, Stoppel, Weide, leuchtend gelbe
  **Rapsfelder** im Nordosten und Osten; Feldzäune; **Laub- und Mischwald** in
  den Ecken und an den Rändern, Baumreihen an Wegen.

## 2. Größe, Symmetrie, Spieler

- **4000 × 2250 m**, Biom `forest` mit einem **sommerlichen, grünen Look**
  (eigener Look je Karte nach 122, falls vorhanden; sonst der Standard-Look des
  Bioms). Wetter **klar**, leichter Dunst am Rand erlaubt.
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y), wie 168: Die
  Session zeichnet die Westhälfte nach dem Bild und dreht sie. Folgen, die so
  gewollt sind: Die mittlere Brücke liegt in der Mitte, Nordbrücke West ↔
  Südbrücke Ost; das Nordwest-Werk hat sein Gegenstück im Südosten, das
  Südwest-Werk im Nordosten; die Kirchenruine im Norden bekommt eine
  Gegen-Ruine im Süden, das Norddorf ein Gegen-Dorf im Süden. Hausvarianten,
  Feldfrüchte und Baumarten dürfen abweichen, Grundrisse, Wege und Deckung
  nicht. Der Bach läuft an jeder Brücke auf 40 m gerade Nord–Süd.
- **Spieler:** 1 gegen 1 und 2 gegen 2 (wie 168). Vorschlag für die Starts:
  Spieler 0 (West) bei (−1650, −350), Spieler 2 (West) bei (−1650, +350),
  Spieler 1 (Ost) bei (1650, +350), Spieler 3 (Ost) bei (1650, −350). Bei 1
  gegen 1 nur 0 und 1 (diagonal). Das Lager liegt zwischen den beiden Starts
  einer Seite. Jeder Start: 60 m flach und frei, mindestens zwei
  Fahrzeugausfahrten, hinter den eigenen Werken.
- **Versorgungspunkte:** 10–12, paarweise um die Mitte gedreht, nach dem Ort
  benannt. Vorschlag: je 1 im Lager, je 1 **auf dem Hof jedes Werks** (also 2
  je Seite, hinter dem Betonbau, mindestens 30 m von Graben und Kuppel), je 1
  in einem Dorf an der Flanke, je 1 an einer Ruine im Niemandsland, 1 an der
  mittleren Brücke (auf dem Drehpunkt, gehört keinem).

## 3. Werke, Gräben, Höcker

- **Werk als Baustein:** Hügel (Höhenfeld), Graben (`[[trench]]`), Höcker
  (`fort_hedgehog` oder vorhandene Panzersperren), Draht (`[[wire]]`),
  Rundbunker (`fort_bunker_mg`). Für den **Betonbau mit Kuppel** gibt es kein
  Modell: ein vorhandenes Objekt (Bunker, Gebäude, das besetzbar ist) oder eine
  Gruppe aus `fort_bunker_mg` als Ersatz, und ein offener Punkt für ein eigenes
  Modell (kein neues Modell in diesem Prompt). Infanterie soll das Werk besetzen
  und daraus kämpfen können.
- Der Grabenring jedes Werks hat **eine Fahrzeuggasse nach hinten** (Querweg
  zum Lager) und **keine** zur Mitte: Panzer kommen nur über die Straßen am Werk
  vorbei, nicht durch das Werk. Infanterie kommt überall durch (langsam).
- **Höckerlinien** stehen vor den Gräben in Reihen wie im Bild, lassen aber an
  jeder Straße eine **freie Gasse von mindestens 12 m** (Mitte 14 m).
- **Reichweiten:** Abstände rechnen und als Tabelle festhalten (wie 124/168):
  Von einem Werk aus erreichen Mörser und leichte Artillerie die eigene Brücke
  und das Niemandsland bis zum Bach, aber nicht das gegnerische Werk gegenüber
  und keinen gegnerischen Start. Ein Werk darf nicht auf den Anmarsch der
  Gegenseite hinabsehen (Sicht 1/8; Höhe der Hügel danach wählen).

## 4. Spielregeln für den Aufbau

Wie 168, Abschnitt 4 (Fahrzeugwege, Querwege hinter den Werken, Furten
zwischen den Brücken, Trichterfeld mit Gassen, größte freie Fläche ohne
Deckung höchstens 500 m, Gerechtigkeit der Wege ≤ 5 %, jeder
Versorgungspunkt mit Fahrzeugen erreichbar). Zusätzlich:

- Das Trichterfeld ist **lockerer** als in 168: Wiesenflecken und Büsche
  bleiben, damit es grün wirkt.
- Die Dörfer und Gehöfte sind heil und **besetzbar**: Sie sind die Deckung
  auf den Flanken. Dorfflächen nach dem Bild, Häuser 8 × 12 m, Ortswege
  12–16 m.
- Gebäudezahl deutlich unter 3000.

## 5. Stufen

### Stufe 1: Skizze

`data/map_sketches/four_forts.toml` nach dem Bild, erzeugt mit
`just gen-sketch four_forts`. Westhälfte nachzeichnen, Osthälfte durch Drehung
(wie in 168 gelöst; ist 168 noch offen, entscheidet die Session und
begründet). Bilder aus dem Spiel: Übersicht, jede Brücke, ein Werk aus der
Nähe, das Niemandsland, ein Dorf, neben dem Ausschnitt der Vorlage.

### Stufe 2: Prüfen

- `just map-check four_forts`, `just map-reach four_forts`: jeder Start und
  jeder Versorgungspunkt für Fahrzeuge erreichbar, auch mit jeder einzelnen
  gesprengten Brücke (Brückenprobe wie 131/152/168).
- Symmetrietest (Drehung), Test wie bei den anderen Skizzenkarten (zweimal
  erzeugt, gleiche Dateien).
- Engste Fahrzeuggasse auf jeder Route messen (≥ 12 m).
- Sichtfeld-Bilder aus jedem Werk-Typ (Nord, Süd), von der Kirchenruine, von
  der mittleren Brücke; Reichweitentabelle.
- Wegsuche und Bildrate wie in 124 (`docs/PERFORMANCE.md`).
- **KI-Turnier** 1 gegen 1 und 2 gegen 2, auch mit vertauschten Seiten,
  Ergebnis in `docs/AI.md`: Greift die KI die Werke an oder umgeht sie sie?
  Nutzt sie alle drei Brücken? Ziel höchstens 55 % für eine Seite. Probleme als
  offene Punkte; die KI nicht umbauen.

### Stufe 3: Look und Abschluss

Sommerlicher Look, blühende Bäume im Niemandsland, Rapsfelder gelb, Pfützen in
Trichtern. Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten
in `docs/MAPS.md` (mit Zuordnung Vorlage → Baustein, Reichweiten- und
Sichttabelle), `docs/SOURCES.md` (Vorlage frei erfunden, kein reales Vorbild,
rote Kreuze weggelassen), `just selftest`, `just lan-selftest`, `just check`.
Der Golden Hash ändert sich durch eine neue Karte nicht.

## 6. Nicht Teil dieses Prompts

- Neue Modelle (Festungswerk mit Panzerkuppel, Wachturm, falls nicht
  vorhanden): offene Punkte, gegebenenfalls ein eigener Prompt.
- Eine Mission mit vorgegebenen Armeen in den Werken.
- KI-Verbesserungen für Festungs- und Grabenkampf.
- Mehr als vier Spieler.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| Frage | Entscheidung | Grund |
|---|---|---|
| Vorlage | nur das Bild; Maße und Regeln aus diesem Prompt | keine SVG vorhanden |
| Größe | 4000 × 2250 m | Seitenverhältnis des Bildes, größte zulässige Seite |
| Spieler | 1 gegen 1 und 2 gegen 2 | wie 168; vom Prompt vorgeschlagen, nicht vom Nutzer entschieden |
| Symmetrie | punktsymmetrisch statt gespiegelt | gewundener Bach auf der Achse, wie 168 |
| Rote Kreuze | weggelassen | `docs/SOURCES.md` |

## Ergebnis

(von der Session auszufüllen)
