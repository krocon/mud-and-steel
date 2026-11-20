# Prompt 157 – Bauten einer Rivierastadt: Fürstenpalast, Hafen mit Molen, Spielbank, Museum am Fels, Stützmauern und Treppen (zerstörbar)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 120 (Mittelmeer-Biom: Pinie, Zypresse, Olive, Gebäudefamilie, Kirche mit
> Campanile, Trockenmauern und Terrassenstufen) und 134 (Großstadthäuser:
> Blockrand-Baukasten, `[[city_block]]`, vorab zerstörte Häuser) sind erledigt
> (`prompts/done/…`). **Empfohlen nach 144** (Fassadenfamilie „Südstadt“,
> Palazzina, Palazzo-Flügel), **139 bzw. 141** (Burg- und Festungsteile:
> Ringmauer, Bastion, Turm, Torbau) und **155** (Kai, Hafenkran, Lagerhallen);
> was davon erledigt ist, wird verwendet, was fehlt, baut dieser Prompt in der
> einfachsten Form selbst. Lies zuerst `CLAUDE.md`, dann diese Prompts,
> `prompts/done/078-hochhaeuser.md` und `prompts/done/020-zerstoerung.md`; in
> `docs/ASSETS.md` Gebäude, Baukasten, Budgets; in `docs/ARCHITECTURE.md`
> „Destruction“, Sicht, Wegfindung an Hängen, Besetzen; `data/destruction.toml`,
> `data/objects.toml`, `docs/SOURCES.md`, `docs/PERFORMANCE.md`; in
> `docs/MAPS.md` „Objects“ und die Hangregeln.
>
> **Vorlagen** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen), in
> `docs/.vorlagen/fuerstenfelsen/`:
> - drei mit KI erzeugte Blätter (`ki_*.webp`): frei, ungenau, voller Bauten von
>   heute; nur für die Wirkung;
> - vier **Luftfotos Dritter** (`foto_*.webp`): der Felsen mit Palast und
>   Altstadt, die beiden Häfen, die Ostküste. **Fotos Dritter sind nur Referenz**
>   (`docs/SOURCES.md`): nichts daraus als Textur verwenden, keine Maße allein
>   aus ihnen ableiten. Sie zeigen den Zustand von heute.
>
> Hier zählen nur die **Bauten**; die Karte baut Prompt 158. **Wunsch des
> Nutzers:** „Der Hafen und der Fürstenpalast sollten besonders detailliert
> nachgebildet werden.“
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben den Vorlagen zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Die Bauten, die eine **Stadt an der Riviera um 1944** ausmachen, als zerstörbare
Kartenobjekte im Mittelmeer-Biom: der Palast auf dem Felsen und der Hafen
ausführlich, der Rest als Bautypen. Maße mit Quelle nach `docs/SOURCES.md`
(Grundrisse aus offenen Geodaten, Höhen und Baujahre aus veröffentlichten
Beschreibungen); nichts aus anderen Spielen; **Stand 1944**: keine Hochhäuser,
keine Bauten und Aufschüttungen der Nachkriegszeit.

## 2. Der Palast (ausführlich)

Ein befestigter Fürstensitz, über Jahrhunderte aus einer Burg gewachsen: aus
Teilen, damit er nach den vorhandenen Regeln betretbar, besetzbar und stückweise
zerstörbar ist.

- **Teile:** Hauptflügel mit Arkadengalerie zum Ehrenhof (zwei Geschosse über
  Sockel, etwa 60 m), Seitenflügel, **Torbau** mit Durchfahrt zum Platz,
  **Uhrturm**, zwei bis drei **mittelalterliche Türme** mit Zinnen (quadratisch,
  um 20–25 m), **Bastionen und Kurtinen** zur Seeseite und zum Hang (geböschte
  Mauern, 8–15 m, mit Brustwehr und Geschützscharten), Kapelle im Hof,
  Nebenbauten, Gartenterrasse mit Mauer.
- **Grundriss:** ein unregelmäßiges Vieleck um den Ehrenhof, nach dem echten
  Grundriss aus offenen Geodaten **vereinfacht** (Kantenlängen auf das Raster
  der Teile gerundet); Gesamtmaß nach Quelle. Keine Innenräume, keine
  Ausstattung, keine Wappen und Hoheitszeichen.
