# Prompt 151 – Polderland: Kanäle, Deiche, Schleusen, Windmühlen und die kleine Polderkarte (vier Spieler)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` ganz, in `docs/MAPS.md` „Maps from
> picture sketches (prompt 088)“ ganz (besonders `[[river]]`, `[[crossing]]`,
> `[[causeway]]`, `[[shoal]]`, `[[lake]]`, `[[field]]`, `[[patchwork]]`,
> `[[farm]]`, `[[village]]`, `[[town]]`, `[[road]]`) und **„Bridges“**
> (Brückenarten, Lastgrenze, Verengen des Wassers an Brücken); in
> `docs/ARCHITECTURE.md` die Wegfindung (Decks, **Lastgrenze**: schwerere
> Fahrzeuge kommen nicht über eine Brücke), die **Pioniere** (Brücken sprengen
> und wiederherstellen), den **Brückenleger** (gelegte Brücke, Aufnehmen),
> Sicht (Sichtlinie über Deckung) und „Destruction“ (Brücken sind bisher in
> keiner Klasse, also durch Beschuss nicht zu zerstören); `data/destruction.toml`,
> `data/objects.toml`, `data/terrain.toml`, `data/scale.toml` (`[range]`),
> `docs/SOURCES.md`, `docs/ASSETS.md` (Gebäude, Brücken, Budgets),
> `crates/cli/src/gen_sketch.rs`, `docs/AI.md`, `docs/PERFORMANCE.md`. Ist Prompt
> 115 (lange Brücken, Teile einzeln sprengbar) erledigt, auch dessen Ergebnis.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/polder/polder.webp`. Genau ansehen. Ein mit KI erzeugtes Blatt
> mit **drei Varianten** einer holländischen Polderlandschaft (groß 12 Spieler,
> mittel 8, klein 4) und einer Legende. Dieser Prompt baut die Bausteine und die
> **kleine** Karte (rechts unten im Blatt); die mittlere und die große baut
> Prompt 152. Die Maßstabsbalken werden ignoriert, es gelten die Größen des
> Nutzers.
>
> **Vorgaben des Nutzers:** Klein – 1 × 1 km, 4 Spieler, schnelle Gefechte mit
> wenigen, hart umkämpften Übergängen. Wichtige Elemente: **Deiche** als erhöhte
> Verteidigungsstellungen, **Brücken** als strategische Engpässe, **Schleusen
> und Wehre** als besondere Kontrollpunkte, **Windmühlen und Bauernhöfe** als
> Orientierungspunkte, **große offene Felder** für Panzerverbände, **Kanäle** als
> natürliche Hindernisse und Flankenschutz. „Besonders interessant wäre eine
> Mechanik, bei der zerstörte Brücken den Vormarsch ganzer Panzerverbände
> unterbrechen.“
>
> **Dieser Prompt läuft in vier Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Das Polderland als Baukasten für Karten, und daraus die erste Karte
`polder_small` (im Spiel „Polder (klein)“ / „Polder (small)“): flaches Land
unter dem Wasserspiegel, von **Kanälen** zerschnitten, die kein Fahrzeug quert,
von **Deichen** gesäumt, über die man hinwegsieht, mit einem Städtchen auf einer
Insel in der Mitte. Wer die Brücken hält, hält die Karte.

## 2. Bausteine im Skizzen-Generator

In `docs/MAPS.md` beschreiben; bestehende Skizzenkarten bleiben Bit für Bit
gleich.

- **`[[canal]]`:** Wasserlauf mit gleichbleibender Breite (Hauptkanal 30–45 m,
  Nebenkanal 15–20 m), gerade Strecken mit weichen Knicken, **tiefes Wasser von
  Ufer zu Ufer** (für Fahrzeuge und Infanterie gesperrt), steile befestigte
  Ufer, kein Strand. Schalter: Deiche an einer oder beiden Seiten, Treidelweg,
  Baumreihe, Schilf. Reicht `[[river]]` mit festen Werten, es dafür verwenden.
- **`[[dike]]`:** Erdwall entlang einer Linie: Höhe 3–5 m über dem Polder,
  Krone 5–8 m breit, Böschungen etwa 1 : 3, auf der Krone wahlweise ein Weg
  (`track`) oder eine Straße. Aus dem Höhenraster gebaut, kein Objekt. **Für
  alle Fahrzeuge befahrbar** (Krone längs, Böschung quer langsam; Steigung
  prüfen), Infanterie ohnehin. Wirkung allein aus den vorhandenen Regeln: Von
  der Krone sieht und schießt man über Hecken, Getreide und niedrige Deckung im
  flachen Land; wer **hinter** dem Deich steht, ist von der anderen Seite nicht
  zu sehen; wer **auf** der Krone fährt, steht frei. Das in Stufe 4 mit
  Sichtbildern belegen. (Die Regel „weiter sehen nach unten“ greift erst ab
  20 m Höhenunterschied und spielt hier keine Rolle.)
- **`[[polder]]`:** Feldflur als Polygon: lange schmale Streifen (30–60 m ×
  200–400 m) im Wechsel von Wiese, Getreide, Rüben, Brache (wie
  `[[patchwork]]`), dazwischen **Wassergräben** von 2–4 m (flaches Wasser: für
  alle passierbar, bremst; Infanterie darin gedeckt, wenn es dafür schon eine
  Regel gibt), Viehzäune, einzelne Kopfweiden. Offen, eben, **0,5–1,5 m unter
  dem Kanalspiegel** (das ist ein Polder). Kein Wald.
- **Schilf und Sumpf:** nasse Ecken mit Schilf (Sichtdeckung für Infanterie,
  für Fahrzeuge sehr langsam oder gesperrt), aus vorhandenem Gelände.

## 3. Bauten (zerstörbar)

Bautypen, keine bestimmten Gebäude; Maße mit Quelle nach `docs/SOURCES.md`,
nichts aus anderen Spielen. Je Bau `Intact`, `Damaged`, `Ruin` wie die
vorhandenen Häuser, Vorschaubild (`just thumbnails --only=…`),
`just units-web-sync`, `just catalog-doc`.

- **Windmühle** `env_windmill`: Holländermühle (achteckiger Turm mit drehbarer
  Kappe und Galerie), um 20–25 m bis zur Kappe, Flügelkreuz um 24 m; Flügel
  drehen langsam (nur Darstellung), stehen nach dem ersten Schaden still.
  Landmarke, von weitem zu sehen; besetzbar mit einem Trupp. Dazu eine kleine
  **Poldermühle** als Variante.
- **Polderhof** `env_farm_polder`: großes Bauernhaus mit tief gezogenem Reet-
  oder Ziegeldach (Wohnteil und Stall unter einem Dach), dazu vorhandene
  Scheunen und Schober für `[[farm]]`.
- **Giebelhaus** `env_house_gable` und Eckhaus: schmales Backsteinhaus mit
  Treppen- oder Glockengiebel zur Straße, drei bis vier Geschosse, in Zeilen
  aufreihbar, zwei bis drei Varianten; für das Städtchen. Kirche: die
  vorhandene, mit Backsteinfarbe, falls das Material es hergibt.
- **Schleuse** `env_lock` und **Wehr** `env_weir`: gemauerte Kammer mit
  Holztoren bzw. ein Wehr mit Schützen und Steg; dazu ein **Schöpfwerk**
  `env_pump_house` (Backsteinbau mit Schornstein). Im Spiel sind Schleuse und
  Wehr **Brücken eigener Art** (Abschnitt 4).
- **Klappbrücke** als Bild für die vorhandene Holz- bzw. Steinbrücke über
  Nebenkanäle (weiße Balken mit Gegengewicht, **fest geschlossen**, keine
  bewegliche Brücke als Regel), wenn der Baukasten das mit wenig Aufwand
  hergibt; sonst die vorhandenen Brücken.

Regeleinträge in `data/objects.toml` und `data/destruction.toml` nach dem Muster
der Häuser (Windmühle etwa wie ein Holzhaus mit Steinsockel, brennbar;
Giebelhaus wie das Stadthaus; Polderhof wie das Bauernhaus, Reet brennt
leicht). **Der Golden Hash ändert sich** (neue Einträge in den Regeltabellen);
gewollter Regelzuwachs: Ursache prüfen, Hash neu setzen, im Commit nennen.
Vergleichslauf auf einer Karte ohne die neuen Bauten zeigt gleiche Ergebnisse
wie vorher.

## 4. Brücken als Engpässe

Vieles gibt es schon: Kanäle sperren, Brücken haben eine **Lastgrenze**,
**Pioniere sprengen und reparieren** sie, **Brückenleger** legen eine eigene.
Zuerst mit Tests festhalten, was heute passiert, wenn eine Brücke vor einer
Kolonne gesprengt wird, und dann nur ergänzen, was fehlt:

- **Was geprüft und gezeigt wird** (Testkarte `canal_test`, etwa 800 m × 500 m:
  ein Hauptkanal, zwei Brücken 400 m auseinander, eine Schleuse):
  - Eine Kolonne von zwölf Panzern ist unterwegs über Brücke A; A wird
    gesprengt, bevor die Spitze sie erreicht: Die Einheiten planen neu (über B)
    oder halten an, wenn es keinen Weg mehr gibt, ohne sich zu verkeilen und
    ohne ins Wasser zu fahren. Der Spieler erfährt es (Meldung „Weg
    unterbrochen“ bzw. das vorhandene Zeichen für unerreichbare Ziele).
  - Fahrzeuge, die **auf** der Brücke stehen, wenn sie fällt: nach der
    vorhandenen Regel (lesen und nennen); fehlt eine, sind sie verloren.
  - Reparieren durch Pioniere unter Feuer, Dauer und Preis; Brückenleger legt
    über einen Nebenkanal, nicht über den Hauptkanal (Spannweite).
  - Lastgrenze: Schwere Panzer müssen über die Steinbrücke, leichte Fahrzeuge
    kommen auch über Holzbrücke und Schleuse.
- **Schleuse und Wehr als Übergang:** schmal (einspurig), niedrige Lastgrenze
  (Richtwert: Infanterie und Fahrzeuge bis etwa 10 t), nicht zu sprengen wie
  eine Brücke, sondern massiv (viele Trefferpunkte bzw. gar nicht; einfachste
  Lösung wählen). Sie sind der Notweg, wenn die Brücken gefallen sind, und
  deshalb ein Ziel.
- **Brücken unter schwerem Feuer** (Vorschlag, vorläufig entscheiden): Bisher
  zerstört nur der Pionier eine Brücke. Für diese Karten wäre es reizvoll, wenn
  auch **Bomben und sehr schwere Geschosse** (ab etwa 15 cm, mit vielen
  Treffern) eine Brücke zum Einsturz bringen. Hat 115 das schon geregelt, es
  übernehmen. Sonst: als eigene Klasse in `data/destruction.toml` mit sehr
  vielen Trefferpunkten einführen, **nur wenn** es ohne neue Regelart geht; das
  ändert das Spiel auf allen Karten mit Brücken (Golden Hash, KI-Turnier vorher
  und nachher auf `river_town`), also im Commit benennen und in den
  Entscheidungen begründen. Wirkt es zu stark oder zu aufwendig, weglassen und
  als offenen Punkt notieren.
- **KI:** Die KI sprengt und repariert heute nicht (prüfen). Das bleibt so; im
  Turnier wird beobachtet, wie sie mit gefallenen Brücken umgeht.

## 5. Die kleine Karte `polder_small`

- **1,0 km × 1,0 km**, Biom `forest`, Wetter klar, **spiegelsymmetrisch** an der
  Nord-Süd-Achse (West gegen Ost), (x, y) → (−x, y). **2 gegen 2**, auch 1 gegen
  1. Richtwerte in Metern (x nach Osten, y nach Süden, Mitte 0/0).
- **Städtchen auf der Insel** in der Mitte: runde Insel von etwa 320 m
  Durchmesser, Kirche auf dem Platz bei (0, 0), zwanzig bis dreißig Giebelhäuser
  in Zeilen an einer Ringgasse und zwei Quergassen, Kai ringsum.
- **Ringkanal** um die Insel, 30 m breit, und der **Hauptkanal** auf der Achse:
  vom Nordrand (0, −500) zum Ring und vom Ring zum Südrand (0, 500). Damit ist
  die Karte in **Westland, Ostland und Insel** geteilt.
- **Vier Übergänge, mehr nicht:**
  - **Westbrücke** (−175, 0) und **Ostbrücke** (175, 0) vom Land auf die Insel
    (Steinbrücken, für alle Fahrzeuge; Kanal dort auf die Brückenlänge verengt);
  - **Nordschleuse** (0, −330) und **Südwehr** (0, 330) über den Hauptkanal,
    direkt von West nach Ost: schmal, niedrige Lastgrenze.
  Schwere Panzer kommen also nur **durch das Städtchen**; leichte Kräfte und
  Infanterie können über Schleuse und Wehr flankieren.
- **Deiche** auf beiden Ufern des Hauptkanals und außen am Ringkanal, mit Weg
  auf der Krone; im Westland ein **Querdeich** von (−480, −60) nach (−210, −60)
  als vorbereitete Linie vor der Westbrücke.
- **Polder** (`[[polder]]`): der ganze Rest, im Norden und Süden je Seite ein
  großes offenes Feld; ein **Polderhof** bei (−330, −250), eine **Windmühle**
  bei (−300, 220), ein Schilfstück in der Ecke bei (−430, 430); gespiegelt.
- **Wege:** je Seite eine Straße vom Lager zur Brücke, Deichwege zu Schleuse
  und Wehr, Feldwege zu Hof und Mühle.
- **Starts** (so viel ebene freie Fläche wie bei den vorhandenen kleinen
  Karten): Team West Nordwest (−380, −380) und Südwest (−380, 360); Team Ost
  gespiegelt. Team 0 West (Spieler 0, 2), Team 1 Ost (1, 3).
- **Versorgungspunkte** (fünf): Kirchplatz (0, 0), umkämpft; Nordschleuse
  (0, −330) und Südwehr (0, 330), umkämpft; Windmühle (−300, 220) und
  gespiegelt.

## 6. Stufen

**Stufe 1 – Bausteine:** Abschnitt 2 und die Testkarte `canal_test`; Bilder
(Deich im Schnitt, Kanal, Polderflur).

**Stufe 2 – Bauten:** Abschnitt 3.

**Stufe 3 – Brücken:** Abschnitt 4 mit Tests und, falls entschieden, der Regel
für schweres Feuer.

**Stufe 4 – Karte, Prüfen, Abschluss:**

- `data/map_sketches/polder_small.toml`, erzeugt mit
  `just gen-sketch polder_small`; `just map-check`, `just map-reach` (alles Land
  für Fahrzeuge erreichbar; mit beiden Brücken gesprengt bleibt jedes Ziel für
  Infanterie und leichte Fahrzeuge über Schleuse und Wehr erreichbar).
- **Symmetrietest**; **Sichtbilder** von der Deichkrone, vom Polder hinter dem
  Deich, vom Kirchturmplatz, von einer Brücke; **Reichweitentabelle** (indirekt
  1/8) in `docs/MAPS.md`.
- **Brückenprobe im Spiel:** Lauf, in dem eine Brücke vor einem Panzerverband
  fällt; Bilder vorher und nachher.
- **Bildrate und Wegsuche** in jeder Ansicht (`docs/PERFORMANCE.md`).
- **KI-Turnier** 2 gegen 2 und 1 gegen 1, Ergebnis in `docs/AI.md`: Staut sich
  die KI an den Brücken, nutzt sie Schleuse und Wehr, endet es im Stillstand,
  wie lange dauert eine Partie? Probleme als offene Punkte; die KI nicht
  umbauen.
- Menüname und Übersetzung, Eintrag in der Tabelle der Skizzenkarten, Test wie
  bei den anderen Skizzenkarten (zweimal erzeugt, gleiche Dateien),
  `docs/ARCHITECTURE.md`, `docs/ASSETS.md`, `docs/SOURCES.md`. `just check`,
  `just selftest`.

## 7. Nicht Teil dieses Prompts

- Die mittlere und die große Polderkarte (Prompt 152).
- **Fluten von Poldern** über Schleusen und Deichbrüche (wäre eine neue Regel
  mit veränderlichem Gelände; lohnender eigener Prompt, hier nur als Idee
  festhalten).
- Bewegliche Brücken (Klapp- und Drehbrücken als Regel), Boote und Kähne auf
  den Kanälen (Prompt 119), Pontonbrücken der Pioniere.
- KI, die Brücken sprengt oder repariert.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
