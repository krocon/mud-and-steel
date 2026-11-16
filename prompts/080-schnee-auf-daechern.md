# Prompt 080 – Schnee auf Dächern

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, in
> `docs/ASSETS.md` den Abschnitt „Environment objects (prompt 007)“ mit „Structure:
> variants“, „Materials and shaders“ und den Budgets, in `docs/ARCHITECTURE.md` „Map
> objects and decoration“ (darin „Snow on fallen objects (prompt 056)“ und „Winter“),
> „Lighting, atmosphere and biomes“ und „Weather, mud and time of day“, dazu bei
> „Terrain rendering“ die Ergebnisse zum Flimmern (Prompt 068 und 071). Dann den Code:
> `snow_cover()` in `godot/assets/shaders/env_common.gdshaderinc`, der Zweig „15
> Roof“ in `env.gdshader`, `env_look.gd` (`LOOK`, `snow`), `_apply_look` in
> `godot/scenes/world/map_objects.gd`, `weather_look.gd` (`snow_cover`) und die
> Dachfunktionen in `tools/blender/kit/buildings.py`.

**Vorlage:** `.temp/schnee_auf_dächern.png` (nicht im Repo, `.temp/` ist ignoriert).
Ein Bildschirmfoto aus einem anderen Strategiespiel, 858 × 475 Pixel: zwei Häuser
mit Satteldach in einem verschneiten Dorf, aus der Nähe. Liegt die Datei nicht vor,
reicht die Beschreibung in Abschnitt 2.

## 1. Ziel

Auf Winterkarten liegt der Schnee auf den Dächern wie in der Vorlage: als dicke,
weiße Decke mit klarem Rand, die an First und Traufe liegen geblieben und dazwischen
in Bahnen abgerutscht ist, mit Wülsten an der Dachkante und Hauben auf Schornstein
und Gauben.

Heute mischt `snow_cover()` auf allem, was nach oben zeigt, die Farbe mit etwas
Rauschen zu Weiß. Ein Dach wird davon gleichmäßig blass und fleckig; der Schnee hat
keinen Rand, keine Dicke und keine Form.

Fertig ist der Prompt, wenn

- ein Dorf im Winter neben der Vorlage liegt, bei ähnlichem Blickwinkel, und der
  Schnee in Form, Rand, Dicke und Verteilung dazu passt,
- jede Dachart ihren Schnee hat (steil, flach, Stroh, Blech, Turm, Flachdach) und
  kein Dach wie das Nachbardach aussieht,
- der Schnee bei 15, 35, 110 und 350 m Kameraabstand stimmt und beim Schwenken
  nichts flimmert,
- wenig Schnee (dünne Decke) und viel Schnee (dicke Decke) beide gut aussehen,
- Brand, Schaden und Ruinen weiter stimmen,
- die Bildrate über einem Winterdorf vorher und nachher gemessen ist,
- `just check`, `just validate-assets` und `just selftest` grün sind.

## 2. Die Vorlage

Das Bild ist **Vorbild für Form und Wirkung**, kein Material: keine Bildausschnitte
und keine Texturen daraus, nichts aus anderen Spielen (`docs/SOURCES.md`). Alle
Muster kommen aus dem Shader, alle Formen aus dem Baukasten. Schrift und Symbole im
Bild gehören nicht dazu.

Was den Schnee auf den Dächern ausmacht:

- **Decke oder nichts.** Eine Stelle ist entweder weiß oder zeigt das Dach. Es gibt
  keinen halb durchscheinenden Schleier. Der Rand ist klar und unregelmäßig.
- **Am First und an der Traufe bleibt er liegen.** Oben ein breites Band entlang
  des Firsts, unten ein breites Band an der Traufe; dazwischen liegt das Dach in
  Bahnen frei, wo der Schnee abgerutscht ist.
- **Abrutschkanten.** Der untere Rand der oberen Decke ist gezackt und gebuchtet:
  Zungen hängen nach unten, einzelne Schollen liegen auf der freien Dachfläche.
- **Dicke.** An Traufe und Giebel steht die Decke über die Dachkante und zeigt
  ihre Stirnseite als weißen Wulst. Man sieht, dass der Schnee eine Handbreit bis
  einen Fuß dick ist.
- **Hauben.** Auf Schornstein, Lüftern, Gauben und allem, was aus dem Dach ragt,
  sitzt eine eigene Schneehaube.
- **Schatten im Schnee.** Die Decke ist nicht rein weiß: im Schatten bläulich, an
  der Abrutschkante mit dunkler Linie darunter.
- **Rundherum.** Am Fuß der Hauswände liegt abgerutschter Schnee in Haufen; Bäume
  tragen Schnee auf den Ästen; der Boden zeigt Spuren.

## 3. Ausgangslage

- `snow_cover(albedo, rough, normal, p, amount)` in `env_common.gdshaderinc`: Menge
  mal `smoothstep(0.45, 0.8, normal.y + Rauschen)`, dann Mischung der Farbe zu
  Weiß. Aufgerufen in `env.gdshader` für alle Slots außer Glas und Boden, nach
  Staub und vor Brand.
