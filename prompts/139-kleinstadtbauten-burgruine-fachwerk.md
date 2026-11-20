# Prompt 139 – Kleinstadtbauten im Mittelgebirge: Burgruine, Fachwerkhäuser, Villen, Kurhaus, Aussichtsturm (zerstörbar)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/020-zerstoerung.md`, `prompts/done/078-hochhaeuser.md` (Zustände
> `Intact` / `Damaged` / `Ruin`, Regeln für Gebäude) und den Prompt, der die
> Stadthäuser brachte (`env_house_town`, in `docs/ASSETS.md` „Town and winter
> buildings“, Prompt 041); in `docs/ASSETS.md` Gebäude, Baukasten
> (`kit/buildings.py`, Generator `house`), Budgets; in `docs/ARCHITECTURE.md`
> „Destruction“, Sicht, Wegfindung, Besetzen; `data/destruction.toml`,
> `data/objects.toml`, `docs/SOURCES.md`, `docs/PERFORMANCE.md`; in
> `docs/MAPS.md` die Skizzen-Tabellen (`[[town]]`, `[[village]]`, `[[row]]`,
> `[[building]]`). Sind die Prompts 127 (Burgfelsen: vereinfachte Burg aus
> mehreren Objekten), 134 (Zusammenstellungen im Generator, vorab zerstörte
> Häuser) oder 137 (Fallregel für hohe Bauten) erledigt, auch deren Ergebnis;
> Bauteile und Regeln wiederverwenden statt neu bauen.
>
> **Vorlage** (nicht im Repo, nur lesen):
> `docs/.vorlagen/taunusfieber/taunusfieber.webp`, ein mit KI erzeugtes Bild
> einer Kleinstadt im Mittelgebirge. Hier zählen nur die **Bauten**: eine große
> Burgruine auf einem Felshügel, ein dichter Altstadtkern mit roten und grauen
> Dächern, lockere Viertel aus Einzelhäusern mit Gärten, ein Turm auf dem
> höchsten Berg. Die Karte baut Prompt 140.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Die Bauten einer **deutschen Kleinstadt und Kurstadt im Mittelgebirge um 1944**,
als zerstörbare Kartenobjekte: eine Burgruine aus Teilen, Fachwerkhäuser für den
Altstadtkern, Villen für die Hangviertel, ein Kurhaus und ein steinerner
Aussichtsturm. Vorbilder sind **Bautypen**, keine bestimmten Gebäude; Maße als
Bautyp mit Quelle nach `docs/SOURCES.md`, nichts aus anderen Spielen.

## 2. Bauten

- **Burgruine** aus Teilen (alle aus Bruchstein, oben ausgebrochen, bewachsen):
  - `env_ruin_wall`: Ringmauer-Stück, etwa 20 m lang, 2 m dick, 6–9 m hoch, mit
    Breschen in zwei Varianten;
  - `env_ruin_bastion`: Rundbastion bzw. Rondell, etwa 18 m Durchmesser, 10 m
    hoch, oben offen (Stellung für Infanterie und ein leichtes Geschütz);
  - `env_ruin_keep`: Bergfried, rund oder quadratisch, etwa 10 m breit, 25–30 m
    hoch, ohne Dach;
  - `env_ruin_palas`: Palas als leere Hülle, drei Geschosse hohe Außenmauern
    mit Fensterhöhlen, ohne Dach und Decken, innen begehbar;
  - `env_ruin_gate`: Torbau mit Durchfahrt (für Fahrzeuge bis zu einer Breite,
    die die Session nach der Wegfindung festlegt).
  Ist die Burg aus Prompt 127 schon gebaut, deren Teile verwenden und nur die
  Ruinenvarianten ergänzen.
- **Fachwerkhaus** `env_house_timber` und **Fachwerk-Eckhaus**
  `env_house_timber_corner`: zwei bis drei Geschosse, Erdgeschoss aus Stein,
  darüber Fachwerk mit hellen Gefachen, steiles Dach mit Schiefer oder roten
  Ziegeln, Giebel oder Traufe zur Straße, in Zeilen aufreihbar; zwei bis drei
  Varianten (Farbe des Holzes, Vorkragung, Laden im Erdgeschoss). Fachwerk als
  Textur, nicht als Geometrie.
- **Villa** `env_villa`: frei stehendes Wohnhaus der Zeit um 1900, zwei
  Geschosse plus ausgebautes Dach, Erker oder Turmerker, Veranda, Schieferdach;
  zwei Varianten (Putz hell, Backstein). Dazu vorhandene Mauern, Zäune und
  Hecken als Gartengrenze.
- **Kurhaus** `env_spa_house`: langer, zwei- bis dreigeschossiger Bau mit
  Mittelteil und zwei Flügeln, Veranda bzw. Wandelgang zur Gartenseite, etwa
  70 m × 18 m; dient auch als Hotel, Sanatorium oder Schule (**ohne Rotes
  Kreuz**).
