# Prompt 161 – Karte nach echten Geodaten: Wasserfallstadt (Vorbild Schaffhausen mit Rheinfall, Stand 1944)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 160 (Strom mit Gefälle, großer Wasserfall, Rundfestung, Erkerhäuser,
> Stadttore, Schloss über dem Fall, Weinberg) und 115 (lange Brücken über große
> Flüsse) sind erledigt (`prompts/done/…`). **Empfohlen nach 112 bzw. 127**
> (Geodaten-Karten: Häuser in Zeilen, Stand-1944-Filter, nachgeschärfte
> Felswände), **139** (Fachwerk, Villenviertel mit Gärten), **141**
> (Bahnanlagen, Viadukt), **155** (Fabriken, Güterzüge) und **122** (farbiger
> Look je Karte). Lies zuerst `CLAUDE.md`, dann
> `prompts/done/081-karte-hafenstadt.md` ganz und die Entscheidungen der
> erledigten Geodaten-Prompts (112, 114, 116, 127, 158, falls in
> `prompts/done/`); in `docs/MAPS.md` „Maps from real geodata (prompt 081)“ und
> `harbor_city` ganz, „Bridges“, „Objects“, das Flussprofil aus 160; in
> `docs/ARCHITECTURE.md` Wegfindung, Sicht, „Destruction“; `docs/SOURCES.md`
> (Karten aus Geodaten), `docs/PERFORMANCE.md` („Harbor city“), `docs/AI.md`,
> `docs/TEAMS.md`, `tools/geo_import.py` ganz,
> `assets-src/geo/harbor_city/site.json`, `data/scale.toml` (`[range]`).
>
> **Vorlage** (nicht im Repo, nur lesen):
> `docs/.vorlagen/wasserfallstadt/wasserfallstadt.webp`. Ein mit KI erzeugtes
> Bild: Es zeigt die Wirkung, die der Nutzer will, ist aber **nicht
> lagegetreu** (es legt den Wasserfall neben die Altstadt; in Wirklichkeit
> liegen gut zwei Kilometer Flusslauf dazwischen) und zeigt Dinge von heute
> (Autobahn mit Anschluss, Parkplätze, neue Hallen). **Maßgeblich sind die
> Geodaten und die Quellen zum Stand 1944.**
>
> **Wunsch des Nutzers:** „Ich möchte, dass der Fluss und alles andere sehr gut
> nachgebildet wird. Eine liebevoll gestaltete, detailreiche Landschaft mit dem
> Rheinfall, der historischen Altstadt, dem Munot, Weinbergen, Wäldern und
> zahlreichen Brücken – im Stil einer hochwertigen Modelleisenbahnlandschaft.“
>
> **Dieser Prompt läuft in vier Stufen.** Nach jeder Stufe den Zwischenstand
> zeigen (Stufenbilder, Bilder aus dem Spiel neben der Vorlage) und ohne Halt
> weiterarbeiten.

## 1. Ziel

Eine Karte aus echten Geodaten nach dem Vorbild von **Schaffhausen und dem
Rheinfall**, Stand um 1944, sorgfältiger ausgearbeitet als `harbor_city`: der
Strom mit seinem echten Lauf vom Stadtufer über Wehr und Schnellen bis zum Fall
und in das Becken, die Altstadt unter der Rundfestung auf dem Rebhügel, die
Industrie am Fall, Dörfer, Weinberge und Wälder am anderen Ufer, die Brücken.
**Kein historisches Gefecht:** Der Ort liegt in einem neutralen Land und wurde
nicht umkämpft (er wurde 1944 irrtümlich bombardiert; das ist nicht Thema der
Karte). Die Karte ist eine frei erfundene Lage an einem echten Ort und heißt wie
die anderen Geodaten-Karten neutral **`falls_town`** (im Spiel „Wasserfallstadt“
/ „Falls town“). Der echte Ort steht nur in `docs/SOURCES.md`, `docs/MAPS.md`
und `assets-src/geo/falls_town/`; keine echten Orts- und Straßennamen, Wappen
oder Fahnen in Karte, Markern, Menüs. (Will der Nutzer den echten Namen im
Spiel, ist das eine Zeile; hier vorläufig neutral, wie in Prompt 127
entschieden.)

