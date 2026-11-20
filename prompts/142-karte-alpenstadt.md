# Prompt 142 – Karte nach Bildvorlage: Alpenstadt (Stadt am Bergfluss, 3 gegen 3)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 134 (Großstadthäuser, `[[city_block]]`) und 141 (Dom, Laubenhäuser,
> Bergfestung, `[[airfield]]`, `[[rail_yard]]`, Bogenviadukt) sind erledigt
> (`prompts/done/134-*`, `prompts/done/141-*`). Empfohlen nach 115 (lange
> Brücken), 132 (Serpentinen-Auffahrten, Fahrprobe) und 139 (Villenviertel mit
> Gärten). Lies zuerst `CLAUDE.md`, dann diese Prompts und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[river]]`, `[[crossing]]`,
> `[[hill]]`, `[[wood]]`, `[[road]]`, `[[rail]]`, `[[square]]`, `[[city_block]]`
> und die Zusammenstellungen aus 141), „Bridges“; die Karten `river_city` und
> `desert_lagoon` (sechs Starts) als Muster; in `docs/ARCHITECTURE.md` Sicht,
> indirektes Feuer, Wegfindung, „Destruction“; `data/scale.toml` (`[range]`),
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`. Sind die
> Prompts 136 oder 138 (Stadtkarten Eisenturm, Uhrturm) erledigt, auch deren
> Ergebnis: dort gefundene Lösungen (Boulevards, Ufermauern, Blockdichte,
> Bildrate) übernehmen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/alpenstadt/alpenstadt.webp`. Genau ansehen. Ein mit KI erzeugtes
> Bild, das eine Stadt im Alpental zum Thema hat, aber nicht lagegetreu ist. Der
> Titelkasten nennt 6 Spieler und 4 km × 4 km, das Bild ist aber breiter als
> hoch; der Maßstabsbalken ist unstimmig und wird ignoriert. Die Karte ist eine
> **frei erfundene Stadt** mit neutralem Namen; der Ortsname aus dem Titelkasten
> erscheint nicht in Karte, Markern oder Menüs.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte `alpine_city` (im Spiel „Alpenstadt“ / „Alpine city“) für **sechs
Spieler, 3 gegen 3** (Team West gegen Team Ost; auch 2 gegen 2 und 1 gegen 1):
eine Stadt in einem Alpental, durch die ein schneller Bergfluss in einem großen
Bogen läuft, mit sieben Brücken, einer Festung auf einem Felshügel, einem
Flugplatz, einer Altstadt um einen Dom, einem Bahnhof mit Gleisfeld und Bergen
an den Rändern. **Bewusst unsymmetrisch**; das Gleichgewicht kommt aus Ausgleich
(wie Prompt 125 und 140).

Die Vorlage zeigt:

- einen **türkisfarbenen Bergfluss**: von Westen in der Südhälfte herein, dann
  schräg nach Nordosten durch die Mitte, dann am Nordrand nach Osten hinaus;
  Ufermauern in der Stadt, Baumreihen, weißes Wasser an den Brücken;
- **sieben Brücken**: zwei im Westen, drei auf der schrägen Mittelstrecke, zwei
  im Nordosten;
- eine **Festung mit Kirchturm** auf einem Felshügel in der Nordmitte, mit
  Serpentinenweg;
- **Schneeberge** in der Nordwestecke, bewaldete Berge mit Felsen am Südrand;
- einen **Flugplatz** im Westen (Rollbahn, Grasfläche, Halle, Turm);
- die **Altstadt** östlich der Mitte: dichte Blöcke mit roten Dächern um einen
  Platz mit einem **Dom mit zwei Türmen**;
- einen **Bahnhof mit breitem Gleisfeld** östlich der Altstadt, die Strecke
  läuft nach Süden und steigt über **Bogenviadukte** in die Berge;
- lockere Viertel und Parks auf beiden Ufern, Wald an den Hängen;
- **sechs Lager**: Nordwest, West, Südwest, Nordost, Ost, Südost; **acht
  Sterne** als Ziele.

**Stand 1944:** Stadion, Autobahnkreuz, Kreisverkehre und der moderne
Kontrollturm der Vorlage gab es so nicht. Dafür: Güterbahnhof mit Lagerhäusern
statt des Stadions, eine Bergstraße und die Bahnstrecke auf Viadukten statt der
Autobahn, Flugleitungsbau der 1930er Jahre.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe, Relief, Spielerzahl

- **4,0 km × 2,664 km** (Seitenverhältnis der Vorlage, Breite am Höchstmaß,
  Vielfache von 4 m), Biom `forest`, Wetter klar. Halten Bildrate oder Wegsuche
  das nicht im Budget (`docs/PERFORMANCE.md`), auf 3,6 km × 2,4 km verkleinern
  (alle Maße mit 0,9) und festhalten.
- **Relief** (Höhen über dem Domplatz): Talboden fast eben, leicht zum Fluss
  geneigt; Festungshügel 80 m; Berge am Nordwest- und Südrand 200 m und mehr,
  **unpassierbar** ab dem Hangfuß (Fels und Steilwald), mit hellen Schneekuppen
  in der Nordwestecke (Look aus Prompt 122 verwenden, falls erledigt; sonst
  heller Fels und als offener Punkt). Bewaldete Vorberge 30–60 m im Südwesten
  und Südosten, befahrbar über Waldstraßen.
- **Sechs Spieler:** Die Simulation trägt das (`desert_lagoon` hat sechs
  Starts). Prüfen, dass Menü, Lobby, LAN, KI-Turnier und Farben sechs Spieler
  auf dieser Karte anbieten; Lücken dort schließen, wo es eine Zeile ist, sonst
  als offenen Punkt melden.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0), aus der Vorlage
abgegriffen. Die Session übernimmt die Werte und glättet sie nach den
Stufenbildern.

- **Fluss** als `[[river]]`, etwa 100 m breit, schnell, **nirgends zu
  durchwaten**: Westrand (−2000, 150) → (−1600, 250) → (−700, 300) → (−350, 220)
  → (−230, 60) → (−50, −100) → (80, −220) → (220, −400) → (340, −560) →
  (560, −860) → (720, −940) → (1000, −940) → (1230, −810) → (1650, −680) →
  Ostrand (2000, −680). In der Stadt **Ufermauern** mit Uferstraßen und
  Baumreihen, außerhalb Kiesbänke und Auwald.
- **Sieben Brücken** (`[[crossing]]`, achsparallel; die schräge Mittelstrecke
  wird an jeder Brücke kurz gerade geführt): Nord-Süd bei (−1600, 250) und
  (−700, 300); West-Ost bei (−230, 60), (80, −220) und (340, −560); Nord-Süd
  bei (720, −940) und (1230, −810). Mit Prompt 115 Brücken über die volle
  Breite, sonst verengt der Generator das Wasser wie bisher (als Entscheidung
  festhalten). Brücken sind sprengbar; weil es keine Furten gibt, prüfen, dass
  nach dem Sprengen aller Brücken kein Spieler ohne Ziele dasteht (Abschnitt 4).
- **Zwei Landmassen:** das **Nordwestland** (nördlich und westlich des Flusses,
  dazu ein etwa 400 m tiefer Streifen am Nordrand bis zum Ostrand) und das
  **Südostland** (alles Übrige).
- **Festungshügel** in der Nordmitte um (−200, −1050), etwa 620 m × 440 m, 80 m,
  **Felswände nach Süden und Osten**, Wald nach Norden. Oben die **Bergfestung**
  (141) mit Ringmauer, Türmen, Wohnbau und Kapelle. Eine **Serpentinenstraße für
  alle Fahrzeuge** von Westen (Fuß bei (−550, −800)) durch den Torbau in den
  Hof und ein zweiter Fahrweg nach Norden hinunter, dazu ein Steig für
  Infanterie von Süden. Steigung und Breite aus den Regeln der Wegfindung
  ablesen (nicht raten); Auffahrt aus 132 verwenden, falls erledigt.
- **Flugplatz** (`[[airfield]]`) im Westen: Rollbahn West-Ost von (−1550, −230)
  bis (−500, −230), 50 m breit, Grasfläche darum etwa 1200 m × 350 m; Vorfeld,
  zwei Hallen, Flugleitungsbau und Baracken südlich davon um (−860, −60). Die
  größte offene Fläche der Karte.
- **Altstadt** östlich der Mitte, zwischen Mittelstrecke und Bahnhof: **Domplatz**
  bei (470, 30), **Dom** an seiner Westseite bei (340, −100), Laubenhäuser in
  geschlossenen Zeilen, Blöcke (`[[city_block]]`, Fassadenfamilie „Alpenstadt“,
  drei bis fünf Geschosse) auf etwa 900 m × 900 m, enge Gassen und drei breite
  Straßen zu den Mittelbrücken; ein großer Hofbau (Kloster oder Residenz) mit
  Garten bei (580, −230).
- **Flussbogen-Viertel** im Nordosten zwischen Nordstrecke und Altstadt: Blöcke
  um einen runden Park bei (800, −630).
- **Bahnhof** (`[[rail_yard]]`): Gleisfeld von (900, 450) nach (1450, −550),
  etwa 150 m breit, acht bis zehn Gleise, Bahnsteige mit Dächern, Empfangsgebäude
  zur Altstadt hin bei (1050, −150), Lokschuppen und Stellwerk am Nordende,
  **Güterbahnhof mit Lagerhäusern** am Südende um (1050, 400) (statt des
  Stadions). Straßen queren das Gleisfeld an zwei Stellen (eine Unterführung
  bzw. Brücke, ein Übergang).
- **Bahnstrecke und Bergstraße nach Süden:** Die Strecke verlässt den Bahnhof
  nach Süden, schwenkt nach Westen und steigt auf einem **Bogenviadukt** von
  etwa (900, 700) über (300, 900) zum Südrand bei (−50, 1332); ein zweiter Ast
  auf einem Viadukt von (700, 600) nach Osten zum Ostrand bei (2000, 250). Unter
  den Viadukten führen Straßen durch (benannte Durchfahrten). Eine Bergstraße
  vom Platz bei (670, 520) nach Süden zum Südrand.
- **Westviertel am Nordufer** zwischen Flugplatz, Festungshügel und Fluss:
  lockere Blöcke, Einzelhäuser und ein Park bei (−370, −540); ein Kasernen- oder
  Schulbau am Hangfuß.
- **Südwestviertel** am Südufer um (−1250, 470): Einzelhäuser und kleine Blöcke
  an einer Uferstraße; südlich davon Wald und Vorberge.
- **Südviertel** um einen Park bei (470, 630); **Nordostsiedlung** auf dem
  Nordstreifen um (1500, −1100) mit Gärten und Feldern; **Gewerbe** (Lagerhallen,
  Werkstätten) im Osten um (1550, 150).
- **Straßen:** Uferstraßen beiderseits; eine **Talstraße** am Nordufer vom
  Westrand über den Flugplatz und das Westviertel zur Brücke (720, −940) und
  weiter über den Nordstreifen zum Ostrand; eine **Südstraße** vom Westrand
  durch das Südwestviertel zur Altstadt und weiter am Güterbahnhof vorbei zum
  Ostrand; Querstraßen über jede Brücke; Waldstraßen zu den Lagern. Hauptstraßen
  etwa 20–25 m breit, Gassen 10–12 m.
- **Wald** an allen Hängen und um die Lager, Auwald am Fluss außerhalb der
  Stadt.

## 4. Spiel: 3 gegen 3, unsymmetrisch und ausgeglichen

- **Starts** (60 m eben und frei, auf Lichtungen bzw. Plätzen mit Baracken als
  Kulisse):
  - **Team West:** Nordwest (−1270, −940), West (−1780, −110), Südwest
    (−1480, 720);
  - **Team Ost:** Nordost (1520, −1120), Ost (1750, −190), Südost (1550, 720).
  Spieler 0, 2, 4 sind Team West, Spieler 1, 3, 5 Team Ost (nach dem Muster der
  anderen Skizzenkarten; Reihenfolge so, dass 1 gegen 1 West gegen Ost und 2
  gegen 2 West und Südwest gegen Ost und Südost ergibt).
- **Jedes Team steht auf beiden Ufern:** West hat zwei Lager im Nordwestland und
  eines (Südwest) im Südostland; Ost hat zwei im Südostland und eines (Nordost)
  auf dem Nordstreifen des Nordwestlands. Die Front läuft also nicht am Fluss
  entlang, sondern quer von der Festung über die Mittelbrücken zur Altstadt und
  nach Süden; der Fluss trennt die Verbündeten. Die Brücken im Westen und
  Nordosten sind Nachschubwege im eigenen Hinterland, die drei Mittelbrücken
  sind umkämpft.
- **Versorgungspunkte** (neun): Festung (−185, −1050); Park im Westviertel
  (−370, −540); Flugplatz (−860, −60); Mittelbrücke (−230, 60), umkämpft;
  Domplatz (470, 30); Park im Flussbogen (800, −630); Bahnhof (1290, −300);
  Südwestviertel (−1250, 470); Südpark (470, 630).
- **Was jede Seite hat:**
  - **West:** Flugplatz (offen, schnelle Wege, kaum Deckung), den kürzeren Weg
    zur Festung, weniger Häuser;
  - **Ost:** Altstadt, Dom und Bahnhof (viel Deckung, kurze Wege), dafür die
    Festung über sich und den Fluss im Rücken der Altstadt.
- **Ausgleich prüfen:** Fahrzeit vom nächsten Lager jedes Teams zur Festung, zur
  Mittelbrücke und zum Domplatz in einer Tabelle (`docs/MAPS.md`); weicht eine
  Seite um mehr als 20 % ab, Wege oder Ziele verschieben. Mit allen Brücken
  gesprengt behält jeder Spieler mindestens zwei erreichbare Ziele.
- **Reichweiten** (indirekt 1/8): Tabelle wie in Prompt 123: Was erreichen
  Geschütze von der Festung, vom Flugplatz, vom Bahnhof, aus den Lagern? Ziel:
  Von der Festung liegen Mittelbrücken und Domplatz in Reichweite leichter
  Haubitzen, kein Lager der Gegenseite.
- **KI-Turnier mit vertauschten Seiten**, Ziel höchstens 60 % für eine Seite.
  Stellschrauben: Lage der Ziele, zweite Zufahrt zur Festung, Deckung am
  Flugplatz (Splitterwälle, Baracken), Lage der Starts. Was verstellt wurde, in
  den Entscheidungen festhalten.

## 5. Stufen

**Stufe 1 – Gelände, Fluss, Verkehr:** `data/map_sketches/alpine_city.toml` mit
Relief, Bergen, Fluss, Brücken, Straßen, Bahn mit Viadukten, Flugplatz, Starts
und Zielen, erzeugt mit `just gen-sketch alpine_city`; Bilder aus dem Spiel.
Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben.

**Stufe 2 – Bebauung:** Festung, Altstadt mit Dom, Blöcke, Bahnhof,
Güterbahnhof, Viertel, Parks, Wald.

**Stufe 3 – Prüfen und Abschluss:**

- `just map-check alpine_city`, `just map-reach alpine_city`: alle Starts und
  Ziele für Fahrzeuge erreichbar, Berge und Felswände für alle zu, keine Blöcke
  im Wasser, auf Straßen oder Gleisen.
- **Fahrprobe**, als Test: der schwerste Panzer und ein Zugmittel mit gezogenem
  Geschütz die Serpentine zur Festung hinauf und über den Nordweg hinunter;
  Durchfahrten unter den Viadukten; Querung des Gleisfelds.
- **Sichtprüfung:** Sichtfeld-Bilder von der Festung, vom Domplatz, von der
  Mittelbrücke, vom Flugplatz und vom Bahnhof.
- **Reichweitenprüfung** mit Test oder Werkzeug (aus 123, falls vorhanden).
- **Zerstörungsprobe:** Beschuss der Altstadt, des Doms und eines Viadukts;
  Wege und Sicht stimmen danach, die Wegsuche bleibt im Zeitbudget.
- **Bildrate und Wegsuche** in jeder Ansicht mit sechs Spielern
  (`docs/PERFORMANCE.md`); reicht das Budget nicht, Randviertel ausdünnen oder
  die Verkleinerung aus Abschnitt 2.
- **KI-Turnier** 3 gegen 3, 2 gegen 2 und 1 gegen 1, auch mit vertauschten
  Seiten, Ergebnis in `docs/AI.md`: Arbeiten die drei KI-Spieler eines Teams
  über den Fluss hinweg zusammen, kommen sie über die Brücken, kämpfen sie um
  Festung und Altstadt? Probleme als offene Punkte; die KI nicht umbauen.
- `just selftest`, `just lan-selftest` (Lobby mit sechs Plätzen auf dieser
  Karte), Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md` mit Fahrzeit- und Reichweitentabelle, Test wie bei den anderen
  Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`. Der Golden
  Hash ändert sich nicht (neue Karte; die Regeln kamen mit 134 und 141); wenn
  doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Eine lagegetreue Karte der echten Stadt nach Geodaten (`tools/geo_import.py`,
  Muster Prompt 127); das wäre ein eigener Prompt.
- Flugplatz als Regel (Bonus für Luftunterstützung), startende Flugzeuge, Züge.
- Schnee im Tal, Wintervariante (über `base` in der Skizze nachzuholen), Föhn
  oder Nebel als Wetter.
- Eine Mission oder ein geschichtliches Szenario; die Karte ist freies Gefecht.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
