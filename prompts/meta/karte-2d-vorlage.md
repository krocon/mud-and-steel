# Meta-Prompt: 2D-Kartenvorlage für „Mud and Steel“ vorbereiten

> Diesen Text als Ganzes an eine andere KI geben (ChatGPT, Gemini, …). Unten bei
> **„Meine Idee“** die eigene Beschreibung einsetzen. Das Ergebnis (SVG und
> Beschreibung, optional ein Bild) kommt nach `docs/.vorlagen/<name>/`; daraus
> wird mit Claude eine Skizze `data/map_sketches/<name>.toml` und mit
> `just gen-sketch <name>` die Karte.

---

## Deine Rolle

Du bist Karten-Designer für ein isometrisches Echtzeit-Strategiespiel im Zweiten
Weltkrieg (Infanterie, Panzer, Artillerie; Europa und Nordafrika, 1939–1945).
Du entwirfst eine **2D-Draufsicht einer Spielkarte**, die ein Programmierer
danach von Hand in ein Kartenformat überträgt. Du zeichnest also **keinen
schönen Prospekt**, sondern einen **maßstabsgetreuen Bauplan**: Jede Linie, die
du zeichnest, muss im Spiel genau so gebaut werden können.

## Meine Idee

```
Name (Englisch, klein, mit _):   <z. B. hedge_valley>
Anzeigename Deutsch / Englisch:  <z. B. Heckental / Hedge valley>
Thema und Stimmung:              <freier Text: Landschaft, Ort, Kampfart>
Größe:                           <z. B. 1,6 × 1,6 km oder 3,0 × 2,0 km>
Spieler und Teams:               <z. B. 2 gegen 2, West gegen Ost>
Biom:                            <forest | winter | desert>
Wetter:                          <clear | rain | snow | fog>
Besonderes:                      <z. B. "viele Brücken", "Strand im Norden">
```

## Was du lieferst

1. **Eine SVG-Datei (Hauptergebnis).** `viewBox` in **Metern**, Ursprung in der
   **Kartenmitte**, **x nach Osten, y nach Süden** (Norden oben), z. B. für
   3000 × 2000 m: `viewBox="-1500 -1000 3000 2000"`. Jedes Element bekommt eine
   `id` und eine Klasse aus der Legende unten (`class="road"`, `class="wood
   conifer"` …). Flüsse und Straßen als `<polyline>` mit `stroke-width` = echte
   Breite in Metern, Flächen als `<polygon>`, Hügel als `<circle>` mit
   `data-height-m`. Keine Transformationen (`transform`), keine Gruppen mit
   Versatz, keine Kurven (`<path>` mit Béziers): nur gerade Teilstücke, Punkte
   auf ganze Meter gerundet. Ein Raster alle 100 m (dünn, eigene Ebene
   `id="grid"`) mit Beschriftung am Rand.
2. **Eine Beschreibung als Text** (Markdown) mit genau diesen Abschnitten:
   *Idee* (3–6 Sätze), *Größe, Biom, Wetter*, *Symmetrie* (welche und wie),
   *Starts* (Spieler, Team, Koordinate), *Versorgungspunkte* (Name
   `supply_<ort>`, Koordinate, warum dort), *Wege zwischen den Hälften* (jede
   Route für Fahrzeuge einzeln, mit Engstellen), *Flüsse und Übergänge*,
   *Orte* (Dorf, Stadt, Gehöft, Einzelbauten mit Art), *Höhen*, *bewusste
   Abweichungen von den Regeln* (mit Grund), *Selbstprüfung* (Checkliste unten,
   jeder Punkt mit „erfüllt“ und einer Zahl als Beleg).
3. **Optional ein gerendertes Bild** (PNG) derselben SVG, mindestens 2048 px auf
   der langen Seite. Kein KI-Bildgenerator-Bild statt der SVG: Solche Bilder
   haben falsche Maßstäbe, falsche Seitenverhältnisse und erfundene Details.

## Maßstab: so groß ist die Welt

Alle Maße sind **echte Meter, 1:1**. Zur Orientierung:

| Ding | Größe |
|---|---|
| Soldat | 0,5 m breit |
| Panzer (Panzer IV, Sherman, T-34) | 6–7 m lang, 3 m breit |
| Bauernhaus | 8 × 12 m; Scheune 10 × 20 m; Kirche 15 × 35 m |
| Landstraße | 6–8 m Fahrbahn; Feldweg 4–6 m |
| Dorf | 20–80 Häuser, 200–500 m Durchmesser |
| Kleinstadt | 500–1000 Gebäude, 600–1200 m |
| Feld | 80–250 m Seitenlänge |

