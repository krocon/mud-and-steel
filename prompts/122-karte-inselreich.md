# Prompt 122 – Karte nach Bildvorlage: Inselreich, farbenfroh

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/088-karten-nach-bildvorlagen.md` (Ziel, Entscheidungen, Ergebnis),
> in `docs/MAPS.md` „Maps from picture sketches (prompt 088)“ ganz (alle Tabellen
> der Skizze, besonders `sea`, `[[land]]`, `[[bay]]`, `[[causeway]]`, `[[shoal]]`,
> `[[crossing]]`, `[[hill]]`, `[[wood]]` mit `rock`, `[[paint]]`), „Bridges“ und
> „Biome“, `data/map_sketches/island_chain.toml` ganz als nächstes Muster,
> `crates/cli/src/gen_sketch.rs`, in `docs/ARCHITECTURE.md` „Lighting, atmosphere
> and biomes (presentation only)“ und „Water (presentation only)“,
> `godot/assets/biomes/forest.tres` und `biome_preset.gd`,
> `godot/scenes/world/atmosphere.gd`, `godot/scenes/terrain/terrain.gd`
> (`apply_biome`), das Wasser-Shader-Setup und `data/objects.toml` (Felsen,
> Brücken). Ist Prompt 115 (lange Brücken) erledigt, auch dessen Ergebnis.
>
> **Vorlagen** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/inseln/inseln_draufsicht.webp` (Draufsicht, Grundlage für die
> Skizze) und `docs/.vorlagen/inseln/inseln_schraeg.webp` (Schrägansicht derselben
> Landschaft, Vorbild für Höhen, Licht und Farben). Beide genau ansehen.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder aus dem Spiel
> neben der Vorlage zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Eine Gefechtskarte `archipelago` (im Spiel „Inselreich“ / „Archipelago“) nach der
Vorlage: acht größere Inseln und Halbinseln um eine **ringförmige Mittelinsel mit
einer Lagune** und einem Eiland darin, verbunden durch **Steinbogenbrücken mit
mehreren Bögen**; zwei bis drei **Felsberge mit Schneekuppen**, Wasserfälle und
Stromschnellen in den Meerengen, Dörfer mit roten Ziegeldächern und Kirchtürmen,
goldene und grüne Felder, dichte Nadelwälder, Felsküsten mit hellen Sandbuchten,
Holzstege an den Außenküsten und viele kleine Felseilande ringsum.

Der Nutzer will die Karte **genauso farbenfroh wie die Vorlage**: sattes Grün,
türkis-blaues Wasser mit hellem Saum an den Küsten, goldgelbe Felder, rote Dächer,
helle Felsen, klarer Himmel. Das ist eine bewusste Ausnahme von der
zurückhaltenden Optik der anderen Karten und gilt nur für diese Karte
(Abschnitt 4).

Die Vorlagen sind nur Vorbild für Anordnung und Wirkung. Nichts daraus übernehmen
(keine Texturen, keine Bildausschnitte), alles mit den Bausteinen des Spiels
nachbauen.

## 2. Skizze (Stufe 1)

`data/map_sketches/archipelago.toml`, erzeugt mit `just gen-sketch archipelago`:

- **Größe** 3,6 km × 2,4 km (Seitenverhältnis der Draufsicht), `sea = true`,
  Biom `forest`, Wetter klar.
- **Inseln** als `[[land]]`-Polygone nach der Draufsicht nachgezeichnet, die
  Meerengen dazwischen als `[[bay]]`, wo sie innen liegen. Die Mittelinsel als Ring
  um die Lagune, das Eiland darin als `[[island]]`. Die Felseilande am Rand als
  kleine `[[land]]`-Polygone ohne Zugang.
- **Berge:** `[[hill]]` für die drei Felsberge (Nord-Mitte, Nordost, Südwest) mit
  deutlich mehr Höhe als bisher in Skizzen (Vorschlag 80–150 m; der Generator muss
  das tragen, ggf. erweitern), steile Flanken und Gipfel über 60 % Steigung
  (unpassierbar), Felsen mit `[[wood]]` `rock` und `env_rock_massif`. **Schnee auf
  den Gipfeln** als reine Darstellung (Abschnitt 4), nicht als Geländeart `snow`,
  damit sich die Wege nicht ändern; oder `[[paint]]` mit `snow` nur dort, wo ohnehin
  niemand hinkommt. Die Session wählt und begründet.
- **Brücken:** In der Vorlage sind es sechs lange Steinbrücken mit drei bis fünf
  Bögen über breite Meerengen. Der Skizzen-Generator verengt das Wasser heute auf
  21 m (längste Steinbrücke). Wenn Prompt 115 erledigt ist: eine Variante
  **mehrbogige Steinbrücke** (Teile aneinandergereiht, 60–120 m) ergänzen und die
  Meerengen breit lassen. Sonst wie bei `island_chain` mit Dämmen (`[[causeway]]`)
  und kurzen Brücken, und die langen Brücken als offener Punkt.
- **Zweiter Weg für Panzer:** wie bei `island_chain` ein bis zwei Untiefen
  (`[[shoal]]`) als Furt, damit eine gesprengte oder gesperrte Brücke nicht das
  ganze Spiel entscheidet.
