# Prompt 170 – Karte nach Bildvorlage: Inselstadt (Altstadt im Flussring, sieben Brücken, Bahnhof)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompts
> 134 (Großstadthäuser: Blockrand, Eckhaus, je `Intact` / `Damaged` / `Ruin`),
> 137 (Bahnhofshalle, Ratsbau) und 155 (Fabriken, Schornstein, Kaimauer,
> Güterzüge) sind erledigt (`prompts/done/`). Empfohlen nach 115 (lange
> Brücken, Stahlbrücke), 156 (Fabrikstadt: Stadt-Skizze mit Kanälen und
> Bahnhof) und 122 (Look je Karte). Fehlt etwas davon, nimmt die Session den
> nächsten vorhandenen Baustein und schreibt einen offenen Punkt; keine neuen
> Modelle in diesem Prompt.
>
> Lies zuerst `CLAUDE.md`, dann `prompts/done/088-karten-nach-bildvorlagen.md`
> ganz und von `134-*`, `137-*`, `155-*` und, falls erledigt, `156-*`, `168-*`
> Ziel, Entscheidungen und Ergebnis. In `docs/MAPS.md` „Maps from picture
> sketches (prompt 088)“ ganz, den Eintrag `river_city` (Fluss in großer
> Schleife um die Mitte, acht Brücken: Muster für Wasser und Brücken),
> „Bridges“, „Objects“; in `docs/ARCHITECTURE.md` Wegfindung (Größenklassen:
> Panzer brauchen 8 m, besser 10–12 m), Sicht, „Destruction“ und Gebäude
> besetzen; `data/scale.toml` (`[range]`), `data/objects.toml`,
> `data/terrain.toml`, `docs/TEAMS.md`, `docs/SOURCES.md`,
> `crates/cli/src/gen_sketch.rs` und `data/map_sketches/river_city.toml`.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/inselstadt/img.webp`: ein mit KI (ChatGPT) erzeugtes, frei
> erfundenes Bild, 1672 × 941 px. **Schrägansicht** von Süden, keine
> Draufsicht: Vorne (unten) ist alles größer als hinten, die Häuser verdecken
> Straßen. Die Session entzerrt die Anordnung zu einem Grundriss (Häuser als
> Maßstab: Blockrandhäuser 12–15 m tief, Straßen 10–16 m) und zeigt den
> entzerrten Plan in Stufe 1 neben dem Bild. Rauch und Brände sind Stimmung.
> Die **rote Fahne** am Ratsbau wird **nicht** übernommen (keine Hoheitszeichen,
> `docs/SOURCES.md`).
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Skizzenkarte `island_town` (im Spiel „Inselstadt“ / „Island town“),
Team Nord gegen Team Süd: Eine dicht gebaute, schon schwer umkämpfte
**Altstadt liegt auf einer Insel**, die ein Fluss ganz umschließt. Kaimauern
fassen die Ufer, **sieben Brücken** verbinden die Insel mit dem Umland. Wer die
Insel hält, sitzt in der Mitte aller Wege, aber jede Brücke ist eine Engstelle.

Die Vorlage zeigt (Lage im Bild, von links nach rechts und oben nach unten;
die Meter ergeben sich erst aus der Entzerrung):

- **Fluss** als Ring um die Insel, etwa 50–80 m breit (im Vordergrund breiter
  gezeichnet), mit **Kaimauern** an der Insel, Uferstraßen und Bäumen; im
  Westen und Süden außen flache, begrünte Ufer;
- **Brücken:** Steinbogenbrücken im Westen (lang, drei Bögen), Nordwesten,
  Norden (Mitte oben), Osten (zwei Bögen) und Süden (vorne Mitte, schräg), eine
  kleine Brücke im Südosten zu einem Uferviertel und eine **stählerne
  Fachwerk-Eisenbahnbrücke** im Nordosten;
- **Insel:** Blockrandviertel aus drei- bis fünfgeschossigen Häusern mit
  Mansarddächern, viele beschädigt oder Ruine, Schutt in den Straßen; in der
  Mitte eine große **neugotische Kirche** mit Turm, südöstlich davon ein
  **Platz mit Denkmal** (Rondell), daneben ein **Ratsbau** mit Mittelrisalit;
  breite Straßen kreuzen die Insel von Brücke zu Brücke, dazwischen enge Gassen
  und Höfe mit Bäumen;
- **Nordufer:** links ein **Fabrikviertel** mit Schornstein und Hallen,
  Mitte Wohnblöcke, rechts der **Bahnhof** mit Empfangsgebäude, breitem
  **Gleisfeld** (Güterwagen, Lokschuppen), zwei **Wassertürmen** und einer
  Ziegelfabrik mit Schornstein; die Bahn verlässt das Gleisfeld über die
  Fachwerkbrücke nach Südosten;
- **Ostufer:** Ziegelbauten und Lagerhäuser, ein kleines Uferviertel;
- **Südufer:** links ein **befestigter Hügel** mit Geschützstellung in einem
  Ringwall, Bunkern und Gräben, Mitte und rechts ein **Grabensystem** mit
  Holzverschalung, Bunkern, Panzersperren und Draht am Ufer entlang, dahinter
  **Felder**;
- **Bäume**: einzelne Laubbäume auf der Insel, Baumreihen an den Uferstraßen,
  blühende Bäume, kleine Wäldchen an den Rändern.

## 2. Größe, Aufteilung, Spieler

- **Kartengröße:** Die Insel ist nach der Entzerrung etwa **1100–1300 m** breit
  und **700–900 m** tief. Die Karte legt um den Flussring Umland an, damit die
  Starts außer Reichweite liegen: Vorschlag **2800 × 2400 m**, Insel in der
  Mitte. Was das Bild nicht zeigt (Nordrand hinter dem Gleisfeld, Südrand
  hinter den Feldern, die Ecken), ergänzt die Session als Vorstadt, Felder und
  Wald im Stil des Bildes.
- **Biom** `forest`, sommerlicher Look, Wetter klar.
- **Aufteilung:** Das Bild ist nicht symmetrisch: Norden ist Industrie und
  Bahn, Süden Felder und Befestigung. Die Karte bleibt **nach dem Bild
  unsymmetrisch** und wird **gleichwertig** gemacht, nicht gespiegelt:
  - jede Seite hat **gleich viele Straßenbrücken** zur Insel (Vorschlag: je
    drei, die Ost- und Westbrücke liegen an den Flanken und gehören keinem;
    die Eisenbahnbrücke ist für Infanterie gangbar, nicht für Fahrzeuge, oder
    für alle, wenn das Gleis es trägt: die Session entscheidet und begründet);
  - Team Nord bekommt am Nordufer eine **eigene Uferbefestigung** (Bunker an
    den Brückenköpfen, Gräben im Bahndamm, Sperren), gleichwertig zum Süden;
    Team Süd bekommt am Südufer Gebäudedeckung (Uferviertel, Gehöfte), damit
    beide Seiten Deckung und Feldbefestigung haben;
  - Weglänge von jedem Start zum Inselplatz und zum nächsten neutralen
    Versorgungspunkt je Team höchstens **5 %** verschieden.
- **Spieler:** 1 gegen 1 und 2 gegen 2. Vorschlag: Team Nord Spieler 0 im
  Nordwesten hinter dem Fabrikviertel, Spieler 2 im Nordosten hinter dem
  Gleisfeld; Team Süd Spieler 1 im Südosten hinter den Feldern, Spieler 3 im
  Südwesten hinter dem Geschützhügel. Bei 1 gegen 1 nur 0 und 1 (diagonal).
  Jeder Start: 60 m flach und frei, mindestens zwei Fahrzeugausfahrten, mehr
  als die Reichweite leichter Artillerie von der Insel entfernt.
- **Versorgungspunkte:** 9–11. Vorschlag: Kirchplatz und Denkmalplatz auf der
  Insel (neutral), je 1 an der Ost- und Westbrücke (neutral), Bahnhof und
  Fabrikviertel (Nord), Geschützhügel und Uferviertel oder Gehöft (Süd), je 1
  in der Vorstadt hinter jedem Start. Namen nach dem Ort.

## 3. Spielregeln für den Aufbau

- **Brücken:** Steinbrücken als `env_bridge_stone` bzw. die Brückenarten aus
  115, jede einzeln sprengbar. Jede Seite behält nach dem Sprengen **einer
  beliebigen** Brücke noch Fahrzeugwege zur Insel. **Keine Furt**: Der Fluss ist
  tief, die Kaimauern sind hoch; das ist der Kern der Karte.
- **Insel:** Hauptstraßen von Brücke zu Brücke mit einer **freien Fahrbahn von
  mindestens 12 m** auch bei Schutt (Schutt nur an den Rändern), Gassen 6–8 m
  nur für Infanterie und leichte Fahrzeuge. Häuser besetzbar, je Block
  mindestens zwei Eingänge. Der **Kirchturm** ist der beste Aussichtspunkt:
  Sichtweite aus dem Turm prüfen; er darf keinen Start einsehen.
- **Zerstörung:** Etwa ein Drittel der Inselhäuser beginnt als `Damaged`, ein
  Sechstel als `Ruin` (wie im Bild), gleich verteilt auf die Nord- und
  Südhälfte der Insel. Die Ufer bleiben überwiegend heil.
- **Gleisfeld:** offen, kaum Deckung außer Güterwagen und Lokschuppen; nicht
  größer als 500 m ohne Deckung (Sicht 1/8).
- **Befestigung:** Gräben, Draht, Sperren und Bunker an den Ufern lassen an
  jedem Brückenkopf eine Fahrzeuggasse von mindestens 12 m.
- **Reichweiten:** Abstände rechnen und als Tabelle festhalten: Mörser und
  leichte Artillerie vom eigenen Ufer erreichen die Insel, nicht das gegnerische
  Ufer hinter dem Brückenkopf und keinen Start.
- Gebäudezahl deutlich unter 3000 (Insel etwa 150–250 Häuser).

## 4. Stufen

### Stufe 1: Entzerren und Skizze

Zuerst den entzerrten Grundriss (Fluss, Kaimauern, Brücken, Straßen,
Blöcke, Plätze, Gleisfeld, Befestigungen) als Bild neben der Vorlage zeigen,
mit den gewählten Maßen. Dann `data/map_sketches/island_town.toml`, erzeugt
mit `just gen-sketch island_town`. Bilder aus dem Spiel: Übersicht, jede
Brücke, Kirche mit Platz, Bahnhof mit Gleisfeld, Geschützhügel, neben dem
Ausschnitt der Vorlage.

### Stufe 2: Prüfen

- `just map-check island_town`, `just map-reach island_town`: jeder Start und
  jeder Versorgungspunkt für Fahrzeuge erreichbar, auch mit jeder einzelnen
  gesprengten Brücke (Brückenprobe wie 131/152).
- Test wie bei den anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien).
- Engste Fahrzeuggasse auf jeder Brücke und Hauptstraße messen (≥ 12 m).
- Sichtfeld-Bilder aus dem Kirchturm, vom Geschützhügel, vom Bahnhof;
  Reichweitentabelle; Gleichwertigkeit der Seiten als Tabelle (Brücken,
  Versorgung, Deckung, Weglängen).
- Wegsuche und Bildrate in der dichten Insel wie bei `river_city` und 156
  (`docs/PERFORMANCE.md`).
- **KI-Turnier** 1 gegen 1 und 2 gegen 2, auch mit vertauschten Teams,
  Ergebnis in `docs/AI.md`: Ziel höchstens 55 % für eine Seite. Weicht es ab,
  zuerst Deckung, Versorgung und Brückenköpfe der Karte anpassen, nicht die KI;
  Probleme der KI im Häuserkampf als offene Punkte.

### Stufe 3: Look und Abschluss

Sommerlicher Look, Schutt und Brandspuren auf der Insel, Wasser im Ring mit
Strömung. Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten
in `docs/MAPS.md` (mit Zuordnung Vorlage → Baustein, Entzerrung, Reichweiten-
und Sichttabelle), `docs/SOURCES.md` (Vorlage frei erfunden, kein reales
Vorbild, Fahne weggelassen), `just selftest`, `just lan-selftest`,
`just check`. Der Golden Hash ändert sich durch eine neue Karte nicht.

## 5. Nicht Teil dieses Prompts

- Neue Modelle (Fachwerk-Eisenbahnbrücke, Wasserturm, Denkmal, neugotische
  Kirche mit hohem Turm, Geschützstellung im Ringwall, falls nicht vorhanden):
  offene Punkte, gegebenenfalls ein eigener Prompt.
- Rauch und Brände als dauerhafte Kartenelemente.
- Fahrende Züge.
- Eine Mission mit vorgegebenen Armeen.
- KI-Verbesserungen für Häuserkampf und Brücken.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| Frage | Entscheidung | Grund |
|---|---|---|
| Vorlage | nur das Bild (Schrägansicht, wird entzerrt); Maße und Regeln aus diesem Prompt | keine SVG vorhanden |
| Symmetrie | keine; nach dem Bild unsymmetrisch, aber gleichwertig gemacht | Nord (Bahn, Industrie) und Süd (Felder, Befestigung) sind das Gesicht der Karte; Drehung würde sie zerstören. Vom Prompt vorgeschlagen, nicht vom Nutzer entschieden |
| Spieler | 1 gegen 1 und 2 gegen 2, Nord gegen Süd | wie 168/169; vom Prompt vorgeschlagen |
| Furten | keine | tiefer Fluss mit Kaimauern; Brücken sind der Kern |
| Fahne am Ratsbau | weggelassen | keine Hoheitszeichen |

## Ergebnis

(von der Session auszufüllen)