- Die Menge `env_snow` kommt je Objekt aus dem Biom (`env_look.gd`: Winter 0,85).
  Laub und Nadeln haben ihren eigenen Schnee in `env_foliage.gdshader`.
- „Snow on winter maps comes from the biome look, not from extra variants“
  (`docs/ASSETS.md`): es gibt keine Schnee-Geometrie.
- Der Shader weiß nicht, wo First und Traufe eines Dachs liegen. Prompt 075
  (Patina) hat dieselbe Frage; sieh nach, ob er erledigt ist und ob er diese
  Angaben ins Modell geschrieben hat.

Halte den Stand vor der ersten Änderung als Bildreihe fest (Abschnitt 7).

## 4. Was zu bauen ist

**Stufe 1: Schnee im Shader**, ohne Modelle anzufassen. Ein eigener Schnee für
Dächer (Slot Roof und andere Dachflächen), der `snow_cover()` dort ersetzt:

| Schicht | Wirkung |
|---|---|
| Deckung mit klarem Rand | Schwellwert statt Mischung: Schnee oder Dach, mit einem schmalen, nach Pixelgröße geglätteten Rand. |
| Verteilung auf der Dachfläche | Liegt am First und an der Traufe, rutscht dazwischen ab. Wie viel frei liegt, hängt an der Neigung (steil hält weniger), an der Menge und am Haus. |
| Abrutschkanten | Gezackte, gebuchtete Ränder in Richtung der Dachneigung, einzelne Schollen darunter. |
| Dicke vortäuschen | Am Rand der Decke eine aufgehellte Kante und darunter eine schmale dunkle Linie, dazu eine leicht gewölbte Normale, damit das Licht die Decke als Körper zeichnet. |
| Schneefarbe | Nicht reines Weiß: leichtes Korn, im Schatten bläulich, auf der Sonnenseite warm. Kein Überstrahlen bei Mittagssonne. |
| Sonne und Wind | Auf der Sonnenseite etwas weniger Schnee, auf der Windseite etwas mehr, aus der Richtung der Dachfläche zur Sonne und zum Wind des Bioms. |
| Je Haus anders | Lage und Form der freien Bahnen aus `v_seed`, der Position des Hauses. |

Die Menge `env_snow` steuert alles von einer dünnen, löchrigen Decke bis zu einem
ganz bedeckten Dach. Bei 0,85 (Winterkarte heute) soll es wie in der Vorlage
aussehen.

**Je Dachart:**

- **Satteldach mit Ziegeln, Schiefer, Brettern:** alle Schichten; das Dach der
  Vorlage.
- **Stroh:** hält den Schnee besser, kaum freie Bahnen, weiche Ränder.
- **Blech** (Halle, Depot): rutscht stark ab, lange freie Bahnen, Wulst an der
  Traufe.
- **Flachdächer** (Wohnblöcke aus Prompt 078, falls vorhanden): geschlossene Decke,
  an der Brüstung angeweht, frei um Schornsteine und Lüfter.
- **Kirchturm und steile Hauben:** fast frei, Schnee nur auf Simsen und Absätzen.
- **Was aus dem Dach ragt** (Schornstein, Gaube, Lüfter): Haube obenauf, soweit der
  Shader es über die Normale kann.

Alles, was kein Dach ist (Mauerkronen, Zäune, Fensterbänke, Felsen, Requisiten),
behält `snow_cover()`. Bekommt es mit wenig Aufwand denselben klaren Rand, umso
besser; sonst bleibt es, wie es ist.

**Stufe 2: Schnee mit Dicke im Modell**, nur wenn Stufe 1 im Vergleich sichtbar zu
kurz greift. Die Dicke an der Dachkante und die Hauben kann ein Shader auf der
Dachfläche nicht zeigen. Dann erzeugt der Baukasten je Gebäude ein eigenes Teil
`<Variant>_Snow`: Schneepolster an First und Traufe mit Überstand über die
Dachkante, Hauben auf Schornstein und Gauben, nur auf Winterkarten sichtbar. Das
heißt: alle Häuser neu bauen (`just env`), Budget und Prüfung in `validate_env.py`
und `docs/ASSETS.md` anpassen, im Spiel je Biom ein- und ausblenden, neue
Vorschaubilder. Zeig vorher die Bilder von Stufe 1 und begründe, warum sie nicht
reichen. Hat Prompt 075 schon Abstand zu First und Traufe ins Modell geschrieben,
nutzt Stufe 1 das mit.

**Schnee am Hausfuß.** Abgerutschter Schnee als flache Haufen an den Traufseiten der
Häuser, wenn es mit der vorhandenen Dekoration geht (sie kennt die Grundrisse der
Objekte, `DecoRasters`). Sonst offener Punkt.

## 5. Regeln

- **Fest am Haus.** Jede Schicht hängt nur von der Position am Modell, der Normale
  und `v_seed` ab. Dasselbe Haus sieht immer gleich aus, und nichts wandert bei
  Kamerabewegung.
