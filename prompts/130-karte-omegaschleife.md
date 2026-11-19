# Prompt 130 – Skizzenkarte: Omegaschleife (eigener Entwurf)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` (Ziel, Entscheidungen, Ergebnis),
> in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz (besonders
> `[[river]]`, `[[crossing]]` mit `stone`, `[[lake]]`, `[[wood]]`, `[[scatter]]`,
> `[[field]]`, `[[village]]`, `[[farm]]`, `[[building]]`, `[[rail]]`) und
> „Bridges“, die Karten `river_fork` und `river_town` als Muster für Flüsse mit
> vielen Brücken (`data/map_sketches/river_town.toml`), `crates/cli/src/gen_sketch.rs`,
> `data/scale.toml` (`[range]`), `docs/AI.md`. Ist Prompt 115 (lange Brücken)
> erledigt, auch dessen Ergebnis.
>
> **Vorlage** (nicht im Repo, nur lesen):
> `docs/.vorlagen/omegaschleife/omegaschleife_entwurf.svg`, ein eigener Entwurf aus
> der Unterhaltung, vom Nutzer nach zwei Runden so angenommen. Maßstab: das
> Kartenquadrat (x 60–620, y 20–580) ist 2 km, 1 px = 3,57 m, Mitte (340, 300) =
> Kartenmitte; x nach Osten, y nach Süden. Die Osthälfte ist das Spiegelbild der
> Westhälfte.
>
> **Dieser Prompt läuft in zwei Stufen.** Nach Stufe 1 Bilder aus dem Spiel neben
> dem Entwurf zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine ländliche Gefechtskarte `omega_bend` (im Spiel „Omegaschleife“ / „Omega
bend“), 2 km × 2 km, **spiegelsymmetrisch** an der Nord-Süd-Achse (West gegen Ost),
als Skizze für `just gen-sketch`. Herkunft: Der Nutzer brachte die Minimap einer
Stadtkarte aus einem kommerziellen Spiel als Idee; die wird **nicht** nachgebaut.
Der Entwurf ist eigen und wurde mit dem Nutzer geändert: **wenige Häuser, viel
Wiese und Wald, alles geschwungen**, kein Wahrzeichen in der Mitte.

## 2. Aufbau nach dem Entwurf

Richtwerte in Metern (x Osten, y Süden, Mitte 0/0); die Osthälfte ist das
Spiegelbild (x, y) → (−x, y). Die Session übernimmt die Werte und glättet sie nach
den Stufenbildern.

- **Fluss mit Omega-Schleife:** Mittellinie vom Westrand (−1000, −179) fast gerade
  nach Osten, bei (−196, −339) in eine fast runde Schleife nach Norden (Mittelpunkt
  (0, −594), Radius etwa 320 m, oberster Punkt etwa 30 m vom Nordrand), zurück bei
  (196, −339) und gespiegelt zum Ostrand (1000, −179). Breite etwa 85 m, Uferstreifen
  beiderseits.
- **Halbinsel in der Schleife:** über einen **Hals** von knapp 400 m (zwischen den
  Flussfüßen) mit dem Süden verbunden. Darauf ein kleines Dorf mit Kirche um
  (0, −593) (vier bis fünf Häuser, Kirche bei (0, −714)), ein Wäldchen bei
  (−100, −779) und gespiegelt, Wiese. Ein Weg vom Hals (0, −307) nach Norden durch
  das Dorf, ein Querweg zu den beiden Schleifenbrücken.
- **Vier Steinbrücken** (`[[crossing]]` `stone`):
  - Nord-Süd über den Fluss bei x −711 (Fluss bei y etwa −179) und gespiegelt bei
    x 711;
  - West-Ost über die Schleifenseiten bei y −593 (Fluss bei x etwa −321) und
    gespiegelt.
  Brücken sind achsparallel. Der Fluss ist breiter als die längste Steinbrücke
  (21 m). Ist Prompt 115 erledigt: mehrbogige Steinbrücken über die volle Breite.
  Sonst verengt der Generator das Wasser an den Brücken wie bisher, und das ist so
  in Ordnung (als Entscheidung festhalten).
- **Nordviertel** (außerhalb der Schleife, nördlich des Flusses), nur über die
  Brücken erreichbar: ein **Gehöft** bei (−804, −768) mit Feld daneben
  (−893 … −571, −950 … −793), ein Wald bei (−964 … −786, −536 … −321), ein
  Wäldchen bei (−371, −893), ein einzelnes Haus bei (−625, −357) an der Brücke; ein
  Weg von der Brücke zur Schleifenbrücke und einer zum Westrand.
