# Prompt 158 – Karte nach echten Geodaten: Fürstenfelsen (Vorbild Monaco, Stand 1944)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 157 (Palast, Hafen mit Molen, Spielbank, Museum am Fels, Altstadthaus,
> Stützmauern, Treppen), 120 (Mittelmeer-Biom) und 148 (Tunnel) sind erledigt
> (`prompts/done/…`). **Empfohlen nach 112 bzw. 127** (Geodaten-Karten: Häuser in
> Zeilen entlang der Blockkanten, Stand-1944-Filter, nachgeschärfte Felswände),
> **144** (Fassadenfamilie „Südstadt“, Palazzina), **132/133** (Auffahrten
> entlang einer Weglinie, Fahrprobe) und **141** (Bahnanlagen, Bogenviadukt).
> Lies zuerst `CLAUDE.md`, dann `prompts/done/081-karte-hafenstadt.md` ganz und
> die Entscheidungen der erledigten Geodaten-Prompts (112, 114, 116, 127, falls
> in `prompts/done/`: Gebäude in Zeilen, Höhengrenze, zweite DEM-Quelle, Nennung
> echter Orte); in `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und
> `harbor_city` ganz (Drehung des Rahmens, Meeresboden, Strände), „Bridges“,
> „Objects“, die Hangregeln und `[[tunnel]]`; in `docs/ARCHITECTURE.md`
> Wegfindung an Hängen, Sicht („Seeing downhill“), Tunnel, Infanterie in
> Gebäuden, „Destruction“; `docs/SOURCES.md` (Karten aus Geodaten),
> `docs/PERFORMANCE.md` („Harbor city“), `docs/AI.md`, `docs/TEAMS.md`,
> `tools/geo_import.py` ganz, `assets-src/geo/harbor_city/site.json`,
> `data/terrain.toml` (`max_grade_pct`), `data/scale.toml` (`[range]`).
>
> **Vorlagen** (nicht im Repo, nur lesen), in `docs/.vorlagen/fuerstenfelsen/`:
> drei mit KI erzeugte Blätter (`ki_*.webp`) und vier Luftfotos Dritter
> (`foto_*.webp`). Die KI-Blätter sind **frei erfunden und unstimmig**
> (Maßstäbe von 1,5 km bis 8 km, Bauten von heute, Flaggen als Starts); sie
> zeigen nur, was der Nutzer sich vorstellt. Die **Fotos sind nur Referenz** und
> zeigen den Zustand von heute. **Maßgeblich sind die Geodaten und die Quellen
> zum Stand 1944.**
>
> **Wunsch des Nutzers:** „Der Hafen und der Fürstenpalast sollten besonders
> detailliert nachgebildet werden. Dazu kommen breite Hauptstraßen für Panzer,
> enge Gassen für Infanterie, Tunnel, Höhenunterschiede und mehrere strategische
> Zufahrten.“
>
> **Dieser Prompt läuft in vier Stufen.** Nach jeder Stufe den Zwischenstand
> zeigen (Stufenbilder, Bilder aus dem Spiel neben den Vorlagen) und ohne Halt
> weiterarbeiten.

## 1. Ziel

Eine Karte aus echten Geodaten nach dem Vorbild von **Monaco**, Stand um 1944,
vereinfacht wie `harbor_city`: ein Felsen mit Palast und Altstadt über dem Meer,
daneben ein Naturhafen zwischen zwei Molen, eine Hangstadt, die in Terrassen
vom Wasser zu den Höhenstraßen steigt, eine Küstenbahn mit Tunneln. **Kein
historisches Gefecht:** Der Ort war von 1942 bis September 1944 besetzt und
wurde kampflos frei. Die Karte ist eine frei erfundene Lage an einem echten Ort
und heißt wie die anderen Geodaten-Karten neutral **`princes_rock`** (im Spiel
„Fürstenfelsen“ / „Prince's rock“). Der echte Ort steht nur in
`docs/SOURCES.md`, `docs/MAPS.md` und `assets-src/geo/princes_rock/`; keine
echten Orts- und Straßennamen, Wappen oder Flaggen in Karte, Markern, Menüs.
(Will der Nutzer den echten Namen im Spiel, ist das eine Zeile; hier vorläufig
neutral, wie in Prompt 127 entschieden.)

Für **sechs Spieler, 3 gegen 3** (Team West gegen Team Ost; auch 2 gegen 2 und
1 gegen 1).

## 2. Ausschnitt und Geodaten (Stufe 1)

Richtwerte; die Session darf sie nach den Stufenbildern anpassen.

- **Rahmen:** etwa **3,6 km × 2,4 km**, so **gedreht**, dass die Küste entlang
  der langen Kante läuft (`x_axis_bearing_deg` um 50–60°, wie `harbor_city` mit
  37°): das Meer an der einen Langseite (etwa ein Drittel der Fläche), die
  Höhenstraße an der anderen. In der Länge vom Kap südwestlich des Felsens bis
  über die Strandbucht im Nordosten hinaus; der **Felsen und der Hafen in der
  Mitte**. Begrenzung grob 43,722–43,752° N, 7,405–7,445° O; nach Drehung und
  Größe nachziehen.
- **Höhen:** Copernicus DEM wie in 081, Gebäude abgezogen. Das Gelände steigt
  hier auf einem Kilometer vom Meer auf mehrere hundert Meter: Den Rahmen so
  legen, dass die Karte hangaufwärts an einer **Höhenstraße bei etwa 150–200 m**
  endet; was höher liegt, bleibt draußen. **Größen sind 1:1**, die Höhen werden
  nicht gestaucht. Trägt das 30-m-Modell die Terrassen der Hangstadt nicht,
  eine zweite Quelle prüfen (offene Geländemodelle des französischen
  Vermessungsamts für das Umland; Lizenz prüfen und in `docs/SOURCES.md`
  festhalten) oder die Straßenterrassen aus den Straßenlinien einarbeiten.
- **Felswände nachschärfen:** der Felsen (rund 60 m, nach drei Seiten steil ins
  Meer bzw. zum Hafen), die Klippen an der Küste und die Felsbänder am Hang aus
  OSM (`natural=cliff`) als steile, unpassierbare Stufen, Höhe nach Quelle (wie
  in 127).
- **Meer und Küste:** Meer als `sea`, Meeresboden und Strände wie bei
  `harbor_city`. **Küstenlinie von 1944**, nicht von heute: Das Viertel
  südwestlich des Felsens und Teile der Ostküste sind erst nach 1945 ins Meer
  gebaut worden. Die alte Küste aus historischen Karten und Seekarten
  (gemeinfrei bzw. mit Lizenz, in `docs/SOURCES.md`) nachzeichnen: unter der
  Westseite des Felsens liegt offenes Wasser mit einer kleinen Bucht und einem
  schmalen Gewerbestreifen; der Strand im Nordosten ist ein schmaler
  Naturstrand; der Hafen hat zwei Molen und keine vorgebaute Seemole.
- **Stand 1944, weglassen:** alle Hochhäuser und Wohntürme, die aufgeschütteten
  Viertel samt Jachthafen und heutigem Stadion, Kongress- und Hotelbauten am
  Meer, Hubschrauberplatz, Schwimmbäder, Straßentunnel und Kreisverkehre der
  Nachkriegszeit, die unterirdische Bahn. Wo das Baujahr fehlt: Höhengrenze
  (alles über sieben Geschossen ist verdächtig) und Lage wie in Prompt 112; als
  Entscheidung festhalten. **Dazunehmen**, was es 1944 gab und heute fehlt: die
  **Küstenbahn an der Oberfläche** mit zwei Bahnhöfen, ihren Tunneln, dem
  Viadukt über die Schlucht am Hafen; das Gewerbe (Brauerei, Mühle, Gaswerk)
  unter dem Felsen; das alte Stadion von 1939, falls die Quellen seine Lage
  belegen.
- **Straßen:** aus OSM, auf den Stand von 1944 gebracht (Nachkriegsstraßen und
  -tunnel raus), in drei Klassen (Abschnitt 4).

## 3. Felsen und Hafen von Hand (Stufe 2)

Der Importer liefert das Grobe; diese beiden Stücke werden **von Hand
nachgearbeitet**, wie der Kern in Prompt 118, mit den Bauten aus 157.

- **Der Felsen:** ein Plateau von etwa 700 m × 250 m in rund 60 m Höhe.
  - **Palast** am landseitigen Ende nach dem echten Grundriss (vereinfacht, aus
    den Teilen von 157) mit Ehrenhof, Türmen, Bastionen zur Hafenseite und zum
    Hang, dem **Vorplatz** mit Brüstung über der Klippe;
  - **Altstadt** dahinter: drei bis vier Längsgassen von 3–5 m und Quergassen,
    lückenlose Zeilen von Altstadthäusern (`env_house_rock`), ein Rathaus, die
    **Kathedrale**, zur Seeseite Gärten mit Pinien und Wegen und an der Spitze
    das **Museum am Fels**; am äußersten Ende über der Hafeneinfahrt das **alte
    Fort** (Bastion mit Geschützstellung);
  - **Zufahrten auf den Felsen** (nach den Quellen zum Stand 1944 festlegen;
    mindestens):
    1. die **Fahrstraße** vom Platz am Fuß des Felsens in einer langen Rampe
       durch ein Tor auf den Vorplatz (für alle Fahrzeuge, die durch das Tor
       passen);
    2. die **alte Rampe** (breiter gepflasterter Fußweg mit Stufen) vom selben
       Platz: Infanterie, nach Breite auch leichte Fahrzeuge; Entscheidung
       festhalten;
    3. eine **Straße bzw. Rampe von der Hafenseite** zur Altstadt, falls belegt;
    4. **Treppen** von der Hafenseite und von der Westseite (nur Infanterie).
    Reicht das für Fahrzeuge nur für einen einzigen Weg, ist das so: Der Felsen
    ist eine Festung. Dann als Eigenschaft der Karte benennen und im Turnier
    beobachten.
  - Durch die Gassen der Altstadt passt **kein Panzer**; Fahrzeuge kommen nur
    auf Vorplatz, Hauptgasse (falls breit genug) und Randweg.
- **Der Hafen:** das Becken zwischen Felsen und Hangstadt, etwa 500 m × 400 m.
  - **Kai** ringsum (157) mit der **breiten Hafenstraße**, Baumreihe, Promenade;
  - **zwei Molen** mit Molenfeuern, die die Einfahrt schließen; befahrbar bzw.
    begehbar nach 157;
  - **Stege**, Slip, Bootsschuppen, Hafenamt, Fischhalle, Lagerhäuser,
    aufgebockte Boote; Fischerboote, Barkassen, ein Küstenfrachter und wenige
    Jachten der Zeit im Becken;
  - die **Küstenbahn** läuft am Hafen entlang (Bahnhof mit zwei bis drei
    Gleisen am landseitigen Hafenende), quert die **Schlucht** an der
    Nordostecke des Hafens auf einem **Viadukt** (141, sonst Damm mit Durchlass)
    neben einer kleinen Kapelle und verschwindet in einem **Tunnel** unter dem
    Spielbankviertel;
  - das **Hafenviertel** dahinter als Raster mit Markt (Blöcke der Familie
    „Südstadt“, drei bis fünf Geschosse).
- Bilder aus dem Spiel neben den Fotos, aus ähnlichem Blickwinkel.

## 4. Stadt, Straßen, Tunnel (Stufe 3)

- **Drei Straßenklassen**, aus den Geodaten abgeleitet und von Hand
  nachgezogen:
  - **Hauptstraßen** (im Spiel 14–20 m breit, zwei Panzer nebeneinander): die
    **Küstenstraße** über die ganze Länge (Kap – Fuß des Felsens – Hafen –
    Spielbankviertel – Strandbucht), die **Mittelstraße** am Hang und die
    **Höhenstraße** am oberen Kartenrand, dazu drei bis vier **Querverbindungen**
    in Serpentinen zwischen ihnen (Steigung und Breite gegen `max_grade_pct`
    prüfen; wo eine echte Straße zu steil oder zu eng für die Regeln ist, die
    Kehre aufweiten oder die Straße als Fußweg führen und festhalten);
  - **Nebenstraßen** (6–10 m, einspurig für Panzer);
  - **Gassen und Treppen** (unter 5 m bzw. `env_stairs`): nur Infanterie. Die
    Hangstadt ist voll davon: Zu fast jeder Serpentine gibt es eine Treppe, die
    sie abkürzt.
- **Tunnel** (`[[tunnel]]` aus 148, Stand 1944, mit Quelle):
  - die **Bahntunnel** der Küstenbahn (unter dem Spielbankviertel, am Kap, am
    Ostende), als Bahntunnel für Infanterie und für Fahrzeuge, die durch das
    Profil passen und auf dem Gleisbett fahren dürfen (Regel aus 148 anwenden;
    wenn Gleisbett für Fahrzeuge nicht geht, nur Infanterie);
  - **Straßentunnel** nur, wo sie für 1944 belegt sind. Findet die Session
    keinen, bleibt es bei den Bahntunneln; **keine erfundenen Tunnel und keine
    der Nachkriegszeit** (das als Entscheidung festhalten und dem Nutzer sagen,
    weil er Tunnel gewünscht hat).
  - „Eine Ebene je Ort“ aus 148: Über den Tunneln liegt Stadt. Prüfen, ob 148
    das trägt (zwei Ebenen); wenn nicht, die Tunnel auf Strecken unter
    gesperrtem Fels bzw. Stützmauern beschränken und den Rest als offenen
    Einschnitt führen.
- **Viertel:**
  - **Spielbankviertel** auf dem Vorsprung nordöstlich des Hafens: die
    **Spielbank** mit Platz, Garten und Seeterrasse über dem Bahnhof, das
    **Grandhotel**, Villen, Gärten mit Palmen;
  - **Hangstadt** darüber: Häuser in Zeilen entlang der Terrassenstraßen
    (Stützmauer talseitig, Hauszeile bergseitig), Villen mit Gärten weiter
    oben, Olivenhaine, Macchia und Fels über der Höhenstraße;
  - **Strandbucht** im Nordosten: Naturstrand, wenige Villen, Fischerhütten;
  - **Gewerbe unter dem Felsen** im Südwesten und das Kap mit Villen und
    Pinienwald dahinter.
- **Gebäude** in Zeilen entlang der Blockkanten wie in 112/127; Höhen aus OSM,
  sonst je Viertel (Altstadt 3–5 Geschosse, Hafenviertel 3–5, Spielbankviertel
  4–6, Hang 2–3). Anzahl nach Bildrate.
- **Grün:** Pinien, Zypressen, Oliven, Palmen an Promenaden und in Gärten,
  Kakteen- und Felsengarten am Westhang als Kulisse.

## 5. Spiel (Stufe 4)

- **Drei Stockwerke, drei Lager je Team:** Team West am südwestlichen Ende,
  Team Ost am nordöstlichen, je ein Lager an der **Küstenstraße**, an der
  **Mittelstraße** und an der **Höhenstraße** (60 m eben und frei: Platz,
  Garten, Terrasse). Gerade Spielernummern Team West, ungerade Team Ost;
  1 gegen 1 nutzt die Küstenlager, 2 gegen 2 Küste und Höhe. Die Session legt
  die Lager nach `just map-reach` und Weglängen fest und begründet es.
- **Versorgungspunkte** (neun, Richtwerte): **Palast** (Vorplatz) und
  **Altstadt mit Museum** auf dem Felsen; **Hafenkai** (Hafenamt) und
  **Molenkopf**; **Bahnhof** am Hafen; **Spielbank**; **Markt** im
  Hafenviertel; **Aussichtsterrasse** an der Mittelstraße über dem Hafen;
  **Höhenkreuzung** in der Mitte der Höhenstraße.
- **Was die Karte erzwingt** (mit Bild oder Test belegen):
  - **Höhe:** Wer die Höhenstraße hält, sieht Hafen und Felsen; Geschütze von
    oben erreichen die halbe Stadt. Der Weg hinunter führt über wenige
    Serpentinen, die Infanterie über Treppen schneller ist.
  - **Felsen:** die stärkste Stellung, mit Sicht über Hafen und Küstenstraße,
    aber nur über ein bis zwei Fahrwege zu erreichen und von der Höhe aus
    einzusehen.
  - **Küstenstraße:** der schnelle Weg für Panzer, unter dem Felsen und entlang
    des Hafens ohne Deckung; die Engstelle am Fuß des Felsens ist der Schlüssel.
  - **Tunnel:** gedeckter Weg unter dem Spielbankviertel hindurch, zu
    verschütten.
  - **Gassen und Treppen:** Infanterie kommt fast überall hin, Panzer an wenigen
    Stellen.
- **Ausgleich:** Die Karte ist nicht symmetrisch (Felsen und Hafen liegen
  einseitig zur Mitte verschoben, je nach Rahmen). Fahrzeiten jedes Lagers zum
  Felsen, zum Hafen und zur Höhenkreuzung als Tabelle; Rahmen, Lager und
  Versorgungspunkte so wählen, dass die Summen je Team höchstens 15 %
  auseinanderliegen. **KI-Turnier mit vertauschten Seiten**, Ziel höchstens
  60 % für eine Seite.
- **Prüfen:**
  - `just map-check princes_rock`, `just map-reach princes_rock` (mit offenen
    und verschütteten Tunneln): alle Lager und Ziele für Fahrzeuge erreichbar
    außer denen, die ausdrücklich nur Infanterie erreicht (Molenkopf, Altstadt,
    je nach Entscheidung); Anteil der Fläche nur für Infanterie nennen;
  - **Fahrprobe:** der schwerste Panzer und ein Zugmittel mit gezogenem Geschütz
    über jede Serpentine, auf den Felsen, an den Hafen; kein Fahrzeug bleibt in
    einer Kehre hängen oder fährt über Kaikante, Stützmauer oder Klippe;
  - **Sichtprüfung:** Sichtfeld-Bilder vom Vorplatz des Palasts, von der
    Museumsterrasse, vom Molenkopf, von der Aussichtsterrasse, von der
    Höhenkreuzung, aus einer Gasse;
  - **Reichweitentabelle** (indirekt 1/8) in `docs/MAPS.md`;
  - **Kamera und Darstellung** am Hang und am Felsen (verdeckte Einheiten,
    Stützmauern, 200 m Höhenunterschied im Bild) in jeder Zoomstufe und Drehung;
  - **Zerstörungsprobe:** Beschuss von Palast, Altstadt und Hafen;
  - **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`,
    Vergleich mit `harbor_city`);
  - **KI-Turnier** 3 gegen 3, 2 gegen 2, 1 gegen 1, Ergebnis in `docs/AI.md`:
    Findet die KI Serpentinen, Felszufahrt und Tunnel, nutzt sie die Höhe, bleibt
    sie in Gassen stecken? Probleme als offene Punkte; die KI nicht umbauen.
