# Prompt 140 – Karte nach Bildvorlage: Taunusfieber (Kurstadt mit Burgruine im Mittelgebirge, 2 gegen 2)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 139
> (Burgruine `[[castle_ruin]]`, Fachwerkhäuser, Villen, Kurhaus, Aussichtsturm)
> ist erledigt (`prompts/done/139-*`). Lies zuerst `CLAUDE.md`, dann diesen
> Prompt und `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in
> `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz (besonders
> `[[hill]]`, `[[wood]]`, `[[road]]`, `[[town]]`, `[[village]]`, `[[row]]`,
> `[[square]]`, `[[field]]`, `[[patchwork]]`, `[[rail]]`, `[[castle_ruin]]`); die
> Karten `river_town`, `winter_town` und `valley_fields` als Muster
> (`data/map_sketches/*.toml`); in `docs/ARCHITECTURE.md` Sicht („Seeing
> downhill“), indirektes Feuer, Wegfindung an Hängen, „Destruction“;
> `data/scale.toml` (`[range]`), `data/vision.toml`,
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`. Sind die
> Prompts 132 (Serpentinen-Auffahrten entlang einer Weglinie, Fahrprobe), 129
> (gestufte Höhen, starkes Relief), 127 (Burgfelsen) oder 123
> (Reichweitentabelle und Reichweitenprüfung) erledigt, auch deren Ergebnis;
> Bausteine und Werkzeuge wiederverwenden statt neu bauen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/taunusfieber/taunusfieber.webp`. Genau ansehen. Ein mit KI
> erzeugtes Bild, das eine Kurstadt im Taunus mit Burgruine zum Thema hat, aber
> nicht lagegetreu ist. Der Titelkasten nennt 4 km × 4 km, das Bild ist aber
> breiter als hoch; der Maßstabsbalken ist unstimmig und wird ignoriert.
>
> **Name:** Der Nutzer nennt die Karte **„Taunusfieber“**. Sie ist eine frei
> erfundene Lage; der Ortsname aus dem Titelkasten erscheint nicht in Karte,
> Markern oder Menüs.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte `taunus_fever` (im Spiel „Taunusfieber“ / „Taunus fever“) für **2
gegen 2** (Team West gegen Team Ost, auch 1 gegen 1): eine Kleinstadt in einem
bewaldeten Mittelgebirge, mit einer **Burgruine auf einem Felshügel** neben dem
Altstadtkern, zwei höheren Bergen im Norden, lockeren Villenvierteln an den
Hängen, einem Wiesental im Westen und dichtem Wald ringsum. **Bewusst
unsymmetrisch**; das Gleichgewicht kommt aus Ausgleich, nicht aus Spiegelung
(wie Prompt 125).

Die Vorlage zeigt:

- die **Burgruine** auf einem steilen, bewaldeten Felshügel links der Mitte, von
  einer Ringstraße umfahren;
- den **Altstadtkern** rechts daneben: dichte Zeilen mit roten und grauen
  Dächern, Marktplatz, Kirche, enge Gassen;
- den **höchsten Berg** im Nordwesten mit Felsbändern nach Süden, einem Turm auf
  dem Gipfel und einer **Serpentinenstraße** hinauf;
- einen **Felsgrat** im Nordosten mit Mauerresten und einem Steig;
- **lockere Wohnviertel** aus Einzelhäusern mit Gärten auf drei Seiten der
  Altstadt, dazwischen Wiesen und Obstgärten;
- ein **Wiesental mit Feldern** im Westen und Südwesten, sonst dichten
  **Mischwald** bis an die Ränder;
- eine **Hauptstraße** schräg von Südwest nach Nordost durch die Stadt, eine
  zweite von Norden nach Süden, die sich an einem Platz kreuzen; geschwungene
  Landstraßen und Waldwege;
- vier **Lager auf Waldlichtungen** in den Ecken; **fünf Sterne** als Ziele
  (Fuß des Nordwestbergs, Felsgrat, Burg, Markt, Südviertel).

