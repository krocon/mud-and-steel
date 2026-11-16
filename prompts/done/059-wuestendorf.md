# Prompt 059 – Wüstendorf: Häuser, Gehöfte und Oase

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ASSETS.md`, `docs/SOURCES.md`, `docs/MAPS.md`, `docs/ARCHITECTURE.md`
> (Kartenobjekte, Zerstörung, Infanterie in Gebäuden, Biome) und das Ergebnis von 041.
>
> Konkretisiert (siehe unten). Ursprünglich Entwurf: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

Die Wüstenkarten brauchen glaubwürdige nordafrikanische Dörfer (Libyen, Tunesien, Ägypten
1940–43): dicht stehende Flachdachhäuser aus Lehmziegel und verputztem Stein, jedes mit
ummauertem Hof, dazwischen schmale Gassen, Palmenhaine und Brunnen. Referenz ist ein
Screenshot des Nutzers (nicht im Repo): nur die Wirkung zählt, Modelle und Details
entstehen neu im Baukasten.

## 2. Was die Wirkung trägt

- **Gehöfte als Einheit:** Haus + Hofmauer (unterschiedlich hoch, teils verfallen) +
  Hoftor; Mauern blockieren Fahrzeuge und geben Infanterie Deckung.
- **Flachdächer mit Brüstung**, Dachaufbauten (Treppenhaus, Dachkammer), Treppe zum Dach;
  Dächer als Feuerstellungen für Infanterie (022).
- **Abwechslung:** Grundrisse in mehreren Größen (1–3 Räume, L-Form, zweigeschossig),
  sandfarbene bis ockerne Putzfarben, abgeplatzter Putz mit Ziegeln darunter.
- **Leben im Hof, zeitgemäß:** Stoffsonnensegel in gedeckten Farben, Palmwedel-Unterstände,
  Tonkrüge, Wasserschläuche, Holzkarren, Brunnen mit Holzgestell, Futterkrippe, Feuerholz.
  Nichts Modernes (keine Kunststofftanks, Satellitenschüsseln, Klimaanlagen).
- **Umgebung:** Dattelpalmen in Hainen und einzeln, Dornbüsche, Felsblöcke, Sandverwehungen
  an Mauern, Reifen- und Kettenspuren im Sand (Darstellung), lange harte Schatten.
- **Größere Bauten:** kleine Moschee mit Minarett (ohne Schrift/Symbole), Marktplatz mit
  Ständen, italienisches Kolonial-Fort/Polizeiposten, Zisterne, Ölmühle.

## 3. Umfang

1. **Baukasten-Teile** (Wand mit Putz, Brüstung, Hofmauer, Tor, Treppe, Sonnensegel,
   Krüge, Palmwedeldach) und daraus 8–12 Haustypen plus Gehöft-Varianten, je mit
   Intact/Damaged/Ruin (020), Polygon-Budgets aus `docs/ASSETS.md`, echte Maße mit Quelle
   (Fakten aus Literatur/Fotos als Referenz, nichts kopiert).
2. **Katalog:** `data/objects.toml`, `data/destruction.toml` (Lehm: weniger
   Lebenspunkte als Stein), besetzbare Fensterplätze und Dachplätze für 022.
3. **Editor (028):** Gehöfte als Gruppe platzierbar und drehbar, Pinsel „Dorf“ verteilt
   Gehöfte mit Gassen dazwischen (deterministisch aus Seed).
4. **Karte:** ein Wüstendorf an einer Piste mit Oase und Palmenhain für 2 und 4 Spieler,
   Versorgungspunkte an Dorfplatz, Brunnen und Fort.
5. **Darstellung:** Wüstenlicht aus 015, Staub bei Treffern in Sandfarbe, Sandverwehungen.

## 4. Akzeptanzkriterien

1. `just kit-test`, `just validate-assets`, `just gallery-check` grün; Galerie-Bilder
   aller Haustypen.
2. `just map-check` der neuen Karte, `just editor-selftest`, `just selftest` grün.
3. Benchmark auf der Dorfkarte im Budget von 016.
4. Quellen in `docs/SOURCES.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach 041/057): Kartenobjekte mit Regeln pro Katalog-Id
(`data/objects.toml`, Zerstörung 020, Besetzen 022 als Kapazität und
Trefferanteil pro Gebäude-Id – Fensterplätze als eigene Positionen gibt es
nicht), Biom `desert` mit Licht, Staub in Sandfarbe und trockenen Pflanzen
(015, 019), ein einzelnes `env_house_mud` (007), Palmen, Feuer (041),
Objektgruppen in der Objektliste des Editors (041), `cli gen-town` als
Vorbild für eigene Kartengeneratoren. Der Editor kennt nur Einzelobjekte
und den Streupinsel.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Priorität:** Baukasten-Teile, zehn Haustypen plus Moschee und Polizeiposten mit Intact/Damaged/Ruin, Hofmauer und Hof-Requisiten, Gehöfte, Editor-Pinsel „Dorf“, eine Dorfkarte mit Oase für 2 und 4 Spieler. Rest offen (Ergebnis). | Auftrag. |
| 2 | **Jeder Haustyp ist eine eigene Katalog-Id** (`env_house_desert_*`), nicht eine Variante: Lebenspunkte, Kapazität, Sicht und Grundriss hängen an der Id; die Schadensstufen sind die Varianten. Präfix `env_house` bekommt so Sichtweite und Schatten der Häuser in `map_objects.gd` ohne Änderung. | Bestehende Regeln wiederverwenden, keine Sim-Änderung. |
| 3 | **Ein Gehöft ist eine Gruppe gewöhnlicher Kartenobjekte** (Haus, Mauersegmente, Ecken, Tor, Requisiten), beschrieben in `data/compounds.toml` und ausgelegt von `editor::compound`; die Sim sieht nur die Objekte. Hofmauer aus 3-m-Segmenten, Teile im Haus entfallen. Drehen in 90°-Schritten. **[vorläufig]** | Keine neue Sim-Regel, kein Hash-Einfluss; der Editor ist ganzzahlig, 90° braucht keine Trigonometrie und passt zu Gassen und Wegegitter. |
| 4 | **Hofmauern blockieren Fahrzeuge, nicht Infanterie** (wie die Trockenmauer), auch Tor und verfallene Segmente; Deckung über „blockiert Panzer“. Infanterie überklettert 2,2-m-Lehmmauern. **[vorläufig]** | Ein 3-m-Tor in einem 4-m-Wegegitter wäre für Infanterie sonst nicht sicher passierbar. |
| 5 | **Lehm hält weniger als Stein:** Häuser 1 400–3 200 HP (Backstein 3 000, Stein 3 500), Mauer 800 (Trockenmauer 1 200), Moschee 4 500, Polizeiposten 5 500. Kapazitäten 4–20 inkl. Dachterrasse, Trefferanteile 8–25 % (schwer 25–50 %). Lehmhäuser brennen nicht (keine Feuerklasse). **[vorläufig]** | Spielregeln, keine historischen Werte. |
| 6 | **Dachplätze** sind in der Kapazität enthalten, nicht als eigene Positionen. **[vorläufig]** | 022 kennt keine Positionen im Gebäude. |
| 7 | **Modelle:** neuer Generator `desert_house` (Raumblöcke mit Brüstung, Dachkammer, Außentreppe, Gewölbe, Arkade, Balkon, Balkenköpfe, Wasserspeier, Läden statt Glas, abgeplatzter Putz, Sandverwehungen im `_Floor`), Tür/Hofseite immer +X; Mauer, Requisiten, Moschee (ohne Schrift/Symbole/Halbmond), Polizeiposten (ohne Hoheitszeichen) in `desert_props.py`. Budgets: Haus 3 000, Moschee/Posten 5 000 (wie Kirche), Requisiten 800. Zwei Subagenten parallel. | Baukasten, Budgets aus `docs/ASSETS.md`. |
| 8 | **AO-Bake pro Variante** in `env.bake_env` (alle Varianten stehen am Ursprung und schatteten sich gegenseitig). Nur neu gebaute Modelle ändern sich. | Fehler im Baukasten, beim Bauen gefunden. |
| 9 | **Karte:** `cli gen-village` → `desert_village`, 800 m, vier Starts in zwei Teams (2 Spieler nutzen 0 und 1), Piste durch den Dorfplatz, Moschee, Ladenzeilen, Brunnen, ~130 Gehöfte aus dem Dorf-Pinsel, Oase mit Teich, Palmenhain und Brunnen im Westen, Polizeiposten im Osten, Versorgungspunkte Dorfplatz/Brunnen/Fort. Objekte über `EditMap` (gleicher Code wie der Editor). | Reproduzierbar, Pinsel wird mitgetestet. |
| 10 | **Darstellung:** Wüstenlicht, Staub in Sandfarbe und Biom-Look gibt es schon (015/019); Sandverwehungen als Modellteile an Mauern und Häusern, lose Sandflächen als Gelände. Keine Änderung an Shadern. | Vorhanden; gemeinsame Dateien klein halten. |

