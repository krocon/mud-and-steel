# Prompt 156 – Karte nach Bildvorlage: Fabrikstadt (Industriestadt mit Kanälen, Hafen und Rangierbahnhöfen, vier Spieler)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 155 (Industriebauten, `[[factory]]`, `[[train]]`, `[[harbor]]`), 134
> (`[[city_block]]`, vorab zerstörte Häuser) und 137 (Backstein-Fassaden,
> Bahnhofshalle, Schornstein, Fallregel) sind erledigt (`prompts/done/…`).
> Empfohlen nach 151 (`[[canal]]`), 141 (`[[rail_yard]]`) und 115 (lange
> Brücken). Lies zuerst `CLAUDE.md`, dann diese Prompts und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz, „Bridges“ und die Karte `river_city` als
> Muster einer Stadt; in `docs/ARCHITECTURE.md` Sicht, Wegfindung, „Destruction“,
> Brand und explodierende Lager; `data/scale.toml` (`[range]`),
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`. Sind 136
> oder 138 (Stadtkarten) erledigt, auch deren Ergebnis: dort gefundene Lösungen
> (Blockdichte, Straßenbreiten, Bildrate) übernehmen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/fabrikstadt/fabrikstadt.webp`. Genau ansehen. Ein mit KI
> erzeugtes, frei erfundenes Bild einer Industriestadt mit vier Lagern in den
> Ecken und vier Sternen.
>
> **Beschreibung des Nutzers** (gilt als Vorgabe): eine dicht bebaute
> Industriestadt mit einem verzweigten Kanalsystem, großen Fabrikkomplexen und
> ausgedehnten Bahnanlagen. Große Fabrikhallen und Schornsteine als markante
> Orientierungspunkte; Rangierbahnhof mit zahlreichen Gleisen und Güterzügen;
> Lagerhallen, Silos, Kräne und Hafenanlagen; **breite Hauptstraßen für
> Panzerverbände, enge Nebenstraßen für Infanterie und Hinterhalte**; mehrere
> Kanalbrücken als strategische Engpässe; Ruinen und zerstörte Gebäude als
> Deckung; vier Startpositionen und zentrale Kontrollpunkte. **Gameplay-Idee:**
> ein urbanes Schlachtfeld mit kurzen Sichtlinien, vielen Deckungsmöglichkeiten
> und heftigen Gefechten um Fabriken, Bahnhöfe und Brücken.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Stadtkarte `factory_city` (im Spiel „Fabrikstadt“ / „Factory city“) für
**vier Spieler, 2 gegen 2** (Team Nord gegen Team Süd; auch 1 gegen 1): eine
zerschossene Industriestadt, in der zwei Kanäle mit Hafenbecken das Gelände in
drei Teile schneiden. In der Mitte ein Band aus Rangierbahnhöfen und
Ruinenvierteln um den Rathausplatz, links und rechts davon je ein Werksviertel
hinter einem Kanal. Kurze Sichtlinien, viele Stellungen, acht Brücken.

Was die Vorlage zeigt:

- **Kanäle mit Kaimauern** in Knicken durch die Stadt, ein **Hafenbecken** mit
  Kränen, Lagerhallen und einem Frachter;
- **Fachwerkbrücken aus Stahl** und **Steinbogenbrücken** über die Kanäle;
- ein **großes Werk** im Norden mit hohen Schornsteinen, Hallen, Rohrbrücken
  und Rauch; weitere **Fabriken** im Süden und Osten, teils in Trümmern;
- **Bahnanlagen**: eine große Bahnhofshalle im Westen, Gleisfelder mit
  Güterzügen, ein **Ringlokschuppen** mit Drehscheibe im Osten;
- **Silos und Tanks** an mehreren Stellen;
- ein **Rathaus mit Uhrturm** an einem Platz in der Mitte;
- **Backsteinblöcke**, viele davon ausgebrannt oder eingestürzt;
- breite Straßen mit Kreuzungen und Plätzen, schmale Gassen dazwischen.

## 2. Größe, Symmetrie