**Sicht- und Schussweiten sind gestaucht (1/8 der echten):** Panzer schießen
sich auf etwa **80–150 m** an, sehen etwa **150–250 m** weit. Daraus folgt:

- Eine offene Fläche von **300 m** ist schon ein weites Panzerfeld; über
  **500 m ohne jede Deckung** sind tote Fläche, die niemand quert. Höchstens ein
  solcher Raum pro Karte, und nur mit Absicht.
- Deckung (Waldstück, Hecke, Häuser, Kuppe) soll man auf jedem Weg spätestens
  alle **100–200 m** finden.
- Hügel sind niedrig: **5–30 m** hoch, Radius **80–300 m**. Eine Kuppe von 10 m
  verdeckt schon ganze Panzergruppen.

## Größe und Format

- Seiten in **Vielfachen von 4 m**, jede Seite **höchstens 4000 m**. Bewährte
  Größen: 1600 × 1600 (2–4 Spieler), 3000 × 2000 oder 3600 × 2000 (4–6
  Spieler), 3600 × 2400 (bis 8 Spieler).
- Seitenverhältnis von SVG und Bild = Seitenverhältnis der Karte. Titelkasten
  (falls überhaupt) außerhalb der Karte; dort genannte Größe und Spielerzahl
  müssen zur Zeichnung passen.

## Befahrbarkeit: Panzer müssen durchkommen

Fahrzeuge brauchen viel mehr Platz als Infanterie. Die Wegfindung rechnet auf
einem 4-m-Raster mit 2 m Sicherheitsabstand zu jedem Hindernis.

- **Jede Lücke, durch die ein Fahrzeug soll, ist mindestens 10 m breit**
  (Haus zu Haus, Baum zu Baum, Heckenlücke, Tor). 8 m ist das absolute
  Minimum, das gerade noch geht; plane mit 10–12 m.
- **Straßen in Orten: 12–16 m von Hauswand zu Hauswand** (Fahrbahn plus Gehweg
  und 3 m Vorgarten). Hauptstraße und Plätze breiter. Keine Sackgassen für
  Fahrzeuge in Orten, außer einzelne Höfe.
- **Landstraße 8–10 m, Feldweg 6 m** in der Zeichnung (die Linienbreite ist die
  Wegbreite).
- **Wald, Hecken und Mauern sperren Panzer** (dünne Lehm- und Trockenmauern
  walzen schwere Panzer um, Hecken nicht). Wälder brauchen deshalb Schneisen
  oder Wege, wenn Fahrzeuge hindurch sollen; Hecken brauchen **Durchfahrten
  von 10–12 m**. Infanterie kommt überall durch.
- **Steigung:** Über 60 % kommt niemand hinauf; Straßen höchstens 12 %.
  Klippen und Felsen nur als bewusste Sperre.
- **Wasser:** Tiefes Wasser sperrt alles, flaches Wasser ist eine Furt. Jede
  Furt und jede Sandbank als eigene Fläche zeichnen.
- **Keine Insel ohne Zugang**, auf der ein Ziel liegt. Randinseln ohne Zugang
  sind nur Kulisse.

## Flüsse und Brücken

- Flussbreite **20–60 m**. Der Generator schneidet die Ufer 3 m tief ein, in
  der Mitte tief, am Rand flach, Sand an den Ufern.
- Brücken stehen **nur in Nord-Süd- oder West-Ost-Richtung** (keine schrägen
  Brücken). Der Fluss muss an der Brücke also ungefähr waagrecht oder
  senkrecht laufen, auf mindestens **40 m** zu beiden Seiten.
- Steinbrücke überspannt höchstens **21 m**, Holzbrücke höchstens **7 m**: Der
  Generator verengt den Fluss an der Brücke von selbst, zeichne die Brücke
  einfach als Punkt mit Art und Richtung. Vor und hinter jeder Brücke **50 m
  gerade Zufahrt**.
- Zwischen zwei Übergängen über denselben Fluss mindestens **150 m**. Ein Fluss
  quer durch die Karte braucht **mindestens drei Übergänge** (Brücken oder
  Furten), sonst entscheidet eine Brücke das Spiel.
- Fließrichtung angeben (in Grad: 0 = nach Osten, 90 = nach Süden).

## Gerechtigkeit: topologisch symmetrisch

Die Karte muss für beide Teams **gleich gut** sein. Das Spiel ist ein
Wettkampf; schon 10 % Vorteil entscheiden Partien.

- Wähle **eine** Symmetrie und halte sie überall durch:
  - **Spiegelung** an der Nord-Süd-Achse, (x, y) → (−x, y): Team West gegen
    Team Ost, Spieler stehen sich gegenüber; oder an der West-Ost-Achse;
  - **Punktsymmetrie** um die Mitte, (x, y) → (−x, −y): gut für diagonale
    Aufstellungen und für Flüsse, die quer durch die Mitte laufen.
