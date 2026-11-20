# Prompt 144 – Bauten einer südlichen Großstadt: Stadion, Palazzi, Mietshausblöcke, Hallen, Sportanlagen (zerstörbar)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 120 (Mittelmeer-Biom: Pinie, Zypresse, Gebäudefamilie, Kirche mit Campanile)
> und 134 (Großstadthäuser: Blockrand-Baukasten, `[[city_block]]`, vorab
> zerstörte Häuser) sind erledigt (`prompts/done/120-*`, `prompts/done/134-*`).
> Lies zuerst `CLAUDE.md`, dann diese Prompts, `prompts/done/078-hochhaeuser.md`
> und `prompts/done/020-zerstoerung.md`; in `docs/ASSETS.md` Gebäude, Baukasten,
> Budgets; in `docs/ARCHITECTURE.md` „Destruction“, Sicht, Wegfindung, Besetzen;
> `data/destruction.toml`, `data/objects.toml`, `docs/SOURCES.md`,
> `docs/PERFORMANCE.md`; in `docs/MAPS.md` die Skizzen-Tabellen. Sind 137 oder
> 141 (weitere Großstadtbauten) erledigt, deren Bauteile wiederverwenden.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/ockerstadt/ockerstadt.webp`,
> ein mit KI erzeugtes Bild eines Stadtviertels in einer südlichen Hauptstadt.
> Hier zählen nur die **Bauten**: ein großes ovales Stadion, ein kleines
> Stadion, große Hofbauten, Blöcke in Ocker und Terrakotta mit flachen
> Ziegeldächern, dichte Viertel kleiner Wohnhäuser, Hallen, Sportplätze. Die
> Karte baut Prompt 145.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Die Bauten einer **südeuropäischen Großstadt um 1944**, als zerstörbare
Kartenobjekte im Mittelmeer-Biom. Vorbilder sind **Bautypen**, keine bestimmten
Gebäude; neutrale Namen im Spiel; Maße als Bautyp mit Quelle nach
`docs/SOURCES.md`, nichts aus anderen Spielen. **Keine politischen Zeichen** der
Zeit an den Bauten (keine Rutenbündel, keine Inschriften, keine Bildnisse).

## 2. Bauten

- **Stadion** aus Teilen `env_stadium_stand` (gerades Tribünenstück, etwa 30 m
  lang, 20 m tief, 10–14 m hoch, offene Stufen aus Beton ohne Dach) und
  `env_stadium_curve` (Kurvenstück), aneinandergereiht zu einem Oval von etwa
  250 m × 190 m um ein Spielfeld mit Laufbahn; dazu ein **Torbau** mit
  Durchfahrt. Bautyp der 1920er/1930er Jahre, **nicht** das überdachte moderne
  Stadion der Vorlage. Das kleine Stadion entsteht aus denselben Teilen (eine
  Tribüne, niedrige Wälle).
- **Mietshausblock „Südstadt“:** eine Fassadenfamilie für die Haustypen aus 134
  (gerade, Eck-, Keil-, Torhaus): vier bis sechs Geschosse, Putz in Ocker,
  Terrakotta, Gelb und Altrosa, Fensterläden, flach geneigte Dächer mit
  Mönch-und-Nonne-Ziegeln oder Dachterrassen, Läden und Werkstätten im
  Erdgeschoss, Balkone mit Eisengittern.
- **Palazzina** `env_palazzina`: frei stehendes Wohnhaus mit drei bis vier
  Geschossen und vier bis acht Wohnungen, etwa 16 m × 14 m, Garten mit Mauer; zwei
  bis drei Varianten. Das ist das Haus der dichten Wohnviertel der Vorlage.
- **Palazzo** `env_palazzo_wing` und Eckstück: drei- bis viergeschossige Flügel
  mit Rustika-Sockel und Kranzgesims, die einen Hof umschließen (Ministerium,
  Kaserne, Schule, Kloster); Hof mit Brunnen oder Bäumen.
- **Halle** `env_hall_concrete`: große Halle der 1930er Jahre (Betonbinder oder
  Bogendach, etwa 80 m × 40 m), als Depot, Markthalle, Fahrzeughalle.
- **Sportanlagen:** Spielfeld, Laufbahn und Tennisplätze als Bodenarten bzw.
  Flächen mit niedrigen Zäunen (überfahrbar); ein Umkleidebau aus vorhandenen
  kleinen Häusern.
- **Kleinbauten der Straße**, soweit sie einfach sind: Brunnenbecken für
  Rondelle, Kiosk, Mauer mit Tor für Gärten und Parks (vorhandene Mauern
  verwenden, wo sie passen).

Alle Bauten in `Intact`, `Damaged`, `Ruin` mit sichtbarem Inneren wie in
078/134. Stadion: beschädigte Tribüne mit eingestürzten Stufen, Ruine als
Schuttwall. Dreiecke je Bau im Budget von 078 (`docs/ASSETS.md`). Fenster,
Läden und Gitter im Shader bzw. als Textur.

## 3. Regeln (Simulation)

- Einträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster der
  Gebäude: Mietshausblock wie das Blockrandhaus aus 134, Palazzina wie ein
  Steinhaus mit mehr Trefferpunkten, Palazzo-Flügel wie ein Blockrandhaus oder
  mehr, Halle schwächer und brennbar, Tribünenstück zäh (massiver Beton).
- **Besetzbar:** Blöcke, Palazzina, Palazzo, Halle. **Tribünen** sind nicht
  besetzbar, aber begehbar gedacht: einfach halten und sie als Mauer mit Höhe
  behandeln (sperren Weg und Sicht, geben Deckung); das **Spielfeld** ist
  offenes Gelände, erreichbar durch den Torbau und durch zerstörte
  Tribünenstücke. Soldaten auf den Rängen wären eine neue Regelart (offener
  Punkt).
- **Der Golden Hash ändert sich** (neue Einträge in den Regeltabellen); gewollter
  Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit nennen. Vergleichslauf
  auf einer Karte ohne die neuen Bauten zeigt gleiche Ergebnisse wie vorher.

## 4. Zusammenstellungen im Skizzen-Generator

In `docs/MAPS.md` beschreiben; bestehende Skizzenkarten bleiben Bit für Bit
gleich.

- **`[[stadium]]`:** Mittelpunkt, Länge, Breite, Drehung in 90°-Schritten, Lage
  der Tore; setzt Tribünenoval, Spielfeld, Laufbahn, Vorplatz.
- **Palazzo** als Rechteck aus Flügeln mit Hof (wie der Hofbau aus 134).
- **Palazzine-Viertel:** ein Schalter oder eine Tabelle, die Blöcke zwischen
  Straßen in Grundstücke von etwa 22 m × 26 m teilt und je eine Palazzina mit
  Gartenmauer, einer Pinie oder Zypresse und einem Tor setzt; Dichte wählbar.
  Für viele hundert Häuser gedacht: auf die Bildrate achten (Stufe 3).
- **Sportfeld-Gruppe:** Rechtecke mit Feldart und Zaun.
- `[[city_block]]` mit der Fassadenfamilie „Südstadt“.

## 5. Stufen

**Stufe 1 – Modelle:** alle Bauten `Intact`; eine Ansichtskarte
`south_city_test` (etwa 1000 m × 800 m im Mittelmeer-Biom: Stadion mit Vorplatz,
ein Palazzo, sechs Mietshausblöcke an einer Allee mit Pinien, ein
Palazzine-Viertel, eine Halle, Sportplätze); Bilder neben der Vorlage.

**Stufe 2 – Zustände, Regeln, Generator:** Abschnitt 3 und 4; Tests: Weg durch
den Torbau aufs Spielfeld, Bresche in der Tribüne öffnet einen Weg, Besetzen,
Zerstörung in Stufen, zweimal gleicher Lauf ergibt gleichen Hash.

**Stufe 3 – Abschluss:** **Bildrate** über `south_city_test` in jeder Ansicht
und hochgerechnet auf die Stadt aus Prompt 145 (etwa 250 Blöcke und 1500
Palazzine); reicht das Budget nicht, Palazzine zu Zeilen zusammenfassen oder
ferne Häuser vereinfachen und festhalten (`docs/PERFORMANCE.md`). Gefechtsreihe
als Bilder (Kampf um das Stadion), Vorschaubilder für jedes neue Modell
(`just thumbnails --only=…`), `just units-web-sync`, `just catalog-doc`,
`docs/ASSETS.md`, `docs/MAPS.md`, `docs/ARCHITECTURE.md`, `docs/SOURCES.md`.
`just check`, `just selftest`.

## 6. Nicht Teil dieses Prompts

- Die Karte (Prompt 145) und die Technik für sehr große Karten (Prompt 143).
- Antike Bauten (Amphitheater, Tempel, Bögen), Kuppelkirchen, Obelisken.
- Hochstraßen und Kleeblattkreuzungen der Vorlage (nach 1945).
- Soldaten auf Tribünenrängen, Innenräume.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