- **2,0 km × 2,0 km**, Biom `forest`, Wetter klar (dunstig bzw. bedeckt, falls
  es ein passendes Wetter gibt).
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y): Kanäle, Brücken,
  Straßen, Gleisfelder, Grundrisse der Werke und Blöcke, Starts und Ziele. Die
  **Art** der Bauten darf sich unterscheiden (Stahlwerk hier, Maschinenfabrik
  dort; Osthafen mit Kränen, Westhafen mit Bahnhof), solange Weggitter, Sicht
  und Objektklassen gespiegelt gleich sind. Die Vorlage ist nicht symmetrisch;
  der Aufbau unten ordnet ihre Teile neu.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0). Was nur für eine
Seite steht, gibt es unter (x, y) → (−x, −y) auch für die andere. Die Session
übernimmt die Werte und glättet sie nach den Stufenbildern.

- **Nordkanal** (`[[canal]]`, 35 m breit, Kaimauern, für Fahrzeuge und
  Infanterie gesperrt): vom Nordrand (−320, −1000) nach Süden bis (−320, −220),
  Knick nach Osten über (−50, −120) und (250, −70) in das **Osthafenbecken**
  (etwa 650 m × 250 m, von x 350 bis zum Ostrand, y −170 … 80). **Südkanal:**
  das Spiegelbild, vom Südrand (320, 1000) nach Norden, Knick nach Westen in das
  **Westhafenbecken** am Westrand. Die beiden Kanäle berühren sich nicht.
- **Drei Stadtteile:**
  - das **Mittelband** zwischen den Kanälen, von der Nordwestecke durch die
    Mitte zur Südostecke; darin die Lager Nordwest und Südost, die
    Rangierbahnhöfe, das Rathausviertel. **Die beiden Gegner im Mittelband sind
    ohne Brücke über Land verbunden.**
  - das **Nordostviertel** hinter dem Nordkanal (Stahlwerk, Lager Nordost);
  - das **Südwestviertel** hinter dem Südkanal (Maschinenfabrik, Lager Südwest).
- **Acht Brücken** (`[[crossing]]`; je Kanal vier):
  - über den **Nordlauf** (West-Ost, verbinden das Lager Nordwest mit dem
    verbündeten Nordostviertel): **Stahlfachwerk** bei (−320, −750),
    **Steinbrücke** bei (−320, −420);
  - über den **Ostlauf** (Nord-Süd, führen aus dem Nordostviertel ins
    Mittelband und damit in die Flanke des Gegners): Steinbrücke bei
    (20, −110), Stahlfachwerk bei (270, −70);
  - gespiegelt über den Südkanal.
  Fachwerkbrücke: Modell aus 115, falls erledigt; sonst die vorhandene
  Steinbrücke und das Fachwerk als offener Punkt. Kanal an den Brücken auf die
  Brückenlänge verengt, falls nötig. Alle Brücken sprengbar.
  **Hafenbecken:** keine Brücke; ein schmaler **Schleusensteg** an der
  Beckenmündung bei (360, −60) nur für Infanterie (aus 151, sonst weglassen).
- **Hauptstraßen** (25–30 m breit, zwei Panzer nebeneinander und mehr, kaum
  Deckung):
  - die **Magistrale** durch das Mittelband vom Lager Nordwest über den
    Rathausplatz zum Lager Südost, mit zwei Plätzen;
  - je eine **Werksstraße** vom Lager Nordost durch das Stahlwerk zu den beiden
    Brücken über den Ostlauf, gespiegelt im Südwesten;
  - je eine **Uferstraße** an jedem Kanal.
  **Nebenstraßen und Gassen** (6–12 m, Gassen für Fahrzeuge zu schmal oder nur
  einspurig): ein dichtes Netz in den Blockvierteln und zwischen den
  Werksbauten; Höfe und Durchgänge, die nur Infanterie nimmt. Zielgröße: Von
  jedem Punkt einer Hauptstraße liegen mindestens zwei Gassenmündungen in
  Panzerfaustreichweite.
