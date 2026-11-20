# Prompt 150 – Karte nach Bildvorlage: Schneepass (Hochgebirge mit Tunneln und Viadukten, zwölf Spieler)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 148 (Tunnel), 143 (sehr große Karten, zwölf Spieler; seine Empfehlung für
> Größe und Einheitengrenze gilt hier) und 115 (lange Brücken, für die Viadukte)
> sind erledigt (`prompts/done/…`). Empfohlen nach 149 (kleine Passkarte
> „Höhenstufen“: Erfahrungen mit Tunneln, Serpentinen, Fußwegen, Kamera im
> Gebirge), 132/133 (Auffahrten entlang einer Weglinie), 139 bzw. 141 (Burg- und
> Festungsteile, Bogenviadukt). Lies zuerst `CLAUDE.md`, dann diese Prompts und
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz, „Bridges“, die neue Größengrenze, die
> Winterkarten und `mountain_pass` (Relief bis 600 m, `--relief mountains`); in
> `docs/ARCHITECTURE.md` Sicht („Seeing downhill“), indirektes Feuer, Wegfindung
> an Hängen, Tunnel; `data/terrain.toml` (`max_grade_pct`, Schnee),
> `data/vision.toml`, `data/scale.toml` (`[range]`), `docs/TEAMS.md`,
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/schneepass/schneepass.webp`. Genau ansehen. Ein mit KI
> erzeugtes, frei erfundenes Bild eines verschneiten Hochgebirges. Der
> Titelkasten nennt 12 Spieler und 4 km × 4 km, das Bild ist aber breiter als
> hoch; der Maßstabsbalken ist unstimmig und wird ignoriert. Der Nutzer dazu:
> **„Tunnel könnte interessant sein.“**
>
> **Dieser Prompt läuft in vier Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Winterkarte `snow_pass` (im Spiel „Schneepass“ / „Snow pass“) für **zwölf
Spieler, 6 gegen 6** (Team West gegen Team Ost; auch kleinere Besetzungen): ein
Hochgebirge, in dem fast alles Fels ist. Befahrbar sind nur ein Talkessel in
der Mitte, die Täler dorthin, Passstraßen in Serpentinen, **vier Tunnel** und
zwei **Viadukte** über die Schlucht eines Bergflusses. Wer von A nach B will,
wählt zwischen Tunnel (schnell, gedeckt, eng, zu sprengen), Pass (langsam,
offen, im Schnee) und Viadukt (schnell, ungeschützt, zu sprengen).

Die Vorlage zeigt:

- **Felsmassive mit Schnee** über die ganze Karte, Nadelwald in den Tälern und
  an den unteren Hängen;
- einen **Talkessel** in der Mitte mit einem **Dorf um eine Kirche** (mit
  Brandruinen);
- einen **Bergfluss** in einer Schlucht: von Nordosten herab am Dorf vorbei
  nach Südwesten, mit Stromschnellen und Wasserfällen;
- zwei **Bogenviadukte** über die Schlucht (nordöstlich und südöstlich des
  Dorfs), eine **Steinbrücke** im Südwesten und kleine Brücken am Rand;
- **Tunnelportale**: zwei im Westmassiv, eines am Nordviadukt, eines im
  Nordosten;
- ein **Kloster bzw. eine Burg** auf dem Nordgipfel, ein **Berghaus** auf dem
  Westberg, eine **Bergstation mit Kuppel** auf dem Ostberg, eine **Festung**
  auf einer Kuppe südlich des Dorfs, ein **Werk mit Schornstein** im Osten;
- **Serpentinenstraßen** über die Sättel;
- **zwölf Lager** auf Lichtungen: sechs im Westen, sechs im Osten; **acht
  Sterne**.

**Stand 1944:** Die Kuppel der Bergstation (Radar bzw. modernes Observatorium)
wird ein **Wetter- und Beobachtungshaus** aus Stein mit Antennenmast. Die
Vorlage ist nur Vorbild für Anordnung und Wirkung.

## 2. Größe und Relief

- **6,0 km × 4,0 km**, wenn Prompt 143 das trägt; sonst die dort empfohlene
  Größe (alle Maße aus Abschnitt 3 im gleichen Verhältnis verkleinern, nicht
  unter 4,0 km × 2,664 km). Biom Winter, Wetter klar.
- **Relief** (Höhen über dem Dorf): Talkessel 0 m, Flussschlucht −30 bis −50 m,
  Seitentäler und Lager +20 bis +80 m, Sättel der Passstraßen +120 bis +160 m,
  Kuppen mit Bauten +100 bis +180 m, Gipfel bis +400 m. **Kamera, Schatten,
  Sicht und Minikarte** bei solchen Höhen zuerst an `mountain_pass` und an der
  Karte aus 149 prüfen und messen; tragen sie 400 m nicht sauber, die Gipfel
  niedriger machen (das Bild der Felswände zählt, nicht die Zahl) und
  festhalten.
- **Befahrbar ist wenig:** Zielgröße 20–30 % der Fläche für Fahrzeuge, etwas
  mehr für Infanterie (`just map-reach` nennt die Anteile). Fels und Steilhang
  sperren von selbst (`max_grade_pct`); tiefer Schnee abseits der Straßen
  bremst.
- **Nicht symmetrisch**, aber beide Seiten bekommen dasselbe Inventar (je ein
  eigener Tunnel, ein Pass, drei Täler, gleich viele Ziele); Ausgleich über
  Fahrzeittabelle und Turnier.

## 3. Aufbau

Richtwerte in Metern für 6 km × 4 km (x nach Osten, y nach Süden, Mitte 0/0),
aus der Vorlage abgegriffen und um zwei Tunnel ergänzt. Die Session übernimmt
die Werte und glättet sie nach den Stufenbildern.

- **Talkessel** um das **Dorf** bei (−110, −100): etwa 1200 m × 800 m ebener
  Boden, Kirche, fünfzehn bis zwanzig Häuser, einige vorab beschädigt (falls
  134 das gebracht hat), Wiesen unter Schnee, Zäune, Heuschober.
- **Bergfluss** (`[[river]]`, 30–50 m breit, schnell, in einer Schlucht mit
  Felsufern, nicht zu durchwaten): Quelle im Nordosten bei (1200, −1100) →
  (880, −600) → (700, −200) → (560, 200) → (300, 600) → (−900, 900) →
  (−1700, 1350) → Südrand (−1900, 2000). Wasserfälle an zwei bis drei Stufen
  (Bild aus 122/146/149, falls vorhanden). Östlich des Dorfs trennt er den
  Kessel vom Ostteil der Karte.
- **Viadukte** (lange Bogenbrücken aus 115 bzw. Bogenviadukt aus 141, hoch über
  der Schlucht, in Teilen sprengbar): **Nordviadukt** bei (960, −470),
  **Südviadukt** bei (730, 260); beide West-Ost (der Fluss wird dort kurz
  Nord-Süd geführt). Dazu die **Steinbrücke** im Südwesten bei (−880, 860)
  (Nord-Süd), eine kleine Brücke bei (−1800, 1460) und eine im Nordosten bei
  (1120, −1040). Fünf Übergänge.
- **Vier Tunnel** (`[[tunnel]]`, Straße, einspurig; je unter einem gesperrten
  Felsmassiv, „eine Ebene je Ort“ aus 148 beachten):
  - **Westtunnel** unter dem Westmassiv: Portal (−1940, −240) ↔ Portal
    (−980, 120), etwa 1000 m; verbindet die westlichen Lager mit dem Talkessel;
  - **Osttunnel** unter dem Ostberg: (1150, −300) ↔ (2100, −500), etwa 970 m;
    verbindet die östlichen Lager mit dem Ostkopf des Nordviadukts;
  - **Nordtunnel** unter dem Klostergipfel: (−350, −800) ↔ (480, −680), etwa
    840 m; führt aus dem Kessel zum Westkopf des Nordviadukts;
  - **Südtunnel** unter der Festungskuppe: (−620, 560) ↔ (420, 640), etwa
    1050 m; führt aus dem Südwesten zum Westkopf des Südviadukts.
  West- und Osttunnel gehören je einer Seite, Nord- und Südtunnel sind
  umkämpft. Für jeden Tunnel gibt es **einen Weg außen herum** (Pass oder
  Tal), der zwei- bis viermal so lange dauert.
- **Passstraßen** (Serpentinen für alle Fahrzeuge, Fahrbahn mindestens 8 m,
  Steigung aus den Regeln abgelesen; Auffahrt entlang einer Weglinie aus
  132/133): **Westpass** über den Sattel bei (−1500, −650) als Umgehung des
  Westtunnels; **Nordpass** vom Lager Nordwest über (−700, −1150) am
  Klostergipfel vorbei in den Kessel; **Ostpass** über (1700, −900) als
  Umgehung des Osttunnels; **Südpass** über (−100, 1150) südlich der
  Festungskuppe vom Südwesten zum Südviadukt. Tiefer Schnee neben der Fahrbahn.
- **Höhenstellungen mit Bauten** (je eine ebene Kuppe mit **zwei Fahrwegen**
  hinauf und einem **Steig für Infanterie**):
  - **Kloster** auf dem Nordgipfel-Vorsprung um (−20, −1470): Kirche, Hofbau,
    Mauer (Burg- bzw. Klosterteile aus 139/141, sonst vorhandene Kirche,
    Steinhäuser, Mauern);
  - **Berghaus** auf dem Westberg um (−1380, −690): zwei bis drei Blockhäuser;
  - **Wetterhaus** auf dem Ostberg um (1590, −840);
  - **Festung** auf der Kuppe südlich des Dorfs um (−110, 720): Ringmauer,
    Bastionen, Geschützterrasse mit Blick auf Dorf und Südviadukt.
- **Werk** im Osten um (1910, 210) in einem Seitental: Hallen, Schornstein
  (aus 137, falls vorhanden), Lagerplatz, Bahnanschluss als Kulisse.
- **Lager** auf Lichtungen in Seitentälern, jedes mit **zwei Ausfahrten**:
  - **West:** (−2400, −1500), (−1380, −1680), (−2700, −540), (−2660, 420),
    (−2420, 1260), (−1360, 1320);
  - **Ost:** (2460, −1600), (1000, −1700), (2680, −580), (2720, 420),
    (2440, 1240), (1020, 1300).
- **Talstraßen** verbinden die Lager jeder Seite untereinander (eine
  Randstraße von Nord nach Süd je Seite) und führen zu Tunneln, Pässen und
  Brücken. **Fußwege** für Infanterie über zwei bis drei Scharten je Seite, die
  kein Fahrzeug nimmt (wie in 149 gelöst).
- **Wald:** Nadelwald in Tälern und an unteren Hängen, Baumgrenze bei etwa
  +150 m; darüber Fels und Schnee.

## 4. Spiel

- **Starts:** die zwölf Lager (60 m eben und frei); gerade Spielernummern Team
  West, ungerade Team Ost, in einer Reihenfolge, bei der kleinere Besetzungen
  gegenüberliegende Lager nutzen; Zuordnung als Tabelle in `docs/MAPS.md`.
- **Versorgungspunkte** (neun): Dorf (−110, −100), umkämpft; Nordviadukt
  (960, −470) und Südviadukt (730, 260), umkämpft; Kloster (−20, −1470);
  Festung (−110, 720); Berghaus (−1380, −690) und Wetterhaus (1590, −840);
  Werk (1910, 210) und als Gegenstück ein **Sägewerk** im Westen bei
  (−1900, 500). Das Dorf liegt westlich des Flusses: Der Osten muss über ein
  Viadukt, hat dafür Werk und Wetterhaus näher an den Viadukten. Ob das trägt,
  zeigt die Fahrzeittabelle; sonst Ziele verschieben.
- **Was die Karte erzwingt** (jede Aussage in Stufe 4 mit Bild oder Test
  belegen):
  - **Tunnel gegen Pass:** Der Tunnel spart Minuten und schützt vor Artillerie
    und Fliegern, lässt aber nur eine Kolonne durch, ist am Ausgang mit einem
    Geschütz zu sperren und mit einer Ladung zu verschütten. Der Pass ist
    breiter, langsam und von den Höhenstellungen einzusehen.
  - **Viadukte** sind die einzigen Fahrwege über die Schlucht auf zwei
    Kilometern; wer beide sprengt, teilt die Karte, bis auf die Übergänge an
    Quelle und Unterlauf.
  - **Höhenstellungen** sehen in Kessel und Täler; Geschütze stehen dahinter am
    Hinterhang.
  - **Infanterie** kommt über Scharten und Steige an Stellen, die kein Fahrzeug
    erreicht.
  - **Abgeschnitten:** Mit allen Tunneln verschüttet und allen Brücken
    gesprengt behält jeder Spieler mindestens zwei erreichbare Ziele (Test).
- **Reichweiten** (indirekt 1/8) als Tabelle in `docs/MAPS.md`: von jeder
  Höhenstellung auf Dorf, Viadukte, Tunnelportale; kein Lager in Reichweite
  eines gegnerischen Lagers.
- **Einheitengrenze** je Spieler aus 143 übernehmen.

## 5. Stufen

**Stufe 1 – Relief:** `data/map_sketches/snow_pass.toml` mit Massiven, Kessel,
Tälern, Schlucht und Fluss, erzeugt mit `just gen-sketch snow_pass`; Höhenbild,
Bild der befahrbaren Fläche (`just map-reach`), Kamera- und Schattenprobe.

**Stufe 2 – Wege:** Talstraßen, Pässe, Tunnel, Viadukte, Brücken, Fußwege;
gleich danach die Fahrprobe (Stufe 4, erster Punkt).

**Stufe 3 – Bauten und Bewuchs:** Dorf, Kloster, Festung, Berghaus, Wetterhaus,
Werk, Sägewerk, Lager, Wald, Wasserfälle, Starts und Ziele.

**Stufe 4 – Prüfen und Abschluss:**

- **Fahrprobe, als Test:** der schwerste Panzer und ein Zugmittel mit gezogenem
  Geschütz über jeden Pass, durch jeden Tunnel (sofern das Profil passt; sonst
  über den Pass, auch das als Test), über jedes Viadukt, auf jede
  Höhenstellung über beide Fahrwege; Fahrzeit Tunnel gegen Umgehung je Tunnel
  nebeneinander; Infanterie über jede Scharte. Zeiten in `docs/MAPS.md`.
- `just map-check snow_pass`, `just map-reach snow_pass` mit offenen und mit
  verschütteten Tunneln.
- **Ausgleich:** Fahrzeit von jedem Lager zum nächsten Ziel, zum Dorf und zum
  nächsten Viadukt; Summe je Team höchstens 15 % verschieden, sonst Wege oder
  Ziele verschieben.
- **Sichtprüfung:** Sichtfeld-Bilder von jeder Höhenstellung, vom Dorfplatz,
  von beiden Viadukten, vor jedem Tunnelportal, von einem Sattel.
- **Kamera und Darstellung:** Einheiten in tiefen Tälern und hinter Graten
  bleiben sichtbar und anwählbar, Marken der Einheiten im Tunnel lesbar auf
  Schnee; Bilder in jeder Zoomstufe.
- **Bildrate, Tick, Wegsuche und Sicht** in jeder Ansicht mit zwölf Spielern
  (`docs/PERFORMANCE.md`); die Sicht nach unten kostet mehr Strahlenschritte:
  Sichtbudget je Tick prüfen.
- **KI-Turnier** 6 gegen 6, 3 gegen 3 und 1 gegen 1, auch mit vertauschten
  Seiten (Ziel höchstens 60 % für eine Seite), Ergebnis in `docs/AI.md`: Nimmt
  die KI Tunnel und Pässe, staut sie sich in der Röhre, kommt sie über die
  Viadukte, endet es im Stillstand? Probleme als offene Punkte; die KI nicht
  umbauen.
- `just selftest`, `just lan-selftest` (zwölf Plätze), Menüname und Übersetzung,
  Eintrag in der Tabelle der Skizzenkarten in `docs/MAPS.md`, Test wie bei den
  anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien). `just check`. Der
  Golden Hash ändert sich nicht (neue Karte; die Regeln kamen mit 148); wenn
  doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Lawinen, Schneesturm, Glätte, Kälte als Regel; Seilbahnen und Bergbahnen.
- Bahntunnel mit Zügen, Kreuzungen im Berg, Straßen über Tunneln (wenn 148 sie
  nicht trägt).
- KI-Verbesserungen für Tunnel, Engstellen und Höhenstellungen.
- Eine lagegetreue Karte eines echten Passes nach Geodaten.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
