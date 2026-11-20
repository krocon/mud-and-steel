# Prompt 145 – Karte nach Bildvorlage: Ockerstadt (sehr große Stadt am Flussbogen, zwölf Spieler)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 143 (sehr große Karten, zwölf Spieler; seine Empfehlung für Größe,
> Gebäudezahl und Einheiten gilt hier), 144 (Stadion, Mietshausblöcke
> „Südstadt“, Palazzine, Palazzo, Halle, Sportanlagen), 134 (`[[city_block]]`)
> und 120 (Mittelmeer-Biom) sind erledigt (`prompts/done/…`). Empfohlen nach 115
> (lange Brücken). Lies zuerst `CLAUDE.md`, dann diese Prompts und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz, „Bridges“, die neue Größengrenze und die
> Zusammenstellungen aus 144; `river_city` als Muster; `docs/TEAMS.md`; in
> `docs/ARCHITECTURE.md` Sicht, Wegfindung, „Destruction“; `data/scale.toml`
> (`[range]`), `crates/cli/src/gen_sketch.rs`, `docs/AI.md`,
> `docs/PERFORMANCE.md`. Sind 136, 138 oder 142 (Stadtkarten) erledigt, auch
> deren Ergebnis: dort gefundene Lösungen übernehmen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/ockerstadt/ockerstadt.webp`. Genau ansehen. Ein mit KI
> erzeugtes Bild, das ein Viertel einer südlichen Hauptstadt zum Thema hat, aber
> nicht lagegetreu ist. Der Titelkasten nennt 12 Spieler und 4 km × 4 km, das
> Bild ist aber breiter als hoch; der Maßstabsbalken ist unstimmig und wird
> ignoriert. Die Karte ist eine **frei erfundene Stadt** mit neutralem Namen;
> der Ortsname aus dem Titelkasten erscheint nicht in Karte, Markern oder Menüs.
>
> **Dieser Prompt läuft in vier Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Die größte Karte des Spiels: `ochre_city` (im Spiel „Ockerstadt“ / „Ochre city“)
für **zwölf Spieler, 6 gegen 6** (Team Nordwest gegen Team Südost; auch 4 × 3,
3 × 4 und kleinere Besetzungen): eine südliche Großstadt in Ocker und
Terrakotta, um die ein Fluss in einem großen Bogen läuft, mit acht Brücken,
einem Stadion, Hofbauten, Parks auf Hügeln, einem dichten Wohnviertel und
bewaldeten Hügeln ringsum, auf denen die zwölf Lager liegen.

Die Vorlage zeigt:

- einen **Fluss im großen C-Bogen**: von Süden herein, nach Westen ausholend,
  im Norden nach Osten zurück, dann in einer engen S-Schleife nach Nordosten
  und am Nordrand hinaus; Ufermauern, Uferstraßen mit Baumreihen;
- **acht Brücken**, darunter eine auf der Nord-Süd-Achse;
- die **Innenstadt im Bogen**: große Blöcke mit Innenhöfen, Hofbauten, ein
  Rondell mit Brunnen in der Mitte, ein kleines Stadion, eine breite
  **Nord-Süd-Achse** und Ringstraßen parallel zum Fluss;
- ein **großes ovales Stadion** am Westufer mit Hallen und einem Hofbau daneben;
- einen **runden Park auf einem Hügel** östlich der Innenstadt und einen Park im
  Norden; **Sportplätze** im Nordosten;
- ein **sehr dichtes Wohnviertel** aus kleinen Häusern im Südosten, weitere
  Viertel im Norden, Westen und Südwesten;
- eine **breite Querstraße** im Norden von Rand zu Rand (in der Vorlage eine
  Hochstraße mit Kleeblättern);
- **bewaldete Hügel** an allen Rändern mit **zwölf Lagern** auf Lichtungen: vier
  am Nordrand, vier am Südrand, je zwei am West- und Ostrand; **elf Sterne** als
  Ziele.

**Stand 1944:** Hochstraße, Kleeblattkreuzungen, Kreisverkehre und das überdachte
moderne Stadion gab es so nicht. Dafür: eine breite Allee mit Baumreihen auf
Geländehöhe, Plätze an den Kreuzungen, ein offenes Stadion der 1930er Jahre
(144).

Die Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe und Biom

- **6,0 km × 4,0 km**, wenn Prompt 143 das trägt; sonst die dort empfohlene
  Größe (alle Maße aus Abschnitt 3 im gleichen Verhältnis verkleinern, nicht
  unter 4,0 km × 2,664 km; darunter passen zwölf Lager nicht). Größe und
  Gebäudezahl nach der Empfehlung aus 143 festlegen und als Entscheidung
  festhalten.
- Biom **`mediterranean`** (120): helles Licht, trockener Boden, Pinien und
  Zypressen an Straßen und in Parks, Macchia und Steineichen an den Hängen.
  Wetter klar.
- **Relief** (Höhen über der Innenstadt): Innenstadt und Flussaue fast eben;
  Parkhügel 30 m; Hügel an den Rändern 40–80 m mit einzelnen Felsbändern, die
  Lager auf Schultern in 30–50 m Höhe.
- **Bewusst unsymmetrisch**, aber die Lager liegen punktsymmetrisch (Abschnitt
  4); Gleichgewicht über Ausgleich und Turnier.

## 3. Aufbau

Richtwerte in Metern für 6 km × 4 km (x nach Osten, y nach Süden, Mitte 0/0),
aus der Vorlage abgegriffen. Die Session übernimmt die Werte und glättet sie
nach den Stufenbildern.

- **Fluss** als `[[river]]`, etwa 110 m breit, nicht zu durchwaten: Südrand
  (−320, 2000) → (−600, 1280) → (−1280, 880) → (−1620, 400) → (−1600, 0) →
  (−1280, −400) → (−760, −660) → (400, −640) → (800, −360) → (1080, −140) →
  (1360, −240) → (1360, −680) → (1160, −1000) → (1320, −1240) → (2000, −1320)
  → (2360, −1480) → Nordrand (2360, −2000). **Ufermauern** in der Stadt,
  Uferstraßen mit Baumreihen; an den Rändern Auwald.
- **Acht Brücken** (`[[crossing]]`, achsparallel; schräge Strecken werden an der
  Brücke kurz gerade geführt): West-Ost bei (−1600, −40) und (−1560, 560);
  Nord-Süd bei (−700, −660) und, auf der Nord-Süd-Achse, bei (−120, −650);
  bei (740, −400); Nord-Süd bei (1200, −190); Nord-Süd bei (2060, −1340);
  West-Ost bei (−360, 1540). Mit Prompt 115 Brücken über die volle Breite,
  sonst verengt der Generator das Wasser wie bisher (als Entscheidung
  festhalten). Brücken sind sprengbar.
- **Innenstadt im Bogen** (Halbinsel, nach Südosten offen), etwa x −1450 … 500,
  y −550 … 1400:
  - **Nord-Süd-Achse** bei x ≈ −120 vom Nordrand über die Achsenbrücke bis zum
    Südrand, etwa 35 m breit, mit Baumreihen;
  - **Rondell** mit Brunnen bei (−240, 230); zwei **Ringstraßen** parallel zum
    Flussbogen; Radialstraßen zu den Brücken;
  - **Mietshausblöcke** (`[[city_block]]`, „Südstadt“), etwa 90–120 Blöcke von
    90–180 m Kantenlänge; drei **Palazzi** (Hofbauten) bei (−760, 300),
    (220, 320) und (−500, −150);
  - **kleines Stadion** bzw. Sportfeld mit Tribüne bei (−150, 580);
  - Plätze mit Brunnen an den Brückenköpfen.
- **Westufer:** das **Stadion** (`[[stadium]]`) bei (−2000, −340), Längsachse
  Nord-Süd, mit Vorplatz zur Brücke (−1600, −40); nördlich davon eine **Halle**
  und ein **Palazzo** um (−1650, −680); südlich ein Viertel aus Blöcken bis
  (−1900, 700).
- **Nordufer:** Blöcke und Palazzine zwischen Fluss und Nordallee; ein **Park**
  bei (780, −780).
- **Nordallee:** breite Allee (etwa 40 m, vier Baumreihen) von Westrand
  (−3000, −1200) über (−400, −1250) und (1300, −1050) zum Ostrand
  (3000, −1150), mit Plätzen an den Kreuzungen und einer eigenen **Brücke über
  die S-Schleife** bei (1160, −1000) (zählt nicht zu den acht; wenn das zu dicht
  an der Schleife liegt, die Allee über die Brücke (2060, −1340) führen und
  festhalten).
- **Parkhügel** östlich der Innenstadt: runder Park um (640, 80), etwa 500 m
  Durchmesser, 30 m hoch, Wege im Stern, Pinien, ein Aussichtsplatz in der
  Mitte.
- **Schleifenviertel** im Nordosten in der S-Schleife um (1050, −450): Blöcke
  und Palazzine; östlich davon die **Sportanlagen** (Spielfelder, Tennis, ein
  Umkleidebau) um (1760, −830).
- **Südostviertel:** das dichte Wohnviertel aus **Palazzine**
  (Palazzine-Viertel aus 144), etwa x 200 … 2800, y 250 … 1450, an leicht
  gebogenen Straßen, mit zwei Plätzen (bei (1680, 640) und (900, 900)) und einer
  Kirche mit Campanile. Größenordnung 1000–1500 Häuser, nach der Empfehlung aus
  143.
- **Weitere Viertel:** Nordwesten um (−1400, −1300), Norden um (0, −1500),
  Südwesten um (−1800, 1300), jeweils Palazzine und kleine Blöcke.
- **Hügel und Wald** an allen Rändern: Pinien- und Eichenwald, Macchia,
  einzelne Felsbänder; Serpentinenstraßen zu den Lagern, **jedes Lager mit zwei
  Zufahrten**.
- **Straßen:** außer Achse, Ringen und Nordallee eine **Südstraße** vom Westrand
  durch das Südwestviertel über die Brücke (−360, 1540) ins Südostviertel und
  zum Ostrand; Uferstraßen; Hauptstraßen 20–25 m, Nebenstraßen 10–14 m.

## 4. Spiel: zwölf Spieler

- **Starts** (60 m eben und frei, auf Lichtungen mit Baracken als Kulisse);
  jedes Lager hat sein Gegenstück unter (x, y) → (−x, −y):
  - **Team Nordwest:** Nord 1 (−1700, −1720), Nord 2 (−600, −1750), Nord 3
    (650, −1750), Nord 4 (1750, −1720), West 1 (−2650, −800), West 2
    (−2650, 300);
  - **Team Südost:** Süd 1 (1700, 1720), Süd 2 (600, 1750), Süd 3 (−650, 1750),
    Süd 4 (−1750, 1720), Ost 1 (2650, 800), Ost 2 (2650, −300).
  Lager, die danach auf dem Fluss oder am falschen Ufer lägen (Nord 4 an der
  Schleife, Süd 3 am Flusseintritt), um bis zu 200 m verschieben und das
  Gegenstück mitziehen. Gerade Spielernummern Team Nordwest, ungerade Team
  Südost, in der Reihenfolge so, dass kleinere Besetzungen sinnvolle Paare
  ergeben (1 gegen 1: West 1 gegen Ost 1; 2 gegen 2: dazu Nord 2 und Süd 2;
  3 gegen 3: dazu Nord 3 und Süd 3). Die Zuordnung als Tabelle in
  `docs/MAPS.md`.
- **Andere Teilungen** als Vorschlag in den Starts hinterlegt bzw. in
  `docs/MAPS.md` beschrieben: **4 × 3** (Nord, Ost, Süd, West: je drei
  benachbarte Lager) und **3 × 4**, soweit 143 mehr als zwei Teams in der Lobby
  gebracht hat.
- **Versorgungspunkte** (dreizehn): Rondell (−240, 230), umkämpft;
  Achsenbrücke Nord (−120, −650); Achse Süd (−250, 940); Stadion-Vorplatz
  (−1780, 10); Westbrücke Süd (−1360, 640); Nordwestufer (−1140, −620);
  Nordallee an der Achse (−370, −1240); Nordpark (780, −780); Sportanlagen
  (1760, −830); Parkhügel (640, 80); Brückenkopf Ost (1230, 10); Platz im
  Südostviertel (1680, 640); Platz Süd (900, 900). Die Verteilung auf beide
  Hälften prüfen und, wenn eine Seite deutlich näher an mehr Punkten liegt, zwei
  bis drei verschieben.
- **Aufbau des Kampfes:** Die Innenstadt im Bogen ist die Mitte; Team Nordwest
  kommt über die Nord- und Westbrücken hinein, Team Südost von Land über den
  offenen Südosten und über die Südbrücke. Dafür hat Nordwest das Stadion und
  die Nordallee (schnelle Querverbindung), Südost das dichte Wohnviertel
  (Deckung, aber enge Straßen) und den Parkhügel (Sicht über Innenstadt und
  Schleife).
- **Ausgleich prüfen:** Fahrzeit von jedem Lager zum nächsten Ziel, zum Rondell
  und zur nächsten Brücke als Tabelle (`docs/MAPS.md`); Summe je Team höchstens
  15 % verschieden, sonst Wege, Brücken oder Ziele verschieben. Mit allen
  Brücken gesprengt behält jeder Spieler mindestens zwei erreichbare Ziele.
- **Reichweiten** (indirekt 1/8): Tabelle wie in Prompt 123: Was erreichen
  Geschütze vom Parkhügel, vom Stadion, von der Nordallee, aus den Lagern? Ziel:
  kein Lager in Reichweite eines gegnerischen Lagers.
- **Einheitengrenze:** die Obergrenze je Spieler bei zwölf Spielern aus 143
  übernehmen (Karteneinstellung bzw. Vorgabe der Lobby), damit Tick und Bildrate
  im Budget bleiben.

## 5. Stufen

**Stufe 1 – Gelände, Fluss, Verkehr:** `data/map_sketches/ochre_city.toml` mit
Relief, Fluss, Brücken, Achse, Ringen, Allee, Hauptstraßen, Starts und Zielen,
erzeugt mit `just gen-sketch ochre_city`; Erzeugungszeit, Dateigröße und
Ladezeit nennen. Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben.

**Stufe 2 – Innenstadt und Wahrzeichen:** Blöcke, Palazzi, Stadion, Hallen,
Parks, Sportanlagen, Plätze.

**Stufe 3 – Viertel und Hügel:** Palazzine-Viertel, Randviertel, Wald, Lager;
dabei laufend Bildrate und Tickzeit messen und die Dichte danach einstellen.

**Stufe 4 – Prüfen und Abschluss:**

- `just map-check ochre_city`, `just map-reach ochre_city`: alle zwölf Starts
  und alle Ziele für Fahrzeuge erreichbar, jedes Lager über zwei Zufahrten,
  keine zugebauten Straßen, keine Häuser im Wasser oder auf Straßen.
- **Sichtprüfung:** Sichtfeld-Bilder vom Parkhügel, vom Rondell, von der
  Achsenbrücke, vom Stadion-Vorplatz, aus einer Gasse des Südostviertels.
- **Zerstörungsprobe:** Beschuss eines Blockviertels, des Stadions und eines
  Stücks Palazzine-Viertel; Wege und Sicht stimmen danach, die Wegsuche bleibt
  im Zeitbudget.
- **Bildrate, Tick und Wegsuche** in jeder Ansicht mit zwölf Spielern und der
  Einheitengrenze (`docs/PERFORMANCE.md`), auch der schlechteste Tick in einem
  großen Gefecht. Reicht das Budget nicht: zuerst Palazzine ausdünnen, dann
  Randviertel durch Parks ersetzen, zuletzt die Karte verkleinern.
- **KI-Turnier** 6 gegen 6, 3 gegen 3 und 1 gegen 1, auch mit vertauschten
  Seiten (Ziel höchstens 60 % für eine Seite), Ergebnis in `docs/AI.md`:
  Arbeiten sechs KI-Spieler zusammen, kommen sie über die Brücken, verteilt sich
  der Kampf oder ballt er sich an einer Stelle, endet es im Stillstand? Dauer
  einer Partie nennen. Probleme als offene Punkte; die KI nicht umbauen.
- `just selftest`, `just lan-selftest` (Lobby mit zwölf Plätzen auf dieser
  Karte), Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md` mit Zuordnungs-, Fahrzeit- und Reichweitentabelle, Test wie bei
  den anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`.
  Der Golden Hash ändert sich nicht (neue Karte; die Regeln kamen mit 134 und
  144); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Antike Bauten und weitere Wahrzeichen; eine lagegetreue Karte der echten Stadt
  nach Geodaten (`tools/geo_import.py`, Muster Prompt 127).
- Straßenbahn, Bahnlinien, Boote auf dem Fluss (Prompt 119).
- KI-Verbesserungen für große Teams.
- Eine Mission oder ein geschichtliches Szenario; die Karte ist freies Gefecht.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