- **Nichts flimmert.** Weiße Ränder auf dunklem Dach sind der härteste Kontrast im
  Bild. Der Rand wird nach der Pixelgröße geglättet; was kleiner als ein Pixel ist,
  geht in einen Deckungsanteil über. In der Ferne bleibt die Verteilung (oben und
  unten weiß, Mitte frei) erkennbar.
- **Reihenfolge.** Dach mit Patina, dann Staub, dann Schnee, dann Brand. Wo ein
  Dach brennt oder verkohlt ist (`env_char`), liegt kein Schnee. Beschädigte Dächer
  (`Damaged`) tragen Schnee auf dem, was vom Dach steht; in Löchern keiner.
- **Umgestürztes** behält den Schnee, wo er lag (Prompt 056, `v_rest_normal`); das
  darf nicht brechen.
- **Kosten.** `env.gdshader` zeichnet alle Umwelt-Objekte; der neue Schnee kostet
  nur auf Dachpixeln und nur, wenn `env_snow` über null ist. Schattendurchgänge
  bleiben unberührt.
- **Nur Darstellung.** Kein Rust-Code in `crates/sim`, der goldene Hash bleibt
  gleich.
- **Nicht gleichzeitig mit Prompt 075** im selben Arbeitsverzeichnis: beide ändern
  den Dach-Zweig von `env.gdshader`. Nacheinander oder in eigenen Worktrees.

## 6. Was nicht kaputtgehen darf

- Wald- und Wüstenkarten sehen aus wie vorher (`env_snow` = 0).
- Schnee auf Bäumen, Nadeln, Boden und Fahrzeugen bleibt und passt in Helligkeit
  und Farbton zum neuen Dachschnee. Fällt der Dachschnee daneben auf, gleiche die
  Farben an, nicht die Formen.
- Stroh und Heu behalten ihre geringere Schneemenge (`_apply_look`).

## 7. Prüfen und Abschluss

1. **Vergleichsbild.** Zwei Häuser auf `winter_village` bei ähnlichem Blickwinkel
   und Abstand neben der Vorlage, vorher und nachher. Nach jeder Änderungsrunde
   neu.
2. **Bildreihe.** Eine Auswahl für `just objects-shots` auf einer Winterkarte: das
   Dorf bei 15, 35, 110 und 350 m, eine Reihe mit allen Dacharten, beschädigte und
   brennende Häuser, eine Ruine, Tag, flache Sonne, Nacht, mit und ohne Schneefall;
   dazu dieselbe Reihe mit Schneemengen 0,3, 0,6, 0,85 und 1,0.
3. **Vielfalt.** Zwanzig gleiche Häuser nebeneinander: keine zwei Dächer gleich.
4. **Bewegung.** Kamerafahrt über das Winterdorf aufnehmen und ansehen; mit dem
   Flimmermaß aus Prompt 071 (`tools/ground_flicker.py`) vorher und nachher messen,
   Messfenster auf einer Dachfläche.
5. **Bildrate.** `just objects-bench` auf einer Winterkarte vorher und nachher.
6. Eine Wald- und eine Wüstenkarte ansehen: unverändert.
7. `just check`, `just validate-assets`, `just selftest`.

Die Bilder dem Nutzer zeigen. Was nicht erreicht wurde, steht im Abschlussbericht mit
Grund, nicht als erledigt.

- **Vorschaubilder.** Sieh nach, in welchem Biom die Vorschaubilder der Gebäude
  entstehen. Zeigen sie keinen Schnee und wurde kein Modell neu gebaut, bleiben sie.
  Nach Stufe 2 alle Gebäude neu rendern (`just thumbnails --only=<id>,<id>`), dann
  `just units-web-sync`.
- `docs/ASSETS.md` („Materials and shaders“: Dachschnee, Schichten; nach Stufe 2 das
  Teil `_Snow`, Budget und Prüfung, und den Satz streichen, dass Schnee nie aus
  eigenen Teilen kommt), `docs/ARCHITECTURE.md` („Map objects and decoration“:
  Winter, Messwerte, offene Punkte). Auf Englisch, ohne das Bild der Vorlage.
- `prompts/README.md`: Status auf „erledigt“ mit einer Zeile Ergebnis, die Datei mit
  `git mv` nach `prompts/done/`.
- Commits auf Englisch, in sinnvollen Schritten. Kein `git push`.

## 8. Nicht Teil dieses Prompts

Als offene Punkte notieren, nicht umsetzen:

- **Schnee, der im Spiel wächst, taut oder abrutscht** (bei Schneefall dicker, bei
  Brand und Beschuss vom Dach fallend).
- **Eiszapfen, Rauch aus Schornsteinen, getaute Ringe um warme Schornsteine.**
- **Schnee auf Fahrzeugen und Soldaten**, Spuren im Schnee, geräumte Wege.
- **Schneeverwehungen im Gelände** an Zäunen, Hecken und Mauern.
