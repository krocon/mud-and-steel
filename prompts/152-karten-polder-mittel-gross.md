# Prompt 152 – Polderkarten mittel und groß (acht und zwölf Spieler)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Die Prompts
> 151 (Polder-Bausteine `[[canal]]`, `[[dike]]`, `[[polder]]`, Windmühle,
> Giebelhaus, Polderhof, Schleuse, Wehr, Brückenregeln, Karte `polder_small`)
> und 143 (mehr als sechs Plätze in der Lobby, zwölf Spieler; seine Empfehlung
> für die Einheitengrenze gilt hier) sind erledigt (`prompts/done/…`).
> Empfohlen nach 115 (lange Brücken) und 137 (Schornstein, Fabrikbauten). Lies
> zuerst `CLAUDE.md`, dann `prompts/done/151-*` ganz (Entscheidungen, was die
> kleine Karte gezeigt hat), `prompts/done/088-karten-nach-bildvorlagen.md`, in
> `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ mit den
> Polder-Bausteinen, „Bridges“, `docs/TEAMS.md`,
> `data/map_sketches/polder_small.toml`, `data/scale.toml` (`[range]`),
> `docs/AI.md`, `docs/PERFORMANCE.md`.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/polder/polder.webp`,
> das Blatt mit den drei Varianten; hier zählen die **große** (links) und die
> **mittlere** (rechts oben). Die Maßstabsbalken werden ignoriert, es gelten die
> Größen des Nutzers.
>
> **Vorgaben des Nutzers:** Groß – 4 × 4 km, 12 Spieler: große Panzerverbände,
> mehrere Fronten und weitläufige Deichsysteme. Mittel – 2 × 2 km, 8 Spieler:
> dichte Gefechte um Brücken, Dörfer und Kanalübergänge. Elemente wie in 151
> (Deiche, Brücken, Schleusen und Wehre, Windmühlen und Höfe, offene Felder,
> Kanäle). „Besonders interessant wäre eine Mechanik, bei der zerstörte Brücken
> den Vormarsch ganzer Panzerverbände unterbrechen.“
>
> **Dieser Prompt läuft in drei Stufen** (mittlere Karte, große Karte, Prüfen).
> Nach jeder Stufe Bilder aus dem Spiel neben der Vorlage zeigen und ohne Halt
> weiterarbeiten.

## 1. Gemeinsames

- Beide Karten sind **spiegelsymmetrisch** an der Nord-Süd-Achse (West gegen
  Ost), (x, y) → (−x, y), mit einem **Hauptkanal auf der Achse**, der sich um
  eine **Stadt auf einer Insel** in der Mitte teilt. Alle Wahrzeichen liegen auf
  der Achse oder paarweise. Biom `forest`, Wetter klar, Land eben (Polder
  0,5–1,5 m unter dem Kanalspiegel, Deiche 3–5 m).
- **Querkanäle** teilen jede Seite in mehrere Polder. Jeder Polder ist für sich
  eine Insel für Fahrzeuge: Verbündete sind untereinander nur über Brücken
  verbunden. So entstehen **mehrere Fronten**, und eine gesprengte Brücke
  trennt nicht nur den Gegner, sondern auch die eigenen Leute.
- **Brückenarten** (aus 151): Steinbrücken für alle Fahrzeuge, Holzbrücken mit
  Lastgrenze über Nebenkanäle, Schleusen und Wehre als schmale Notwege mit
  niedriger Lastgrenze. Über den Hauptkanal mit 115 lange Brücken, sonst
  verengt der Generator das Wasser an den Brücken (als Entscheidung
  festhalten).
- **Regel für jeden Polder:** mindestens **zwei Übergänge für schwere Panzer**
  in verschiedene Richtungen und mindestens ein Notweg (Schleuse, Wehr oder
  Holzbrücke). Kein Spieler ist mit einer einzigen Sprengung eingeschlossen.
  Das als Test (Abschnitt 4).
- **Deiche** an allen Hauptkanälen mit Weg auf der Krone, dazu **Binnendeiche**
  quer durch das Land als vorbereitete Linien; Polder mit langen Feldstreifen,
  Gräben, Höfen, Mühlen, Baumreihen an Straßen, Schilf in nassen Ecken. Kein
  Wald außer kleinen Gehölzen an Höfen.