Für **sechs Spieler, 3 gegen 3** (auch 2 gegen 2 und 1 gegen 1).

## 2. Ausschnitt und Geodaten (Stufe 1)

Richtwerte; die Session darf sie nach den Stufenbildern anpassen.

- **Rahmen:** etwa **4,0 km × 3,0 km** (heutige Höchstbreite; hat Prompt 143
  größere Karten gebracht, darf der Rahmen bis 4,4 km × 3,2 km wachsen, damit
  Altstadt und Fall nicht am Rand kleben). Er muss **beides** fassen: im
  Nordosten Rundfestung, Altstadt und Stadtbrücke, im Südwesten Fall, Becken
  und Schloss; dazwischen den ganzen Flusslauf. Begrenzung grob 47,672–47,702° N,
  8,600–8,648° O; leicht gedreht, falls das die Diagonale besser nutzt
  (`x_axis_bearing_deg`).
- **Höhen:** Copernicus DEM wie in 081, Gebäude abgezogen; für Fallstufe,
  Schlucht und Uferterrassen ist es zu grob. Eine zweite Quelle prüfen (das
  amtliche Schweizer Höhenmodell ist offen verfügbar; Lizenz und Nennung prüfen
  und in `docs/SOURCES.md` festhalten). Felswände am Fall, am Schlossfelsen und
  im Seitental aus OSM (`natural=cliff`) nachschärfen wie in 127.
- **Der Strom, so genau wie möglich:**
  - **Uferlinien** aus OSM auf den Stand von 1944 gebracht (Uferverbauungen und
    das Stauwehr von heute raus; das **alte Wehr mit Kanal und Wasserwerk** am
    Stadtufer rein, nach historischen Karten);
  - **Breite** 100–160 m, an der Enge oberhalb des Falls schmaler, das
    **Becken** unter dem Fall in echter Form und Größe;
  - **Höhenprofil** (Flussprofil aus 160) nach Quelle: ruhiger Lauf am
    Stadtufer, Stufe am Wehr, **Stromschnellen** über die Felsbänke ab der
    Biegung, der **Fall** (gut 20 m), das Becken, der engere Lauf danach;
  - **Felsen im Fall**, Kiesbänke, Inseln, die Mündungen der Seitenbäche;
  - Fließrichtung für die Darstellung entlang des Laufs.
- **Brücken** (Stand 1944, mit Quelle; lange Brücken aus 115, jede in ihrer
  Bauart, soweit Modelle da sind): die **Stadtbrücke** zum Vorort am anderen
  Ufer, die **Bahnbrücke** der Linie flussaufwärts, die **Straßenbrücke** auf
  halbem Weg zum Fall, die **Bahnbrücke unmittelbar über dem Fall** (Steinbögen,
  mit Fußsteg), dazu Stege und Brücken über Kanal und Seitenbäche. Was 1944
  nicht stand, bleibt weg; was heute fehlt, kommt dazu.
- **Bahn:** der große **Bahnhof** am Altstadtrand mit Gleisfeld, die Linien
  flussabwärts auf beiden Ufern, der Einschnitt bzw. Tunnel unter dem
  Schlossfelsen am Fall (als `[[tunnel]]`, falls 148 erledigt ist; sonst als
  offener Einschnitt bzw. Lücke und offener Punkt).
- **Stand 1944, weglassen:** Autobahn und Anschlüsse, Umfahrungen, Parkplätze
  und Parkhäuser, Besucherbauten am Fall, große Hallen, Wohnblöcke und
  Einkaufsbauten nach 1945, das neue Flusskraftwerk. Wo das Baujahr fehlt:
  Höhengrenze und Lage wie in Prompt 112; als Entscheidung festhalten.

## 3. Die Schaustücke von Hand (Stufe 2)

Der Importer liefert das Grobe; diese vier Stücke werden **von Hand
nachgearbeitet** (wie der Kern in Prompt 118), mit den Bauten aus 160, und jedes
für sich neben Quelle und Vorlage gezeigt.

