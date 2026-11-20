# Prompt 154 – Karte nach Bildvorlage: Schärenfjord (Fjord mit Inselkette, sechs Brücken, vier Spieler)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `sea`, `[[land]]`, `[[island]]`,
> `[[bay]]`, `[[lake]]`, **`[[causeway]]`** und **`[[shoal]]`**, `[[crossing]]`,
> `[[hill]]`, `[[wood]]`, `[[village]]`, `[[farm]]`, `[[ring]]`), „Bridges“ und
> die Karte **`island_chain`** als Muster (Inseln im Meer, Steinbrücken auf
> Dämmen, Untiefen als zweiter Weg; `data/map_sketches/island_chain.toml`); in
> `docs/ARCHITECTURE.md` Wegfindung, Sicht („Seeing downhill“), Pioniere
> (Brücken sprengen und reparieren); `data/scale.toml` (`[range]`),
> `data/terrain.toml` (flaches Wasser, Steigung), `crates/cli/src/gen_sketch.rs`,
> `docs/AI.md`, `docs/PERFORMANCE.md`. Sind die Prompts 115 (lange Brücken), 122
> (Inselreich: Felsberge mit Schneekuppen, Wasserfälle, farbiger Look), 132/133
> (Auffahrten entlang einer Weglinie) oder 139 (Burg- und Ruinenteile) erledigt,
> auch deren Ergebnis; Bausteine wiederverwenden statt neu bauen.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/schaerenfjord/schaerenfjord.webp`. Genau ansehen. Ein mit KI
> erzeugtes, frei erfundenes Bild einer nordischen Fjordlandschaft mit vier
> Lagern in den Ecken und vier Sternen.
>
> **Vorgeschichte:** Der Nutzer fragte, ob das Bild brauchbar sei und ob mehr
> Brücken nötig seien. Befund: Die Landschaft trägt, aber so wie gezeichnet ist
> die Karte nicht spielbar. Das Lager im Nordwesten erreicht nur eine Insel ohne
> Weiterweg, die Inseln in der Mitte sind nur von Osten angebunden, die
> Leuchtturminsel gar nicht, und alle vier Brücken begünstigen den Osten. Der
> Nutzer hat den Vorschlag angenommen: **mehr Verbindungen, punktsymmetrisch,
> 2 gegen 2.** Der Aufbau unten ist danach neu geordnet und weicht von der
> Vorlage ab.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte `skerry_fjord` (im Spiel „Schärenfjord“ / „Skerry fjord“) für **vier
Spieler, 2 gegen 2** (Team West gegen Team Ost; auch 1 gegen 1): Ein Fjord
trennt zwei felsige Festländer. Hinüber führen nur **drei Routen über drei
Inseln**, jede mit zwei Brücken: im Norden über eine Fortinsel, in der Mitte
über die Insel mit der Burgruine, im Süden über eine zweite Fortinsel. Dazu
Untiefen über Schären als Weg für Infanterie. Wenig befahrbare Fläche zwischen
Fels und Wasser; jede Brücke zählt.

Was die Vorlage zeigt und der Entwurf aufnimmt:

- **tiefblaues Meer** mit Buchten, Felsküsten, vielen kleinen **Schären**;
- eine **Insel mit Burgruine** in der Mitte, **Inseln mit kleinen Forts**, eine
  **Leuchtturminsel**;
- **Steinbogenbrücken** von Ufer zu Insel und von Insel zu Insel;
- ein **Hafenort** mit Kais, Kränen, Lagerhäusern und roten Holzhäusern;
  Weiler mit Feldern, einzelne Höfe;
- **Felsberge mit Schneeresten** an den Rändern, **Wasserfälle** über
  Klippen, dichter Nadelwald;
- einen **Sandstrand mit Tschechenigeln**;
- **Lager auf Felsterrassen** in den Ecken, geschwungene Küstenstraßen.

## 2. Größe, Symmetrie, Abgrenzung

- **2,4 km × 2,4 km**, Biom `forest`, Wetter klar, Meer (`sea`) als
  Grundfläche.
- **Punktsymmetrisch** um die Kartenmitte, (x, y) → (−x, −y): Fjord, Inseln,
  Brücken, Untiefen, Straßen, Orte, Starts und Ziele. Hausvarianten und
  Kulisse (Leuchtturm hier, Leuchtfeuer dort; Kran hier, Bootsschuppen dort)
  dürfen sich unterscheiden, solange Weggitter, Sicht und Objektklassen
  gespiegelt gleich sind.
- **Abgrenzung:** `island_chain` ist eine Inselkette mit einer einzigen Straße
  über vier Brücken; Prompt 122 (Inselreich) ein Archipel mit Ringinsel. Diese
  Karte ist ein **Fjord zwischen zwei Festländern** mit drei gleichwertigen
  Routen, steilen Küsten und Höhe über dem Wasser.

## 3. Aufbau

Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0). Beschrieben sind
die **Westseite** und die Inseln; alles Übrige ist das Bild unter
(x, y) → (−x, −y). Die Session übernimmt die Werte und glättet sie nach den
Stufenbildern.

- **Fjord:** Das Meer läuft als Band von 500–900 m Breite vom Nordrand zum
  Südrand durch die Mitte und trennt **Westland** und **Ostland vollständig**.
  Ufer des Westlands grob bei x ≈ −450 im Norden, mit einer **Halbinsel** nach
  Osten bis (−420, −30) in der Mitte und einer **Bucht** dahinter (Bucht um
  (−700, 150), sie reicht **nicht** bis zum Westrand: dort bleibt ein
  Landstreifen von mindestens 200 m für die Küstenstraße). Küste überall
  buchtig und felsig, mit vorgelagerten Schären.
- **Drei Inseln:**
  - **Mittelinsel** um (0, 0), etwa 460 m × 330 m, Felsufer, Rücken 20–25 m über
    dem Wasser; darauf die **Burgruine** (Ringmauer mit Breschen, Turmstumpf,
    Hof; Teile aus 139, sonst Mauern über `[[ring]]` und Steinhäuser), Wald an
    den Hängen;
  - **Nordinsel** um (−80, −720), etwa 520 m × 340 m, mit einem **kleinen Fort**
    auf einer Kuppe (ummauerte Höhe mit Geschützstellung), einem Weiler von vier
    Häusern und einer sandigen Bucht nach Norden;
  - **Südinsel** um (80, 720): das Spiegelbild, ihre Bucht nach Süden ist der
    **Strand mit Tschechenigeln** der Vorlage (Hindernisse und Draht als
    Kulisse und Deckung, Gassen frei).
- **Sechs Brücken** (alle West-Ost; die Wasserlücke läuft an jeder Brücke kurz
  Nord-Süd):
  - **Nordroute:** Westland → Nordinsel bei (−400, −650); Nordinsel → Ostland
    bei (240, −770);
  - **Mittelroute:** Halbinsel des Westlands → Mittelinsel bei (−330, −20);
    Mittelinsel → Halbinsel des Ostlands bei (330, 20);
  - **Südroute:** gespiegelt zur Nordroute: Westland → Südinsel bei
    (−240, 770); Südinsel → Ostland bei (400, 650).
  Die Lücken sind 100–180 m breit. **Ist Prompt 115 erledigt:** lange
  Bogenbrücken von Ufer zu Ufer, in Teilen sprengbar. **Sonst** wie bei
  `island_chain`: ein Damm (`[[causeway]]`) über flache Schären, in der Mitte
  von der vorhandenen Steinbrücke überspannt; die langen Brücken als offener
  Punkt. Brücken sind sprengbar und von Pionieren wiederherzustellen.
- **Untiefen für Infanterie** (`[[shoal]]`, als schmale Kette über Schären; für
  Fahrzeuge gesperrt, wenn sich das über Breite oder Blockfelsen einfach machen
  lässt, sonst sehr langsam und als Entscheidung festhalten): vom Westland bei
  (−860, 240) zur **Leuchtturminsel** (−600, 180) und weiter zum Südwestufer
  der Mittelinsel bei (−250, 110); gespiegelt im Osten. Das ist der Weg in die
  Mitte, wenn die Brücken gefallen sind, und der einzige zu den
  Leuchtturminseln.
- **Leuchtturminsel** um (−600, 180), etwa 180 m × 120 m: Leuchtturm (ein
  einfaches Modell `env_lighthouse` aus dem Baukasten, zerstörbar wie ein
  kleines Steinhaus mit Turm, mit Vorschaubild; oder ein Stellvertreter aus
  vorhandenen Teilen und das Modell als offener Punkt), Wärterhaus. Im Osten
  das Gegenstück mit Leuchtfeuer bzw. Mole.
- **Westland:**
  - **Lager Nordwest** auf einer Felsterrasse bei (−920, −950) und **Lager
    Südwest** bei (−920, 720), je 30–40 m über dem Wasser, **mit je zwei
    Abfahrten**;
  - **Küstenstraße** von Lager zu Lager hinter der Bucht entlang
    (x ≈ −1000 … −850), mit Abzweigen zu den drei Brückenköpfen; Serpentinen an
    den Abstiegen (Steigung und Breite aus den Regeln ablesen, Fahrbahn
    mindestens 8 m, für alle Fahrzeuge);
  - **Hafenort** an der Bucht um (−880, 330): Kai aus Stein, zwei Stege, sechs
    bis zehn rote Holzhäuser, Lagerhaus, Bootsschuppen; im Osten das Gegenstück
    als etwas größerer Hafen mit Kran und Lagerhäusern (gleiche Grundrisse);
  - ein **Weiler mit Feldern** bei (−700, −500) am Nordbrückenkopf, ein Hof bei
    (−650, 900);
  - **Felsberge** am Rand (unpassierbar, 120–200 m, Schneereste auf den Kuppen
    mit dem Look aus 122, falls erledigt; sonst heller Fels): in der
    Nordwestecke hinter dem Lager, am Westrand bei (−1100, −200), in der
    Südwestecke; **Wasserfälle** über zwei Klippen als Kulisse (Bild aus 122,
    falls erledigt; sonst offener Punkt);
  - **Nadelwald** auf allen Hängen, die nicht Fels sind.
- **Relief:** Festland als felsige Hochfläche 20–60 m über dem Meer mit Klippen
  zum Fjord; die Brückenköpfe liegen auf Vorsprüngen in 8–15 m Höhe, die
  Straßen steigen dorthin ab. Von den Klippen sieht man weit über das Wasser
  (Sicht nach unten greift ab 20 m).

## 4. Spiel

- **Starts** (60 m eben und frei): Team West Nordwest (−920, −950) und Südwest
  (−920, 720); Team Ost gespiegelt Südost (920, 950) und Nordost (920, −720).
  Team 0 West (Spieler 0, 2), Team 1 Ost (1, 3). 1 gegen 1 nutzt Nordwest gegen
  Südost.
- **Versorgungspunkte** (sieben): Burgruine auf der Mittelinsel (0, 0),
  umkämpft; Fort der Nordinsel (−80, −720) und Fort der Südinsel (80, 720),
  umkämpft; Hafenorte (−880, 330) und (880, −330); Leuchtturminseln
  (−600, 180) und (600, −180), **nur für Infanterie erreichbar**.
- **Was die Karte erzwingt** (in Stufe 3 mit Bild oder Test belegen):
  - Jede Route ist ein Paar von Brücken mit einer Insel dazwischen: Wer die
    Insel hält, hält beide Brücken; wer eine sprengt, macht aus der Insel eine
    Sackgasse.
  - Mit **einer** gesprengten Brücke bleiben zwei Routen; mit **allen sechs**
    kommt nur Infanterie über die Untiefen in die Mitte. Kein Fahrzeug erreicht
    dann das andere Ufer, bis Pioniere eine Brücke wiederherstellen. Das ist
    gewollt; ein Brückenleger überbrückt die Lücke der Steinbrücke auf dem Damm
    (falls ohne 115 gebaut) bzw. keine der langen Brücken.
  - Die Forts sehen und beschießen die Nachbarrouten über das Wasser; die
    Burgruine in der Mitte sieht alle drei.
  - Über das offene Wasser gibt es keine Deckung: Eine Brücke unter Feuer zu
    queren ist teuer.
- **Reichweitentabelle** (indirekt 1/8, Sicht und direktes Feuer 1/4) in
  `docs/MAPS.md` wie in Prompt 123: von jedem Fort und der Burgruine auf jede
  Brücke und jeden Brückenkopf; aus den Lagern (Ziel: kein Lager in Reichweite
  eines gegnerischen Lagers).

## 5. Stufen

**Stufe 1 – Küste und Relief:** `data/map_sketches/skerry_fjord.toml` mit Meer,
Festländern, Inseln, Schären, Felsbergen und Relief, erzeugt mit
`just gen-sketch skerry_fjord`; Höhenbild und Bild der befahrbaren Fläche.

**Stufe 2 – Wege und Orte:** Brücken (oder Dämme), Untiefen, Küstenstraßen,
Serpentinen, Burgruine, Forts, Hafenorte, Weiler, Leuchtturminseln, Wald,
Strandhindernisse, Starts und Ziele.

**Stufe 3 – Prüfen und Abschluss:**

- `just map-check skerry_fjord`, `just map-reach skerry_fjord`: alle Starts und
  die fünf befahrbaren Ziele für Fahrzeuge erreichbar; die Leuchtturminseln nur
  für Infanterie; Felsberge für alle zu.
- **Brückenprobe, als Test:** für jede einzelne Brücke gesprengt bleibt jedes
  Lager mit jedem befahrbaren Ziel verbunden außer höchstens der betroffenen
  Insel über die zweite Brücke; mit allen sechs gesprengt erreicht Infanterie
  über die Untiefen die Mittelinsel; Fahrzeuge fahren nicht ins tiefe Wasser.
- **Fahrprobe:** der schwerste Panzer und ein Zugmittel mit gezogenem Geschütz
  von jedem Lager über jede Abfahrt zu jedem Brückenkopf und über jede Route
  ans andere Ufer; Fahrzeiten der drei Routen nebeneinander in `docs/MAPS.md`
  (Ziel: höchstens 30 % auseinander).
- **Symmetrietest:** unter Punktspiegelung gleiche Höhen, Gelände, Weggitter,
  Objektklassen und Weglängen.
- **Sichtprüfung:** Sichtfeld-Bilder von der Burgruine, von beiden Forts, von
  einer Klippe am Lager, von einer Brückenmitte, von einer Leuchtturminsel.
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`); viel
  Wasserfläche und Felsküste: messen.
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Verteilt
  die KI sich auf die drei Routen, staut sie sich auf den Brücken, nimmt sie
  die Inseln, endet es im Stillstand? Probleme als offene Punkte; die KI nicht
  umbauen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten in
  `docs/MAPS.md`, Test wie bei den anderen Skizzenkarten (zweimal erzeugt,
  gleiche Dateien). `just check`, `just selftest`. Der Golden Hash ändert sich
  nur, wenn der Leuchtturm einen neuen Eintrag in den Regeltabellen braucht;
  dann Ursache nennen, Hash neu setzen und im Commit sagen. Sonst bleibt er.

## 6. Nicht Teil dieses Prompts

- Boote und Landungen über den Fjord (Prompt 119); mit ihnen bekäme die Karte
  einen vierten Weg, das ist ein späterer Schritt.
- Schiffe, Kräne und Kähne als Modelle (Kulisse, offene Punkte).
- Küstenbatterien als eigene Objekte, Seeminen, Gezeiten.
- Die Landverbindung über den Strand am Südrand der Vorlage (sie würde den
  Fjord umgehen und die Routen entwerten).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
