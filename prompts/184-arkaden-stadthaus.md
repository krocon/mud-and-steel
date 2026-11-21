# Prompt 184 – Stadthaus mit Arkaden: Walmdach, Rundbögen und Gesimse im `house`-Generator

> In einer frischen Session im Repo-Ordner ausführen. Empfohlen nach Prompt
> 185 (Vorschau-Blatt) und 186 (feinere Oberflächen). Lies zuerst `CLAUDE.md`,
> dann `docs/ASSETS.md` (Abschnitte „Polygon and texture budgets“, „Modular
> variants“, „Town and winter buildings“ aus Prompt 041 und „Desert village“
> aus Prompt 059), `docs/SOURCES.md`, `docs/SCALE.md` und
> `data/catalog/SCHEMA.md`. Im Code `tools/blender/kit/buildings.py`
> (`House`, `wall()`, `gable_roof()`, `plinth()`, `_spread()`, `house()`),
> zum Vergleich die Bögen in `tools/blender/kit/desert.py` (Block-Art
> `arcade`, Parameter `arches`) und die Bogenreihe der Steinbrücke
> (`buildings.py`, „voussoir band“). Vorbilder für Blueprint und Katalog:
> `assets-src/blueprints/neutral/env_house_town.toml` und der Eintrag
> `env_house_town` in `data/catalog/neutral.toml`.
>
> **Vorlage** (nicht im Repo, `docs/.vorlagen/` ist ignoriert, nur lesen):
> `docs/.vorlagen/arkadenhaus/referenz.webp`: vom Nutzer geliefertes
> Referenzbild vom 2026-11-21 (Ansicht schräg, vier Seitenansichten,
> Draufsicht, drei Schadensstufen). Es dient nur als Referenz für Form und
> Maße. Der Text im Bild („Generator-Prüfung“, Katalog- und Blueprint-Felder)
> ist **nicht** richtig. Der `house`-Generator kann weder Walmdach noch
> Arkaden, und die Felder passen nicht zu `SCHEMA.md`. Nichts daraus wörtlich
> übernehmen.

## 1. Das Gebäude

Ein zweigeschossiges, verputztes Stadthaus einer südeuropäischen Kleinstadt mit
offener Arkade im Erdgeschoss zur Straße.

- **Grundriss:** rechteckig, etwa **14,0 × 9,0 m** (Länge entlang Y, Breite
  entlang X; Straßenseite mit den Arkaden auf **+X**, wie die anderen Häuser).
  Den Grundriss so legen, dass die Arkadenfront die lange Seite ist.
- **Höhe:** Traufe etwa **7,5 m**, First etwa **11,0 m** (`height_mm` =
  Firsthöhe). Zwei Vollgeschosse, das Erdgeschoss höher als das obere.
- **Dach:** **Walmdach** (vier Dachflächen, kurzer First), Tonziegel,
  geringer Dachüberstand mit Traufgesims.
- **Wände:** Putz über Ziegel, Putz stellenweise abgeplatzt mit sichtbarem
  Ziegel; Sockel, Bögen und Pfeiler aus Ziegel bzw. Werkstein.
- **Erdgeschoss +X:** **zwei Rundbögen**, mittig durch einen Pfeiler getrennt,
  dahinter eine offene, begehbar wirkende Halle (Laubengang) mit Rückwand.
  Bogenlichte etwa 2,6 m breit, Kämpfer um 2,2 m, Bogenscheitel entsprechend
  höher; die Session wählt die Maße, so dass die Bögen wie im Bild das
  Erdgeschoss fast ausfüllen.
- **Obergeschoss +X:** fünf hohe Rechteckfenster mit flachem Stichbogen, das
  mittlere als **Doppelfenster**.
- **Seiten ±Y:** je zwei Fenster pro Geschoss. **Rückseite −X:** je zwei
  Fenster pro Geschoss (im Bild links und rechts außen).
- **Gesimsband** zwischen den Geschossen und unter der Traufe, Fensterrahmen
  aus Werkstein. Keine Verglasung, dunkle Öffnungen (wie die anderen
  Häuser); Reste von Holzläden oder Balken nur, wenn der Generator sie schon
  kann.

Alle Werte sind Schätzungen aus dem Bild (±0,5 m) und kommen als `uncertain`
in den Katalog.

## 2. Generator erweitern (nicht neu bauen)

Den bestehenden `house`-Generator in `kit/buildings.py` erweitern, so dass
alle bisherigen Häuser **unverändert** bleiben (neue Parameter mit
Standardwerten, die das heutige Verhalten ergeben):

- **`roof = "hip"`:** Walmdach über dem Rechteck, gleiche Dachneigung auf allen
  vier Seiten, Tonziegel über `roof_kind` wie beim Satteldach, Grate und First
  mit Firstziegeln, Traufe ringsum mit `eave`. Kein Giebel, also keine
  Giebelwand und keine `attic_windows`. Schäden (Löcher, freiliegende Sparren)
  wie beim Satteldach, damit `Damaged` das große Loch mit Balken zeigt.
- **Arkade:** zum Beispiel `arcade = ["+x", count, width, spring]` (Seite,
  Anzahl der Bögen, Lichte, Kämpferhöhe). Rundbögen mit Ziegel-Bogenband
  (Keilsteine), Pfeiler mit Kämpferplatte, Rücksprung zu einer Rückwand
  (Tiefe der Halle als Parameter, Vorschlag 3 m), Boden der Halle in
  `_Floor`. Die Bogenform aus `desert.py` oder der Steinbrücke wiederverwenden,
  wo es geht, statt eine dritte zu schreiben. Die Tür liegt dann in der
  Rückwand der Halle oder entfällt (die Session entscheidet; `door` muss nicht
  gesetzt sein).
