# Prompt 059 – Wüstendorf: Häuser, Gehöfte und Oase

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ASSETS.md`, `docs/SOURCES.md`, `docs/MAPS.md`, `docs/ARCHITECTURE.md`
> (Kartenobjekte, Zerstörung, Infanterie in Gebäuden, Biome) und das Ergebnis von 041.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
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