- **Vorplatz:** gepflasterter Platz vor dem Tor mit Brüstung zur Klippe, alten
  Kanonen und Kugelstapeln als Kulisse (Deckung für Infanterie), Wachhäuschen.
- **Regeln:** Flügel und Torbau besetzbar; Türme nicht (Beobachter auf Türmen
  sind ein offener Punkt); Bastionen als offene Plattformen mit Deckung wie eine
  Sandsackstellung und Platz für ein Geschütz; Mauern zäh wie die Burgmauern
  aus 139 (kleine Kaliber tun nichts, eine Bresche öffnet einen Weg für
  Infanterie). Der Ehrenhof ist durch den Torbau befahrbar, wenn die
  Torhaus-Regel aus 134 es trägt.
- Zustände `Intact`, `Damaged`, `Ruin` je Teil. Dreiecke je Teil im Budget von
  078 (`docs/ASSETS.md`).

## 3. Der Hafen (ausführlich)

Ein natürlicher Hafen zwischen Felsen und Stadt, mit zwei Molen geschlossen.

- **Kaimauer** `env_quay` als Segment (gerade, Ecke, Treppe zum Wasser, Rampe
  bzw. Slip), Kante 2–3 m über dem Wasser, mit Pollern und Ringen; die
  Kaifläche ist befahrbar, die Kante für alle gesperrt (niemand fährt ins
  Becken).
- **Mole** `env_mole`: gemauerter Damm ins Meer als Segment (Breite 8–12 m,
  befahrbar bzw. zumindest begehbar; Entscheidung nach der Wegfindung), mit
  Brustwehr zur Seeseite; am Kopf ein **Molenfeuer** `env_harbor_light`
  (gemauerter Rundturm mit Laterne, um 15–20 m). Zwei Molen bilden die
  Hafeneinfahrt.
- **Stege** aus Holz bzw. Stein für Boote (begehbar für Infanterie).
- **Hafenbauten:** Zollhaus bzw. Hafenamt (zwei Geschosse, mit Uhr),
  Bootsschuppen mit Slip, Lagerhäuser (vorhandene), Fischhalle (offene Halle auf
  Stützen), Netzgestelle, Fässer und Kisten als Stapel; ein kleiner Kran
  (Handkran oder der Hafenkran aus 155 in klein).
- **Boote als Kulisse und Deckung:** Fischerboote, Barkassen, ein bis zwei
  Segeljachten der Zeit und ein kleiner Küstenfrachter am Kai (schlichte
  Modelle; sie liegen im Wasser, sperren keinen Landweg, können brennen und
  sinken als Bild). Aufgebockte Boote an Land als Hindernis und Deckung. **Keine
  modernen Jachten.**
- **Uferpromenade** mit Baumreihe (Palmen, Platanen), Geländer, Laternen als
  Kulisse.

## 4. Weitere Bauten

- **Spielbank** `env_casino` aus Teilen: Bau der Belle Époque mit zwei
  Ecktürmchen und grünem Kupferdach über einem Saal, Freitreppe, Terrasse zur
  Seeseite mit Balustrade; etwa 80 m × 50 m; davor ein Platz mit Rondell und
  Garten. Kein bestimmter Bau im Detail, keine Schriftzüge.
- **Grandhotel** `env_grand_hotel`: fünf bis sechs Geschosse, Mittelrisalit,
  Mansarddach mit Kuppeln, Markisen; als Mittel- und Seitenstück (60–100 m).
- **Museum am Fels** `env_cliff_museum`: monumentaler Steinbau mit hoher
  Seefassade, der aus der Klippe wächst (Sockel in der Felswand, etwa 85 m über
  dem Wasser bis zum Dach, zur Stadt hin zwei Geschosse); als Körper mit hohem
  Sockel, der auf eine Klippenkante gesetzt wird.
- **Kathedrale:** die Kirche aus 120 in heller Steinfarbe mit Vierungsturm bzw.
  das Langhaus mit Turmpaar aus 141 in schlichter, romanisierender Form.
