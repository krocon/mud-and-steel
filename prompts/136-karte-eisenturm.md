# Prompt 136 – Karte nach Bildvorlage: Eisenturm (Großstadt am Fluss, 2 gegen 2)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 134 (Großstadthäuser: `[[city_block]]`, Palast, Hofbau, vorab zerstörte Häuser)
> und 135 (Gitterturm `env_tower_iron`) sind erledigt (`prompts/done/134-*`,
> `prompts/done/135-*`). Empfohlen nach 115 (lange Brücken). Lies zuerst
> `CLAUDE.md`, dann diese beiden Prompts und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[river]]`, `[[island]]`,
> `[[crossing]]`, `[[road]]`, `[[ring_road]]`, `[[square]]`, `[[wood]]`,
> `[[scatter]]`, `[[city_block]]`), „Bridges“ und die Punktsymmetrie von
> `city_large`; die Karte `river_city` als Muster für eine Stadt am Fluss
> (`data/map_sketches/river_city.toml`); in `docs/ARCHITECTURE.md` Sicht,
> Wegfindung und „Destruction“; `data/scale.toml` (`[range]`),
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/eisenturm/eisenturm.webp`. Genau ansehen. Ein mit KI erzeugtes
> Stadtbild, frei nach der Gegend um den Pariser Turm, aber nicht lagegetreu (zwei
> Inseln, andere Brücken). Der Maßstabsbalken ist unstimmig und wird ignoriert.
> Die Karte ist eine **frei erfundene Stadt** mit neutralem Namen; keine
> Ortsnamen in Karte, Markern, Menüs.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Stadtkarte `iron_tower` (im Spiel „Eisenturm“ / „Iron tower“) für **2 gegen
2** (Team West gegen Team Ost, auch 1 gegen 1): eine dichte Großstadt aus
Blockrandhäusern beiderseits eines breiten Flusses mit zwei Inseln und fünf
Brücken. Auf dem einen Ufer ein Palast mit gebogenen Flügeln über einem Garten,
auf dem anderen der Eisenturm in einem großen Park; beide liegen auf einer Achse
über die mittlere Brücke. Häuser, Palast und Turm sind zerstörbar.

Die Vorlage zeigt:

- einen **breiten Fluss** in weitem Bogen mit Kaimauern, Uferstraßen,
  Baumreihen und vertäuten Kähnen, zwei **bewaldete Inseln** mit Wegen;
- **fünf Steinbrücken**, die mittlere auf der Achse Palast–Turm, dazu ein Steg
  auf eine Insel;
- den **Palast**: zwei Viertelkreis-Flügel mit Kopfbauten um eine Terrasse,
  davor ein Garten mit langem Rasenstück, geschwungenen Wegen und dichtem
  Baumbestand;
- den **Turm** auf einem Platz mitten in einem Park mit Rasenflächen,
  Baumreihen und einem Netz geschwungener Wege;
- **Blockrandviertel** mit Innenhöfen an geraden, schrägen und ringförmigen
  Straßen, Sternplätze mit Rondell und Brunnen, baumbestandene Boulevards;
  einzelne **Blöcke in Trümmern**;
- einen **großen Hofbau mit rotem Dach** im Süden;
- vier **Lager in Parkecken** als Starts; **sechs Sterne** als Ziele (drei im
  Palastgarten, je einer auf den Inseln, einer am Turm).

Die Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe, Drehung und Symmetrie

- **3,6 km × 2,4 km** (Seitenverhältnis der Vorlage, Größe von `river_city`),
  Biom `forest`, Wetter klar.
- **Um etwa 45° gedreht übernommen:** In der Vorlage läuft der Fluss schräg, die
  Achse Palast–Brücke–Turm von Nordwest nach Südost. Brücken sind im Spiel
  achsparallel. Deshalb läuft der Fluss hier **von Nord nach Süd** durch die
  Mitte und die Achse **von West nach Ost**: Palast im Westen, Turm im Osten.
  Die Straßen dürfen weiter schräg laufen.
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y), für Fluss, Inseln,
  Brücken, Straßennetz, Blöcke, Starts und Ziele. **Ausnahme sind die
  Wahrzeichen:** Palast mit Garten im Westen steht dem Turm mit Park im Osten
  gegenüber (beides große Grünflächen gleicher Ausdehnung; der Hofbau am
  Ostende des Parks gleicht die Deckung der Palastflügel aus). Ob das im Spiel
  ausgeglichen ist, zeigt das Turnier mit vertauschten Seiten.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0). Die Session
