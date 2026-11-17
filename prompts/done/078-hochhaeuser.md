# Prompt 078 – Hochhäuser: mehrgeschossige Wohnblöcke

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ASSETS.md` den Abschnitt „Environment objects (prompt 007)“ ganz (Aufbau mit
> Schadensstufen, Bauplan-Format, „Materials and shaders“ mit den Absätzen zu Stadt-
> und Wüstenhäusern, Budgets, Prüfungen) und „Thumbnails“, in `docs/ARCHITECTURE.md`
> „Map objects and decoration“, die Abschnitte zu Zerstörung (Prompt 020) und „Vision
> and fog of war“, in `docs/INFANTRY.md` das Besetzen von Gebäuden (`Garrison`), in
> `docs/MAPS.md` Objekte, Stadtgenerator (`gen-town`) und Wüstendorf, dazu
> `docs/SOURCES.md`. Dann den Bestand: `env_house_town` und `env_house_desert_tall` in
> `data/catalog/neutral.toml`, `data/objects.toml`, `data/destruction.toml`,
> `data/infantry.toml` (`[[building.kind]]`) und `data/vision.toml` (`[objects]`),
> ihre Baupläne in `assets-src/blueprints/neutral/`, `tools/blender/kit/buildings.py`
> und `kit/desert.py`.

**Vorlagen:** `.temp/hochhäuser.png` und `.temp/hochhäuser2.png` (nicht im Repo,
`.temp/` ist ignoriert). Zwei Bildschirmfotos aus einem anderen Strategiespiel, etwa
1090 Pixel breit: eine Wüstenstadt mit fünfgeschossigen Wohnblöcken, einmal im
Überblick, einmal aus der Nähe. Liegen die Dateien nicht vor, reicht die Beschreibung
in Abschnitt 2.

## 1. Ziel

Das Spiel bekommt hohe Stadthäuser: Wohnblöcke mit vier bis sechs Geschossen, die
eine Stadt nach Stadt aussehen lassen und um die gekämpft wird. Das höchste Wohnhaus
ist heute das dreigeschossige Stadthaus (`env_house_town`, 14,5 m mit Dach).

Fertig ist der Prompt, wenn

- die Blöcke aus Abschnitt 3 als Umwelt-Objekte mit den Schadensstufen `Intact`,
  `Damaged` und `Ruin` gebaut, geprüft und im Spiel sind,
- ein Straßenzug im Spiel neben den Vorlagen liegt und in Größe, Gliederung, Dach
  und Schadensbild dazu passt,
- die Blöcke alle Regeln eines Gebäudes haben: sie versperren den Weg, verdecken die
  Sicht, halten Geschosse auf, lassen sich besetzen, beschädigen und zerstören,
- Einheiten hinter einem Block auffindbar bleiben und die Kamera nie im Haus steckt,
- sie sich im Karteneditor setzen lassen und ein Stadtviertel zum Ansehen existiert,
- die Bildrate über diesem Viertel gemessen ist,
- `just check`, `just validate-assets`, `just selftest` und `just editor-selftest`
  grün sind.

## 2. Die Vorlagen

Die Bilder sind **Vorbild für Form, Größe und Wirkung**, kein Material: keine
Bildausschnitte und keine Texturen daraus, nichts aus anderen Spielen
(`docs/SOURCES.md`). Die Vorlage spielt in der Gegenwart; dieses Spiel spielt im
Zweiten Weltkrieg. Übernommen wird der Bautyp, nicht die Einzelheiten: nichts, was
es um 1940 nicht gab (Klimageräte, Antennen, Satellitenschüsseln, moderne Autos).
Die Bedienelemente in den Bildern gehören nicht dazu.

| Bild | Zeigt |
|---|---|
| `hochhäuser.png` | Überblick: vier Wohnblöcke mit fünf Geschossen und Flachdach in einer Wüstenstadt, dazwischen breite Sandstraßen, Hofmauern, niedrige Flachdachhäuser und Palmen. Ein Block ist oben aufgerissen: die obersten Geschosse fehlen an einer Ecke, man sieht Zimmerwände und Decken. Lange Schlagschatten über die Straßen. |
| `hochhäuser2.png` | Nahsicht auf einen Block: Lochfassade mit weißen Sprossenfenstern in strengem Raster, flache Betonvordächer über den Eingängen, ockerfarbener Putz mit dunklen Rußfahnen über und unter den Fenstern, ausgebrannte und zerschlagene Fenster, ein Mauerdurchbruch. Auf dem Flachdach eine niedrige Brüstung, Treppenhausaufbauten, schwarze Wassertanks, Lüfter; Soldaten stehen auf dem Dach. |

Was die Blöcke ausmacht:

- **Schlichter Quader**, etwa dreimal so lang wie tief, fünf Geschosse.
- **Strenges Fensterraster** über die ganze Fassade, alle Fenster gleich, an einer
  Seite Balkone.
- **Flachdach als fünfte Ansicht.** Aus der Spielkamera sieht man vor allem das
  Dach: Brüstung, Aufbauten, Tanks, Schornsteine und Lüfter gliedern es.
- **Gebrauchsspuren.** Schmutzfahnen unter den Fensterbänken, Ruß über Fenstern,
  fleckiger Putz.
- **Schaden zeigt das Innere.** Wo Wände fehlen, sieht man Geschossdecken und
  Zwischenwände, kein hohles Gehäuse.
- **Der Schatten.** Ein Block wirft einen Schatten, der eine ganze Straße füllt.

## 3. Die Gebäude

„Hochhaus“ heißt für die Zeit: Wohn- und Geschäftsblöcke mit vier bis sechs
Geschossen, keine Wolkenkratzer. Schlag den Bautyp nach und belege jeden Katalogwert
mit einer Quelle (`docs/SOURCES.md`): Geschosshöhen, übliche Tiefen und Längen,
Dachformen. Ansatzpunkte zum Nachschlagen, nicht ungeprüft übernehmen:
Mietshausblöcke europäischer Großstädte mit fünf Geschossen und fester Traufhöhe,
Wohnblöcke der Zwischenkriegszeit mit Flachdach in den Städten Nordafrikas,
sowjetische Wohnblöcke der Dreißigerjahre, wie sie in Stalingrad umkämpft waren.

Mindestens diese drei, als neue Einträge im Katalog `neutral`; die Namen sind
Vorschläge, halte dich an die Benennung im Bestand:

| Vorschlag id | Was | Größe (Anhalt, mit Quelle festlegen) |
|---|---|---|
| `env_block_desert` | Wohnblock mit Flachdach, vier Geschosse, für Wüstenstädte | etwa 24 × 12 m, 13 bis 14 m |
| `env_block_desert_long` | Langer Wohnblock mit Flachdach, fünf Geschosse, mit Balkonen und zwei Eingängen; der Block der Vorlage | etwa 40 × 13 m, 16 bis 17 m |
| `env_block_town` | Mietshausblock für europäische Städte, fünf Geschosse, Putz oder Ziegel, flaches Walmdach oder Flachdach, Durchfahrt zum Hof | etwa 30 × 13 m, 18 bis 20 m |

Ein Eckblock oder ein sechstes Geschoss nur, wenn der Baukasten es ohne viel
Mehraufwand hergibt.

**Schadensstufen**, wie bei allen Gebäuden:

- `Intact`.
- `Damaged`: Granatlöcher, ausgebrannte Fenster mit Rußfahnen, eine aufgerissene
  Ecke über ein bis zwei Geschosse, in der Decken und Zwischenwände zu sehen sind.
  Nicht größer als `Intact`.
- `Ruin`: die unteren ein bis zwei Geschosse als gezackte Mauerreste mit
  Deckenresten, davor und darin ein Schutthaufen, der zur Masse des Hauses passt.

## 4. Modelle bauen

Mit dem Umwelt-Baukasten (`just env`), als Baupläne, nicht von Hand.

- **Generator.** Prüfe, welcher vorhandene Generator die Blöcke tragen kann: `house`
  (`buildings.py`, Stadthaus) oder `desert_house` (`kit/desert.py`, Raumblöcke mit
  Flachdach, Brüstung und Dachaufbauten). Erweitere ihn um das Nötige: viele
  Geschosse, Fensterraster, Balkone, Vordächer, Dachaufbauten, Innenwände und Decken
  für die Schadensstufen. Ein neuer Generator nur, wenn keiner passt.
- **Dach.** Brüstung, ein bis zwei Treppenhausaufbauten, Wassertanks auf Sockeln,
  Schornsteine, Lüftungshauben. Je Block anders verteilt.
- **Fenster.** Bei mehr als hundert Fenstern je Block entscheidet ihre Bauweise über
  das Budget. Sieh nach, wie die Stadthäuser ihre Fenster bauen, und nimm die
  sparsamste Art, die aus der Spielkamera stimmt.
- **Budget.** Die Zeile „Haus ≤ 3 000, Kirche ≤ 5 000 Dreiecke je Schadensstufe“
  reicht für einen langen Block voraussichtlich nicht. Vorschlag, in
  `docs/ASSETS.md` eintragen und in `validate_env.py` prüfen: Wohnblock ≤ 6 000 je
  Schadensstufe. Bleib darunter, wo es geht, und begründe die Zahl.
- **Oberfläche.** Keine Texturdateien; Putz, Ziegel und Dach kommen aus
  `env.gdshader`. Die Blöcke nutzen die Vielfalt je Haus aus Prompt 056 (Wandfarbe
  aus der Position). Dazu Schmutzfahnen unter den Fensterbänken und Rußfahnen an
  ausgebrannten Fenstern, über die gebackene Vertexfarbe oder ein Muster. Prompt 075
  gibt den Dächern Patina; ist er erledigt, gilt sie auch hier, sonst nichts
  vorwegnehmen.
- **Biome.** Über `EnvLook` wie bisher: Wüste staubig und ocker, Wald und Winter für
  den europäischen Block, Schnee auf Dach und Brüstung.
- **Größenprüfung.** `just env-lineup` stellt die Blöcke neben die 1,80-m-Figur,
  den Panzer IV und das Stadthaus. Türen, Fenster und Geschosshöhen müssen zur Figur
  passen.

## 5. Spielregeln

Die Blöcke sind Gebäude wie das Stadthaus und bekommen dieselben Einträge. Alle
Werte sind Spielregeln, vorläufig und als `[provisional]` markiert.

| Regel | Festlegung |
|---|---|
| Weg (`data/objects.toml`) | `blocks = "all"`, Kasten aus den Katalogmaßen; `Ruin` hält nur Fahrzeuge auf, wie bei den anderen Häusern. |
| Sicht (`data/vision.toml`) | In die Liste `solid`: verdeckt bis zur Kataloghöhe. |
| Zerstörung (`data/destruction.toml`) | Eigene Klasse. Trefferpunkte aus dem Stadthaus (3 500) hochgerechnet nach umbautem Raum, dann gedeckelt, damit ein Block kein unzerstörbarer Bunker wird. `ruin_height_pct` so, dass die Ruine ein bis zwei Geschosse hoch deckt. |
| Besetzen (`data/infantry.toml`) | `[[building.kind]]` mit `capacity` und den Trefferchancen. Die Faustregel „ein Soldat je 10 bis 15 m² Boden“ ergäbe bei fünf Geschossen über hundert Mann; deckle die Zahl auf etwa das Dreifache des Stadthauses und begründe sie. |
| Einsturz | Wie bisher (`collapse_loss_pct`). |

- Sieh nach, ob es weitere Stellen gibt, die Gebäude aufzählen (Brand, KI,
  Deckung, Minikarte, Editor), und trag die Blöcke dort ein.
- **Zur Simulation:** Neue Objektarten sind eine beabsichtigte Regeländerung, wirken
  aber nur dort, wo sie stehen. Der goldene Hash in
  `crates/sim/tests/determinism.rs` bleibt gleich, solange die Karte des Tests keine
  Blöcke bekommt. Ändert er sich, finde zuerst heraus, warum. Karten, an denen Tests
  oder Messreihen hängen (Determinismus, KI-Turnier, Balance, Benchmarks), bekommen
  keine Blöcke.
- Tests in `crates/sim/tests/` neben den vorhandenen: eine Einheit hinter einem
  Block ist nicht zu sehen; ein Trupp besetzt einen Block bis zur Obergrenze; ein
  Block geht über `Damaged` in `Ruin` über und verliert dabei Besatzung.

## 6. Hohe Häuser im Bild

Ein Haus von 17 bis 20 m ist höher als alles, was bisher auf den Karten steht. Prüfe
diese Punkte im Spiel und behebe, was nicht stimmt:

- **Kamera.** Beim nächsten Zoom ist die Kamera etwa 15 m vom Blickpunkt entfernt,
  also unter Dachhöhe. Sie darf nie im Haus stecken oder durch Wände schneiden.
  Sieh nach, wie sie sich heute an Kirche und Rathaus (17 m) verhält.
- **Verdeckte Einheiten.** Hinter einem Block verschwinden Einheiten aus der
  schrägen Sicht. Stell fest, was heute hinter Kirche und Stadthaus von ihnen zu
  sehen ist. Mindestens Auswahlmarken, Lebensbalken und Symbole bleiben sichtbar.
  Reicht das nicht, bau die einfachste Lösung, die hilft (zum Beispiel ein Umriss
  der Einheit durch das Gebäude hindurch), oder notiere den Punkt mit einem
  Vorschlag als offen. Die Kamera lässt sich drehen; das ist kein Ersatz.
- **Schatten.** Die Schatten reichen weit über Straßen und Nachbarhäuser. Prüfe,
  dass sie bei flacher Sonne nicht abreißen oder flimmern und dass die Schattenweite
  je Grafikstufe reicht.
- **Nebel des Krieges, Brand, Schnee.** An hohen Wänden und auf dem Dach wie an
  jedem Haus.
- **Besatzung.** Besetzte Blöcke zeigen ihre Besatzung so, wie besetzte Häuser es
  heute tun. Soldaten auf dem Dach und in einzelnen Fenstern wie in der Vorlage
  sind nicht Teil dieses Prompts (Abschnitt 9).

## 7. Karteneditor und Karten

- Die Blöcke lassen sich im Editor setzen und drehen wie andere Gebäude;
  `just map-check` und `just editor-selftest` kennen sie.
- **Ein Stadtviertel zum Ansehen:** sechs bis zehn Blöcke an zwei oder drei
  Straßen, mit Höfen, Hofmauern, niedrigen Häusern und Bäumen dazwischen, auf einer
  neuen kleinen Karte oder auf einer Karte, an der keine Tests und Messreihen
  hängen. Straßen so breit, dass zwei Panzer aneinander vorbeikommen; die Wegzellen
  sind 4 m groß. Prüfe, dass Infanterie und Fahrzeuge alle Straßen und Höfe
  erreichen.
- Den Stadtgenerator (`cli gen-town`) um ein Blockviertel zu erweitern ist ein
  offener Punkt, außer es geht mit wenig Aufwand.

## 8. Prüfen und Abschluss

1. **Vergleichsbilder.** Das Viertel im Spiel bei ähnlichem Blickwinkel neben den
   beiden Vorlagen: Überblick und Nahsicht. Nach jeder Änderungsrunde neu.
2. **Bildreihe.** Eine Auswahl für `just objects-shots`: das Viertel bei 15, 35, 110
   und 350 m, jede Schadensstufe jedes Blocks, alle passenden Biome, Tag, flache
   Sonne und Nacht, ein brennender Block.
3. **Im Spiel.** Einen Block besetzen, beschießen, bis er `Damaged` und `Ruin` ist;
   Fahrzeuge und Infanterie durch das Viertel schicken; als Bildreihe festhalten.
4. **Bildrate.** `just objects-bench` vorher und nachher, dazu eine Ansicht über
   dem Viertel in jeder Grafikstufe. Dreiecke und Zeichenaufrufe notieren.
5. `just validate-assets`, `just check`, `just selftest`, `just editor-selftest`,
   `just map-check` für jede geänderte Karte.

Die Bilder dem Nutzer zeigen. Was nicht erreicht wurde, steht im Abschlussbericht mit
Grund, nicht als erledigt.

- **Vorschaubilder.** Jeder neue Katalogeintrag braucht eines:
  `just thumbnails --only=<id>,<id>` (braucht ein Fenster). Danach
  `just units-web-sync` und `just catalog-doc`.
- `docs/ASSETS.md` (Generator, Budget, Schadensstufen mit Innerem), `docs/MAPS.md`
  (Objekttabelle, das Viertel), `docs/ARCHITECTURE.md` (Regeln, hohe Häuser im Bild,
  Messwerte, offene Punkte), `docs/INFANTRY.md` (Besetzen), `docs/SOURCES.md`. Auf
  Englisch, ohne die Bilder der Vorlagen.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten (Generator und Modelle, Regeln und
  Tests, Darstellung hoher Häuser, Karte). Kein `git push`.

## 9. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Geschosse als Spielregel:** Soldaten in einzelnen Stockwerken und auf dem Dach,
  Stockwerk-Auswahl, Kampf im Haus, weitere Sicht von oben aus dem Gebäude.
- **Schaden je Geschoss** statt dreier Stufen für das ganze Haus.
- **Wolkenkratzer, Fabriken, Bahnhöfe** und andere Großbauten.
- **Blockviertel im Stadtgenerator** und eine eigene Stadtkarte für Gefechte.
- **Patina an Wänden** über die Fahnen an den Fenstern hinaus.

## Entscheidungen (autonom, Nachtlauf 17.11.2026)

Der Prompt hat keine offenen Fragen; diese Urteile wurden ohne Rückfrage getroffen (vorläufig):

| Frage | Entscheidung |
|---|---|
| Generator | neuer Generator `apartment_block` (`kit/blocks.py`) auf der gemeinsamen `House`-Basis: `house` baut Fenster aus vier Kästen (~80 Dreiecke), `desert_house` ist ein Raumblock-Plan ohne Inneres |
| Fenster | ein Glas-Quad je Fenster mit Sprossenmuster im Shader (UV 102 + u), Laibung und Bank als Einzelflächen, ~15 Dreiecke |
| Fahnen | Schmutz- und Rußfahnen als Plaster-Karten 1 cm vor der Wand (UV 100/101), Muster im Shader |
| Maße | Wüstenblock 24 × 12 × 14,2 m (4 × 2,9 m), langer Block 40 × 13 × 17,1 m (5 × 2,9 m), Mietshaus 30 × 13 × 20 m (EG 3,6 m, 4 × 3,2 m, Traufe 17 m, Sattel ~20°); Höhe = höchster Punkt (Treppenhaus bzw. Schornstein) |
| Budget | 6 000 Dreiecke je Schadensstufe (gebaut 2 600–4 800) |
| Trefferpunkte | nach umbautem Raum vom Stadthaus hochgerechnet, gedeckelt bei 9 000 (Block 6 000) |
| Besatzung | 24 / 36 / 36 (≈ dreimal Stadthaus; Pavlov-Haus ~25–30 Mann) |
| Brand | Klasse Massivbau (Decken und Dach brennen aus, Wände bleiben) |
| Kamera | bleibt 1,5 m über jedem stehenden Gebäude im Umkreis von 2 m (statt Kollision oder Ausblenden) |
| Verdeckte Einheiten | Auswahlringe mit zweitem Durchscheinen-Pass; Umriss durch Gebäude als offener Punkt |
| Viertel | neue Karte `city_blocks` (400 m, Wüste) aus `cli gen-blocks`, elf Blöcke; Stadtgenerator unverändert |
| Stadthaus/Lagerhaus | `env_town_hall` und `env_warehouse` in `map_objects.gd` nachgetragen (vorher 300 m Sichtweite, kein Schatten) |
