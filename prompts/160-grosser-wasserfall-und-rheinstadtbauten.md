# Prompt 160 – Großer Wasserfall, Strom mit Gefälle und die Bauten einer Stadt am Hochrhein (zerstörbar)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> in `docs/MAPS.md` „Water flow“, das Kartenformat mit `water_levels`
> (Wasserhöhe je Ort), „Bridges“ und die Karte `valley_fields`
> (Stromschnellen); in `docs/ARCHITECTURE.md` Wasser in der Simulation (tiefes
> und flaches Wasser, Waten, Ertrinken), Darstellung der Wasserflächen
> (`godot/scenes/world/water.gd`, `water_mesh`), Sicht, Wegfindung,
> „Destruction“; `data/terrain.toml`, `data/destruction.toml`,
> `data/objects.toml`, `docs/ASSETS.md` (Gebäude, Baukasten, Budgets),
> `docs/SOURCES.md`, `docs/PERFORMANCE.md`; `prompts/done/078-hochhaeuser.md`,
> `prompts/done/020-zerstoerung.md`. Sind die Prompts 122 (Wasserfall als Bild),
> 149 (Wasserfälle im Bergbach), 139 (Fachwerkhäuser, Burgteile, Aussichtsturm),
> 141 (Festungsteile, Bahnanlagen), 137 (Bahnhofshalle, Schornstein), 155
> (Fabrikhallen) oder 115 (lange Brücken) erledigt, auch deren Ergebnis: alles
> davon wird verwendet und nicht neu gebaut.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/wasserfallstadt/wasserfallstadt.webp`, ein mit KI erzeugtes
> Bild, das eine Stadt am Hochrhein mit ihrem großen Wasserfall zum Thema hat
> (nicht lagegetreu: Es rückt Fall und Altstadt zusammen). Hier zählen der
> **Wasserfall** und die **Bauten**; die Karte baut Prompt 161.
>
> **Wunsch des Nutzers:** „Ich möchte, dass der Fluss und alles andere sehr gut
> nachgebildet wird. Eine liebevoll gestaltete, detailreiche Landschaft mit dem
> Rheinfall, der historischen Altstadt, dem Munot, Weinbergen, Wäldern und
> zahlreichen Brücken – im Stil einer hochwertigen Modelleisenbahnlandschaft.“
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Zweierlei, das die Karte aus Prompt 161 braucht:

1. einen **breiten Strom, der fällt**: Wasser auf verschiedenen Höhen entlang
   des Laufs, Stromschnellen und ein **großer Wasserfall** (rund 150 m breit,
   gut 20 m hoch) mit Felsen mitten im Sturz, Gischt, Becken darunter;
2. die **Bauten** einer alten Stadt am Hochrhein um 1944: Rundfestung auf dem
   Rebhügel, Erkerhäuser, Stadttore und Türme, Schloss über dem Fall,
   Schlösschen im Becken, Weinberg.

## 2. Strom mit Gefälle und Wasserfall

Zuerst feststellen, was es gibt (Wasserhöhe je Ort über `water_levels`,
Stromschnellen in `valley_fields`, Fließrichtung nur als Bild), und dann nur
ergänzen, was fehlt.

- **Wasser auf verschiedenen Höhen:** Ein Fluss hat entlang seiner Mittellinie
  ein **Höhenprofil** (Stützpunkte mit Wasserspiegel): fast eben in den
  ruhigen Strecken, mit kurzen Stufen an Wehren, Schnellen und dem Fall. Der
  Skizzen-Generator und der Geodaten-Importer bekommen dafür ein Feld am Fluss
  (`level_m` je Stützpunkt bzw. aus dem Höhenmodell abgeleitet und geglättet).
  Ufer und Flussbett folgen dem Profil. Bestehende Karten bleiben Bit für Bit
  gleich (ohne Profil ein Spiegel wie bisher).
- **Der Fall als Gelände:** eine **Felsstufe** quer durch den Strom, in zwei bis
  drei Absätzen, mit **zwei bis drei Felsen**, die aus dem Sturz ragen, einem
  weiten **Becken** unterhalb (rund, 250–350 m, tief, mit Strudelzonen am Fuß)
  und einer **Schlucht** bzw. einem engeren Lauf danach. Die Felsstufe ist für
  alle gesperrt; **niemand quert den Fall und niemand fährt hinein**. Oberhalb
  eine Strecke Stromschnellen über Felsbänke.
- **Regeln, so wenig wie möglich:** Der Strom ist tiefes Wasser (für Fahrzeuge
  und Infanterie gesperrt) mit einzelnen flachen Kiesbänken am Ufer.
  Stromschnellen und Fall sind gesperrt wie tiefes Wasser. **Keine Strömung als
  Regel** (nichts treibt ab); Boote (Prompt 119) dürfen Schnellen und Fall
  nicht befahren, falls es sie schon gibt. Ändert sich an den Regeln nichts,
  bleibt der Golden Hash.
- **Darstellung (hier liegt die Arbeit):**
  - die **Fallwand** als eigene Wasserfläche über die Stufe: fließendes,
    weißes Wasser mit laufender Struktur, an den Felsen geteilt; in Absätzen,
    nicht als glatte Rampe;
  - **Gischt** am Fuß (Partikel bzw. Nebelkörper, die mit der Höhe ausdünnen),
    **Schaumteppich** im Becken, der flussab verläuft; Schaumränder an Felsen
    und Schnellen;
  - **Wasserfarbe** grün bis türkis im Becken, dunkler im Strom; Wellenbild
    nach Fließgeschwindigkeit (ruhig oberhalb, rau in den Schnellen);
  - **Ton:** Rauschen des Falls als Ortsklang, der mit der Entfernung abnimmt
    und Gefechtslärm in der Nähe nicht übertönt (Lautstärke begrenzen);
  - **Sicht:** Gischt ist nur Bild und sperrt keine Sicht (eine Sichtsperre wäre
    eine Regel: offener Punkt);
  - in jeder Zoomstufe prüfen: von weit oben ein klares weißes Band, aus der
    Nähe Bewegung.
- **Leistung:** Wasserfall und Gischt in jeder Ansicht messen
  (`docs/PERFORMANCE.md`); Qualitätsstufen (Gischt aus, einfache Fallwand) für
  schwache Rechner.
- **Testkarte `falls_test`** (etwa 900 m × 700 m): ein Strom mit Schnellen, Fall,
  Becken und Schlucht, eine Brücke oberhalb, ein Fels mit Haus mitten im Fall,
  ein Schloss auf der Klippe daneben.

## 3. Bauten

Bautypen bzw. vereinfachte Nachbildungen nach veröffentlichten Maßen; Maße mit
Quelle nach `docs/SOURCES.md` (Grundrisse aus offenen Geodaten), nichts aus
anderen Spielen, keine Wappen und Fahnen. Stand 1944.

- **Rundfestung** `env_round_fort` aus Teilen: ein **Ringbau** von etwa 50 m
  Durchmesser mit dicker Mauer und Geschützscharten, oben eine **offene
  Plattform (Zinne)** mit Brüstung, ein **hoher Rundturm** mit Kegeldach am
  Rand, ein Tor mit Brücke über einen trockenen **Graben**, zwei
  **Flankenmauern**, die den Hang hinunter zur Stadt laufen. Regeln: Ringbau
  und Plattform als starke Stellung für Infanterie und leichte Geschütze (zäh
  wie Festungsmauern, kleine Kaliber tun nichts); Turm nicht besetzbar; Graben
  als Geländestufe.
- **Erkerhaus** `env_house_oriel` und Eckhaus: Altstadthaus mit drei bis fünf
  Geschossen, verputzt und farbig gefasst, mit **Erker** über der Gasse,
  steilem Ziegeldach mit Aufzugsgaube; in lückenlosen Zeilen an Gassen von
  6–10 m; drei bis vier Varianten (auch mit bemalter Fassade als Textur). Als
  Fassadenfamilie der Fachwerk- bzw. Altstadthäuser aus 139/141, falls
  vorhanden.
- **Stadttor mit Turm** `env_town_gate` (Durchfahrt für Fahrzeuge bis zu einer
  Breite nach der Wegfindung, Uhr, Wehrgang) und **Stadtmauerstück**
  `env_town_wall`; ein **Wehrturm** am Fluss.
- **Stadtkirche** mit hohem Turm und **Klosterkirche** mit Kreuzgang: aus der
  vorhandenen Kirche und Mauern bzw. den Teilen aus 141, in romanischer bzw.
  gotischer Form; Kreuzgang als Hofbau.
- **Schloss über dem Fall** `env_cliff_castle`: kleines Schloss mit Torbau,
  Wohnbau, Turm und Ringmauer auf einer Klippe (aus Burgteilen, intakt).
- **Schlösschen im Becken** `env_water_castle`: massiver, turmartiger Bau auf
  einem Felsen im Wasser mit Steg zum Ufer.
- **Weinberg** als Feldart `vineyard` (Rebzeilen an Pfählen in Hangrichtung,
  niedrige Deckung für Infanterie, bremst Fahrzeuge leicht), mit
  **Rebhäuschen** und Trockenmauern an den Terrassen; als Baustein
  `[[vineyard]]` im Skizzen-Generator und als Flächenart im Geodaten-Importer.
- **Am Strom:** Wehr mit Kanal und **Wasserwerk** (Turbinenhaus aus Backstein),
  Lagerhäuser am Ufer (ein altes Salz- bzw. Güterhaus), Bootslände mit
  Weidlingen und einem kleinen Dampfboot als Kulisse.
- **Industrie am Fall und im Seitental:** aus den Fabrikbauten von 155 bzw. den
  vorhandenen Hallen; **Bahnhof** aus 137/141.

Alle Bauten in `Intact`, `Damaged`, `Ruin` wie 078, mit Vorschaubild
(`just thumbnails --only=…`), `just units-web-sync`, `just catalog-doc`.
Einträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster der
vorhandenen Bauten. **Der Golden Hash ändert sich** durch die neuen Einträge in
den Regeltabellen (und die Feldart Weinberg, falls sie neu ist); gewollter
Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit nennen. Vergleichslauf
auf einer Karte ohne die neuen Bauten zeigt gleiche Ergebnisse wie vorher.

## 4. Stufen

**Stufe 1 – Wasser:** Abschnitt 2 mit `falls_test`; Bilder und ein kurzer Film
bzw. Bildfolge des Falls in drei Zoomstufen; Messung.

**Stufe 2 – Bauten:** Abschnitt 3, `Intact`; auf `falls_test` Schloss und
Schlösschen, dazu eine Ansichtskarte `old_town_test` (etwa 700 m × 600 m:
Rundfestung auf einem Rebhügel mit Flankenmauern, darunter eine Altstadtgasse
mit Erkerhäusern, Tor, Kirche, Flussufer mit Wehr); Bilder neben der Vorlage.

**Stufe 3 – Zustände, Regeln, Abschluss:** `Damaged` und `Ruin`; Tests: niemand
quert Fall und Schnellen, Durchfahrt durch das Stadttor, Besetzen von Ringbau
und Plattform, Geschütz auf der Plattform, Weinberg als Deckung, zweimal
gleicher Lauf ergibt gleichen Hash; Bildrate über beiden Testkarten in jeder
Ansicht; `docs/MAPS.md` (Flussprofil, `[[vineyard]]`), `docs/ARCHITECTURE.md`,
`docs/ASSETS.md`, `docs/SOURCES.md`, `docs/PERFORMANCE.md`. `just check`,
`just selftest`.

## 5. Nicht Teil dieses Prompts

- Die Karte (Prompt 161).
- Strömung als Spielregel, Gischt als Sichtsperre, Hochwasser, Eis.
- Boote am Fall, Felsenfahrt, Beleuchtung bei Nacht.
- Bauten nach 1945 (Autobahn, Parkplätze, neue Kraftwerke, moderne Hallen der
  Vorlage).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