- **Abschluss:** `assets-src/geo/princes_rock/` (Rahmen, `site.json`,
  nachgezeichnete Küste, Quellenliste), `just gen-geo princes_rock` erzeugt die
  Karte zweimal gleich; Menüname und Übersetzung, Eintrag in `docs/MAPS.md` mit
  Zahlen wie bei `harbor_city` (Gebäude, Straßen, Flächen, Weggelassenes),
  `docs/SOURCES.md` (Geodaten mit Lizenz und Nennung, historische Karten, die
  Fotos als „nur Referenz“). `just check`, `just selftest`. Der Golden Hash
  ändert sich nicht (neue Karte; die Regeln kamen mit 148 und 157); wenn doch,
  Ursache finden.

## 6. Nicht Teil dieses Prompts

- Die Varianten der KI-Blätter: vier Karten von je 1,5 km oder ein Raster von
  8 km × 8 km. Hier entsteht **eine** Karte in echter Größe; Ausschnitte daraus
  (nur Felsen und Hafen als kleine Karte für vier Spieler) sind mit dem Importer
  leicht nachzuholen und ein offener Punkt.
- Der Zustand von heute, eine Rennstrecke durch die Stadt, Jachten und
  Hochhäuser.
- Landung von See, Boote im Hafen als Einheiten (Prompt 119), Kriegsschiffe,
  Küstenbatterien als Regel.
- Eine Mission; die Karte ist freies Gefecht.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