- Koordinaten in Metern (x nach Osten, y nach Süden, Mitte 0/0); beschrieben
  ist die **Westhälfte** und die Achse, die Osthälfte ist das Spiegelbild. Die
  Session übernimmt die Werte und glättet sie nach den Stufenbildern.

## 2. Mittlere Karte `polder_medium` („Polder (mittel)“ / „Polder (medium)“)

**2,0 km × 2,0 km, acht Spieler, 4 gegen 4** (auch 3 gegen 3, 2 gegen 2).
Dichte Gefechte um Brücken, Dörfer und Kanalübergänge.

- **Stadt auf der Insel:** rund, etwa 560 m Durchmesser, Kirche und Markt bei
  (0, 0), Giebelhäuser in Ringen und Radialgassen (sechzig bis achtzig Häuser),
  Kai ringsum. **Ringkanal** 35 m breit.
- **Hauptkanal** auf der Achse vom Nordrand zum Ring und vom Ring zum Südrand,
  40 m breit.
- **Querkanal West** (20 m) vom Ring bei (−300, 0) zum Westrand (−1000, 60):
  teilt die Westseite in **Nordpolder** und **Südpolder**.
- **Übergänge** (Westseite und Achse; zusammen vierzehn):
  - Insel: **Nordwestbrücke** (−295, −140) vom Nordpolder und
    **Südwestbrücke** (−295, 140) vom Südpolder auf die Insel (Stein);
  - Hauptkanal: **Nordbrücke** (0, −520) und **Südbrücke** (0, 520) (Stein, für
    alle), **Schleuse** (0, −850) und **Wehr** (0, 850) (Notwege);
  - Querkanal West: **Steinbrücke** bei (−520, 20) und **Holzbrücke** bei
    (−820, 50).
- **Dörfer:** je Seite ein Dorf im Nordpolder um (−560, −480) und eines im
  Südpolder um (−560, 500), je acht bis zwölf Häuser mit Kirche bzw. Kapelle;
  drei **Polderhöfe** und zwei **Windmühlen** je Seite (eine am Querkanal bei
  (−700, −60), eine am Hauptkanal bei (−120, 700)).
- **Deiche:** an Hauptkanal, Ring und Querkanal; ein **Binnendeich** von
  (−900, −300) nach (−350, −320) und gespiegelt im Süden.
- **Offene Felder:** je Polder ein großer Schlag ohne Gräben (etwa 400 m ×
  300 m) für Panzer.
- **Starts** (60 m eben und frei): Team West (−780, −780), (−860, −280),
  (−860, 300), (−780, 780); Team Ost gespiegelt. Zwei Lager je Polder. Gerade
  Spielernummern Team West, ungerade Team Ost; Reihenfolge so, dass kleinere
  Besetzungen gespiegelte Lager nutzen.
- **Versorgungspunkte** (neun): Markt (0, 0), umkämpft; Nordbrücke (0, −520)
  und Südbrücke (0, 520), umkämpft; Schleuse (0, −850) und Wehr (0, 850),
  umkämpft; Dorf Nord (−560, −480) und Dorf Süd (−560, 500), jeweils gespiegelt.

## 3. Große Karte `polder_large` („Polder (groß)“ / „Polder (large)“)

**4,0 km × 4,0 km** (die heutige Höchstgröße; sie braucht aus 143 nur die zwölf
Plätze), **zwölf Spieler, 6 gegen 6** (auch 4 × 3 und kleinere Besetzungen).
Große Panzerverbände, mehrere Fronten, weitläufige Deichsysteme.

- **Festungsstadt** in der Mitte: eine Stadt mit **Erdwällen und Bastionen im
  Sternform** und breitem **Festungsgraben**, wie die Vorlage sie zeigt. Die
  Wälle sind Deiche (`[[dike]]`, 5–6 m, begrünt, Wallweg auf der Krone), die
  sechs Bastionen ebene Plattformen für Geschütze; Durchmesser über die
  Bastionsspitzen etwa 900 m, Graben 40–50 m. Innen Markt und Kirche bei
  (0, 0), hundertzwanzig bis hundertsechzig Giebelhäuser in Zeilen, zwei
  Kasernenbauten, ein kleiner Hafen am Graben. **Zwei Tore** für Fahrzeuge:
  **Westtor** mit Brücke bei (−450, 0) und Osttor gespiegelt; **zwei Pforten**
  mit Stegen nur für Infanterie im Norden und Süden.
