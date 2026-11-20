# Prompt 137 – Großstadtbauten II: Uhrturm, Stadtpalast, Bahnhofshalle, Kraftwerk, Riesenrad, Backsteinblöcke (zerstörbar)

> In einer frischen Session im Repo-Ordner ausführen. **Voraussetzung:** Prompt 134
> (Großstadthäuser: Blockrand-Baukasten, `[[city_block]]`, vorab zerstörte
> Häuser) ist erledigt (`prompts/done/134-*`). **Empfohlen nach 135**
> (Gitterturm: Fallregel mit Trümmerband für hohe Bauten, Kamera an sehr hohen
> Objekten); ist 135 nicht erledigt, baut dieser Prompt die Fallregel selbst
> (Abschnitt 4) und 135 verwendet sie später. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/134-*`, `prompts/done/078-hochhaeuser.md`,
> `prompts/done/020-zerstoerung.md` und, falls erledigt, `prompts/done/135-*`; in
> `docs/ASSETS.md` Gebäude, Baukasten, Budgets; in `docs/ARCHITECTURE.md`
> „Destruction“, Sicht, Wegfindung; `data/destruction.toml`, `data/objects.toml`,
> `docs/SOURCES.md`, `docs/PERFORMANCE.md`; in `docs/MAPS.md` die Skizzen-Tabellen
> (`[[building]]`, `[[rail]]`, `[[city_block]]`).
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/uhrturm/uhrturm.webp`,
> ein mit KI erzeugtes Stadtbild. Hier zählen nur die **Bauten**: ein Stadtpalast
> mit Vorplatz im Park, ein gotischer Bau mit hohem Uhrturm am Fluss, ein
> Riesenrad, eine Bahnhofshalle mit Tonnendach aus Eisen und Glas, ein
> Backstein-Kraftwerk mit Schornstein, Blöcke und gebogene Häuserzeilen aus
> Backstein. Die Karte baut Prompt 138.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Die Bauten einer **nordeuropäischen Großstadt um 1944**, als zerstörbare
Kartenobjekte: fünf Wahrzeichen und eine Backstein-Fassadenfamilie für die
Blöcke aus 134. Vorbilder sind **Bautypen**, keine bestimmten Gebäude: Nichts
wird lagegetreu oder im Detail nachgebildet, im Spiel tragen die Bauten neutrale
Namen. Maße als Bautyp mit Quelle nach `docs/SOURCES.md`; nichts aus anderen
Spielen.

## 2. Bauten

- **Uhrturm** `env_clock_tower`: schlanker, quadratischer Turm im Stil der
  Neugotik, um 90–100 m hoch, etwa 12 m Seitenlänge, Zifferblätter auf allen
  vier Seiten, spitzes Dach. Eigenes Objekt (fällt für sich).
- **Ratsbau** `env_council_hall` (der lange Bau am Fuß des Turms): neugotischer
  Flügelbau, drei bis vier Geschosse, aus zwei bis drei Stücken von je etwa 60 m
  (Mittelstück, Endstück mit kleinem Turm), am Kai aufreihbar.
- **Stadtpalast** `env_city_palace`: klassizistischer Bau, drei Geschosse,
  Mittelrisalit mit Säulen, als Mittelstück und zwei Seitenflügel (zusammen etwa
  110 m Front), dazu ein gepflasterter **Vorplatz** mit Gitter und Torpfeilern
  (Mauerstücke, überfahrbar nach den Regeln für Mauern).
- **Bahnhofshalle** `env_station_hall`: Tonnendach aus Eisen und Glas auf
  Backsteinlängswänden, etwa 60–70 m Spannweite, in Stücken von 40 m aufreihbar
  (vier bis fünf ergeben die Halle), dazu ein **Kopfbau** aus Backstein
  `env_station_front`. Die Halle ist an den Stirnseiten offen: Infanterie und
  Fahrzeuge können hindurch, wenn die Wegfindung das mit Gebäuden schon kann
  (Torhaus aus 134); sonst nur Infanterie und als Entscheidung festhalten. Glas
  als Durchsicht-Textur.
- **Kraftwerk** `env_power_station`: großer Backsteinblock (Kesselhaus und
  Maschinenhaus, etwa 120 m × 60 m, 30 m hoch, hohe schmale Fenster) und ein
  eigener **Schornstein** `env_chimney_tall` (um 90–100 m, Backstein oder Beton,
  rund oder kanneliert). Bautyp der 1930er Jahre.
- **Riesenrad** `env_big_wheel`: Bauart um 1900 (eisernes Speichenrad mit
  Gondeln wie kleine Waggons, um 60–65 m Durchmesser, auf zwei A-Böcken), nicht
  das moderne Aussichtsrad der Vorlage. Speichen als Durchsicht-Textur. Dreht
  sich langsam (nur Darstellung), steht nach dem ersten Schaden still.
- **Backsteinblöcke:** für jeden Haustyp aus 134 (gerade, Eck-, Keil-, Torhaus)
  eine Fassadenfamilie **Backstein**: vier bis fünf Geschosse, dunkelroter und
  gelber Ziegel, Schieferdach mit Schornsteinreihen, Erker, Läden im Erdgeschoss.
  Dazu neu ein **gebogenes Stück** `env_block_city_curve` (Innen- und
  Außenbogen, für Ringstraßen und halbrunde Häuserzeilen), das `[[city_block]]`
  an gekrümmten Blockkanten setzt.