- **Aussichtsturm** `env_lookout_tower`: steinerner Turm mit Zinnenkranz oder
  Haube, um 25–30 m hoch, etwa 7 m breit, wie sie um 1900 auf vielen Gipfeln
  gebaut wurden. Die Vorlage zeigt einen rot-weißen Sendemast; den gab es 1944
  so nicht, er wird durch diesen Turm ersetzt.

Häuser, Villa, Kurhaus und Turm in `Intact`, `Damaged`, `Ruin` wie die
vorhandenen Häuser. Die Burgteile sind schon Ruinen: `Intact` ist der heutige
Zustand, `Damaged` mit frischen Breschen, `Ruin` ein Schuttwall. Dreiecke je Bau
im Budget der vorhandenen Häuser (`docs/ASSETS.md`).

## 3. Regeln (Simulation)

- Einträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster der
  Häuser: Fachwerkhaus etwas schwächer als das Steinhaus und **brennt leicht**
  (Brandregel der Gebäude, höchste vorhandene Stufe), Villa wie das
  Backsteinhaus, Kurhaus wie ein großes Stadthaus.
- **Burgmauern sind zäh:** Ringmauer, Bastion, Bergfried mit sehr vielen
  Trefferpunkten (Richtgröße: ein Mauerstück wie eine Kirche, Bergfried das
  Doppelte); kleine Kaliber tun nichts (Schwelle wie bei Bunkern prüfen und die
  einfachste Regel wählen). Eine Bresche öffnet einen Weg für Infanterie, ein
  Schuttwall bleibt für Fahrzeuge gesperrt.
- **Besetzbar:** Palas-Hülle, Bastion (offen, Deckung wie eine Sandsackstellung
  mit Höhe), Kurhaus, Häuser, Villa. Nicht besetzbar: Bergfried, Aussichtsturm
  (Beobachter auf Türmen sind ein offener Punkt). Mauern geben Deckung und
  sperren Sicht und Weg.
- **Bergfried und Aussichtsturm stürzen um**, wenn Prompt 137 bzw. 135 die
  Fallregel gebracht hat (nur in die Tabelle eintragen); sonst werden sie zur
  Ruine an Ort und Stelle wie der Kirchturm.
- **Der Golden Hash ändert sich** (neue Einträge in den Regeltabellen); gewollter
  Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit nennen. Vergleichslauf
  auf einer Karte ohne die neuen Bauten zeigt gleiche Ergebnisse wie vorher.

## 4. Zusammenstellungen im Skizzen-Generator

In `docs/MAPS.md` beschreiben; bestehende Skizzenkarten bleiben Bit für Bit
gleich.

- **`[[castle_ruin]]`**: Umriss der Ringmauer als Polygon, daraus Mauerstücke
  mit Breschen, Bastionen an den Ecken, Torbau an einer benannten Kante,
  Bergfried und Palas an benannten Punkten, Hof mit Gras und Schutt. Oder als
  `[[building]]`-Listen, wenn das einfacher ist.
- **Altstadt:** `[[town]]` bzw. `[[row]]` mit Fachwerkhäusern als wählbarer
  Hausfamilie (Zeilen ohne Lücken, enge Gassen).
- **Villenviertel:** ein Schalter oder eine Tabelle, die an einer Straße
  Grundstücke von etwa 25 m × 40 m abteilt und je ein Haus mit Garten, Zaun oder
  Hecke und ein bis zwei Bäumen setzt (die Vorlage besteht zu drei Vierteln
  daraus).

## 5. Stufen

**Stufe 1 – Modelle:** alle Bauten `Intact`; eine Ansichtskarte `spa_town_test`
(etwa 800 m × 600 m: Burgruine auf einer Kuppe, eine Altstadtgasse mit Platz und
Kirche, ein Villenhang, Kurhaus mit Garten, Aussichtsturm); Bilder neben der
Vorlage.

**Stufe 2 – Zustände, Regeln, Generator:** Abschnitt 3 und 4; Tests: Bresche
öffnet Weg für Infanterie, Schuttwall sperrt Fahrzeuge, Fachwerk brennt,
Besetzen von Palas und Bastion, Durchfahrt durch den Torbau, zweimal gleicher
Lauf ergibt gleichen Hash.

**Stufe 3 – Abschluss:** Bildrate über `spa_town_test` und hochgerechnet auf
eine Stadt von 700–900 Häusern (Prompt 140) in jeder Ansicht
(`docs/PERFORMANCE.md`), Gefechtsreihe als Bilder (Sturm auf die Burgruine),
Vorschaubilder für jedes neue Modell (`just thumbnails --only=…`),
`just units-web-sync`, `just catalog-doc`, `docs/ASSETS.md`, `docs/MAPS.md`,
`docs/ARCHITECTURE.md`, `docs/SOURCES.md`. `just check`, `just selftest`.

## 6. Nicht Teil dieses Prompts

- Die Karte (Prompt 140).
- Eine bestimmte Burg lagegetreu; unterirdische Gänge, Kasematten, Keller.
- Sendemast, Sportplatz, Gewerbehallen und Kreisverkehre der Vorlage (nach 1945).
- Innenräume über das sichtbare Innere der Ruinen hinaus.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
