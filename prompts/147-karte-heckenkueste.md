# Prompt 147 – Karte nach Bildvorlage: Heckenküste (Bocage, Dörfer, Strand mit Bunkern, zwölf Spieler)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 143
> (sehr große Karten, zwölf Spieler; seine Empfehlung für Größe und
> Einheitengrenze gilt hier) ist erledigt (`prompts/done/143-*`). Empfohlen nach
> 124 (Gräben, Draht, Trichter als Skizzen-Bausteine). Lies zuerst `CLAUDE.md`,
> dann `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md`
> „Objects“ (Hecken und Mauern: für Panzer zu, für Infanterie offen), die Karte
> **`farms`** (Feldwege mit Hecken und Lücken für Panzer; Heckenabstände) und
> „Maps from picture sketches (prompt 088)“ ganz (besonders `[[field]]`,
> `[[patchwork]]`, `[[farm]]`, `[[village]]`, `[[wood]]`, `[[river]]`,
> `[[crossing]]`, `[[shoal]]`, `[[lake]]` mit Meer, `sea`), „Bridges“; in
> `docs/ARCHITECTURE.md` Sicht (Hecken sperren die Sicht bis zu ihrer
> Kataloghöhe, „bocage“), Wegfindung (Hindernisse für große Einheiten),
> „Destruction“ und die Regeln zum Niederwalzen
> (`prompts/done/093-panzer-mauern-ueberrollen-mg.md`);
> `data/destruction.toml` (`env_hedge`, Mauern, `crush_min_mass_kg`),
> `data/objects.toml`, `data/terrain.toml`, `data/scale.toml` (`[range]`),
> `docs/TEAMS.md`, `docs/SOURCES.md`, `crates/cli/src/gen_sketch.rs`,
> `docs/AI.md`, `docs/PERFORMANCE.md`.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/heckenkueste/heckenkueste.webp`. Genau ansehen. Ein mit KI
> erzeugtes, frei erfundenes Bild einer Heckenlandschaft an der Küste. Der
> Titelkasten nennt 12 Spieler und 4 km × 4 km, das Bild ist aber breiter als
> hoch; der Maßstabsbalken ist unstimmig und wird ignoriert. Die Karte bekommt
> einen neutralen Namen; die Landschaft der Normandie von 1944 ist das Vorbild
> und steht nur in `docs/MAPS.md` und `docs/SOURCES.md`.
>
> **Beschreibung des Nutzers** (gilt als Vorgabe): ausgelegt auf
> Infanteriekämpfe, Panzergefechte und taktische Hinterhalte. Küstenabschnitt mit
> Bunkern, Strandhindernissen und Verteidigungsstellungen; Bocage mit hohen
> Hecken und schmalen Durchfahrten; kleine Dörfer mit Kirchen, Bauernhöfen und
> Ruinen; zahlreiche Flussübergänge und Steinbrücken; offene Felder für
> Panzerverbände; Waldstücke und Hecken für Hinterhalte der Infanterie; zwölf
> Startpositionen und mehrere neutrale Kontrollpunkte. **„Panzer können nicht
> überall querfeldein fahren. Die wenigen Durchgänge durch die Hecken werden zu
> wichtigen taktischen Positionen.“**
>
> **Dieser Prompt läuft in vier Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Karte `hedgerow_coast` (im Spiel „Heckenküste“ / „Hedgerow coast“) für
**zwölf Spieler, 6 gegen 6** (Team West gegen Team Ost; auch kleinere
Besetzungen): Bauernland aus kleinen Feldern, die von **Wallhecken** umschlossen
sind, mit Hohlwegen, Dörfern, Gehöften, Bächen mit Steinbrücken und einem
Strand mit Bunkern am Nordrand. **Fahrzeuge kommen nur durch die Durchfahrten
der Hecken**; Infanterie kommt überall durch, langsam und ungesehen.

Die Vorlage zeigt:

- **Meer und Strand** am Nordrand: Brandung, Sandstrand mit Tschechenigeln und
  Pfählen, **Bunker** am Dünenrand, Landungsstege bzw. Wracks im Wasser, vier
  bis fünf Wege vom Strand ins Land;
- ein **Flickenmuster kleiner Felder** (Getreide, Stoppel mit Garben, Wiese,
  Gemüse, Obst), jedes von dunklen Hecken mit Bäumen eingefasst;
- ein **Kirchdorf** in der Mitte an einer Kreuzung, mehrere **Weiler** und
  **Gehöfte**, zwei **Ruinen auf Kuppen** (eine Abtei im Nordwesten, eine
  befestigte Ruine im Nordosten), zerschossene Häuser mit Rauch;
- **Bäche** mit **Steinbrücken** im Westen und Osten;
- **Waldstücke** an den Rändern, Baumreihen an Straßen;
- geschwungene Landstraßen und Feldwege;
- **zwölf Lager**: vier hinter dem Strand, vier am Südrand, je zwei am West- und
  Ostrand; **neun Sterne**.

Die Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe und Symmetrie

- **6,0 km × 4,0 km**, wenn Prompt 143 das trägt; sonst die dort empfohlene
  Größe (alle Maße aus Abschnitt 4 im gleichen Verhältnis verkleinern, nicht
  unter 4,0 km × 2,664 km; die Feldgrößen bleiben gleich, es werden weniger
  Felder). Biom `forest`, Wetter klar.
- **Spiegelsymmetrisch** an der Nord-Süd-Achse (West gegen Ost), (x, y) →
  (−x, y): Die Lager der Vorlage liegen schon so. Gelände, Hecken mit ihren
  Durchfahrten, Bäche, Brücken, Dörfer, Starts und Ziele gespiegelt; das
  Kirchdorf und zwei weitere Orte liegen auf der Achse. Die Küste ist für beide
  Teams die Nordflanke. Hausvarianten, Feldfrüchte und Farben dürfen sich
  unterscheiden, solange Weggitter, Sicht und Objektklassen gespiegelt gleich
  sind (das prüft der Symmetrietest).

## 3. Wallhecken: der Kern der Karte

Das Spiel kennt Hecken (`env_hedge`: für Panzer zu, für Infanterie offen,
sperren die Sicht, 600 Trefferpunkte) und die Karte `farms` hat schon
Heckenwege mit Lücken. Zuerst dort nachsehen, was davon trägt, und nur ergänzen,
was fehlt.

- **Neues Objekt `env_hedge_bank`** (Wallhecke): ein Erdwall von etwa 1–1,5 m
  Höhe und 1,5–2 m Breite, darauf dichte Hecke und Bäume, zusammen 3–5 m hoch.
  Maße mit Quelle nach `docs/SOURCES.md` (Berichte und Vermessungen der
  Heckenlandschaft von 1944); Modell als Segment wie `env_hedge`, mit Varianten
  (mit Baum, mit Lücke im Bewuchs), Vorschaubild, `just units-web-sync`.
- **Regeln** (über `data/objects.toml` und `data/destruction.toml`, keine neue
  Regelart, wenn es geht):
  - **Fahrzeuge:** gesperrt für alle, **auch für schwere Panzer** (kein
    Niederwalzen: `crush_min_mass_kg = 0`). Das ist die Vorgabe des Nutzers.
  - **Infanterie:** kommt durch, deutlich langsamer als durch eine einfache
    Hecke (vorhandene Art von Verlangsamung an Objekten verwenden).
  - **Sicht und Feuer:** sperrt die Sicht bis zur Kataloghöhe, auch für
    Panzerkommandanten; stoppt flache Geschosse wie eine Mauer bis zur Wallhöhe,
    darüber wie Laub. Infanterie **an** der Hecke sieht hindurch und ist
    gedeckt, wenn es die Regel für besetzte Deckung an Objekten schon gibt;
    sonst offener Punkt.
  - **Zerstörung:** zäh (Richtgröße 2500–3500 Trefferpunkte je Segment, das
    Vier- bis Sechsfache der einfachen Hecke); Sprengwirkung schwerer Kaliber
    und Sprengladungen der Pioniere reißen eine **Lücke** (`gone`), durch die
    dann Fahrzeuge passen. Wer keine Durchfahrt hat, kann sich also eine
    sprengen: teuer, laut, langsam.
  - **Der Golden Hash ändert sich** (neue Klasse in den Regeltabellen);
    gewollter Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit nennen.
    Vergleichslauf auf `farms` zeigt gleiche Ergebnisse wie vorher.
- **Durchfahrten:** Lücken von 4–5 m (ein Panzer passt, zwei nicht) mit
  Feldgatter (vorhandener Zaun als Tor, überfahrbar). **Jedes Feld hat ein bis
  zwei Durchfahrten**, selten drei; sie liegen so, dass ein Panzerweg quer durch
  die Felder im Zickzack führt und nie geradeaus. Je Durchfahrt gibt es auf der
  anderen Feldseite eine Stelle, von der aus sie zu sehen ist (Stellung für
  Pak, Panzerfaust, MG).
- **Hohlwege:** Feldwege zwischen zwei Wallhecken, 1–2 m eingetieft, 4–6 m
  breit: schnelle, gedeckte Wege, in denen ein Wrack alles sperrt.
- **Skizzen-Baustein `[[bocage]]`** (in `docs/MAPS.md` beschreiben): ein Gebiet
  als Polygon; der Generator teilt es in unregelmäßige Felder von 120–280 m
  Kantenlänge (Vierecke, an Wegen und Bächen ausgerichtet, deterministisch aus
  dem Seed), setzt Wallhecken auf alle Feldgrenzen, Durchfahrten nach der Regel
  oben, Feldfrüchte je Feld (wie `[[patchwork]]`), Garben und Heuhaufen,
  einzelne Bäume. Schalter: Feldgröße, Durchfahrten je Feld, Anteil offener
  Grenzen (für die offenen Felder in Abschnitt 4). Mit `mirror = "x"` oder über
  die vorhandene Spiegelung erzeugt er beide Hälften gleich. Bestehende
  Skizzenkarten bleiben Bit für Bit gleich.

## 4. Aufbau

Richtwerte in Metern für 6 km × 4 km (x nach Osten, y nach Süden, Mitte 0/0).
Beschrieben ist die **Westhälfte** und die Achse; die Osthälfte ist das
Spiegelbild. Die Session übernimmt die Werte und glättet sie nach den
Stufenbildern.

- **Küste:** Meer (`sea`) am Nordrand bis y ≈ −1650; **Strand** 120–150 m breit
  bis y ≈ −1500, leicht gebogen; dahinter eine **Dünenstufe** von 8–12 m, für
  Fahrzeuge nur an **Strandausgängen** zu nehmen: auf der Achse (0, −1500), bei
  (−1250, −1500) und bei (−2350, −1500). Der Strand ist eine schnelle, offene
  Querverbindung von West nach Ost unter den Augen der Bunker.
- **Strandverteidigung** (gespiegelt, also für beide Teams gleich; sie gehört
  niemandem): Bunker (`fort_bunker_mg`) am Dünenrand bei (−350, −1480),
  (−1000, −1470), (−1800, −1480) mit Schussfeld über den Strand und auf die
  Ausgänge; Tschechenigel (`fort_hedgehog`), Höckerhindernisse und Stacheldraht
  in zwei Reihen auf dem Strand mit Gassen an den Ausgängen; Sandsackstellungen
  und Grabenstücke (`fort_trench_section`, mit 124 `[[trench]]`) auf der Düne;
  Wracks bzw. Pfähle im flachen Wasser als Kulisse aus vorhandenen Objekten.
  Geschützbunker und Landungsstege als eigene Modelle sind offene Punkte.
- **Kirchdorf** auf der Achse um (0, −200): Kirche, Platz, zwölf bis sechzehn
  Häuser an einer Kreuzung von vier Straßen, Mauern, ein paar Ruinen (vorab
  beschädigte Häuser, falls Prompt 134 das gebracht hat; sonst intakte).
- **Nordweiler** auf der Achse um (0, −850) und **Süddorf** auf der Achse um
  (0, 790), je sechs bis acht Häuser.
- **Abteiruine auf einer Kuppe** (15–20 m) um (−1500, −1000): Mauerreste,
  Ruinenhäuser, ein Ring aus Trockenmauern (Burg- bzw. Ruinenteile aus 139
  verwenden, falls erledigt; sonst vorhandene Mauern und Steinhäuser). Sicht
  über Strand und Hinterland.
- **Gehöfte** (`[[farm]]`): bei (−1180, −330), (−1300, 550), (−2100, −700),
  (−2000, 1000), (−450, 1300) und weitere verstreut; ein **Weiler am Bach** bei
  (−1750, 250).
- **Westbach:** vom Westrand (−3000, 900) → (−2000, 600) → Brücke (−1700, 130)
  → Brücke (−2200, −400) → (−2300, −1000) → Brücke der Küstenstraße
  (−2330, −1300) → Mündung am Strand (−2350, −1600). 12–18 m breit, flach:
  **Infanterie überall, Fahrzeuge nur über die drei Steinbrücken und eine Furt**
  (`[[shoal]]`) bei (−2100, 250). Ufer mit Bäumen und Büschen.
- **Südbach:** entspringt in einem Teich bei (−900, 420), fließt über die
  Brücke (−600, 800) nach (−150, 1150), trifft auf der Achse sein Spiegelbild
  und läuft nach Süden zum Rand (0, 2000), mit einer Brücke auf der Achse bei
  (0, 1500) (Bach dort kurz West-Ost geführt, Brücke Nord-Süd) und einer Furt.
  Zusammen **neun Steinbrücken** (vorhandene Steinbrücke, keine langen Brücken
  nötig) und vier Furten.
- **Heckenland** (`[[bocage]]`): alles zwischen Düne, Bächen, Dörfern und
  Wäldern, etwa zwei Drittel der Landfläche.
- **Offene Felder für Panzer:** zwei große Schläge ohne Hecken (nur niedrige
  Raine, Garben, einzelne Bäume): ein **Südfeld** beiderseits der Achse um
  (0, 250), etwa 1200 m × 500 m zwischen Kirchdorf und Süddorf, und je ein
  **Randfeld** um (−2300, 100), etwa 700 m × 500 m. Wer sie quert, ist schnell
  und von den Heckenrändern aus zu sehen.
- **Wald:** Stücke von 200–500 m am Rand bei (−2750, 0), (−2800, 1700),
  (−1600, 1750) und kleine Gehölze im Heckenland (je Seite fünf bis sechs),
  Baumreihen an den Landstraßen.
- **Straßen:** eine **Küstenstraße** hinter der Düne von Rand zu Rand
  (y ≈ −1300); eine **Mittelstraße** von Rand zu Rand durch das Kirchdorf
  (y ≈ −200, über die Brücke (−1700, 130) ausweichend); eine **Südstraße** durch
  das Süddorf (y ≈ 800 bis 1000); die **Achsenstraße** vom Strand durch
  Nordweiler, Kirchdorf und Süddorf zum Südrand; dazu Hohlwege und Feldwege zu
  jedem Gehöft und Lager. Landstraßen 8–10 m, alles geschwungen.
- **Relief:** sanft gewellt (Kuppen 10–20 m), Bachtäler 5–8 m eingetieft, zur
  Küste leicht abfallend.

## 5. Spiel

- **Starts** (60 m eben und frei, auf Wiesen mit Zelten als Kulisse, jedes Lager
  mit **zwei Ausfahrten** durch seine Hecken): Team West (−2400, −1400),
  (−780, −1400), (−2750, −530), (−2750, 560), (−2400, 1450), (−900, 1500);
  Team Ost gespiegelt. Die beiden Lagerpaare nahe der Achse stehen sich auf
  1,5 km gegenüber; prüfen, dass zwischen ihnen keine direkte Sicht und kein
  gerader Fahrweg ohne Durchfahrt liegt. Gerade Spielernummern Team West,
  ungerade Team Ost, in einer Reihenfolge, bei der kleinere Besetzungen
  gespiegelte Lager nutzen; Zuordnung als Tabelle in `docs/MAPS.md`.
- **Versorgungspunkte** (elf): Kirchdorf (0, −200), umkämpft; Nordweiler
  (0, −850) und Süddorf (0, 790), umkämpft; Abteiruine (−1500, −1000) und
  gespiegelt; Gehöft (−1180, −330) und gespiegelt; Weiler am Bach (−1750, 250)
  und gespiegelt; Strandbatterie (−1000, −1470) und gespiegelt.
- **Was die Karte erzwingt** (jede Aussage in Stufe 4 mit Bild oder Test
  belegen):
  - **Panzer** fahren von Durchfahrt zu Durchfahrt, über Hohlwege, Straßen und
    die offenen Felder; jede Durchfahrt ist ein Engpass und ein möglicher
    Hinterhalt. Der Weg durch drei Felder ist zwei- bis dreimal so lang wie die
    Luftlinie.
  - **Infanterie** geht quer durch, ungesehen, und erreicht jede Durchfahrt vor
    den Fahrzeugen; im Heckenland sieht man selten weiter als ein Feld.
  - **Sprengen** öffnet neue Wege (Pioniere, schwere Artillerie) und nimmt dem
    Gegner die Gewissheit, wo ein Panzer herkommt.
  - **Brücken** sperren Fahrzeuge, nicht Infanterie; die Furten sind der zweite
    Weg.
  - **Strand** und offene Felder sind schnell und tödlich.
- **Reichweiten** (indirekt 1/8, Sicht und direktes Feuer 1/4): Tabelle in
  `docs/MAPS.md` wie in Prompt 123; im Heckenland zählt weniger die Reichweite
  als die Sichtweite: mittlere und größte Sichtweite je Gebiet (Heckenland,
  offenes Feld, Strand, Kuppe) messen und nennen.
- **Einheitengrenze** je Spieler aus 143 übernehmen.

## 6. Stufen

**Stufe 1 – Wallhecke und Baustein:** Abschnitt 3: Modell, Regeln, `[[bocage]]`;
eine Ansichtskarte `bocage_test` (800 m × 800 m: zwölf Felder, zwei Hohlwege,
ein Gehöft, ein offenes Feld); Tests: Panzer kommt nicht durch die Hecke, wohl
durch die Durchfahrt; Infanterie kommt durch; Sicht endet an der Hecke;
Sprengladung und schweres Kaliber reißen eine Lücke, durch die ein Panzer passt;
ein Wrack im Hohlweg sperrt ihn.

**Stufe 2 – Gelände und Verkehr:** `data/map_sketches/hedgerow_coast.toml` mit
Küste, Düne, Relief, Bächen, Brücken, Furten, Straßen, Hohlwegen, Starts und
Zielen, erzeugt mit `just gen-sketch hedgerow_coast`.

**Stufe 3 – Heckenland und Orte:** `[[bocage]]`, offene Felder, Dörfer,
Gehöfte, Ruinen, Wald, Strandverteidigung.

**Stufe 4 – Prüfen und Abschluss:**

- `just map-check hedgerow_coast`, `just map-reach hedgerow_coast`: alle Starts
  und Ziele für Fahrzeuge erreichbar; **jedes Feld für Fahrzeuge erreichbar**
  (kein eingeschlossenes Feld); für Infanterie alles Land.
- **Durchfahrten-Probe:** Zahl der Durchfahrten je Feld als Verteilung; Fahrweg
  eines Panzers zwischen zehn Punktepaaren gegen die Luftlinie; mit je einer
  gesperrten Durchfahrt bleibt jedes Ziel erreichbar.
- **Symmetrietest:** unter Spiegelung gleiche Höhen, Gelände, Weggitter,
  Objektklassen und Weglängen.
- **Sichtprüfung:** Sichtfeld-Bilder aus einem Feld, an einer Hecke, an einer
  Durchfahrt, im Hohlweg, vom offenen Feld, von der Abteiruine, aus einem
  Strandbunker.
- **Bildrate, Tick, Wegsuche und Sicht** in jeder Ansicht mit zwölf Spielern
  (`docs/PERFORMANCE.md`): Die Karte hat sehr viele Heckensegmente
  (Größenordnung zehntausende); Objektindex, Sichtstrahlen und Darstellung
  (Instanzen, ferne Vereinfachung) messen und, wenn nötig, Segmente länger
  machen oder die Feldgröße anheben und festhalten.
- **KI-Turnier** 6 gegen 6, 3 gegen 3 und 1 gegen 1, auch mit vertauschten
  Seiten, Ergebnis in `docs/AI.md`: Findet die KI die Durchfahrten, stauen sich
  ihre Panzer, nutzt die Infanterie die Hecken, sprengt sie Lücken, endet es im
  Stillstand? Probleme als offene Punkte; die KI nicht umbauen.
- `just selftest`, `just lan-selftest` (zwölf Plätze), Menüname und Übersetzung,
  Eintrag in der Tabelle der Skizzenkarten in `docs/MAPS.md` mit Zuordnungs-,
  Sicht- und Reichweitentabelle, Test wie bei den anderen Skizzenkarten (zweimal
  erzeugt, gleiche Dateien), `just catalog-doc`, `docs/ARCHITECTURE.md`
  („Destruction“, Wallhecke), `docs/ASSETS.md`, `docs/SOURCES.md`. `just check`.

## 7. Nicht Teil dieses Prompts

- Heckenschneider an Panzern als Ausrüstung (wäre eine neue Regel und ein
  eigener Prompt; hier sprengen nur Pioniere und schwere Kaliber).
- Eine Landungsmission mit Booten (Prompt 119 und eine Mission nach dem Muster
  von 121), Geschützbunker, Landungsstege, Schiffe.
- KI-Verbesserungen für Engstellen und Hinterhalte.
- Eine lagegetreue Karte eines echten Küstenabschnitts nach Geodaten.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