**Stand 1944:** Sendemast, Sportplatz mit Laufbahn, Gewerbehallen und
Kreisverkehre der Vorlage gab es so nicht. Dafür: Aussichtsturm (139), Bahnhof
einer Stichbahn mit Güterschuppen, Kurhaus mit Kurpark, Sägewerk; Kreuzungen als
Plätze.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe, Relief, Abgrenzung

- **3,6 km × 2,4 km** (Seitenverhältnis der Vorlage), Biom `forest`, Wetter klar.
- **Starkes Relief** (Höhen über dem Marktplatz): Nordwestberg 110 m, Felsgrat
  75 m, Burghügel 45 m, Waldrücken im Südwesten und Südosten 40–60 m, das
  Wiesental 15 m tiefer als der Markt. Die Stadt liegt in einem Sattel, die
  Straßen folgen dem Gelände. Den Generator erweitern, falls die Höhen nicht
  reichen (Bausteine aus 129 verwenden, falls erledigt); bestehende
  Skizzenkarten bleiben Bit für Bit gleich.
- **Abgrenzung zu `castle_rock` (Prompt 127):** Jene Karte ist eine Großstadt
  nach Geodaten mit Burg auf einem Vulkanfels. Diese ist eine Kleinstadt im
  Wald nach einer Bildvorlage, mit Ruine, Bergen und viel Gelände außerhalb der
  Bebauung.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0), aus der Vorlage
abgegriffen. Die Session übernimmt die Werte und glättet sie nach den
Stufenbildern.

- **Burghügel:** Felshügel um (−330, 60), etwa 620 m × 480 m, 45 m hoch,
  **Felswände nach Süden und Westen**, steiler Waldhang nach Norden, flacherer
  Rücken nach Osten zur Altstadt. Oben die **Burgruine** (`[[castle_ruin]]`) um
  (−320, −50), Ringmauer etwa 130 m × 100 m mit vier Bastionen, Bergfried, Palas
  und Torbau nach Osten. **Zwei Zugänge:** eine Fahrstraße in einer Kehre von
  der Altstadt durch den Torbau in den Hof (für alle Fahrzeuge, die durch den
  Torbau passen; sonst bis vor das Tor) und ein Steig für Infanterie von Norden
  durch eine Bresche. Eine **Ringstraße** umfährt den Hügel am Fuß.
- **Nordwestberg („Turmberg“ nur als Arbeitsname):** Kuppe bei (−600, −960),
  Fläche etwa x −1150 … −180, y −1150 … −500, 110 m; **Felsbänder in zwei Stufen
  an der Südflanke**, dichter Wald. Oben eine Lichtung mit dem **Aussichtsturm**.
  Eine **Serpentinenstraße für alle Fahrzeuge** vom Platz am Fuß (−880, −650) in
  vier bis fünf Kehren über die West- und Nordseite zum Gipfel, und ein zweiter
  Fahrweg vom Gipfel nach Osten hinunter zur Nordstraße bei (−150, −800): Der
  Berg ist keine Sackgasse. Steigung und Breite aus den Regeln der Wegfindung
  ablesen (nicht raten), Fahrbahn mindestens 8 m, in Kehren breiter; Auffahrt
  entlang einer Weglinie aus 132 verwenden, falls erledigt, sonst hier bauen.
- **Felsgrat im Nordosten:** schmaler Grat von (480, −350) nach (800, −700),
  Gipfel bei (690, −615), 75 m; Felswände nach Nordwesten, Wald nach Südosten.
  Oben **Mauerreste** (zwei bis drei Stücke `env_ruin_wall`, ein Turmstumpf).
  Ein Fahrweg von Osten bis auf eine Schulter bei (850, −560), von dort nur
  Infanterie über einen Steig zum Gipfel.
- **Altstadt** um den **Marktplatz** (160, 110): Fachwerkzeilen an engen Gassen
  auf etwa 450 m × 400 m, Kirche am Markt, Rathaus, ein Brunnen; nach Westen
  steigt sie zum Burghügel an.