übernimmt die Werte und glättet sie nach den Stufenbildern. Was nur für eine
Seite steht, gibt es unter (x, y) → (−x, −y) auch für die andere.

- **Fluss** als `[[river]]`, etwa 150 m breit, in flachem S-Bogen: Nordrand
  (250, −1200) → Brücke (200, −900) → Brücke (120, −500) → (60, −280) →
  Mittelbrücke (0, 0) → (−60, 280) → Brücke (−120, 500) → Brücke (−200, 900) →
  Südrand (−250, 1200). An den Inseln auf etwa 300 m verbreitert. **Kaimauern**
  auf beiden Ufern (senkrechte Kante, kein Strand; Fahrzeuge kommen nur über
  Brücken ans andere Ufer), dahinter je eine **Uferstraße** mit Baumreihe.
- **Zwei Inseln** (`[[island]]`), je etwa 380 m × 120 m, längs im Fluss: um
  (70, −270) und (−70, 270), mit Bäumen, Wegen und einer Kaimauer. Die
  Nordinsel hängt mit einem Steg (schmale Brücke, nur Infanterie; sonst kleinste
  Brücke) am Ostufer, die Südinsel am Westufer. Sie sind Sprungbretter für
  Infanterie, für Fahrzeuge nicht erreichbar.
- **Fünf Brücken** (`[[crossing]]`, alle West-Ost) an den genannten Punkten. Der
  Fluss ist breiter als die vorhandene Steinbrücke: mit Prompt 115 lange Brücken
  über die volle Breite (die Mittelbrücke als breiteste, mit Vorplätzen); sonst
  verengt der Generator das Wasser an den Brücken wie bisher (als Entscheidung
  festhalten). Brücken sind sprengbar. Sind alle fünf gesprengt, bleiben nur
  Infanterie über die Inseln und, wenn Prompt 119 erledigt ist, Boote; das ist
  gewollt und wird im Turnier beobachtet.
- **Achse West–Ost** auf y = 0: Palast – Garten – Mittelbrücke – Turm – Park –
  Hofbau.
  - **Palast** (`[[palace]]` aus 134) mit Terrasse um (−900, 0), nach Osten
    geöffnet, Flügel im Viertelkreis mit etwa 250 m Radius;
  - **Palastgarten** von x −850 bis −150, y −400 bis 400: langes Rasenstück mit
    Wasserbecken auf der Achse (x −700 … −250, 90 m breit), beiderseits dichter
    Baumbestand mit geschwungenen Wegen, zwei Rondelle;
  - **Eisenturm** `env_tower_iron` bei (520, 0) auf einem gepflasterten Platz;
  - **Turmpark** von x 150 bis 1150, y −400 bis 400: Rasenflächen auf der Achse,
    Baumreihen, geschwungene Wege, kleine Teiche, ein Pavillon;
  - **Hofbau** (aus 134, rotes Dach) am Ostende des Parks um (1300, 0), etwa
    200 m × 160 m mit Hof.
  Garten und Park sind die offenen Flächen der Karte: freies Schussfeld auf der
  Achse, Deckung unter den Bäumen.
- **Stadtviertel:** der ganze Rest aus `[[city_block]]`, etwa 110–130 Blöcke von
  80–160 m Kantenlänge, fünf bis sechs Geschosse:
  - ein **Boulevardring** um Garten bzw. Park im Abstand von etwa 150 m;
  - **Sternplätze** mit Rondell bei (−1150, −550) und (−1150, 550) (gespiegelt
    im Osten), von denen je fünf bis sechs Straßen ausgehen;
  - **Boulevards** (breit, mit Baumreihen): von jedem Sternplatz zur nächsten
    Brücke, zum Start und zum Palast bzw. Park; die Uferstraßen; zwei
    Querboulevards parallel zum Fluss bei x ≈ −1200 und x ≈ −500;
  - dazwischen Nebenstraßen in zwei gegeneinander gedrehten Rastern (wie in der
    Vorlage und in `river_city`), sodass schiefe Blöcke und spitze Ecken
    entstehen;
  - Boulevards etwa 30 m breit, Nebenstraßen 12–15 m (Panzer passen durch, zwei
    nebeneinander nur auf Boulevards).
- **Trümmer:** etwa jeder zwölfte Block ganz oder zur Hälfte als Ruine bzw.
  beschädigt (über den Schalter aus 134), punktsymmetrisch verteilt, bevorzugt
  an Boulevards und Brückenköpfen: Lücken für Infanterie, Sichtschneisen,
  Schutthaufen als Deckung.
