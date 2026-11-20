# Prompt 162 – Karte nach echten Geodaten: Fachwerkstadt (Vorbild Hameln an der Weser, Stand 1944)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 139
> (Fachwerkhaus und Eckhaus, Villa, Villenviertel mit Gärten) ist erledigt
> (`prompts/done/139-*`). **Empfohlen nach 112 bzw. 127** (Geodaten-Karten:
> Häuser in Zeilen entlang der Blockkanten, Stand-1944-Filter), **115** (lange
> Brücken über große Flüsse), **151** (Schleuse, Wehr, Kai), **160** (Flussprofil
> mit Stufe am Wehr, Erkerhaus, Stadttor, Wehrturm), **141** (Bahnanlagen) und
> **122** (farbiger Look je Karte); was davon erledigt ist, wird verwendet, was
> fehlt, baut dieser Prompt in der einfachsten Form selbst oder ersetzt es durch
> einen Stellvertreter. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/081-karte-hafenstadt.md` ganz und die Entscheidungen der
> erledigten Geodaten-Prompts (112, 114, 116, 127, 158, 161, falls in
> `prompts/done/`); in `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und
> `harbor_city` ganz, „Bridges“, „Objects“; in `docs/ARCHITECTURE.md`
> Wegfindung, Sicht, Infanterie in Gebäuden, Brand, „Destruction“;
> `docs/SOURCES.md` (Karten aus Geodaten), `docs/ASSETS.md` (Gebäude,
> Baukasten, Budgets), `docs/PERFORMANCE.md` („Harbor city“), `docs/AI.md`,
> `tools/geo_import.py` ganz, `assets-src/geo/harbor_city/site.json`,
> `data/destruction.toml`, `data/scale.toml` (`[range]`).
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/fachwerkstadt/fachwerkstadt.webp`. Ein mit KI erzeugtes Bild:
> Es trifft die Wirkung (runde Altstadt im Ring der Wallstraßen, Fluss mit
> Insel, Bogenbrücke, Kirche am Markt), ist aber nicht lagegetreu und zeigt
> Bauten von heute (Flachdachblöcke und Parkplätze im Nordosten). **Maßgeblich
> sind die Geodaten und die Quellen zum Stand 1944.**
>
> **Wunsch des Nutzers:** „Eine detailreiche, leicht verspielte
> Modelleisenbahnlandschaft mit der historischen Hamelner Altstadt, ihren
> Fachwerkhäusern, dem Münster, der Weser, der Münsterbrücke und den
> charakteristischen Flussschleifen. Dazu kommen grüne Uferpromenaden, kleine
> Inseln, Schiffe, Parks und verwinkelte Gassen – alles in einer warmen,
> idyllischen Diorama-Optik.“
>
> **Dieser Prompt läuft in vier Stufen.** Nach jeder Stufe den Zwischenstand
> zeigen (Stufenbilder, Bilder aus dem Spiel neben der Vorlage) und ohne Halt
> weiterarbeiten.

## 1. Ziel

Eine kleine, dichte Karte aus echten Geodaten nach dem Vorbild der **Altstadt
von Hameln an der Weser**, Stand um 1944: die fast kreisrunde Altstadt im Ring
der baumbestandenen Wallstraßen, der Fluss mit Insel, Wehren und Schleuse, die
Brücken, das Münster am Ufer, der Marktplatz mit Kirche und Prunkhäusern, die
Vorstädte und der Berg am anderen Ufer. Sie heißt wie die anderen
Geodaten-Karten neutral **`timber_town`** (im Spiel „Fachwerkstadt“ / „Timber
town“). Der echte Ort steht nur in `docs/SOURCES.md`, `docs/MAPS.md` und
`assets-src/geo/timber_town/`; keine echten Orts- und Straßennamen, Wappen oder
Sagenfiguren in Karte, Markern, Menüs. (Will der Nutzer den echten Namen im
Spiel, ist das eine Zeile; hier vorläufig neutral, wie in Prompt 127
entschieden.)

**Anders als bei den übrigen Geodaten-Karten wurde hier gekämpft:** Die Stadt
wurde Anfang April 1945 nach Sprengung der Weserbrücken eingenommen. Die Karte
ist trotzdem **freies Gefecht**, keine Nachstellung; eine Mission dazu wäre ein
eigener Prompt.

Für **vier Spieler, 2 gegen 2** (auch 1 gegen 1).

## 2. Ausschnitt und Geodaten (Stufe 1)

Richtwerte; die Session darf sie nach den Stufenbildern anpassen.

- **Rahmen:** etwa **2,4 km × 2,0 km**, die Altstadt (rund 700 m Durchmesser)
  etwas östlich der Mitte, der Fluss durch die Westhälfte, am Westrand der Fuß
  des Bergs mit dem Aussichtsturm, am Ostrand der Bahnhof. Begrenzung grob
  52,094–52,113° N, 9,340–9,376° O; nicht oder leicht gedreht.
- **Höhen:** Copernicus DEM wie in 081, Gebäude abgezogen. Die Stadt liegt fast
  eben in der Flussaue; nur der Berg im Westen steigt steil an (in der Karte bis
  etwa 80–100 m über dem Fluss, der Gipfel liegt außerhalb).
- **Der Fluss, so genau wie möglich:** Ufer und **Inseln** aus OSM auf den Stand
  von 1944 (historische Karten; `docs/SOURCES.md`): Breite 100–130 m, die lange
  **Flussinsel** vor der Altstadt mit ihren Bäumen und Häusern, die **Wehre**
  beiderseits der Insel und die **Schleuse** mit Schleusenkanal, die große
  **Mühle** am Altstadtufer, der kleine **Hafen** bzw. die Lände. Flussprofil
  mit Stufe an den Wehren (160), falls erledigt; sonst ein Spiegel und die Wehre
  als gesperrte Linie mit Schaumkante (Darstellung) und offener Punkt.
- **Brücken** (Stand 1944, mit Quelle): die **Straßenbrücke** von der Altstadt
  über die Insel zum Westufer (die Brücke am Münster), die **Bahnbrücke**
  flussaufwärts, Stege und kleine Brücken zur Insel und über den
  Schleusenkanal, die Übergänge über Schleuse und Wehr (schmal, mit
  Lastgrenze; aus 151). Lange Brücken aus 115; sonst verengt der Generator das
  Wasser an den Brücken wie bisher (als Entscheidung festhalten). Brücken sind
  sprengbar.
- **Bahn:** Bahnhof mit Gleisfeld am Ostrand, die Strecke zur Bahnbrücke und
  weiter nach Westen.
- **Stand 1944, weglassen:** Flachdachblöcke, Kaufhäuser, Parkhäuser und
  Parkplätze, Schul- und Verwaltungsbauten nach 1945, die verbreiterten
  Durchgangsstraßen und Kreisverkehre, Umgehungsstraßen und ihre Brücken. Wo das
  Baujahr fehlt: Höhengrenze und Lage wie in Prompt 112; als Entscheidung
  festhalten. Die **Wallstraßen** auf dem Ring der alten Befestigung gab es:
  als Alleen von etwa 20 m Breite mit doppelter Baumreihe.

## 3. Bauten (Stufe 2)

Die Fachwerkhäuser kommen aus 139. Dazu, als Bautypen bzw. vereinfachte
Nachbildungen nach veröffentlichten Maßen (Quellen in `docs/SOURCES.md`, nichts
aus anderen Spielen), je `Intact` / `Damaged` / `Ruin` mit Vorschaubild
(`just thumbnails --only=…`), `just units-web-sync`, `just catalog-doc`:

- **Fachwerk in Fülle:** drei bis vier weitere Varianten des Fachwerkhauses
  (Giebel zur Straße, mehrfach vorkragende Geschosse, Schnitzwerk und
  Farbfassung als Textur, Utlucht bzw. Erker), damit eine Gasse aus zwanzig
  Häusern nicht geklont wirkt; ein **Ackerbürgerhaus** mit großem Tor.
- **Steinernes Giebelhaus** `env_house_gable_stone`: Prunkbau der Renaissance
  mit hohem Stufen- bzw. Volutengiebel, Zwerchhäusern und Erker, drei Geschosse
  plus hohes Dach; als Einzelhaus und als langer Festsaalbau am Markt (etwa
  45 m, mit Reihe von Zwerchgiebeln).
- **Münster** `env_minster`: romanisch-gotische Kirche mit **achteckigem
  Vierungsturm** und Westturm, etwa 60 m × 25 m, aus Teilen (Langhaus, Turm,
  Chor); steht frei auf einem Platz am Fluss.
- **Marktkirche** mit schlankem, hohem Spitzhelm (grüne Kupferhaube) aus der
  vorhandenen Kirche bzw. als Variante; der Turm fällt mit der Fallregel aus
  137, falls erledigt.
- **Wehrturm** `env_wall_tower` (runder Stadtmauerturm mit Kegeldach, um 25 m)
  und ein Stück **Stadtmauer** (aus 160, falls vorhanden).
- **Flussmühle** `env_river_mill`: großer, mehrgeschossiger Mühlenbau am Wehr
  (Backstein bzw. Putz, etwa 40 m × 18 m).
- **Am Wasser:** Schleuse und Wehr (151), Kai und Lände, Bootshaus,
  **Raddampfer bzw. Fahrgastschiff** und Kähne als Kulisse am Anleger
  (schlichte Modelle der Zeit; keine modernen Schiffe).
- **Aussichtsturm** auf dem Berg (139).

Regeleinträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster
der vorhandenen Bauten (Fachwerk brennt leicht, Steinhäuser zäher, Münster wie
eine große Kirche). **Der Golden Hash ändert sich**, wenn neue Einträge in die
Regeltabellen kommen; gewollter Regelzuwachs: Ursache prüfen, Hash neu setzen,
im Commit nennen. Vergleichslauf auf einer Karte ohne die neuen Bauten zeigt
gleiche Ergebnisse wie vorher.

## 4. Die Stadt (Stufe 3)

- **Altstadt von Hand nachgearbeitet** (wie der Kern in Prompt 118): das echte
  Gassennetz, vereinfacht auf das, was das Raster trägt:
  - zwei **Hauptstraßen** (10–12 m, einspurig für Panzer), die sich am **Markt**
    kreuzen und zu den vier alten Torstellen an der Ringallee führen;
  - der **Marktplatz** mit Brunnen, **Marktkirche**, dem langen steinernen
    **Festsaalbau** und Prunkhäusern;
  - der **Münsterplatz** am Fluss mit dem Münster und dem Brückenkopf;
  - **verwinkelte Gassen** (3–6 m, nur Infanterie) zwischen lückenlosen
    Fachwerkzeilen, Höfe und Gärten im Blockinneren, einzelne Durchgänge;
  - zwei **Wehrtürme** und Mauerreste am Ring;
  - die **Flussmühle** am Wehr, die Lände mit Dampfer.
- **Ringallee** rund um die Altstadt: breite Straße mit Baumreihen, die einzige
  schnelle Umfahrung für Panzer; dahinter die **Vorstädte**: Villen und
  Mietshäuser der Gründerzeit mit Gärten (139 bzw. Blockhäuser aus 134),
  Kirche, Schule, kleine Fabriken, Bahnhofsviertel.
- **Insel im Fluss:** Park mit alten Bäumen, Wegen, zwei bis drei Häusern,
  Schleusenwärterhaus; über Brücke und Stege erreichbar.
- **Westufer:** Uferpromenade mit Baumreihe, Wiesen und ein Park, eine Vorstadt
  an der Brückenstraße, dahinter der **Berghang** mit Laubwald, Waldwegen und
  dem **Aussichtsturm** auf einer Kuppe am Kartenrand.
- **Grün:** Uferpromenaden auf beiden Seiten, Gärten, Obstwiesen und Felder am
  Rand der Aue.
- **Diorama-Anspruch:** viele kleine, richtige Dinge: Brunnen, Bänke und
  Geländer an der Promenade, Bootsstege, Wäscheleinen und Holzstapel in Höfen,
  Marktstände, Litfaßsäule, Laternen als Kulisse, blühende Bäume; warmes Licht
  und satte Farben mit dem Look aus 122, falls erledigt und vom Nutzer
  angenommen. Alles im Budget der Bildrate; gespart wird zuerst in den
  Vorstädten, nie an Altstadt und Fluss.

## 5. Spiel (Stufe 4)

- **Teams:** Der Fluss teilt die Karte in ein schmales Westufer und ein breites
  Ostufer mit der ganzen Altstadt. Vorschlag deshalb **Nord gegen Süd**, jedes
  Team mit einem Lager je Ufer: Die Front läuft quer durch die Altstadt und über
  die Insel, die Straßenbrücke verbindet Verbündete, und um Markt, Münster und
  Brückenkopf wird gekämpft. Alternative: West gegen Ost (Fluss als Front,
  Altstadt als Vorteil des Ostens, Berg als Vorteil des Westens). Die Session
  rechnet beide mit `just map-reach` und Fahrzeiten durch, wählt die
  ausgeglichenere und begründet es.
- **Starts:** vier (60 m eben und frei: Wiese in der Aue, Sportplatz,
  Bahnhofsvorplatz, Park), Zuordnung für 1 gegen 1 in `docs/MAPS.md`.
- **Versorgungspunkte** (sieben bis neun, Richtwerte): **Marktplatz**,
  **Münsterplatz mit Brückenkopf**, **Flussinsel mit Schleuse**,
  **Flussmühle**, **Bahnhof**, **Brückenkopf am Westufer**, **Aussichtsturm**,
  dazu je ein Platz an der Ringallee im Norden und Süden.
- **Was die Karte erzwingt** (mit Bild oder Test belegen):
  - **Altstadt:** Häuserkampf auf engstem Raum; Panzer nur auf den beiden
    Hauptstraßen und der Ringallee; Fachwerk brennt, ganze Zeilen können
    abbrennen (Brandregel prüfen und den schlechtesten Tick messen).
  - **Fluss:** nur über die Straßenbrücke und die Bahnbrücke für Fahrzeuge;
    Schleuse, Wehr und Stege als schmale Wege für Infanterie und leichte
    Fahrzeuge über die Insel.
  - **Berg und Kirchtürme:** Sicht über Fluss und Dächer; die Türme sind
    Landmarken, keine Stellungen.
- **Prüfen:**
  - `just map-check timber_town`, `just map-reach timber_town`: alle Starts und
    Ziele für Fahrzeuge erreichbar (Insel für leichte Fahrzeuge bzw. Infanterie,
    wie entschieden); Anteil der Fläche nur für Infanterie nennen;
  - **Brückenprobe:** jede Brücke einzeln gesprengt; mit beiden großen Brücken
    gesprengt kommt Infanterie über die Insel hinüber;
  - **Fahrzeittabelle** je Team (Summen höchstens 15 % auseinander),
    **KI-Turnier mit vertauschten Seiten**, Ziel höchstens 60 % für eine Seite;
  - **Sichtprüfung:** Sichtfeld-Bilder vom Markt, vom Münsterplatz, von der
    Brückenmitte, von der Insel, aus einer Gasse, vom Aussichtsturm-Platz;
    mittlere Sichtweite in Gasse, Hauptstraße, Ringallee, am Ufer;
  - **Reichweitentabelle** (indirekt 1/8) in `docs/MAPS.md`; auf dieser Größe
    reicht fast alles überall hin, das benennen;
  - **Vergleich mit der Wirklichkeit:** Flussufer, Insel, Ring und Lage von
    Münster, Marktkirche und Brücke über die Geodaten gelegt (Abweichung in
    Metern); Bilder aus dem Spiel aus dem Blickwinkel der Vorlage und aus
    Bodennähe;
  - **Bildrate, Tick und Wegsuche** in jeder Ansicht, über der Altstadt mit
    Brand (`docs/PERFORMANCE.md`);
  - **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Kommt die
    KI durch die Gassen, über Brücke und Insel, bleibt sie in der Altstadt
    stecken? Probleme als offene Punkte; die KI nicht umbauen.
- **Abschluss:** `assets-src/geo/timber_town/` (Rahmen, `site.json`,
  nachgezeichnete Ufer und Inseln, Quellenliste), `just gen-geo timber_town`
  erzeugt die Karte zweimal gleich; Menüname und Übersetzung, Eintrag in
  `docs/MAPS.md` mit Zahlen wie bei `harbor_city`, `docs/SOURCES.md`,
  `docs/ASSETS.md`. `just check`, `just selftest`.

## 6. Nicht Teil dieses Prompts

- Eine Mission zum April 1945 (Brückensprengung, Übergang über den Fluss); mit
  Booten (Prompt 119) ein lohnender eigener Prompt.
- Der Zustand von heute; Sagenfiguren und Festspiel als Kulisse.
- Fahrende Schiffe und Züge, Strömung als Regel.
- Die frei zusammengerückte Fassung der Vorlage als erfundene Skizzenkarte.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
