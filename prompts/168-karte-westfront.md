# Prompt 168 – Karte nach Vorlage: Westfront (Bach mit drei Brücken, Grabenringe, Trichterfeld)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt
> 124 (Grabenlinie: Skizzen-Tabellen `[[trench]]`, `[[crater_field]]`,
> `[[wire]]`, Pfützen in Trichtern, Schlamm-Look) ist erledigt
> (`prompts/done/124-*`). Empfohlen nach 131 (Batteriehöhen: ebenfalls Bach in
> S-Kurve mit drei Steinbrücken, punktsymmetrisch; dessen Lösung für die Brücken
> und die Symmetrieprobe übernehmen) und 122 (Look je Karte).
>
> Lies zuerst `CLAUDE.md`, dann `prompts/done/088-karten-nach-bildvorlagen.md`
> ganz und von `prompts/done/124-*` (und, falls erledigt, `131-*`) Ziel,
> Entscheidungen und Ergebnis. In `docs/MAPS.md` „Maps from picture sketches
> (prompt 088)“ ganz (mit den Tabellen aus 124), „Bridges“, „Objects“
> (`fort_*`, kahle Bäume) und den Symmetrie-Abschnitt von `city_large`
> (Punktsymmetrie); in `docs/ARCHITECTURE.md` Wegfindung (Größenklassen: Panzer
> brauchen 8 m, besser 10–12 m), Sicht, „Field works“, „Destruction“;
> `data/scale.toml` (`[range]`: direktes Feuer und Sicht 1/8, indirektes Feuer
> 1/8), `data/objects.toml`, `data/terrain.toml`, `docs/TEAMS.md`,
> `docs/SOURCES.md`, `crates/cli/src/gen_sketch.rs`, die Skizzen
> `data/map_sketches/river_fork.toml` und die aus 124/131 als Muster;
> `prompts/meta/karte-2d-vorlage.md` (danach ist die Vorlage entstanden).
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/westfront/`
>
> - `img.png`: ein mit KI erzeugtes, frei erfundenes Bild (1774 × 887 px,
>   genau 2 : 1, also etwa **2,25 m pro Pixel** bei 4000 × 2000 m). Gilt für die
>   **Anordnung**: Bach, Brücken, Straßenverlauf, Grabenringe, Hügel, Dörfer,
>   Ruinen, Trichterfeld, Wälder. Rauch, Brände, Fahnen und Wolken am Rand sind
>   Stimmung.
> - `westfront.svg` und `westfront_beschreibung.md`: von einer anderen KI nach
>   dem Meta-Prompt gezeichneter Bauplan in Metern. Gilt für **Maße und
>   Regeln**: Kartengröße, Lage der Starts und der drei Routen, Wegbreiten,
>   Anzahl und Art der Versorgungspunkte, Hügelhöhen, Wetter. Die SVG hat
>   **keinen Bach** und starre Grabenraster; beides gilt **nicht** (siehe
>   Abschnitt 3).
>
> **Vom Nutzer entschieden:** Anordnung nach dem Bild (mit Bach und drei
> Brücken), Maße und Regeln aus SVG und Beschreibung, Fehler der SVG werden
> korrigiert. **1 gegen 1 und 2 gegen 2**.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Skizzenkarte `western_front` (im Spiel „Westfront“ / „Western front“),
**4000 × 2000 m**, Team West gegen Team Ost: ein festgefahrener Stellungskrieg
in trockenem, zerschossenem, leicht hügeligem Bauernland. Jede Seite hat einen
**Grabenring** um ihre Hügel und ihr Lager; dazwischen liegt ein breites,
zerwühltes **Niemandsland** aus Trichtern, kahlen Bäumen und Ruinen, das ein
gewundener **Bach** von Nord nach Süd teilt. **Drei Straßen** führen von Lager
zu Lager, jede über eine **Steinbrücke**. Keine davon ist ein sicherer
Durchbruch.

Die Vorlage zeigt (Koordinaten aus dem Bild umgerechnet, x Osten, y Süden,
Ursprung Mitte, ± 30 m):

- **Bach** von Nord nach Süd in engen Schleifen, etwa 15–25 m breit, mit
  sumpfigen Tümpeln an den Ufern in der Mitte;
- **drei Steinbrücken** bei etwa (−70, −650), (−125, −85), (−80, +570): auf
  den drei Straßen;
- **drei Straßen** West–Ost bei y ≈ −600, 0, +600, geschwungen, an den Enden
  zu den Lagern gebogen; Querwege an den Flanken;
- **Lager** im Westen bei etwa (−1820, −50) und im Osten bei (1830, −50):
  Zelte, große Halle, Fahrzeuge, Zaun;
- **Grabenringe** je Seite: Zickzack-Gräben mit Holzverschalung, die sich um
  zwei Hügelstellungen (Nordwest um (−1320, −680), Südwest um (−1430, +260);
  gespiegelt im Osten) und um das Lager legen, mit Rundbunkern, Kasematten mit
  Geschütz und Laufgräben nach hinten;
- **vordere Bunkerlinie** je Seite bei x ≈ ±(850–1150), Rundbunker an den
  Straßen und auf Kuppen;
- **Niemandsland** etwa von x = −750 bis +750: dichte Trichter (viele mit
  Wasser), kahle und abgebrochene Bäume, Panzersperren, Drahtreste;
- **Ruinen** im Niemandsland: zerschossene Kirche bei (−455, −550),
  Gehöftruinen bei (−700, −155), (130, −165), Kirchenruine bei (−70, +265);
- **Dörfer:** Kirchdorf im Nordwesten um (−870, −840) (etwa 15 Häuser),
  Dorf im Nordosten um (660, −820) (etwa 12 Häuser); **Gehöfte** mit Silo im
  Süden um (−870, +670) und (750, +650);
- **Felder** (Getreide, Stoppel) an den Rändern, **Nadel- und Mischwald** in
  den vier Ecken und an den Rändern, Baumreihen an den Straßen.

Verwandt, aber anders als 124 (enge Grabenfront, 250–400 m Niemandsland), 125
(Flussgabel, unsymmetrisch) und 131 (Höhen mit Steilabbruch, viel Artillerie):
Hier sind die Gräben **Verteidigungsringe der Hälften**, das weite Niemandsland
mit Ruinen, Brücken und Dörfern ist der umkämpfte Raum.

## 2. Größe, Symmetrie, Spieler

- **4000 × 2000 m** (größte zulässige Seite), Biom `forest` mit dem Look aus
  124 (trocken, erdig) oder einem eigenen trockenen Herbst-Look, Wetter
  **`fog`** (leichter Nebel, aus der Beschreibung).
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y), **nicht**
  spiegelsymmetrisch wie in der SVG: Ein gewundener Bach auf der Achse lässt
  sich nicht spiegeln, aber um die Mitte drehen (S-Kurve wie in 131). Die
  Session zeichnet die Westhälfte nach dem Bild und dreht sie. Folgen, die so
  gewollt sind:
  - die mittlere Brücke liegt genau in der Mitte, die Nordbrücke West ↔ die
    Südbrücke Ost; der Bach wird so geführt, dass jede Brücke auf ihrer Straße
    liegt und der Bach dort auf 40 m gerade Nord–Süd läuft (Brückenachse
    `we`);
  - die Nordstraße des Westens wird zur Südstraße des Ostens;
  - das Kirchdorf im Nordwesten hat sein Gegenstück im **Südosten**, die
    Gehöfte im Südwesten ihr Gegenstück im **Nordosten** (statt Dorf im
    Nordosten wie im Bild). Hausvarianten, Feldfrüchte und Baumarten dürfen
    abweichen, Grundrisse, Wege und Deckung nicht.
- **Starts:** je Team zwei, für 2 gegen 2; bei 1 gegen 1 nur die Spieler 0 und
  1, die diagonal stehen. Vorschlag: Spieler 0 (West) bei (−1600, −300),
  Spieler 2 (West) bei (−1600, +300), Spieler 1 (Ost) bei (1600, +300),
  Spieler 3 (Ost) bei (1600, −300). Das Lager aus dem Bild liegt zwischen den
  beiden Starts einer Seite. Jeder Start: 60 m flach und frei, mindestens zwei
  Fahrzeugausfahrten (eigene Straße und Querweg zur Mittelstraße), innerhalb
  des eigenen Grabenrings.
- **Versorgungspunkte:** 10–12, paarweise um die Mitte gedreht, Namen nach dem
  Ort (nicht nach der Seite wie in der SVG). Vorschlag:
  - je 2 im eigenen Grabenring (Lager, Hügelstellung);
  - je 1 im Dorf bzw. Gehöft an der Flanke;
  - je 1 an einer Ruine im Niemandsland (zerschossene Kirche, Gehöftruine);
  - 1 an der mittleren Brücke (liegt auf dem Drehpunkt, gehört keinem) und
    gegebenenfalls je 1 an der Nord- und Südbrücke.

## 3. Was aus der SVG gilt und was korrigiert wird

Übernehmen: Größe, Lage der drei Routen (y ≈ −600, 0, +600; Landstraße in der
Mitte 14 m, Feldwege 10 m), Hügelradien 130–190 m und -höhen 8–18 m, Wetter,
Gebäudezahl deutlich unter 3000, die Selbstprüfung als Liste für Stufe 2.

Korrigieren (Fehler oder Widerspruch zum Bild):

- **Kein Bach** in der SVG: gilt nicht, der Bach mit drei Brücken ist der Kern
  der Karte.
- **Grabenraster** (vier gerade Linien je Seite bei x = −1200, −820, −420,
  −120): gilt nicht. Gräben nach dem Bild als Ringe um Hügel und Lager plus
  eine vordere Bunkerlinie. Außerdem fehlerhaft: `trench_w_3_3` läuft bis
  (2000, 930) quer über die ganze Karte, die `trench_w_*_0` beginnen alle im
  selben Punkt (−1600, −930) und liegen übereinander, die `front_trench_*`
  kreuzen die Mittelachse und überlappen ihr Spiegelbild.
- **Dörfer zu klein:** 18–22 Häuser auf 90 × 70 m geht nicht (Häuser 8 × 12 m,
  Ortsstraßen 12–16 m). Dorfflächen nach dem Bild, etwa 200–300 m.
- **Engstellen am Graben:** Versorgungspunkte bei (∓1250, ∓120) liegen 50 m
  neben einem Graben, Gehöft `farm_w_3` berührt einen. Plätze für Starts und
  Versorgung mindestens 30 m von Gräben, Draht und Bunkern.
- **Namen:** östliche Punkte heißen `supply_west_rear_east` usw.; neu benennen.
- **Hecken** mit 22-m-Toren auf geraden Linien kommen im Bild nicht vor; nur
  übernehmen, wo das Bild Zäune oder Hecken an Feldern zeigt.

## 4. Spielregeln für den Aufbau

- **Fahrzeugwege:** Die drei Straßen sind die Hauptwege. Jeder Graben, jeder
  Draht und jedes Trichterfeld lässt an jeder Straße eine **freie Gasse von
  mindestens 12 m** (Mitte 14 m). Zusätzlich je Seite **mindestens ein
  Querweg** hinter dem Grabenring, der die drei Straßen verbindet, damit eine
  gesprengte Brücke nicht eine Route abschneidet.
- **Bach:** tief in der Mitte, flach an den Rändern (wie `[[river]]`). Zu den
  drei Brücken je eine **Furt** zwischen den Brücken (etwa y ≈ ∓300, um die
  Mitte gedreht), sumpfig und langsam: Panzer kommen auch nach einer Sprengung
  noch hinüber, aber nicht schnell. Ob die Furten bleiben, entscheidet das
  KI-Turnier (Stufe 2); Ergebnis festhalten.
- **Trichterfeld:** dicht im Niemandsland (Werte aus 124), aber so, dass auf
  jeder Straße und auf je einer Gasse zwischen den Straßen Fahrzeuge
  durchkommen. Größte freie Fläche ohne Deckung höchstens 500 m (Sicht 1/8:
  etwa 150–250 m); Ruinen, Trichter, kahle Baumgruppen und Kuppen brechen die
  Sicht.
- **Grabenringe:** Gräben, Draht und Bunker als Verteidigung der Hälfte,
  Infanterie kommt überall durch (langsam), Fahrzeuge nur durch die Gassen an
  den Straßen und Querwegen. Bunker: `fort_bunker_mg` für die Rundbunker; für
  die Kasematten mit Geschütz ein vorhandenes Objekt oder `fort_bunker_mg` als
  Ersatz und ein offener Punkt (kein neues Modell in diesem Prompt).
- **Reichweiten:** Abstände rechnen und als Tabelle festhalten (wie 124):
  Mörser und leichte Artillerie aus dem eigenen Grabenring erreichen das
  Niemandsland bis zum Bach, aber nicht den gegnerischen Start.
- **Gerechtigkeit:** Weglänge von jedem Start zur mittleren Brücke und zum
  nächsten neutralen Versorgungspunkt je Team gleich (≤ 5 % durch die
  Drehung von selbst, prüfen); kein Hügel blickt auf den Anmarsch der anderen
  Seite hinab; jeder Versorgungspunkt von jedem Start mit Fahrzeugen
  erreichbar.

## 5. Stufen

### Stufe 1: Skizze

`data/map_sketches/western_front.toml` aus Bild und SVG, erzeugt mit
`just gen-sketch western_front`. Westhälfte nachzeichnen, Osthälfte durch
Drehung erzeugen (im Generator oder als Rechenhilfe; die Session wählt und
begründet). Bilder aus dem Spiel: Übersicht, jede Brücke, ein Grabenring, das
Niemandsland, ein Dorf, neben dem Ausschnitt der Vorlage.

### Stufe 2: Prüfen

- `just map-check western_front`, `just map-reach western_front`: jeder
  Start und jeder Versorgungspunkt für Fahrzeuge erreichbar, auch mit jeder
  einzelnen gesprengten Brücke (Brückenprobe wie 131/152).
- Symmetrietest (Drehung), Test wie bei den anderen Skizzenkarten (zweimal
  erzeugt, gleiche Dateien).
- Engste Fahrzeuggasse auf jeder Route messen (≥ 12 m).
- Sichtfeld-Bilder aus einem Grabenring, von einer Hügelstellung, aus der
  zerschossenen Kirche, von der mittleren Brücke; Reichweitentabelle.
- Wegsuche und Bildrate (viele Trichter, Gräben, Draht) wie in 124
  (`docs/PERFORMANCE.md`).
- **KI-Turnier** 1 gegen 1 und 2 gegen 2, auch mit vertauschten Seiten,
  Ergebnis in `docs/AI.md`: Nutzt die KI alle drei Brücken, bleibt sie an den
  Grabengassen hängen, endet es im Patt? Ziel höchstens 55 % für eine Seite.
  Probleme als offene Punkte; die KI nicht umbauen.

### Stufe 3: Look und Abschluss

Trockener, erdiger Look (aus 124 oder als zweiter Look je Karte), Nebel über
dem Niemandsland und dem Bach, Pfützen in Trichtern. Menüname und
Übersetzung, Eintrag in der Tabelle der Skizzenkarten in `docs/MAPS.md` (mit
Zuordnung Vorlage → Baustein, Reichweiten- und Sichttabelle),
`docs/SOURCES.md` (Vorlage frei erfunden, kein reales Vorbild),
`just selftest`, `just lan-selftest`, `just check`. Der Golden Hash ändert
sich durch eine neue Karte nicht.

## 6. Nicht Teil dieses Prompts

- Neue Modelle (Kasematte mit Geschütz, Zelte, falls 124 sie nicht gebracht
  hat): offene Punkte.
- Eine Mission mit vorgegebenen Armeen in den Gräben.
- Rauch und Brände als dauerhafte Kartenelemente.
- KI-Verbesserungen für Grabenkampf und Brücken.
- Mehr als vier Spieler.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| Frage | Entscheidung | Grund |
|---|---|---|
| Vorlage | Bild für die Anordnung, SVG und Beschreibung für Maße und Regeln | vom Nutzer entschieden (2026-11-21) |
| Spieler | 1 gegen 1 und 2 gegen 2 | vom Nutzer entschieden (2026-11-21) |
| Symmetrie | punktsymmetrisch statt gespiegelt | gewundener Bach auf der Achse; in diesem Prompt festgelegt |

## Ergebnis

(von der Session auszufüllen)
