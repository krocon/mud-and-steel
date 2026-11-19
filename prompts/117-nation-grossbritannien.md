# Prompt 117 – Vierte Nation: Großbritannien (Schwerpunkt 1944: Fallschirmtruppen und Panzer)

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `prompts/done/003-masstab-und-einheitenkatalog.md`, `data/catalog/SCHEMA.md`,
> `data/catalog/us.toml` ganz (als Muster), `docs/SCALE.md`, `docs/SOURCES.md`,
> `docs/ASSETS.md`, `docs/INFANTRY.md`, `docs/TEAMS.md`, `docs/GAME_DESIGN.md`
> (Fraktionen, Wirtschaft, Fabriken) und `docs/UNIT_CATALOG.md`. Dann alle Stellen,
> an denen eine Nation fest verdrahtet ist: `rg -l 'Faction::|"su"|"us"' crates godot
> web tools` (heute gut 70 Dateien, darunter `crates/sim/src/catalog.rs`,
> `economy.rs`, `air.rs`, `strike.rs`, `scenario.rs`, `crates/net/src/lobby/wire.rs`,
> `crates/bridge/src/match_funcs.rs`, `godot/scenes/menu/match_setup.gd`,
> `godot/scenes/hud/*`, `godot/scenes/audio/battle_voices.gd`,
> `godot/assets/models/infantry/uniform_palettes.json`, die Tarnmaterialien unter
> `godot/assets/materials/camo/`).
>
> **Dieser Prompt läuft in Stufen** (Abschnitt 6). Er ist groß; nach jeder Stufe
> Zwischenstand zeigen und weiterarbeiten. Läuft die Session zu lang, an einer
> Stufengrenze anhalten und den Rest als neuen Prompt vorschlagen.

## 1. Ziel

Eine vierte Nation `uk` (Großbritannien und Commonwealth, Stand 1944), spielbar wie
`de`, `su` und `us`: im Menü wählbar, mit Basis, Fabriken, Wirtschaft, KI,
Stimmen und Bildern. Vorrang haben die Einheiten, die Prompt 118 (Arnhem) braucht:
die britischen Fallschirmtruppen und die Panzer des XXX Corps. Danach kann Prompt 116
(Nijmegen) die Gardetruppen statt der US-Panzer bekommen.

## 2. Einheiten (erste Ausbaustufe)

Echte Werte mit Quellen wie in jedem Katalog (`docs/SOURCES.md`: keine Daten aus
anderen Spielen). Vorschlag, die Session prüft gegen die Kategorien in `SCHEMA.md`
und ergänzt, was für eine vollständige Fraktion fehlt (z. B. Pioniere, Aufklärung,
Lkw, Zugmaschine):

- **Infanterie:** Schützengruppe (Lee-Enfield No. 4, Bren, Sten), Fallschirmjäger-
  Gruppe (Airborne, Sten, Bren, Gammon-Bomben), PIAT-Trupp, Vickers-MG-Trupp,
  3-Zoll-Mörser, 2-Zoll-Mörser, Scharfschütze, Pioniere, Offizier/Funker nach dem
  Muster der anderen Nationen.
- **Geschütze:** 6-Pfünder-Pak (auch luftlandefähig), 17-Pfünder-Pak, 25-Pfünder
  Feldkanone-Haubitze, 75-mm-Pack-Howitzer der Airborne Light Regiment.
- **Fahrzeuge:** Jeep (Airborne), Universal Carrier, Lkw (z. B. Bedford QL),
  Zugmaschine (z. B. Morris C8 „Quad“), Daimler Armoured Car, Panzer Cromwell,
  Sherman Firefly, Churchill, dazu Sexton (Selbstfahrlafette) und M10 Achilles
  (Jagdpanzer), soweit Zeit bleibt.
- **Luft:** Transportflugzeug (Dakota, gleiches Muster wie `us_c47`) und
  Lastensegler Horsa für die Luftlandung, wenn das Luftlande-System aus 055 Segler
  ohne großen Umbau tragen kann; sonst nur die Dakota und Segler als offener Punkt.

## 3. Aussehen

- **Modelle** aus dem Baukasten wie die anderen Nationen: Uniform Battledress mit
  Brodie-Helm (Mk II/Mk III), Airborne mit Denison-Smock und eigenem Helm,
  Ausrüstung 37er Muster. Neue Uniform-Paletten in `uniform_palettes.json`.
- **Fahrzeuge** in den drei Tarnungen (Wald, Wüste, Winter) über das
  Material-System. Britisches Grün (SCC 15 für 1944) als Grundton.
- **Hoheitszeichen:** Kokarde bzw. die weißen Sterne der alliierten Fahrzeuge 1944
  nach Quelle; keine Verbandsabzeichen echter Einheiten nötig.
- Vorschaubilder jeder Einheit (`just thumbnails --only=…`), `just units-web-sync`,
  `just catalog-doc`.

## 4. Spiel

- Fabriken, Bauliste und Preise nach dem Muster der anderen Nationen, abgestimmt
  über die Wirtschaftsregeln (`docs/GAME_DESIGN.md`).
- KI spielt die Nation (Bauliste, Gewichtung); KI-Turnier `uk` gegen jede andere
  Nation, Ergebnis in `docs/AI.md`. Krasse Ungleichgewichte beschreiben und
  maßvoll nachjustieren (nur Preise und Bauliste, keine Katalogwerte verfälschen).
- Teams: `uk` kann mit `us` und `su` in einem Team spielen (`docs/TEAMS.md`).
- **Stimmen:** Britisches Englisch (Freesound/CC0 laut `docs/AUDIO_LIBRARY.md`;
  „British soldier voices“ ist dort als Kandidat aufgeführt). Ohne geeignete Quelle
  vorerst die US-Stimmen und als offenen Punkt notieren.
- Menü, Lobby (LAN/online, `crates/net` Wire-Format), Einheitenbildschirm,
  HUD-Farben/Abzeichen für die neue Nation.

## 5. Prüfen

- `just check`, `just selftest`, `just lan-selftest`, `just export-selftest`.
- Golden Hash: Eine neue Nation darf bestehende Partien nicht ändern. Ändert sich
  der Hash, die Ursache finden (z. B. Reihenfolge im Katalog) und beheben, statt
  ihn neu zu setzen.
- Wire-Format der Lobby: Versionsnummer erhöhen, falls die Nation dort kodiert wird.

## 6. Stufen

1. **Verdrahtung:** `Faction::Uk` überall, Menü, Lobby, leerer Katalog mit einer
   Einheit, Tests grün.
2. **Infanterie und Geschütze** mit Modellen und Bildern.
3. **Fahrzeuge** mit Modellen und Bildern.
4. **Wirtschaft, Fabriken, KI, Stimmen, Turnier, Doku.**

## 7. Nicht Teil dieses Prompts

- Weitere Commonwealth-Länder als eigene Nationen (Kanada, Polen in britischer
  Ausrüstung) und Sonderfahrzeuge (Crocodile, Kangaroo, AVRE).
- Mission für Arnhem (Prompt 118).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
