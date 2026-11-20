# Prompt 135 – Wahrzeichen: eiserner Gitterturm, 300 m, zerstörbar

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/078-hochhaeuser.md` (hohe Bauten: Kamera bleibt über Gebäuden,
> Auswahlringe, Zustände) und `prompts/done/020-zerstoerung.md`; in
> `docs/ARCHITECTURE.md` „Destruction“, Sicht, Geschossflug und Wegfindung um
> Objekte; `data/destruction.toml`, `data/objects.toml`; in `docs/ASSETS.md`
> Gebäude, Budgets und modulare Varianten; `docs/SOURCES.md`, `docs/SCALE.md`,
> `docs/CONTROLS.md` (Zoom, Neigung), `docs/PERFORMANCE.md`. Ist Prompt 115
> (lange Brücken: Stahlfachwerk, Einsturzzustand) erledigt, auch dessen Ergebnis;
> dessen Fachwerk-Material wiederverwenden.
>
> **Vorlage** (nicht im Repo, nur lesen): `docs/.vorlagen/eisenturm/eisenturm.webp`.
> Hier zählt nur der **Turm** mit seinem Platz. Vorbild ist der Pariser Turm von
> 1889, ein Bauwerk ohne Schutzrechte; gebaut wird nach veröffentlichten Maßen
> und eigenen Zeichnungen, nicht nach Modellen Dritter und nicht nach anderen
> Spielen. Im Spiel heißt er neutral „Eisenturm“ / „Iron tower“.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Ein Wahrzeichen `env_tower_iron` als Kartenobjekt: ein eiserner Gitterturm in
**echter Größe** (Größen sind 1:1), der im Spiel als Gelände zählt (vier Füße
als Hindernis, Durchfahrt darunter), von überall auf der Karte zu sehen ist,
**in Stufen zerstört werden kann** und beim Einsturz ein Trümmerfeld hinterlässt.

## 2. Maße und Quellen

Echte Maße mit Quelle in `docs/SOURCES.md` eintragen und danach bauen; Richtwerte
zum Prüfen: Höhe des Bauwerks um 300 m, quadratischer Grundriss um 125 m
Seitenlänge, vier schräge Füße auf gemauerten Sockeln, drei Plattformen (um
57 m, 115 m und 276 m), große Bögen zwischen den Füßen unter der ersten
Plattform. Stand 1944 (keine späteren Antennenaufbauten, keine Reklame). Weichen
die Quellen von den Richtwerten ab, gelten die Quellen.

## 3. Modell

- **Gitterwerk nicht als Geometrie:** Hauptgurte und Plattformen als schlichte
  Körper, das Fachwerk dazwischen als Flächen mit Durchsicht-Textur (Alpha-Test,
  kachelbar, selbst gezeichnet). Budget: so viele Dreiecke wie ein großes
  Gebäude aus 078, höchstens das Doppelte; in `docs/ASSETS.md` festhalten.
- **Zustände** (modulare Teile wie bei Gebäuden):
  - `Intact`;
  - `Damaged`: Spitze oberhalb der zweiten Plattform abgeknickt oder
    abgebrochen, verbogene Träger, Brandspuren, Trümmer am Fuß;
  - `Ruin`: eingestürzt. Die vier Fußstümpfe bis etwa zur ersten Plattform
    stehen schief, der Rest liegt als **langes Trümmerband** in Fallrichtung
    über dem Park.
- **Einsturz als Bild:** kurze Bewegung (Knicken und Fallen in einer Richtung),
  Staubwolke, Ton; nur Darstellung, die Simulation schaltet den Zustand in einem
  Tick um.
- **Schatten und Fernsicht:** Der Schatten ist sehr lang; prüfen, dass er nicht
  flackert und die Schattenreichweite nicht für die ganze Szene verschlechtert
  (sonst Schattenwurf des Turms begrenzen und festhalten). Ferne Ansicht mit
  vereinfachter Stufe.
- **Vorschaubild** `godot/assets/thumbnails/env_tower_iron.png`
  (`just thumbnails --only=env_tower_iron`), `just units-web-sync`.

## 4. Kamera

Die Kamera schaut aus der Höhe schräg herab; ein Bauwerk von 300 m ragt hinein.
Zuerst die Kamerahöhen in jeder Zoomstufe und Neigung messen und dann das
Einfachste wählen, das gut aussieht:

- der Turmteil oberhalb einer Höhe unter der Kamera wird **weich ausgeblendet**
  (im Shader, abhängig vom Abstand zur Kamera), sodass man in der Nahansicht
  durch den Turm auf den Platz sieht und er in der Fernansicht ganz dasteht;
- Einheiten unter und hinter dem Turm bleiben sichtbar und anwählbar (Regel für
  verdeckte Einheiten aus 078 wiederverwenden);
- die Kamera fährt nicht in den Turm hinein.

Die echte Höhe bleibt. Nur wenn nichts davon trägt, eine Verkleinerung
vorschlagen und begründet als offene Frage an den Nutzer festhalten, nicht
selbst entscheiden.

## 5. Regeln (Simulation)

- **Grundriss:** vier Fußsockel als feste Hindernisse (je etwa 25 m × 25 m,
  nach Quelle); dazwischen und darunter **frei befahrbar und begehbar**. Der
  Platz unter dem Turm ist ein Kreuzweg.
- **Sicht und Feuer:** Gitterwerk sperrt die Sicht nicht. Die Füße sperren Sicht
  und Geschosse wie Mauerwerk. Deckung: gering (hinter einem Fuß gut). Keine
  Besatzung, kein Aufstieg, keine Beobachter auf dem Turm (das wäre eine neue
  Regelart: offener Punkt).
- **Zerstörung:** eigene Klasse in `data/destruction.toml`. Der Turm ist zäh:
  sehr viele Trefferpunkte (Richtgröße: das Zehn- bis Zwanzigfache einer
  Kirche); **nur schwere Wirkung zählt** (Geschosse und Sprengwirkung ab etwa
  10 cm Kaliber, Bomben, Sprengladungen der Pioniere, explodierende Lager).
  Panzerkanonen kleiner Kaliber und MG tun nichts. Beschädigt bei 50 %,
  zerstört bei 0.
- **Fallrichtung:** deterministisch aus der Richtung des letzten Treffers (wie
  bei fallenden Bäumen, Sinustabelle), auf acht Richtungen gerundet.
- **Trümmerband:** beim Einsturz ein Streifen von etwa 250 m × 40 m in
  Fallrichtung: sperrt Fahrzeuge, bremst Infanterie und gibt ihr Deckung, sperrt
  die Sicht bis zu einer niedrigen Höhe. Was im Streifen steht, nimmt Schaden:
  Gebäude und Bäume werden zerstört, Einheiten erleiden schweren Schaden
  (einfach halten: ein fester Wert je Klasse; mit Test). Die Fußstümpfe bleiben
  Hindernisse.
- Alles in Festkomma, ohne Zufall außer `sim::Rng`, ohne Iteration über
  `HashMap`. **Der Golden Hash ändert sich** durch die neue Klasse und die neue
  Regel (Trümmerband); das ist ein gewollter Regelzuwachs: Ursache prüfen, Hash
  neu setzen und im Commit nennen. Ein Vergleichslauf auf einer Karte ohne Turm
  zeigt gleiche Ergebnisse wie vorher.
- **KI:** behandelt den Turm als Hindernis und schießt nicht gezielt darauf. Die
  Wegsuche rechnet nach dem Einsturz neu (wie bei gesprengten Brücken).

## 6. Stufen

**Stufe 1 – Modell und Kamera:** Abschnitt 2 bis 4, `Intact`; eine Ansichtskarte
`tower_test` (600 m × 600 m, Park, Turm in der Mitte, ein paar Häuser als
Größenvergleich); Bilder in jeder Zoomstufe.

**Stufe 2 – Regeln und Zustände:** Abschnitt 5, `Damaged` und `Ruin`,
Einsturzbild; Tests: Durchfahrt unter dem Turm, Sicht durch das Gitter, kein
Schaden durch kleine Kaliber, Einsturz in jede der acht Richtungen, Trümmerband
sperrt Fahrzeuge, Schaden im Streifen, zweimal gleicher Lauf ergibt gleichen
Hash.

**Stufe 3 – Abschluss:** Bildrate mit Turm in jeder Ansicht
(`docs/PERFORMANCE.md`), Gefechtsreihe (Artillerie bringt den Turm zum Einsturz)
als Bilder, `docs/ARCHITECTURE.md` („Destruction“), `docs/ASSETS.md`,
`docs/SOURCES.md`, `just catalog-doc`. `just check`, `just selftest`.

## 7. Nicht Teil dieses Prompts

- Die Stadtkarte (Prompt 136) und die Großstadthäuser (Prompt 134).
- Beobachter oder Scharfschützen auf dem Turm, Aufzüge, Beleuchtung bei Nacht.
- Weitere Wahrzeichen (Triumphbogen, Kuppelbau, Kathedrale).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