- **Der Fall:** Felsstufe, Felsen im Sturz, Schnellen, Becken, Gischt (160);
  am Südufer das **Schloss auf der Klippe** mit Torbau, Hof und einem Weg hinab
  zu einer Kanzel am Wasser; im Becken das **Schlösschen** auf seinem Felsen
  mit Steg; am Nordufer die **Mühlen und Fabriken**, die die Wasserkraft nutzen
  (Kanäle, Radhäuser, Hallen mit Schornsteinen; aus 155 bzw. vorhandenen
  Hallen), Uferwege, eine Bootslände.
- **Die Rundfestung** auf dem Rebhügel über der Altstadt: Ringbau mit
  Plattform und Turm, Graben, die beiden **Flankenmauern** zur Stadt hinab, der
  **Weinberg** am Hang (`[[vineyard]]`), Fußwege und eine Fahrstraße von der
  Rückseite.
- **Die Altstadt:** das echte Gassennetz, vereinfacht auf das, was das Raster
  trägt: zwei Hauptgassen mit Plätzen und Brunnen, **Erkerhäuser** in
  lückenlosen Zeilen, die **Stadtkirche** mit hohem Turm, die **Klosterkirche**
  mit Kreuzgang, zwei **Stadttore** mit Türmen und Mauerreste, das alte
  Güterhaus am Fluss, die Schifflände.
- **Das Stadtufer:** Wehr, Kanal, Wasserwerk, Stadtbrücke, Uferstraße mit
  Baumreihe, Bootshäuser.
- **Modellbahn-Anspruch:** viele kleine, richtige Dinge statt großer Flächen:
  Brunnen auf Plätzen, Gärten mit Zäunen hinter den Häusern, Obstbäume,
  Rebhäuschen, Wegkreuze, Holzstapel, Bootsstege, Baumreihen an Straßen,
  Hecken, Telegrafenleitung an der Bahn, stehende Wagen am Güterschuppen (155).
  Der satte Look aus 122, falls erledigt und vom Nutzer angenommen. Alles im
  Budget der Bildrate; wo es nicht reicht, zuerst am Rand sparen, nie an den
  vier Schaustücken.

## 4. Übrige Stadt und Land (Stufe 3)

- **Nordufer:** Vorstädte mit Villen und Gärten am Hang über der Altstadt, das
  **Industrietal** nördlich des Bahnhofs (Gießerei und Stahlwerk in einem engen
  Seitental: Hallen, Schornsteine, Gleise), der Ort am Fall mit Fabriken,
  Arbeiterhäusern und Bahnhof.
- **Südufer:** der Vorort an der Stadtbrücke, zwei **Dörfer** mit Kirche und
  Fachwerkhäusern, **Weinberge** an den Südhängen, Obstwiesen, Felder
  (Getreide, Kartoffeln, Wiese), der **Wald** auf dem Höhenrücken.
- **Wälder** auf den Höhen im Nordwesten und Süden (Mischwald, Waldwege, ein
  **Aussichtsturm** auf einer Kuppe im Nordwesten wie in der Vorlage; aus 139,
  sonst Stellvertreter).
- **Straßen** aus OSM auf den Stand von 1944: Landstraßen auf beiden Ufern von
  der Stadt zum Fall, die Gassen der Altstadt (einige nur für Infanterie),
  Feld- und Rebwege. Hauptstraßen 10–14 m, Gassen 4–8 m.
- **Gebäude** in Zeilen entlang der Blockkanten wie in 112/127; Höhen aus OSM,
  sonst je Viertel (Altstadt 3–5 Geschosse, Vorstädte 2–3, Dörfer 2). Anzahl
  nach Bildrate.

## 5. Spiel (Stufe 4)

- **Teams:** Vorschlag **West gegen Ost**, jedes Team auf beiden Ufern: Team
  West am Fall (Fabrikort am Nordufer, Schloss und Dorf am Südufer), Team Ost an
  der Altstadt (Altstadtrand bzw. Bahnhof am Nordufer, Vorort und Dorf am
  Südufer). Dann hat jede Seite ein Schaustück im Rücken (Fall bzw.
  Rundfestung mit Altstadt), die Front läuft quer zum Strom durch die Mitte
  (Industrie, mittlere Brücke, Rebhänge), und die Brücken verbinden
  Verbündete. Alternative: Nordufer gegen Südufer (Stadt gegen Land, der Strom
  als Front). Die Session rechnet beide mit `just map-reach` und Fahrzeiten
  durch, wählt die ausgeglichenere und begründet es.