## Ergebnis

- **Modelle** (`tools/blender/kit/desert.py`, `desert_props.py`, 18
  Baupläne): `env_house_desert_{hut,two,long,corner,stair,roofroom,tall,
  vault,large,shop}`, `env_mosque`, `env_police_post` (je Intact/Damaged/Ruin),
  `env_wall_mud` (Segment/Low/Broken/Gate/End/Corner), `env_awning`
  (Cloth/Wide/Palm), `env_jars`, `env_well_desert`, `env_cart_wood`,
  `env_market_stall`; Häuser 552–2 802 Dreiecke, Moschee 3 612, Posten 4 416.
- **Regeln:** `data/objects.toml`, `destruction.toml`, `infantry.toml`
  (Kapazität, Deckung hinter Krügen/Karren/Ständen), `vision.toml`.
- **Editor:** `data/compounds.toml` (9 Gehöfte, Moschee, Polizeiposten,
  Ladenzeile), Werkzeuge „Place compound“ und „Village (desert)“, Gruppe
  „Desert village“ in der Objektliste; Tests in `crates/editor` und im
  Editor-Selbsttest.
- **Karte:** `data/maps/desert_village` (3 349 Objekte, 130 Gehöfte, 400
  Palmen); `just map-check desert_village` ok.
- **Messung** (M1 Max): `objects-bench` auf der Dorfkarte 1080p: nah
  112 FPS, 120 m 91 FPS, Übersicht 600 m 66 FPS (3 265 Draw Calls), Dorf aus
  150 m 60 FPS. `path-bench desert_village 200`: Planung im Mittel 11 ms/Tick
  (max 52 ms), alle Ticks im Mittel 1,2 ms (vergleichbar mit `town_small`).