- **Rathausviertel** in der Mitte: **Rathausplatz** bei (0, 0) mit Rathaus und
  Uhrturm an der Nordseite, ringsum zwölf bis sechzehn **Backsteinblöcke**
  (`[[city_block]]`, vier bis fünf Geschosse), davon **die Hälfte beschädigt
  oder Ruine**; Trümmerfelder, zwei eingestürzte Ecken, die Gassen öffnen.
- **Rangierbahnhof West** im Mittelband zwischen Lager Nordwest und
  Rathausviertel: Gleisfeld (`[[rail_yard]]` bzw. parallele `[[rail]]`) von
  (−900, −600) nach (−420, −180), 180 m breit, zehn bis zwölf Gleise, davon
  etwa die Hälfte mit **Güterzügen** (`[[train]]`) belegt, sodass Gassen und
  Sackgassen entstehen; **Bahnhofshalle** (137) am Nordrand des Felds bei
  (−700, −330), Stellwerk, Güterschuppen. Die Magistrale quert das Gleisfeld
  auf einem breiten Übergang. Gespiegelt der **Rangierbahnhof Ost** mit
  **Ringlokschuppen** und Drehscheibe statt der Bahnhofshalle.
- **Stahlwerk** (`[[factory]]`, Art `steel`) im Nordostviertel um (130, −680),
  etwa 550 m × 400 m: Shedhallen in drei Zeilen, Geschossfabrik an der
  Werksstraße, Kesselhaus, **drei hohe Schornsteine**, Werksturm, Rohrbrücken,
  Werkshof, Gleisanschluss mit Zug; ein **Tanklager** am Rand bei (600, −850).
  Gespiegelt die **Maschinenfabrik** (Art `machine`) mit zwei Schornsteinen und
  einer **Silogruppe**.
- **Osthafen** (`[[harbor]]`): Kai am Südufer des Beckens (im Mittelband) von
  (380, 90) bis (950, 110) mit fünf **Kränen**, drei **Lagerhallen** dahinter
  um (570, 180), Kaigleis mit Wagen; am Nordufer (im Nordostviertel) ein
  Kohlenlager. Gespiegelt der **Westhafen**. Ein Frachter am Kai, falls 155 ein
  Modell gebracht hat.
- **Arbeiterviertel:** Backsteinblöcke und Reihen kleiner Häuser zwischen
  Werken, Bahn und Lagern; kleine Parks und Baumreihen sparsam; ein
  Gasbehälter bzw. Silos als Landmarken.
- **Lager** auf freien Plätzen (Lagerplatz, Sportplatz, Brache) in den Ecken.

## 4. Spiel

- **Starts** (60 m eben und frei): Team Nord Nordwest (−780, −830) und Nordost
  (830, −830); Team Süd gespiegelt Südost (780, 830) und Südwest (−830, 830).
  Team 0 Süd (Spieler 0, 2), Team 1 Nord (1, 3). 1 gegen 1 nutzt Nordwest gegen
  Südost (die beiden im Mittelband).
- **Versorgungspunkte** (sieben): Rathausplatz (0, 0), umkämpft; Stahlwerk
  (130, −680) und Maschinenfabrik (−130, 680); Osthafen-Lagerhallen (570, 180)
  und Westhafen (−570, −180); Rangierbahnhof West (−660, −400) und Ost
  (660, 400).
- **Aufbau des Kampfes:**
  - **Mittelband:** der direkte Weg, über Land: Magistrale, zwei
    Rangierbahnhöfe, Ruinenviertel. Panzer auf der Magistrale, Infanterie in
    Zügen, Blöcken und Trümmern.
  - **Werksviertel:** Jedes Team hat einen Spieler hinter einem Kanal. Sein
    Viertel ist über zwei Brücken mit dem Verbündeten verbunden und über zwei
    weitere mit dem Mittelband auf Höhe des Rathauses: Von dort fällt er dem
    Gegner in die Flanke, oder der Gegner sperrt die Brücken und isoliert ihn.
  - **Hafenbecken:** offene Wasserfläche mit langen Sichtlinien quer durch die
    Stadt, die einzige; Kräne und Lagerhallen als Deckung am Kai.
  - **Tanklager und Kesselwagen** explodieren (155): gefährliche Deckung.
  - **Schornsteine und Kräne** fallen und sperren Straßen.