Jeder Bau in `Intact`, `Damaged`, `Ruin` mit sichtbarem Inneren wie in 078/134.
Uhrturm und Schornstein: `Damaged` mit Löchern und fehlender Spitze, `Ruin` als
Stumpf (unterstes Viertel) plus Trümmerband. Riesenrad: `Damaged` verbogen und
mit fehlenden Gondeln, `Ruin` vom Bock gekippt. Dreiecke je Bau im Budget von
078 (große Bauten höchstens das Doppelte; in `docs/ASSETS.md` festhalten).

## 3. Regeln (Simulation)

- Einträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster der
  Gebäude: Grundriss, Höhe, Sichtsperre, Besatzung, Brand, Trefferpunkte nach
  Masse (Richtgrößen: Ratsbau-Stück und Palast-Stück wie ein Blockrandhaus oder
  mehr, Kraftwerk etwa das Doppelte, Hallenstück weniger als ein Haus, Uhrturm
  und Schornstein wie eine Kirche, Riesenrad wenig).
- **Besetzbar:** Ratsbau, Palast, Kopfbau, Kraftwerk. Nicht besetzbar: Uhrturm,
  Schornstein, Riesenrad, Hallenstücke. Beobachter auf Türmen sind ein offener
  Punkt (neue Regelart).
- **Riesenrad:** nur die beiden Böcke sind Hindernisse; das Rad sperrt keine
  Sicht.
- **Der Golden Hash ändert sich** (neue Einträge in den Regeltabellen, ggf. die
  Fallregel); gewollter Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit
  nennen. Vergleichslauf auf einer Karte ohne die neuen Bauten zeigt gleiche
  Ergebnisse wie vorher.

## 4. Fallregel für hohe Bauten

Uhrturm und Schornstein **stürzen um**, wenn sie zerstört werden: deterministisch
in Richtung des letzten Treffers (auf acht Richtungen gerundet, wie bei
fallenden Bäumen), und hinterlassen ein **Trümmerband** in der Länge ihrer Höhe
und etwa der doppelten Breite ihres Grundrisses. Das Band sperrt Fahrzeuge,
bremst Infanterie und gibt ihr Deckung; was darunter steht, nimmt Schaden
(Gebäude und Bäume zerstört, Einheiten schwer getroffen; ein fester Wert je
Klasse).

- Ist 135 erledigt: dessen Regel verwenden und nur über die Tabelle auf die
  neuen Klassen ausdehnen (Länge aus der Objekthöhe).
- Sonst die Regel hier bauen, als Eigenschaft einer Zerstörungsklasse
  (`fall = true` oder ähnlich), sodass 135 sie nur noch einträgt.
- Fällt ein Turm in den Fluss, liegt das Band im Wasser und sperrt nichts
  (Boote: offener Punkt). Fällt er auf eine Brücke, nimmt die Brücke Schaden wie
  durch eine Sprengladung; mit Test.
- Alles in Festkomma, Zufall nur über `sim::Rng`.

## 5. Zusammenstellungen im Skizzen-Generator

Damit eine Karte die Bauten mit einer Zeile setzt (in `docs/MAPS.md`
beschreiben): **Ratsbau mit Uhrturm** (Linie am Kai, Turm an einem Ende),
**Stadtpalast mit Vorplatz** (Mittelpunkt, Blickrichtung), **Bahnhof** (Halle
aus n Stücken, Kopfbau, Gleise als `[[rail]]` aus der offenen Stirnseite),
**Kraftwerk mit Schornstein und Kohlenhof**. Als Tabellen oder als
`[[building]]`-Listen, je nachdem, was einfacher ist. `[[city_block]]` bekommt
den Schalter für die Fassadenfamilie und setzt gebogene Stücke an gekrümmten
Kanten. Bestehende Skizzenkarten müssen Bit für Bit gleich bleiben.

## 6. Stufen

**Stufe 1 – Modelle:** alle Bauten `Intact`, Backsteinfamilie und gebogenes
Stück; eine Ansichtskarte `city_landmarks` (etwa 1000 m × 800 m: Kai mit Ratsbau
und Uhrturm, Park mit Palast, Bahnhof, Kraftwerk, Riesenrad, dazwischen einige
Backsteinblöcke an einer Ringstraße); Bilder neben der Vorlage.

**Stufe 2 – Zustände und Regeln:** `Damaged` und `Ruin`, Abschnitt 3 bis 5;
Tests: Fall in jede der acht Richtungen, Trümmerband sperrt Fahrzeuge, Schaden
darunter, Turm auf Brücke, Durchgang durch die Halle, Besetzen, zweimal gleicher
Lauf ergibt gleichen Hash.

**Stufe 3 – Abschluss:** Kamera an Uhrturm und Schornstein (Regel aus 078 bzw.
135: oberer Teil wird in der Nahansicht weich ausgeblendet, Einheiten dahinter
bleiben sichtbar), Bildrate über `city_landmarks` in jeder Ansicht
(`docs/PERFORMANCE.md`), Gefechtsreihe als Bilder (Artillerie fällt den
Schornstein), Vorschaubilder für jedes neue Modell (`just thumbnails --only=…`),
`just units-web-sync`, `just catalog-doc`, `docs/ASSETS.md`, `docs/MAPS.md`,
`docs/ARCHITECTURE.md`, `docs/SOURCES.md`. `just check`, `just selftest`.

## 7. Nicht Teil dieses Prompts

- Die Stadtkarte (Prompt 138).
- Züge, Waggons und Lokomotiven in der Halle, Hochbahn und Viadukte.
- Glockenschlag als Spielregel (als Geräusch der Kulisse erlaubt, wenn einfach).
- Strom als Rohstoff; das Kraftwerk ist Kulisse und Stellung.
- Weitere Wahrzeichen (Kuppelkirche, Zugbrücke mit Türmen, Säule).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