- **Hauptkanal** auf der Achse, 45 m breit, vom Nordrand zum Festungsgraben und
  vom Graben zum Südrand.
- **Zwei Querkanäle** (25 m) über die ganze Breite: **Nordkanal** bei y ≈ −900
  (vom Hauptkanal zum Westrand (−2000, −980)) und **Südkanal** bei y ≈ 900.
  Jede Seite hat damit **drei Polder**: Nord, Mitte, Süd.
- **Wahrzeichen auf der Achse:**
  - **Werk im Norden** um (0, −1500), auf beiden Ufern des Hauptkanals: Hallen,
    Schornstein (aus 137, falls vorhanden; sonst das Schöpfwerk aus 151 in
    groß), Lagerplatz, Kai, Werksbrücke;
  - **Schleusenanlage im Süden** um (0, 1500): Doppelschleuse und Wehr mit
    Schöpfwerk an einem **Sammelbecken** (See von etwa 500 m × 350 m südlich
    davon, mit Schilfufern), wie die Vorlage unten links.
- **Übergänge** (Westseite und Achse; zusammen etwa zwanzig):
  - Hauptkanal: Steinbrücken bei (0, −1150), (0, −650), (0, 650), (0, 1150); die
    **Werksbrücke** (0, −1520) und die **Schleusenanlage** (0, 1500) als Notwege
    mit niedriger Lastgrenze;
  - Festung: Westtor (−450, 0); Pforten Nord und Süd nur für Infanterie;
  - Nordkanal: Steinbrücke bei (−700, −920), Holzbrücke bei (−1500, −960);
    Südkanal ebenso bei (−700, 920) und (−1500, 960).
- **Deichsystem:** Deiche an allen Kanälen; ein **Binnendeich** je Seite von
  Nord nach Süd bei x ≈ −1100 durch alle drei Polder (alte Deichlinie, mit
  Straße auf der Krone und Durchlässen an den Querkanalbrücken); kurze
  **Schlafdeiche** quer in den Mittelpoldern. Die Deiche bilden gestaffelte
  Linien zwischen Lagern und Hauptkanal.
- **Dörfer und Höfe** (Westseite): Dorf im Nordpolder um (−1300, −1350), Dorf im
  Mittelpolder um (−1150, −450) am Binnendeich, Dorf im Südpolder um
  (−1300, 1300); acht bis zehn Polderhöfe; **Windmühlen** in einer Reihe von
  dreien am Südkanal um (−1100, 780) und einzeln bei (−1700, −300) und
  (−500, −1250).
- **Offene Felder:** je Polder ein Schlag von mindestens 700 m × 500 m ohne
  Gräben; im Mittelpolder der größte (bis 1000 m), vor der Festung. Schilf und
  Sumpf in den Außenecken.
- **Straßen:** auf den Deichkronen; je Seite eine Landstraße von Nord nach Süd
  über die Querkanalbrücken; je Polder eine Straße vom Rand zur
  Hauptkanalbrücke; Feldwege zu Höfen und Mühlen.
- **Starts** (60 m eben und frei), zwei je Polder: Team West (−1700, −1500),
  (−900, −1650), (−1800, −350), (−1800, 400), (−1700, 1500), (−900, 1650); Team
  Ost gespiegelt. Gerade Spielernummern Team West, ungerade Team Ost; Zuordnung
  und andere Teilungen (4 × 3: Nord, Mitte-West, Mitte-Ost, Süd über beide
  Seiten ist nicht sinnvoll; stattdessen Nordwest, Südwest, Nordost, Südost
  mit je drei Lagern, wozu die Mittellager geteilt werden) als Tabelle in
  `docs/MAPS.md`.
- **Versorgungspunkte** (elf): Markt der Festung (0, 0), umkämpft; Werk
  (0, −1500) und Schleusenanlage (0, 1500), umkämpft; je Seite Dorf Nord
  (−1300, −1350), Dorf Mitte (−1150, −450), Windmühlenreihe (−1100, 780), Dorf
  Süd (−1300, 1300).