- **Wasserfälle und Stromschnellen:** Stromschnellen gibt es (`valley_fields`). Ein
  Wasserfall vom Berg in die Meerenge ist neu: als reine Darstellung (Objekt oder
  Effekt mit Gischt, Geräusch) an einer Felskante, ohne Regelwirkung. Zwei bis drei
  Stück wie in der Vorlage.
- **Dörfer:** vier bis fünf Dörfer mit Kirche (NW, NO mit größerer Kirche, W, Mitte
  unten bei der Südwest-Insel, SO), `[[village]]`/`[[town]]` mit Häusern mit roten
  Ziegeldächern aus dem Katalog; Feldwege zwischen den Dörfern und zu den Brücken
  wie in der Draufsicht.
- **Felder:** `[[patchwork]]` und `[[field]]` mit `grain` (gold) und `green`.
- **Wälder:** viel Nadelwald (`conifer`), an den Rändern gemischt; Sandbuchten an
  den Außenküsten.
- **Holzstege:** Die Vorlage hat Stege an den Außenküsten. Es gibt kein
  Steg-Objekt. Ein einfaches Objekt `env_pier_wood` (Bohlen auf Pfählen, blockiert
  nichts, Darstellung im Wasser) neu bauen, mit Vorschaubild, oder als offener
  Punkt notieren, wenn Stufe 3 sonst zu groß wird. Kleine Boote am Steg nur als
  Dekoration.
- **Starts und Versorgungspunkte:** zwei Teams, West (Südwest- und Westinsel)
  gegen Ost (Nordost- und Südostinsel), je zwei Starts, 60 m eben und frei.
  Versorgungspunkte auf der Mittelinsel (umkämpft), auf den Berginseln an den
  Pässen, in den Dörfern und an den Brückenköpfen. Mit `just map-reach`
  ausgleichen.

## 3. Prüfen der Karte (Stufe 2)

- `just map-check archipelago`, `just map-reach archipelago` (alle Starts und Punkte
  für Fahrzeuge erreichbar, über Brücken und Furten), Test wie bei den anderen
  Skizzenkarten (zweimal erzeugt, gleiche Dateien).
- KI-Turnier auf der Karte, Ergebnis in `docs/AI.md` (bleiben Kolonnen an Brücken
  hängen, wird um die Mittelinsel gekämpft?).
- Messung der Bildrate in jeder Ansicht (`just bench-view`), `docs/PERFORMANCE.md`.
- Menüname und Übersetzung, Eintrag in der Tabelle in `docs/MAPS.md`.

## 4. Farbenfroher Look (Stufe 3)

Reine Darstellung, nur für diese Karte, andere Karten bleiben Bit für Bit gleich im
Aussehen:

- Ein **Look je Karte**: neues optionales Feld in `map.toml` (Vorschlag
  `look = "vivid"`, wie `biome` nicht im Karten- oder Spielstand-Hash, von der
  Simulation nur geprüft), das auf das Biom-Preset eine Abwandlung legt. Oder ein
  eigenes Preset `forest_vivid`, das `camo_biome` und `env_biome` von `forest`
  übernimmt. Die Session wählt das Einfachere und begründet es.
- **Wirkung** wie in den Vorlagen: kräftigere Sättigung und etwas mehr Kontrast,
  warmes, helles Sonnenlicht, klarer Himmel, wenig Dunst; Gras saftig grün, Wald
  dunkler grün, Felder goldgelb, Felsen hellgrau, Sand hell; Wasser in der Tiefe
  kräftig blau, in flachem Wasser türkis, deutlicher heller Saum und Schaum an den
  Küsten. Dächer der Dörfer kräftig rot (die vorhandenen Häuser tönen, nicht neu
  modellieren).
- **Schnee auf den Gipfeln** ab einer Höhe in diesem Look (Shader-Anteil wie die
  Schneedecke im Winter, nur oberhalb einer Grenze und auf flacheren Stellen).
- Wolkenfetzen am Bildrand wie in der Schrägansicht nur, wenn der Fotomodus oder
  die Übersicht es ohne Aufwand trägt; sonst nicht.
- **Vergleich:** dieselben drei Ansichten mit `forest` und mit dem neuen Look neben
  der Vorlage. Der Nutzer entscheidet am Ende, ob es farbig genug ist; die Werte
  liegen in einer Datei, damit sie leicht nachzustellen sind.
- Unit-Farben und Tarnungen bleiben unverändert (Einheiten sollen auf der bunten
  Karte erkennbar bleiben). Prüfen, dass Auswahlringe, Nebel des Krieges und HUD
  noch gut lesbar sind.

## 5. Abschluss

`just check`, `just selftest`, Vorschaubild der Karte im Menü (falls Karten Bilder
haben), Doku in `docs/MAPS.md` (Tabelle der Skizzenkarten, neues Feld bzw. Preset,
Wasserfall, Steg) und `docs/ARCHITECTURE.md` (Look je Karte). Der Golden Hash
ändert sich nicht (neue Karte, reine Darstellung); wenn doch, Ursache finden.

## 6. Nicht Teil dieses Prompts

- Andere Karten auf den bunten Look umstellen (erst nach dem Urteil des Nutzers).
- Boote als Einheiten (Prompt 119); Stege sind hier nur Kulisse.
- Fließendes Wasser mit Gefälle zwischen Meer und Bergsee (Wasserfälle bleiben
  Darstellung).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