- **Kurze Sichtlinien, belegt:** mittlere und größte Sichtweite auf Magistrale,
  Nebenstraße, Gasse, Gleisfeld, Hafenkai und Rathausplatz messen und in
  `docs/MAPS.md` nennen (Ziel: außerhalb von Magistrale, Uferstraßen und Hafen
  selten über 150 m).
- **Wege sichern:** Fällt ein Schornstein oder Kran auf eine Hauptstraße oder
  wird eine Brücke gesprengt, bleibt jedes Lager mit jedem Ziel verbunden (Test
  mit Fall in jede Richtung und mit jeder einzelnen Brücke gesprengt). Mit
  allen acht Brücken gesprengt sind die Werksviertel für Fahrzeuge
  abgeschnitten; das Mittelband bleibt offen.
- **Reichweitentabelle** (indirekt 1/8) in `docs/MAPS.md`: auf dieser Größe
  reicht Artillerie weit; nennen, was aus den Lagern erreichbar ist, und die
  Lager so legen, dass sie nicht im Feuer leichter Haubitzen aus dem
  gegnerischen Lager liegen (sonst Lager in die Ecken rücken oder durch Bauten
  decken, was bei indirektem Feuer nicht hilft: dann als Eigenschaft der Karte
  benennen).

## 5. Stufen

**Stufe 1 – Wasser, Verkehr, Gliederung:** `data/map_sketches/factory_city.toml`
mit Kanälen, Hafenbecken, Brücken, Hauptstraßen, Gleisfeldern, Umrissen der
Werke und Viertel, Starts und Zielen, erzeugt mit
`just gen-sketch factory_city`. Bestehende Skizzenkarten müssen Bit für Bit
gleich bleiben.

**Stufe 2 – Bebauung:** Werke, Bahnhöfe mit Zügen, Häfen, Rathausviertel,
Arbeiterviertel, Ruinen, Gassen. Laufend Bildrate messen und die Dichte danach
einstellen.

**Stufe 3 – Prüfen und Abschluss:**

- `just map-check factory_city`, `just map-reach factory_city`: alle Starts und
  Ziele für Fahrzeuge erreichbar, keine zugebauten Hauptstraßen, keine Bauten
  im Wasser oder auf Gleisübergängen; Anteil der Fläche, die nur Infanterie
  erreicht, nennen.
- **Symmetrietest:** unter Punktspiegelung gleiche Höhen, Gelände, Weggitter,
  Objektklassen und Weglängen.
- **Sichtprüfung:** die Messung aus Abschnitt 4 und Sichtfeld-Bilder vom
  Rathausplatz, von der Magistrale, aus einer Gasse, vom Gleisfeld zwischen
  Zügen, vom Hafenkai, von einer Brücke.
- **Zerstörungsprobe:** schwerer Beschuss des Stahlwerks, des Tanklagers und
  eines Blockviertels; Schornsteine fallen, Tanks explodieren, Wege und Sicht
  stimmen danach, Tick und Wegsuche bleiben im Zeitbudget (schlechtesten Tick
  nennen).
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`), über dem
  dichtesten Viertel und dem Rangierbahnhof; reicht das Budget nicht, Wagen und
  Stapel ausdünnen, Blöcke vergrößern, und festhalten.
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Kommt die KI
  durch Gassen und Gleisfelder, nutzt der Spieler im Werksviertel seine
  Brücken, besetzt sie Fabriken, endet es im Stillstand am Rathausplatz?
  Probleme als offene Punkte; die KI nicht umbauen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md` mit Sicht- und Reichweitentabelle, Test wie bei den anderen
  Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`,
  `just selftest`. Der Golden Hash ändert sich nicht (neue Karte; die Regeln
  kamen mit 134, 137 und 155); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Fahrende Züge und Straßenbahnen, Schiffe in Fahrt, Boote auf den Kanälen
  (Prompt 119).
- Fabriken als Produktionsstätten oder Rohstoffquellen.
- Kanalisation und Keller als Wege.
- KI-Verbesserungen für Häuserkampf.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
