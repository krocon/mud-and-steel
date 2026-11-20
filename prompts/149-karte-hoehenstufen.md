# Prompt 149 – Karte nach Bildvorlage: Höhenstufen (kleiner Alpenpass, drei Ebenen, Tunnel, vier Spieler)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 148
> (Tunnel: `[[tunnel]]`, Regeln, Portale) ist erledigt (`prompts/done/148-*`).
> Empfohlen nach 132 oder 133 (Auffahrten entlang einer Weglinie, Fahrprobe),
> 129 (gestufte Höhen) und 139 (Burg- und Ruinenteile). Lies zuerst `CLAUDE.md`,
> dann `prompts/done/148-*` und `prompts/done/088-karten-nach-bildvorlagen.md`
> ganz, in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz
> (besonders `[[hill]]`, `[[mesa]]` falls vorhanden, `[[river]]`, `[[crossing]]`,
> `[[road]]`, `[[tunnel]]`, `[[village]]`, `[[wood]]`), „Bridges“, die
> Winterkarten `winter_town` und `winter_village` (Biom, Schnee als Gelände) und
> `mountain_pass` (Relief, Kamera und Sicht im Gebirge); in
> `docs/ARCHITECTURE.md` Sicht („Seeing downhill“), indirektes Feuer, Wegfindung
> an Hängen; `data/terrain.toml` (`max_grade_pct` 60 %, Steigungsfaktoren,
> Schnee), `data/vision.toml`, `data/scale.toml` (`[range]`),
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`.
>
> **Vorlage** (nicht im Repo, nur lesen):
> `docs/.vorlagen/hoehenstufen/hoehenstufen.webp`. **Der Nutzer legt das Bild
> dort ab**; fehlt es, nach der Beschreibung in Abschnitt 1 bis 3 bauen und das
> im Ergebnis sagen. Ein mit KI erzeugtes Bild „Alpenpass – Höhenstufen, 4
> Spieler“ mit Legende der drei Ebenen. Der Maßstabsbalken ist unstimmig und
> wird ignoriert.
>
> **Beschreibung des Nutzers** (gilt als Vorgabe): deutlich kompakter, **drei
> klar erkennbare Höhenebenen**. Ebene 1 – Tal: Hauptkampfzone mit Dorf,
> Versorgung und breiteren Panzerstraßen. Ebene 2 – Hochplateaus: zwei große
> seitliche Plateaus mit Ruinen und mehreren Zufahrten. Ebene 3 – Gipfel:
> zentrale Höhenstellung mit Festung und Artillerieposition. Vier Startbasen,
> jeweils in einer Ecke; Tunnel durch Felsmassive; mehrere Brücken und
> Wasserfälle; Serpentinen als Panzerzufahrten; alternative Fußwege für
> Infanterie; Höhenvorteile für Artillerie und Aufklärung. Etwa 1 × 1 km, für
> schnellere 4-Spieler-Gefechte.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine kleine Winterkarte `height_steps` (im Spiel „Höhenstufen“ / „Height
steps“) für **vier Spieler, 2 gegen 2** (Team West gegen Team Ost; auch 1 gegen
1): ein verschneiter Passkessel mit drei Ebenen, zwei Tunneln und einem
Bergbach mit Wasserfällen. Kurze Wege, schnelle Gefechte; wer höher steht,
sieht mehr, kommt aber nur über Serpentinen oder durch den Berg dorthin.

Die Vorlage zeigt:

- einen **Gipfel mit Festung** oben in der Mitte, von einer Straße in
  Serpentinen umrundet;
- zwei **Hochplateaus mit Ruinen** links und rechts, mit Felswänden zum Tal;
- im **Tal** in der Mitte ein **Dorf mit Kirche**, darunter auf einer niedrigen
  Kuppe eine **Mine mit Förderturm** und Schuppen;
- einen **Bergbach** mit mehreren **Wasserfällen**, zwei **Bogenbrücken** und
  einer kleinen Brücke;
- **Tunnelportale** an beiden Plateaus (je eines talseitig, je eines zur
  Gipfelstraße hin);
- **vier Lager** auf Felskanzeln in den Ecken, ein Wachturm aus Holz;
- Felsmassive und Nadelwald im Schnee ringsum.

## 2. Größe, Symmetrie, Höhen

- **1,2 km × 1,0 km**, Biom Winter (wie `winter_town`), Wetter klar. Der Nutzer
  nennt etwa 1 × 1 km; die Vorlage ist breiter als hoch. Passen Serpentinen und
  Tunnel nicht in das Maß, bis 1,5 km × 1,0 km gehen und festhalten.
- **Spiegelsymmetrisch** an der Nord-Süd-Achse (West gegen Ost), (x, y) →
  (−x, y): Gipfel, Dorf, Mine und Bach liegen auf der Achse, die Plateaus und
  Lager links und rechts. Die Vorlage ist fast so gebaut (dort liegt der Bach
  rechts der Mitte; er wird auf die Achse gelegt).
- **Drei Ebenen**, klar gestuft und im Spiel auf einen Blick zu erkennen
  (Felswand zwischen den Ebenen, oben jeweils eben):

| Ebene | Höhe über dem Dorf | Was |
|---|---|---|
| 1 – Tal | 0 m (Bachbett bis −8 m) | Dorf, Mine, Brücken, breite Straßen (8–10 m, zwei Panzer nebeneinander) |
| 2 – Hochplateaus | +22 bis +26 m | je ein Plateau im Westen und Osten, etwa 300 m × 220 m, Ruinen |
| 3 – Gipfel | +45 bis +50 m | Kuppe etwa 200 m × 140 m mit Festung und Geschützstellung |

  Die Lager liegen auf Kanzeln bei etwa +12 m. Die Höhen sind so gewählt, dass
  Serpentinen mit reichlich Abstand zur Grenze von `max_grade_pct` auf die
  Fläche passen; die Session rechnet das nach (Länge der Auffahrt bei der
  gewählten Steigung) und passt Höhen oder Kartengröße an, nicht die Regel.
- **Felsmassive** zwischen und um die Ebenen sind unpassierbar; sie dürfen hoch
  sein (bis 80–100 m), soweit Kamera und Schatten das tragen (bei
  `mountain_pass` nachsehen und messen).

## 3. Aufbau

Richtwerte in Metern für 1,2 km × 1,0 km (x nach Osten, y nach Süden, Mitte
0/0). Beschrieben ist die **Westhälfte** und die Achse; die Osthälfte ist das
Spiegelbild. Die Session übernimmt die Werte und glättet sie nach den
Stufenbildern.

- **Gipfel** auf der Achse um (0, −360): Kuppe mit **Festung** (Ringmauer, Turm,
  zwei Bauten; Burgteile aus 139 bzw. 141, sonst vorhandene Steinhäuser und
  Mauern) und einer **Geschützstellung** davor auf einer ebenen Terrasse mit
  Blick ins Tal. **Zufahrten:** je eine **Serpentine für Panzer** von Westen
  (Fuß bei (−260, −330)) und gespiegelt von Osten; ein **Steig für Infanterie**
  aus dem Dorf die Südwand hinauf (Treppenweg, für Fahrzeuge zu schmal und zu
  steil).
- **Westplateau** um (−330, −120), etwa 300 m × 220 m, Felswand nach Osten und
  Süden zum Tal. Darauf **Ruinen** (Mauerreste, zwei Hüllen), Schnee, wenige
  Bäume. **Zufahrten:** eine **Serpentine vom Tal** (Fuß bei (−170, 40)), eine
  **Auffahrt vom Lager Nordwest** über den Nordrand, eine **Verbindung zur
  Gipfelstraße** im Norden; dazu ein **Fußweg** vom Dorf die Ostwand hinauf.
  Also drei Fahrwege und ein Fußweg.
- **Tunnel West** (`[[tunnel]]`, Straße, einspurig) **unter dem Westplateau**:
  vom talseitigen Portal Südwest bei (−500, 20) zum Portal Nord bei
  (−210, −270) an der Gipfelstraße, etwa 410 m. Er verbindet das Lager Südwest
  und das Tal auf kurzem, gedecktem Weg mit der Gipfelstraße, am Plateau vorbei.
  Über dem Tunnel liegt Fels bzw. das Plateau: prüfen, dass 148 das zulässt
  (eine Ebene je Ort). Trägt 148 kein begehbares Gelände über dem Tunnel, den
  Tunnel so führen, dass er nur unter der gesperrten Felsflanke des Plateaus
  läuft (Plateau entsprechend schmaler), und das festhalten.
- **Tal:** **Dorf** auf der Achse um (0, 0): Kirche, zehn bis zwölf Häuser
  beiderseits des Bachs, ein Platz. **Mine** auf einer niedrigen Kuppe (+6 m) um
  (0, 250): Förderturm (Modell aus 146, falls vorhanden; sonst ein einfacher
  Holzturm aus dem Baukasten bzw. Stellvertreter), Schuppen, Halde. Breite
  **Talstraße** von Lager Südwest über die Dorfbrücke zu Lager Südost, eine
  zweite Querstraße über die Mine.
- **Bergbach** auf der Achse, 12–18 m breit, schnell, in einem Bett mit
  Felsufern: entspringt in einem **Wasserfall** am Gipfelfuß bei (0, −230),
  fällt in zwei weiteren Stufen (bei (0, −110) und (0, 90)) durch das Dorf,
  teilt sich vor der Mine und läuft in zwei Armen mit je einem Wasserfall nach
  Südwesten (Rand bei (−260, 500)) und Südosten. **Für Fahrzeuge nur über
  Brücken**, für Infanterie zusätzlich an zwei flachen Stellen (Trittsteine
  oberhalb des Dorfs und unterhalb der Mine). Wasserfälle als Geländestufe mit
  Wasserbild (aus 122 oder 146, falls erledigt; sonst steile Stromschnelle und
  offener Punkt).
- **Brücken** (vorhandene Steinbrücke, `[[crossing]]` `stone`): **obere Brücke**
  der Gipfelstraße bei (0, −200), **Dorfbrücke** bei (0, 10), je eine Brücke
  über die beiden Arme bei (−170, 330) und gespiegelt. Vier Fahrbrücken; ein
  Holzsteg für Infanterie bei (0, 150).
- **Lager** auf Felskanzeln: Nordwest (−470, −340), Südwest (−470, 300), jeweils
  mit **zwei Ausfahrten** (Nordwest: zur Gipfelstraße und hinab zum Westplateau;
  Südwest: zur Talstraße und zum Tunnelportal). Ein hölzerner Wachturm als
  Kulisse.
- **Wald und Fels:** Nadelwald im Schnee auf allen Hängen, die nicht Wand sind,
  Felsblöcke; tiefer Schnee abseits der Straßen (Gelände `snow`, langsamer).

## 4. Spiel

- **Starts** (60 m eben und frei; wenn 60 m auf der Kanzel nicht passen, das
  kleinste Maß der vorhandenen kleinen Karten nehmen und festhalten): Team West
  Nordwest (−470, −340) und Südwest (−470, 300); Team Ost gespiegelt.
  Spielerzuordnung wie bei den anderen Skizzenkarten (Team 0 West: Spieler 0,
  2; Team 1 Ost: 1, 3). 1 gegen 1 nutzt Südwest gegen Südost.
- **Versorgungspunkte** (fünf): Dorf (0, 0) und Mine (0, 250) im Tal;
  Westplateau (−330, −120) und Ostplateau (330, −120); Gipfelfestung (0, −360).
  Tal und Gipfel sind umkämpft, die Plateaus liegen näher am eigenen Team.
- **Was die Ebenen bedeuten** (jede Aussage in Stufe 3 mit Bild oder Test
  belegen; trägt eine Regel sie nicht, als offenen Punkt melden statt die Regel
  zu ändern):
  - **Tal:** die meisten Ziele, die breitesten Straßen, von überall einzusehen.
  - **Plateaus:** Flankenstellung über dem Dorf; Sicht ins Tal, Deckung in den
    Ruinen; mehrere Zufahrten, also schwer zu sperren.
  - **Gipfel:** sieht alles und ist mit Geschützen von dort aus der stärkste
    Punkt, hat aber nur zwei Serpentinen und einen Steig. Auf einer Karte dieser
    Größe reicht fast jedes Geschütz überall hin; der Vorteil der Höhe ist die
    **Sicht** (Aufklärung für indirektes Feuer), nicht die Reichweite. Das mit
    der Reichweitentabelle zeigen.
  - **Tunnel:** der schnelle, gedeckte Weg vom Tal zur Gipfelstraße; einspurig,
    zu sprengen, mit einem Geschütz am Ausgang zu sperren.
  - **Fußwege:** Infanterie erreicht Plateau und Gipfel auf kürzerem Weg als
    Fahrzeuge und an Stellen, die kein Panzer deckt.
- **Fußwege technisch:** Wege, die nur Infanterie nimmt. Zuerst prüfen, was die
  Regeln hergeben (Größenklasse `Small` kommt durch schmale Lücken zwischen
  Felsen; gibt es je Bewegungsart verschiedene Steigungsgrenzen?), und den Steig
  damit bauen (schmale Rinne zwischen Felsblöcken, Stufen). Keine neue Regel.

## 5. Stufen

**Stufe 1 – Relief und Wege:** `data/map_sketches/height_steps.toml` mit den
drei Ebenen, Felsmassiven, Bach, Brücken, Straßen, Serpentinen, Fußwegen,
Tunneln, Starts und Zielen, erzeugt mit `just gen-sketch height_steps`;
Höhenbild und ein Schnitt West–Ost durch Plateaus und Tal sowie Nord–Süd durch
Gipfel, Dorf und Mine. Gleich danach die Fahrprobe (Stufe 3, erster Punkt).

**Stufe 2 – Bauten und Bewuchs:** Festung, Ruinen, Dorf, Mine, Lager, Wald,
Wasserfälle, Schnee.

**Stufe 3 – Prüfen und Abschluss:**

- **Fahrprobe, als Test:** Der schwerste Panzer und ein Zugmittel mit gezogenem
  Geschütz fahren jede Serpentine und jede Auffahrt hinauf und hinunter und
  durch beide Tunnel (sofern das Fahrzeug durch das Tunnelprofil passt; sonst
  nimmt es den Pass, auch das als Test); mit je einem gesperrten Weg bleibt
  jedes Ziel erreichbar; mit beiden Tunneln verschüttet ebenfalls. Infanterie
  nimmt jeden Fußweg, Fahrzeuge keinen. Fahr- und Gehzeiten in `docs/MAPS.md`.
- `just map-check height_steps`, `just map-reach height_steps`.
- **Symmetrietest:** unter Spiegelung gleiche Höhen, Gelände, Weggitter,
  Objektklassen, Tunnel und Weglängen.
- **Sichtprüfung:** Sichtfeld-Bilder vom Gipfel (Geschützstellung), von beiden
  Plateaukanten, vom Dorfplatz, von einem Tunnelportal, aus einem Lager.
- **Reichweitentabelle** (indirekt 1/8) in `docs/MAPS.md`.
- **Kamera und Darstellung:** Auf engem Raum stehen hohe Felsen neben tiefem
  Tal; prüfen, dass Einheiten im Tal und hinter Felsen sichtbar und anwählbar
  bleiben (Regel für verdeckte Einheiten), in jeder Zoomstufe und Drehung;
  Bilder.
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`).
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Nimmt die KI
  Tunnel, Serpentinen und Fußwege, besetzt sie Gipfel und Plateaus, wie lange
  dauert eine Partie (Ziel: deutlich kürzer als auf den 2-km-Karten)? Probleme
  als offene Punkte; die KI nicht umbauen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md`, Test wie bei den anderen Skizzenkarten (zweimal erzeugt,
  gleiche Dateien). `just check`, `just selftest`. Der Golden Hash ändert sich
  nicht (neue Karte); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Die große Passkarte für zwölf Spieler (Prompt 150).
- Neue Regeln für Höhe (Reichweitenbonus), Lawinen, Schneesturm, Glätte.
- Seilbahn, Bergbahn, Stollen in der Mine.
- KI-Verbesserungen für Tunnel und Höhenstellungen.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