- **Starts:** drei je Team (60 m eben und frei: Wiese, Platz, Sportplatz),
  mindestens einer je Ufer; gerade Spielernummern das eine Team, ungerade das
  andere; Zuordnung für 1 gegen 1 und 2 gegen 2 in `docs/MAPS.md`.
- **Versorgungspunkte** (neun bis elf, Richtwerte): **Rundfestung**,
  **Marktplatz der Altstadt**, **Bahnhof**, **Stadtbrücke**; **mittlere
  Brücke**, **Industrietal**; **Fabriken am Fall**, **Schloss über dem Fall**,
  **Bahnbrücke über dem Fall**; **Dorf am Südufer**, **Aussichtsturm**.
- **Was die Karte erzwingt** (mit Bild oder Test belegen):
  - Der Strom ist auf ganzer Länge nur über die **Brücken** zu queren; am Fall
    und in den Schnellen gar nicht.
  - **Rundfestung** und **Schloss** sind starke Höhenstellungen mit Sicht über
    Strom und Brücken.
  - **Altstadtgassen** für Infanterie, **Uferstraßen und Landstraßen** für
    Panzer, **Weinberge** als niedrige Deckung an den Hängen, **Wald** auf den
    Höhen für die Umgehung.
- **Prüfen:**
  - `just map-check falls_town`, `just map-reach falls_town`: alle Starts und
    Ziele für Fahrzeuge erreichbar (Rundfestung und Schloss mindestens bis vor
    das Tor); niemand gelangt in Fall, Schnellen oder Becken;
  - **Brückenprobe:** jede Brücke einzeln gesprengt, jedes Lager bleibt mit
    jedem Ziel auf seinem Ufer verbunden und über eine andere Brücke mit dem
    anderen Ufer; mit allen Brücken gesprengt ist die Karte geteilt (benennen);
  - **Fahrzeittabelle** je Team und Ausgleich (Summen höchstens 15 %
    auseinander), **KI-Turnier mit vertauschten Seiten**, Ziel höchstens 60 %
    für eine Seite;
  - **Sichtprüfung:** Sichtfeld-Bilder von der Plattform der Rundfestung, vom
    Schloss, von jeder Brücke, vom Marktplatz, aus dem Weinberg, vom
    Aussichtsturm-Platz;
  - **Reichweitentabelle** (indirekt 1/8) in `docs/MAPS.md`;
  - **Vergleich mit der Wirklichkeit:** Flusslauf, Becken, Lage der
    Schaustücke und der Brücken über die Geodaten gelegt (Abweichung in Metern
    nennen); vier bis sechs Bilder aus dem Spiel aus dem Blickwinkel der
    Vorlage und aus Bodennähe;
  - **Bildrate, Tick und Wegsuche** in jeder Ansicht, besonders über Fall und
    Altstadt (`docs/PERFORMANCE.md`, Vergleich mit `harbor_city`);
  - **KI-Turnier** 3 gegen 3, 2 gegen 2, 1 gegen 1, Ergebnis in `docs/AI.md`:
    Kommt die KI über die Brücken, durch die Altstadt, auf die Rundfestung?
    Probleme als offene Punkte; die KI nicht umbauen.
- **Abschluss:** `assets-src/geo/falls_town/` (Rahmen, `site.json`,
  nachgezeichnete Ufer und Flussprofil, Quellenliste), `just gen-geo falls_town`
  erzeugt die Karte zweimal gleich; Menüname und Übersetzung, Eintrag in
  `docs/MAPS.md` mit Zahlen wie bei `harbor_city`, `docs/SOURCES.md` (Geodaten
  mit Lizenz und Nennung, historische Karten). `just check`, `just selftest`.
  Der Golden Hash ändert sich nicht (neue Karte; die Regeln kamen mit 160);
  wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Die zusammengerückte Fassung der Vorlage (Fall neben der Altstadt) als eigene,
  erfundene Karte; mit dem Skizzen-Generator leicht nachzuholen und ein offener
  Punkt.
- Der Zustand von heute: Autobahn, Parkplätze, Besucheranlagen, neue Kraftwerke.
- Boote und Felsenfahrt (Prompt 119), fahrende Züge, Strömung als Regel.
- Eine Mission; die Karte ist freies Gefecht.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