- **Gesimse:** zum Beispiel `cornice = true` für ein umlaufendes Band an jeder
  Geschossgrenze und unter der Traufe, plus Werkstein-Fensterrahmen
  (`window_frame = true`). Material und Kind aus den vorhandenen Slots, keine
  neuen Slots.
- **Fenster pro Seite:** heute gelten `windows_long` und `windows_short`
  gleich für beide Längs- bzw. Schmalseiten und alle Geschosse. Für dieses Haus
  braucht es Fenster **pro Seite** (fünf vorne, zwei hinten) und ein
  Doppelfenster in der Mitte. Vorschlag: optional `windows = { "+x" = 5,
  "-x" = 2, ... }` und `double_mid = ["+x"]`; ohne diese Felder gilt das
  Alte. Stichbogen-Sturz als einfache Option (`window_arch = true`), falls
  billig.
- **Schaden:** Die neuen Teile (Bögen, Pfeiler, Gesimse, Walmdach) müssen
  wie alle anderen Teile beschädigt werden. `Ruin` zeigt wie im Bild ein
  eingestürztes Dach und eine weggebrochene Ecke mit Schutt (`_Debris`), eine
  Arkade bleibt stehen.
- Docstring von `house()` mit allen neuen Parametern ergänzen.

## 3. Asset, Katalog, Bilder

- Blueprint `assets-src/blueprints/neutral/env_town_house_arcade.toml` mit den
  Varianten `Intact`, `Damaged`, `Ruin` (gleiches Muster wie
  `env_house_town.toml`, `damage_of`, `fit = false`, `damage = 1/2`).
- Katalogeintrag `env_town_house_arcade` in `data/catalog/neutral.toml` im
  echten Schema: `category = "env_building"`, `name_de = "Stadthaus mit
  Arkaden"`, `name_en = "Town house with arcade"`, `length_mm = 14000`,
  `width_mm = 9000`, `height_mm = 11000`, `uncertain` für die Maße, `note`
  mit „Judgement values“ und Hinweis auf die Referenz.
- **Quelle:** „Eigenes Asset, inspiriert durch Referenzbild“ ist keine Quelle
  im Sinn von `docs/SOURCES.md`. Echte allgemeine Quellen angeben, zum
  Beispiel Wikipedia „Arcade (architecture)“ / „Loggia“ für Laubengänge im
  Erdgeschoss und „Hip roof“ für das Dach, Geschosshöhen wie bei
  `env_house_town`.
- Dreieck-Budget aus `docs/ASSETS.md` für Häuser einhalten (Rathaus und
  Stadthaus als Maßstab) und die Zahl im Ergebnis nennen; die Bögen mit so
  wenigen Segmenten bauen, dass die Silhouette aus Spielhöhe rund wirkt.
- `just export-model` bzw. der übliche Bau der Umwelt-Modelle, `just import`,
  Thumbnail `just thumbnails --only=env_town_house_arcade`,
  `just units-web-sync`, `just catalog-doc`.
- **Prüfen, dass sich die bisherigen Häuser nicht ändern:** alle anderen
  Blueprints mit `part = "house"` neu bauen und Dreieckszahlen bzw. Hash der
  Meshes vorher/nachher vergleichen (oder die Thumbnails von
  `env_house_town`, `env_town_hall`, `env_warehouse`, `env_house_log`,
  `env_house_farm*`, `env_house_stone` vergleichen). Abweichungen nur, wenn
  bewusst und begründet.

## 4. Spiel und Karten

- Die Simulation sieht nur Grundriss und Höhe aus dem Katalog; die Arkade ist
  **rein optisch**, der Grundriss bleibt ein volles Rechteck (keine Durchfahrt).
  Ob Infanterie die Halle als Deckung oder Gebäude nutzen soll, ist **nicht**
  Teil dieses Prompts; als offenen Punkt notieren.
- Das Haus im Karteneditor verfügbar machen wie die anderen Stadthäuser
  (Palette), und auf einer passenden bestehenden Stadtkarte ein bis zwei
  Exemplare setzen, wenn es dort ins Bild passt (die Session entscheidet,
  welche; `just map-check`).
- Golden Hash: Ein neuer Katalogeintrag allein darf ihn nicht ändern; wenn
  eine Karte geändert wird und sich dadurch ein Hash ändert, Ursache nennen.

## 5. Prüfen und zeigen

- Bilder der drei Varianten aus der Thumbnail-Ansicht und eine Ansicht im
  Spiel (Stadtkarte, normaler Zoom) in die Session zeigen; dazu die Ansicht
  von vorn neben das Referenzbild legen.
- `just check`, `just selftest`, `just editor-selftest`.
- Doku: `docs/ASSETS.md` (neue Parameter des `house`-Generators, das neue
  Haus im Abschnitt zu den Stadthäusern), `docs/UNIT_CATALOG.md` über
  `just catalog-doc`.

## 6. Nicht Teil dieses Prompts

- Begehbare Innenräume, Infanterie in der Arkade, Durchfahrt für Fahrzeuge.
- Weitere Varianten (anderer Putz, drei Geschosse); höchstens als Hinweis,
  welche Parameter dafür reichen würden.
- Neue Material-Slots oder Texturen.

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| Frage | Entscheidung | Grund |
|---|---|---|
| Neuer Generator oder Erweiterung | Erweiterung von `house` | Rechteck, Geschosse, Putz, Ziegeldach und Schäden gibt es schon; nur Walmdach, Arkade, Gesimse fehlen |
| Arkade im Spiel | nur optisch, Grundriss bleibt voll | kleinster Schritt, keine Regeländerung |

## Ergebnis

(von der Session auszufüllen)