- **Kreuzplatz** bei (520, 310): Hier treffen sich die Straßen (statt des
  Kreisverkehrs). Daneben **Bahnhof** der Stichbahn bei (700, 250) mit
  Empfangsgebäude, Güterschuppen und zwei Gleisen, die Strecke läuft nach
  Südosten zum Kartenrand bei etwa (1100, 1200); ein **Sägewerk** mit Holzplatz
  bei (760, 420).
- **Kurviertel:** **Kurhaus** mit **Kurpark** (Rasen, Wege, Teich, alte Bäume,
  Musikpavillon) bei (120, −420) nördlich der Altstadt, etwa 300 m × 250 m.
- **Villenviertel** (Einzelhäuser mit Gärten, etwa 500–650 Häuser zusammen):
  - Nordviertel zwischen den Bergen, x −150 … 900, y −1100 … −450, an
    geschwungenen Hangstraßen;
  - Ostviertel x 900 … 1700, y −350 … 150;
  - Südviertel um (100, 540), x −300 … 600, y 350 … 950;
  - Südwestsiedlung als Straßendorf x −1250 … −550, y 780 … 950.
- **Wiesental** im Westen: x −1200 … −400, y −300 … 900, offene Wiesen, Felder
  (`[[patchwork]]`), Obstgärten, Hecken, zwei Gehöfte, ein kleiner **Bach** nach
  Süden (schmal, überall für Infanterie, für Fahrzeuge an zwei Wegbrücken und
  einer Furt). Es ist die einzige große offene Fläche und liegt unter den Augen
  der Burg.
- **Wald:** alles Übrige, Mischwald mit Nadelwald an den Nordhängen, von
  Waldwegen durchzogen; Waldrücken im Südwesten (um (−1300, 300)) und Südosten
  (um (1100, 700)).
- **Straßen:** **Hauptstraße** vom Südrand (−750, 1200) über den Kreuzplatz
  (520, 310) zum Ostrand (1800, −500); **Nordstraße** vom Nordrand (−160, −1200)
  durch Kurviertel und Altstadt zum Kreuzplatz und weiter zum Südrand
  (380, 1200); **Höhenstraße** vom Westrand (−1800, −250) am Fuß des
  Nordwestbergs vorbei (−880, −650) und nördlich des Burghügels in die Altstadt;
  eine Talstraße durch das Wiesental; Landstraßen von jedem Lager zur nächsten
  dieser Straßen. Alles geschwungen.

## 4. Spiel: unsymmetrisch und ausgeglichen

- **Starts** (60 m eben und frei, auf Waldlichtungen mit Baracken als Kulisse):
  Team West (−1540, −800) und (−1520, 790); Team Ost (1560, −850) und
  (1530, 780). Spielerzuordnung wie bei den anderen Skizzenkarten (Team 0 West:
  Spieler 0, 2; Team 1 Ost: 1, 3). 1 gegen 1 nutzt den ersten Start jedes Teams.
- **Versorgungspunkte** (neun): Burgruine (−300, 0) und Marktplatz (160, 110),
  beide umkämpft; Platz am Fuß des Nordwestbergs (−880, −650); Gipfel mit
  Aussichtsturm (−600, −960); Felsgrat-Schulter (850, −560); Bahnhof
  (700, 250); Südviertel (100, 540); Gehöft im Wiesental (−800, 400);
  Ostviertel (1250, −100).
- **Was jede Seite hat:**
  - **West:** den höchsten Berg mit Fahrstraße (beste Sicht und Feuerstellung)
    und den kürzeren Weg zur Burg; dafür das offene Wiesental vor der eigenen
    Tür und weniger Häuser als Deckung.
  - **Ost:** die Altstadt und die meisten Viertel (Deckung bis an den
    Burghügel), Bahnhof und Hauptstraße (kurze Wege), den Felsgrat als
    Beobachtungspunkt; dafür ist die Burg nur bergauf durch den Torbau zu
    nehmen.
- **Burg als Schlüssel:** Wer sie hält, sieht Markt und Wiesental; sie ist für
  Infanterie stark, von den Bergen aus aber zu beschießen. Prüfen, dass sie von
  beiden Seiten in etwa gleicher Zeit erreichbar ist (Fahrzeit vom ersten Start
  jedes Teams bis vor das Tor höchstens 15 % verschieden; sonst Wege anpassen).