- **Startlager** in vier baumbestandenen Parkecken (je etwa 300 m × 250 m, ohne
  Häuser).

## 4. Spiel

- **Starts** (60 m eben und frei): Team West (−1550, −900) und (−1550, 900);
  Team Ost gespiegelt (1550, 900) und (1550, −900). Spielerzuordnung wie bei den
  anderen Skizzenkarten (Team 0 West: Spieler 0, 2; Team 1 Ost: 1, 3). 1 gegen 1
  nutzt den ersten Start jedes Teams.
- **Versorgungspunkte** (elf): Mittelbrücke (0, 0), umkämpft; Nordinsel
  (70, −270) und Südinsel (−70, 270); Palastgarten (−520, 0) und Turmplatz
  (520, 0); Brückenplätze (−300, −700) und (300, 700); Sternplätze
  (−1150, −550), (−1150, 550) und gespiegelt.
- **Stadtkampf:** Sichtweiten in den Straßen kurz, auf Boulevards, Fluss, Garten
  und Park lang. Besetzte Blöcke beherrschen Kreuzungen und Brückenköpfe;
  Artillerie und Panzer schlagen Breschen. Prüfen, dass jede Brücke von
  mindestens zwei Blöcken je Ufer eingesehen wird und dass es zu jedem Ziel
  mindestens zwei Wege gibt.
- **Der Turm fällt:** Sein Trümmerband (Prompt 135) kann die Achse oder einen
  Boulevard am Park sperren. Die Wege so legen, dass danach kein Start und kein
  Ziel abgeschnitten ist (Test mit Einsturz in jede Richtung).
- **Reichweiten** (indirekt 1/8): Tabelle in `docs/MAPS.md` wie in Prompt 123:
  Was erreicht Artillerie aus den Startparks, aus Garten und Park? Ziel: aus den
  Startparks die eigenen Brückenköpfe, nicht die gegnerischen Starts.

## 5. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/iron_tower.toml` nach Abschnitt 2 bis 4,
erzeugt mit `just gen-sketch iron_tower`. Zuerst Fluss, Inseln, Brücken, Achse,
Garten, Park und Boulevards; dann die Blöcke. Bestehende Skizzenkarten müssen
Bit für Bit gleich bleiben.

**Stufe 2 – Prüfen:**

- `just map-check iron_tower`, `just map-reach iron_tower`: alle Starts und Ziele
  für Fahrzeuge erreichbar (die Inseln nur für Infanterie), keine zugebauten
  Straßen, keine Blöcke im Wasser oder auf Straßen.
- **Symmetrietest:** unter Punktspiegelung gleiche Höhen, Gelände, Wege und
  Objekte; die Wahrzeichen der Achse sind als Ausnahme benannt.
- **Sichtprüfung:** Sichtfeld-Bilder von der Mittelbrücke, vom Turmplatz, von
  der Palastterrasse, von einem Sternplatz und aus einer Nebenstraße.
- **Zerstörungsprobe:** Lauf mit schwerem Beschuss eines Viertels und des Turms;
  Wege und Sicht stimmen danach, die Wegsuche bleibt im Zeitbudget.
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`), auch über
  dem dichtesten Viertel und mit Blick auf den Turm. Reicht das Budget nicht:
  Blöcke vergrößern, Randviertel durch Parks und Bahngelände ersetzen oder die
  Karte auf 3,0 km × 2,0 km verkleinern; als Entscheidung festhalten.
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, **auch mit vertauschten Seiten**,
  Ergebnis in `docs/AI.md`: Kommt die KI durch die Straßen und über die Brücken,
  besetzt sie Häuser, ist eine Seite durch Palast oder Turm im Vorteil (Ziel:
  höchstens 60 % für eine Seite)? Probleme als offene Punkte; die KI nicht
  umbauen.

**Stufe 3 – Abschluss:** Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md` mit der Reichweitentabelle, Test wie bei den
anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
`just selftest`. Der Golden Hash ändert sich nicht (neue Karte; die Regeln
kamen mit 134 und 135); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Kähne und Hausboote am Kai, Laternen, Brunnenfiguren, Straßenbahn (Kulisse,
  offene Punkte).
- Weitere Wahrzeichen und eine lagegetreue Karte nach Geodaten (dafür gäbe es
  `tools/geo_import.py`, Muster Prompt 127).
- Eine Mission oder ein geschichtliches Szenario; die Karte ist freies Gefecht.
- Boote (Prompt 119), Nacht, Regen.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