- **Gespiegelt werden müssen:** Starts, Versorgungspunkte, Straßen und Wege,
  Flüsse, Brücken und Furten, Wald- und Heckenränder mit ihren Lücken, Hügel
  (Lage und Höhe), Gebäude-Grundrisse und -Größen, offene Felder.
- **Abweichen dürfen:** Hausvarianten, Feldfrüchte, Baumarten, Farben,
  kleine Formdetails unter 10 m. Abweichungen dürfen die Wege und Sichtlinien
  nicht ändern.
- **Topologisch heißt:** Wenn die Landschaft sich nicht exakt spiegeln lässt
  (Küste nur im Norden, Fluss in einem Bogen), müssen trotzdem gleich sein:
  - die **Zahl der Fahrzeugrouten** von jedem Start zu jedem Ziel;
  - die **Weglänge** von jedem Start zum nächsten neutralen Versorgungspunkt
    und zur Mitte (Abweichung **höchstens 5 %**);
  - die **Deckung** auf diesen Wegen und die **Höhe**: kein Team blickt von
    einer Höhe auf den Weg des anderen hinab;
  - die Zahl der **Brücken und Engstellen**, die jedes Team bis zur Mitte
    überqueren muss.
- Keine **Sonnen- oder Wetterseite** als Vorteil denken: Das spielt keine Rolle.

## Starts und Versorgungspunkte

- **Start:** ein Kreis von **60 m Radius, flach und frei** (keine Bäume, Häuser,
  Wasser). Mindestens 100 m vom Kartenrand. Gegnerische Starts mindestens
  **60 % der Kartenbreite** auseinander, Teamkameraden mindestens **300 m**.
  Jeder Start hat **mindestens zwei** Fahrzeugausfahrten in verschiedene
  Richtungen (keine Basis hinter einer einzigen Brücke).
- **Versorgungspunkte** (`supply_<ort>`, z. B. `supply_church_square`): je
  ein flacher, freier Platz von etwa 30 m. **6–9 Stück** auf 1,6 km, **8–12**
  auf größeren Karten. Verteilung:
  - je **1–2 nahe jeder Basis** (sicher, 200–400 m vom Start);
  - **2–4 umkämpft** auf der Symmetrieachse oder paarweise gespiegelt in der
    Mitte;
  - der Rest gespiegelt an den Flanken, damit sich der Kampf nicht nur auf
    eine Mittelstraße drängt.
- Jeder Versorgungspunkt ist **von jedem Start mit Fahrzeugen erreichbar**.

## Gutes Spiel: Wege, Engstellen, Flanken

- Zwischen den Hälften **mindestens drei Fahrzeugrouten**, die mindestens
  **300 m** auseinanderliegen (Mitte und zwei Flanken). Dazu Infanteriewege
  durch Wald, Hecken und Häuser, die Panzer nicht nehmen können.
- Jede Route hat ein **Gesicht**: offenes Panzerfeld, Ort für Häuserkampf,
  Wald oder Hecken für Hinterhalte. Nicht alles gleich.
- Engstellen (Brücke, Heckenlücke, Dorfstraße) sind gewollt, aber **keine
  einzelne Engstelle darf alle Fahrzeugwege einer Hälfte bündeln**.
- Ränder: kein toter Streifen am Kartenrand; Wege laufen bis an den Rand
  (Verstärkung fährt dort ein).
- Gebäude: höchstens **etwa 3000** auf der ganzen Karte (Leistung).

## Bausteine, die es gibt

Benutze **nur** diese Bausteine. Etwas anderes kann nicht gebaut werden.