- **Altstadthaus am Fels** `env_house_rock`: schmales Haus mit drei bis fünf
  Geschossen, Putz in Ocker, Rosa und Gelb, Fensterläden, flaches Ziegeldach; in
  lückenlosen Zeilen an Gassen von 3–5 m; zwei bis drei Varianten (aus der
  Familie „Südstadt“ aus 144, falls vorhanden).
- **Villa der Riviera** `env_villa_riviera`: zwei bis drei Geschosse, Loggia,
  Turmzimmer, Garten mit Mauer und Palmen (oder die Palazzina aus 144 mit
  anderem Dach).
- **Stützmauern, Treppen, Rampen** (wichtig für die Hänge): **Stützmauer**
  `env_retaining_wall` als Segment in 3, 6 und 9 m Höhe mit Brüstung (sperrt
  den Weg, nicht die Sicht von oben); **Freitreppe** `env_stairs` als
  Geländestück, das **Infanterie begehen kann und Fahrzeuge nicht** (so entstehen
  die Fußwege zwischen den Straßen); **Rampe** als gepflasterter Weg mit Brüstung.
  Prüfen, was die Hangregeln und die Terrassenstufen aus 120 schon können, und
  nur ergänzen, was fehlt.
- **Bahnhof der Küstenbahn:** das Empfangsgebäude aus 141 bzw. ein schlichter
  Bau mit Bahnsteigdach; **Tunnelportale** aus 148 (Bahnvariante).

Alle in `Intact`, `Damaged`, `Ruin` wie 078/134, mit Vorschaubild
(`just thumbnails --only=…`), `just units-web-sync`, `just catalog-doc`.

## 5. Regeln (Simulation)

- Einträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster
  der vorhandenen Bauten (Spielbank, Hotel, Museum wie große Stadthäuser bis zum
  Doppelten; Altstadthaus wie ein Steinhaus; Mole und Kaimauer nicht zerstörbar
  oder sehr zäh, einfachste Lösung).
- **Treppen** als Gelände bzw. Objekt, das nur die Größenklasse der Infanterie
  durchlässt; mit Test.
- **Museum am Fels:** besetzbar; seine Seefassade steht in der Klippe und ist
  von See her nicht zu betreten.
- **Der Golden Hash ändert sich** (neue Einträge in den Regeltabellen);
  gewollter Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit nennen.
  Vergleichslauf auf einer Karte ohne die neuen Bauten zeigt gleiche Ergebnisse
  wie vorher.

## 6. Stufen

**Stufe 1 – Palast und Hafen:** Abschnitt 2 und 3, `Intact`; eine Ansichtskarte
`rock_harbor_test` (etwa 900 m × 700 m im Mittelmeer-Biom: ein Fels von 60 m mit
Palast, Vorplatz und einer Altstadtzeile, darunter ein Hafenbecken mit Kai, zwei
Molen, Booten, Promenade; eine Rampe und eine Treppe auf den Fels); Bilder neben
den Vorlagen.

**Stufe 2 – Übrige Bauten, Zustände, Regeln:** Abschnitt 4 und 5; Tests:
Infanterie über die Treppe, Fahrzeug über die Rampe, Durchfahrt durch den
Torbau, Bresche in der Kurtine, Geschütz auf der Bastion, niemand fährt über
die Kaikante, zweimal gleicher Lauf ergibt gleichen Hash.

**Stufe 3 – Abschluss:** Kamera an Fels, Palast und Museum (verdeckte Einheiten
sichtbar, Regel aus 078), Bildrate über `rock_harbor_test` in jeder Ansicht
(`docs/PERFORMANCE.md`), Gefechtsreihe als Bilder (Sturm auf den Palast,
Beschuss des Hafens), `docs/ASSETS.md`, `docs/MAPS.md`, `docs/ARCHITECTURE.md`,
`docs/SOURCES.md` (jede Maßquelle; die Fotos als „nur Referenz“). `just check`,
`just selftest`.

## 7. Nicht Teil dieses Prompts

- Die Karte (Prompt 158).
- Alles nach 1945: Hochhäuser, aufgeschüttete Stadtviertel, Kongressbauten,
  Strandanlagen, das heutige Stadion, moderne Jachten.
- Innenräume, Kunstwerke, Wappen, Flaggen und Namen des echten Ortes an den
  Bauten.
- Fahrende Schiffe, Kriegsschiffe, Küstenbatterien als eigene Objekte.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
