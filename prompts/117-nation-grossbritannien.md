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

| # | Entscheidung (vorläufig) | Begründung |
|---|---|---|
| 1 | `Faction::Uk` steht im Enum **nach** `Neutral`, `uk.toml` wird als letzte Katalogdatei geladen | Reihenfolge der alten Einträge, Typ- und Flugzeugindizes bleiben gleich |
| 2 | Regeldaten einer **neuen** Nation (Bauliste, Befestigungen, Flugzeuge, Raketenkaliber, Bunker der Pioniere) behält eine Partie nur, wenn die Nation mitspielt (`economy::keeps_faction`, `ALWAYS_KEPT = de, su, us`) | sonst ändern sich Hash und Regeln jeder alten Partie; so bleibt der Golden Hash ohne Neusetzen gleich |
| 3 | Fallschirmjäger: neue Liste `paratroopers` in `data/air.toml` (`uk_inf_para_section`), gehasht nur die Typen, die die Partie hat | Dakota setzt die Airborne-Gruppe ab statt der Schützengruppe; alte Partien unverändert |
| 4 | Lastensegler Horsa **nicht** gebaut | das Luftlandesystem aus 055 kennt nur Absprung, keine Landung; offener Punkt |
| 5 | Kein Raketenwerfer (`rocket_launcher`): Land Mattress war ein gezogener Werfer; die KI baut an seiner Stelle den schweren Panzer. Schwerer Schlag: Kaliber 127 mm (5-Zoll-Granaten als Gefechtskopf; Quellen uneins) | kein passendes Fahrzeug 1944 |
| 6 | Leichtes MG-Team = **Bren-Gruppe** (3 Mann) der Schützengruppe | die Briten hatten kein eigenes LMG-Team |
| 7 | Universal Carrier in der Kategorie `halftrack` (Truppen und Zugmittel) | Kategorie „Schützenpanzer/Zugmaschine“ der KI-Liste |
| 8 | Bauliste der Fahrzeuge in den Plätzen der anderen Nationen (0 Daimler, 1 Carrier, 2 Cromwell, 3 Achilles, 4 Churchill, 5 Sexton, 6 Bedford, 7 6-Pfünder, 8 25-Pfünder, 9 Valentine-Brückenleger, 10 Cromwell VI, dann Firefly, 17-Pfünder, Morris Quad, Jeep, Pack Howitzer); Infanterie dazu am Ende die Airborne-Gruppe | die KI-Mischungen indizieren die Listen |
| 9 | Vollständige Fraktion zusätzlich mit Stuart VI (leichter Panzer), 4,2-Zoll-Mörser, Bofors, 3,7-Zoll-Flak, Bunker und Minenfeldern | jede Nation deckt jede militärische Kategorie ab (Test) |
| 10 | Fahrzeugkennzeichen: weißer alliierter Stern **ohne Kreis**; Flugzeuge: RAF-Rundel Typ C1 (`marking = "roundel"`); HUD-Abzeichen: Rundel | Quelle „British military vehicle markings of WW II“ |
| 11 | Tarnung: SCC 15 Olive Drab einfarbig (Wald), Light Stone mit Mustern (Wüste), Weißanstrich (Winter) | Stand 1944 |
| 12 | Uniform: Battledress, Brodie Mk II, 37er Ausrüstung; Airborne-Rollen und Scharfschützen im Denison-Smock (Muster „amoeba“), Airborne-Helm | Baukasten der anderen Nationen, ohne neuen Shader |
| 13 | Stimmen: keine eigenen; `uk` fällt auf die `us`-Stimmen zurück (die auch noch fehlen) | keine geeignete freie Quelle ohne Download des 48-GB-Pakets |
| 14 | Lobby-Wire-Format **nicht** erhöht | die Nation ist dort ein freier String, das Format ändert sich nicht |
| 15 | Modelle teils aus nahen Bauplänen abgeleitet (Firefly aus M4A4, Achilles aus M10, Sexton aus M7, Dakota aus C-47, Auster aus L-4, Spitfire/Typhoon/Mosquito aus P-51/P-47/B-25 skaliert) | Zeit; Silhouetten nur näherungsweise |
| 16 | Panzergruppen der Schlacht: `uk = 112 %` wie die US (Cromwell mit derselben 75-mm-Munition) | siehe Turnier |