- **Golden-Hashes** bewusst aktualisiert (Regeltabellen werden als Ganzes
  gehasht): allein Demo `0x8ab2aa30ae0b23b5`, Basisbau `0x3e262432a66238fd`;
  nach dem Merge mit `main` (035/036/046/048/053) Demo `0x0773c8ed1eac669d`,
  Basisbau `0x787fc4698fa2b0d1`. Die Szenarien nutzen keine Wüstenobjekte,
  die Schlacht ist unverändert.
- **Prüfungen:** `just check`, `kit-test`, `validate-assets`,
  `gallery-check`, `map-check desert_village`, `editor-selftest`,
  `selftest` grün.

**Vorläufig (bitte bestätigen):** Lebenspunkte, Kapazitäten und
Trefferanteile der Wüstenbauten; Hofmauern für Infanterie überkletterbar;
Drehen von Gehöften nur in 90°-Schritten; Schadensanteile des Dorf-Pinsels
(10 % beschädigt, 5 % Ruine, 8 % verfallene Mauern); alle Maße (Schätzwerte).

**Offen:**
- Ölmühle, Zisterne, Futterkrippe, Wasserschläuche als eigenes Objekt,
  Dornbusch-Modell (die Karte nutzt `env_bush` mit Wüstenlook), Reifen- und
  Kettenspuren im Sand, Treffer-Staub an Lehmwänden (Putzstaub).
- Dachplätze als eigene Feuerstellungen (022 kennt keine Positionen),
  Treppe zum Dach als Weg; Sonnensegel und Palmwedeldächer brennen nicht.
- Gehöfte frei drehbar (nicht nur 90°); Gehöft-Vorschau im Editor vor dem
  Klick.
- Optik nur in Blender-Renders und Godot-Aufnahmen aus der Vogelperspektive
  geprüft; Galerie in Godot nicht visuell abgenommen.