- **Süden (weite Wiese):**
  - **Weiler** am südlichen Brückenkopf um (−711, 18) (fünf Häuser um einen
    kleinen Platz) und gespiegelt;
  - **Weiler** am Start bei (−750, 429) (drei Häuser) und gespiegelt;
  - **Waldstreifen** entlang der Mitte bei x −536 … −393, y 107 … 536 (Deckung beim
    Vorrücken) und gespiegelt; **Wäldchen** bei (−250, 36), (−143, 579),
    (−929, 357) und ein größerer Wald im Südwesten (−571 … −357, 786 … 964);
    gespiegelt;
  - **Felder** bei (−964 … −793, −14 … 179) und (−536 … −321, 579 … 750);
    gespiegelt; eine etwas dunklere Wiesenfläche westlich der Mitte;
  - **Teich** in der Mitte bei (0, 214), etwa 230 m × 180 m;
  - **kleiner Bahnhof** bei (0, 857) mit Vorplatz, ein Gleis zum Südrand; ein
    einzelnes Haus bei (−179, 857) und gespiegelt.
- **Wege, alle geschwungen** (Feldwege `track`, Hauptwege `road`): vom Start über
  den Weiler zur Südbrücke; vom Südbrückenkopf im Bogen zum Hals; vom Hals in
  Schlangenlinie am Teich vorbei zum Bahnhof; vom Weiler im Bogen zum Teich; vom
  zweiten Start im Bogen zum Bahnhof; ein Randweg vom Westrand zum Weiler. Kein
  gerader Boulevard.

## 3. Spiel

- **Starts** (60 m eben und frei): Team West (−857, 607) und (−821, 914), Team Ost
  (857, 607) und (821, 914). 1 gegen 1 nutzt die beiden ersten.
- **Versorgungspunkte:** Teich-Nord (0, 36) umkämpft; Halbinseldorf (0, −464);
  Schleifenbrücken West (−500, −607) / Ost (500, −607); Südbrückenköpfe (−711, 0) /
  (711, 0); Bahnhof (0, 729); Weiler (−750, 536) / (750, 536).
- **Prüfen:** `just map-check omega_bend`, `just map-reach omega_bend` (alle Starts
  und Punkte für Fahrzeuge erreichbar; die Halbinsel über den Hals und über die
  Schleifenbrücken; die Nordviertel nur über Brücken), Symmetrietest (gespiegelte
  Höhen, Gelände, Objekte gleich), Sichtfeld-Bilder vom Halbinseldorf, vom Teich
  und aus den Waldstreifen, Reichweitentabelle wie in Prompt 123 (indirekt 1/8).
- **KI-Turnier**, Ergebnis in `docs/AI.md`: Nutzt die KI die Waldstreifen, kämpft
  sie um den Hals und die Brücken? Probleme als offene Punkte; die KI nicht umbauen.

## 4. Stufen

**Stufe 1 – Skizze:** `data/map_sketches/omega_bend.toml` nach Abschnitt 2 und 3,
erzeugt mit `just gen-sketch omega_bend`, Biom `forest`, Wetter klar, sanftes
Gelände (leichte Wellen, Flussufer etwas tiefer). Kann der Generator die fast
runde Schleife nicht als `[[river]]` aus Punkten legen (zu enge Kurve), die
Punktdichte erhöhen oder die Schleife als See-Polygon mit `[[bay]]`-artiger
Verbindung bauen; die einfachste Lösung wählen und festhalten. Bestehende
Skizzenkarten müssen Bit für Bit gleich bleiben.

**Stufe 2 – Prüfen und Abschluss:** Abschnitt 3, Messung von Bildrate und Wegsuche
(`docs/PERFORMANCE.md`), Menüname und Übersetzung, Eintrag in der Tabelle der
Skizzenkarten in `docs/MAPS.md`, Test wie bei den anderen Skizzenkarten (zweimal
erzeugt, gleiche Dateien). `just check`, `just selftest`. Der Golden Hash ändert
sich nicht (neue Karte); wenn doch, Ursache finden.

## 5. Nicht Teil dieses Prompts

- Ein Wahrzeichen in der Mitte (der Turm aus dem ersten Entwurf wurde verworfen).
- Winter- oder Herbstvariante (über `base` in der Skizze leicht nachzuholen).
- Boote auf dem Fluss (Prompt 119).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