| Klasse (SVG) | Was es ist | Zeichnen als |
|---|---|---|
| `start` | Startplatz, mit `data-player`, `data-team` | Kreis r = 60 |
| `supply` | Versorgungspunkt, mit `data-name` | Kreis r = 15 |
| `marker` | sonstiger benannter Punkt | Kreis r = 5 |
| `river` | Fluss, Breite = Linienbreite | Polyline |
| `lake deep` / `lake shallow` | See, tief oder flach | Polygon |
| `island` | Insel in Fluss oder See | Kreis |
| `crossing stone` / `crossing wood` / `crossing ford` | Übergang, mit `data-axis="ns"` oder `"we"` | Kreis r = 10 auf der Flussmitte |
| `road` / `track` | Landstraße / Feldweg | Polyline |
| `square` | gepflasterter Platz | Polygon |
| `ring_road` | Rondell, Kreisweg | Kreis, Linie |
| `wood mixed` / `deciduous` / `conifer` / `birch` / `palm` / `grove` / `rock` | Wald (rock = Felsen mit Nadelbäumen) | Polygon |
| `scatter` | lose Einzelbäume | Polygon |
| `field grain` / `green` / `ploughed` | Feld | Polygon |
| `patchwork` | Flickenteppich kleiner Felder, mit `data-plot-m` | Polygon |
| `hedge` / `wall` / `fence` | Hecke, Mauer, Zaun; Lücken = Unterbrechungen der Linie | Polyline |
| `town` | Straßenbebauung entlang aller Wege im Polygon, mit `data-kinds` | Polygon |
| `village` | lose Häuser, mit `data-count`, `data-damaged` (0–1) | Polygon |
| `farm` | Gehöft (Haus, Scheune, Hof) | Kreis r = 40 |
| `building` | Einzelbau: Kirche, Rathaus, Halle, Bahnhof, Bunker, Burgruine, Turm … | Rechteck in echter Größe |
| `row` / `ring` | Reihe oder Ring von Baracken und Hütten | Polyline / Ellipse |
| `hill` | Kuppe, mit `data-height-m` | Kreis |
| `rail` | Bahnlinie (nur Kulisse, 7 m Damm, befahrbar) | Polyline |
| `sea` + `land` | Meer überall außer in den Land-Polygonen | Polygon |
| `bay` | Bucht, Lagune (Meer im Land) | Polygon |
| `causeway` | Damm durchs Meer, Breite = Linienbreite | Polyline |
| `shoal` | Sandbank, Furt für Panzer durchs Meer | Polyline |
| `paint` | besondere Bodenart (Schlamm, Sand, Geröll) | Polygon |

Farben (damit die Zeichnung lesbar bleibt, keine Schattierung, keine
Verläufe): Wasser tief `#2f6fae`, flach `#7fb5e0`, Straße `#8a7f6a`,
Feldweg `#c2a878`, Wald `#2f5d2a`, Feld `#d9c46a` / `#9cc46a` / `#9a7752`,
Hecke `#1f3d1a` (2 m Strich), Gebäude `#6b4a3a`, Platz `#b0a090`, Hügel als
Höhenring `#a0703a` gestrichelt, Start `#e03030`, Versorgung `#f0b000`.

## Verboten

- Perspektive, 3D, Schatten, Rauch, Explosionen, Einheiten, Kompassrosen oder
  Beschriftungen **auf** der Karte (Beschriftung nur am Rand oder in der
  Beschreibung).
- Moderne Dinge: Autobahnen, Kreisverkehre, Hochhäuser aus Glas,
  Industriegebiete der Nachkriegszeit, Golfplätze, Windräder.
- Hakenkreuze, SS-Runen, das Rote Kreuz, echte Firmennamen und Logos.
- **Karten aus anderen Spielen** (auch nicht „angelehnt“). Echte Orte als
  Vorbild sind erlaubt, nenne sie dann in der Beschreibung.
- Unmögliche Geometrie: schräge Brücken, Brücken über mehr als 21 m Wasser
  ohne Verengung, Wege, die in Wasser oder Wald enden, Häuser auf Straßen.

## Selbstprüfung (in der Beschreibung abhaken, jeweils mit Zahl)

- [ ] Größe in Vielfachen von 4 m, höchstens 4000 m je Seite; viewBox passt.
- [ ] Symmetrie benannt; für jedes Objekt gibt es das Gegenstück
      (Ausnahmen aufgelistet).
- [ ] Weglänge jedes Starts zum nächsten neutralen Versorgungspunkt und zur
      Mitte: Zahlen je Team, Abweichung ≤ 5 %.
- [ ] Mindestens drei Fahrzeugrouten zwischen den Hälften, ≥ 300 m
      auseinander; keine Engstelle bündelt alle.
- [ ] Engste Fahrzeuglücke auf jeder Route ≥ 10 m (Stelle und Maß nennen).
- [ ] Ortsstraßen ≥ 12 m von Wand zu Wand.
- [ ] Jede Brücke N-S oder W-O, 50 m gerade Zufahrt, ≥ 150 m zur nächsten.
- [ ] Jeder Start: 60 m frei, ≥ 100 m vom Rand, ≥ 2 Ausfahrten.
- [ ] Jeder Versorgungspunkt von jedem Start mit Fahrzeugen erreichbar.
- [ ] Größte offene Fläche ohne Deckung (Maß nennen) ≤ 500 m.
- [ ] Höhen gespiegelt, kein Team hat Blick von oben auf den Anmarsch des
      anderen.
- [ ] Nur Bausteine aus der Tabelle; nichts Verbotenes.
- [ ] Gebäudezahl geschätzt (≤ 3000).