- **Reichweiten** (indirekt 1/8, Sicht und direktes Feuer 1/4): Tabelle in
  `docs/MAPS.md` wie in Prompt 123: Was erreichen Geschütze vom Gipfel des
  Nordwestbergs, von der Felsgrat-Schulter, aus der Burg, aus den Lagern? Ziel:
  Vom Nordwestberg liegen Burg und Markt in Reichweite leichter Haubitzen, die
  Lager des Ostens nicht; vom Felsgrat aus der Markt und der Kurpark, die Lager
  des Westens nicht.
- **Ausgleich über das Turnier:** KI-Turnier **mit vertauschten Seiten**, Ziel
  höchstens 60 % für eine Seite. Stellschrauben, falls es kippt: Lage der
  Versorgungspunkte, zweite Zufahrt zum Felsgrat, Deckung im Wiesental (Hecken,
  Hohlweg), Lage der Starts. Was verstellt wurde, in den Entscheidungen
  festhalten.

## 5. Stufen

**Stufe 1 – Gelände und Straßen:** `data/map_sketches/taunus_fever.toml` mit
Relief, Felsbändern, Wald, Wiesental, Bach, Straßen, Serpentinen, Starts und
Zielen, erzeugt mit `just gen-sketch taunus_fever`; Höhenbild und Bilder aus dem
Spiel. Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben.

**Stufe 2 – Bebauung:** Burgruine, Altstadt, Kurviertel, Bahnhof, Sägewerk,
Villenviertel, Gehöfte, Aussichtsturm, Mauerreste am Felsgrat. Häuser stehen
eben (Geländestufen am Hang wie bei den anderen Karten), keine Häuser auf
Straßen oder an Felswänden.

**Stufe 3 – Prüfen und Abschluss:**

- `just map-check taunus_fever`, `just map-reach taunus_fever`: alle Starts und
  Ziele für Fahrzeuge erreichbar (Gipfel des Felsgrats nur für Infanterie),
  Felswände für alle zu.
- **Fahrprobe**, als Test: Der schwerste Panzer und ein Zugmittel mit gezogenem
  Geschütz fahren die Serpentine auf den Nordwestberg hinauf und über den
  Ostweg hinunter, und durch den Torbau in den Burghof (oder bis vor das Tor);
  Fahrzeiten in `docs/MAPS.md`.
- **Sichtprüfung:** Sichtfeld-Bilder vom Aussichtsturm-Gipfel, von der
  Felsgrat-Schulter, aus der Burg (alle vier Bastionen), vom Marktplatz und aus
  dem Wiesental.
- **Reichweitenprüfung** mit Test oder Werkzeug (aus 123, falls vorhanden).
- **Zerstörungsprobe:** Beschuss der Burg und der Altstadt; Breschen öffnen
  Wege, Fachwerk brennt, die Wegsuche bleibt im Zeitbudget.
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`), über
  Altstadt und Wald; reicht das Budget nicht, Villenviertel ausdünnen und
  festhalten.
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, auch mit vertauschten Seiten,
  Ergebnis in `docs/AI.md`: Findet die KI die Serpentine und den Burgweg, nutzt
  sie die Höhen für Artillerie, kämpft sie um Burg und Markt? Probleme als
  offene Punkte; die KI nicht umbauen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md` mit der Reichweitentabelle, Test wie bei den anderen
  Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
  `just selftest`. Der Golden Hash ändert sich nicht (neue Karte; die Regeln
  kamen mit 139); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Eine lagegetreue Karte der echten Stadt nach Geodaten (`tools/geo_import.py`,
  Muster Prompt 127); das wäre ein eigener Prompt.
- Eine Mission oder ein geschichtliches Szenario (März 1945); die Karte ist
  freies Gefecht.
- Züge auf der Stichbahn, Herbst- oder Wintervariante (über `base` in der Skizze
  leicht nachzuholen), Nebel im Tal.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