- **Einheitengrenze** je Spieler aus 143 übernehmen.

## 4. Stufen

**Stufe 1 – Mittlere Karte:** `data/map_sketches/polder_medium.toml`, erzeugt
mit `just gen-sketch polder_medium`; prüfen wie unten; KI-Turnier.

**Stufe 2 – Große Karte:** `data/map_sketches/polder_large.toml`; zuerst
Kanäle, Deiche, Festungswälle, Brücken und Straßen (Fahrprobe), dann Polder,
Stadt, Dörfer, Werk, Schleusenanlage.

**Stufe 3 – Prüfen und Abschluss, für beide Karten:**

- `just map-check`, `just map-reach`: alle Starts und Ziele für Fahrzeuge
  erreichbar; jeder Polder für Fahrzeuge erreichbar.
- **Brückenprobe, als Test:** Für **jede einzelne Brücke**: ist sie gesprengt,
  bleibt jedes Lager mit jedem Ziel verbunden (für schwere Panzer über einen
  anderen Übergang). Für jeden Polder: mindestens zwei Übergänge für schwere
  Panzer und ein Notweg. Mit **allen** Steinbrücken gesprengt erreichen
  Infanterie und leichte Fahrzeuge über Schleusen, Wehre und Holzbrücken noch
  Ziele auf der anderen Seite. Dazu ein Lauf im Spiel: Eine Brücke fällt vor
  einem Panzerverband, Bilder vorher und nachher, Umweg in Sekunden.
- **Symmetrietest:** unter Spiegelung gleiche Höhen, Gelände, Weggitter,
  Objektklassen und Weglängen.
- **Sichtprüfung:** Sichtfeld-Bilder von Deichkrone, Bastion, Windmühle,
  Brückenmitte, aus dem offenen Feld und hinter einem Deich.
- **Reichweitentabelle** (indirekt 1/8) in `docs/MAPS.md`: Was erreichen
  Geschütze von den Bastionen, von den Binnendeichen, aus den Lagern? Kein
  Lager in Reichweite eines gegnerischen Lagers (auf der mittleren Karte so
  weit wie möglich; nennen, was nicht geht).
- **Ausgleich** ist durch die Spiegelung gegeben; zusätzlich die Fahrzeit von
  jedem Lager zur Stadt und zur nächsten Hauptkanalbrücke nennen, damit die
  Lager eines Teams nicht zu ungleich liegen (innerhalb eines Teams höchstens
  das Doppelte).
- **Bildrate, Tick und Wegsuche** in jeder Ansicht mit acht bzw. zwölf Spielern
  (`docs/PERFORMANCE.md`); viele Gräben und Feldstreifen sind viele kleine
  Flächen: messen und, wenn nötig, Streifen breiter machen.
- **KI-Turnier** mit voller Besetzung, halber Besetzung und 1 gegen 1, Ergebnis
  in `docs/AI.md`: Verteilt die KI ihre Kräfte auf mehrere Fronten, staut sie
  sich an Brücken, unterstützen sich Verbündete über die Querkanäle, endet es im
  Stillstand am Hauptkanal? Probleme als offene Punkte; die KI nicht umbauen.
- `just selftest`, `just lan-selftest` (acht und zwölf Plätze), Menünamen und
  Übersetzungen, Einträge in der Tabelle der Skizzenkarten in `docs/MAPS.md` mit
  Zuordnungs- und Reichweitentabelle, Test wie bei den anderen Skizzenkarten
  (zweimal erzeugt, gleiche Dateien). `just check`. Der Golden Hash ändert sich
  nicht (neue Karten; die Regeln kamen mit 151); wenn doch, Ursache finden.

## 5. Nicht Teil dieses Prompts

- Fluten von Poldern, Deichbrüche, bewegliche Brücken, Boote und Kähne (siehe
  151, Abschnitt 7).
- KI, die Brücken sprengt oder repariert, und KI-Verbesserungen für mehrere
  Fronten.
- Eine lagegetreue Karte einer echten Festungsstadt nach Geodaten.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
