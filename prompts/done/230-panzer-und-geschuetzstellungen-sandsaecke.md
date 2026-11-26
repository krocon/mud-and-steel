# Prompt 230 – U-förmige Panzer- und Geschützstellungen mit Sandsackwall

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` („Positions“), `data/works.toml` (`[positions]`),
> `godot/scenes/world/trench_builder.gd` (insb. `nest()` und `pit()`) sowie
> `godot/scenes/world/works_view.gd` (`_work_node`, `_draw_dug_in`).
>
> **Stil-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/152 mm howitzer-gun M1937 (ML-20) ｜ Sudden Strike 5 #shorts #shortsvideo #gaming [l77ywaqaGrg].webm`
> (rechts/oben neben der Haubitzen-Batterie steht ein eingegrabenes
> Panzerfahrzeug in einer markanten hufeisen-/U-förmigen Sandsack- und
> Holzstellung mit offener Rückseite).
> **Keine Bilder, Texturen oder Assets aus dem Video ins Repo.**
> Reine Darstellung – **keine Änderungen in `crates/sim`**, Golden Hashes
> unverändert.

## 1. Problem

Pioniere können seit Prompt 052 eine **Fahrzeugdeckung (`vehicle_pit`)** bauen
(`data/works.toml`), in der Fahrzeuge eingegraben („hull-down“) stehen.
Während das MG-Nest (`TrenchBuilder.nest()`) in `trench_builder.gd` bereits ein
stimmiges, hinten offenes Sandsack-Hufeisen mit Außenböschung zeichnet, besteht
`TrenchBuilder.pit()` (Zeilen 353–378) bisher nur aus einem **geschlossenen
360°-Erdoval** ohne Einfahrt, ohne Sandsäcke und ohne innere Holzverschalung.
Fahrzeuge fahren optisch durch einen 1 m hohen geschlossenen Erdwall hinein,
und die Stellung wirkt aus Spielhöhe wie ein formloser Erdhaufen statt wie eine
militärische Panzer-/Geschützstellung.

## 2. Ziel

Reine Darstellung in `trench_builder.gd` und `works_view.gd` (keine
Regeländerung in `crates/sim`, `works.toml` und Golden Hashes unverändert):

1. **Hufeisen-/U-förmige Stellung in `TrenchBuilder.pit()` (`godot/scenes/world/trench_builder.gd`):**
   - Statt eines geschlossenen 360°-Rings wird `pit(sim_host, at, facing, damaged)`
     als **hinten offene U-/Hufeisenstellung** gebaut (~240–260° umschlossen,
     hinten eine ~3,6–4,0 m breite, flach abfallende Einfahrtsrampe aus
     festgefahrener Erde/Holzbohlen).
   - **Wandaufbau (im Stil von `trench()` und `nest()`, dasselbe gemeinsame
     Vertex-Color-Material `TrenchBuilder.material()` → 0 neue Materialien oder
     Draw-Calls):**
     - Innenseite: **Holzbohlen-/Rundholz-Verschalung** mit senkrechten
       Stützpfählen (wie in `trench()`), leicht abgesenkter dunkler
       Stellungsboden.
     - Krone: **3–4 versetzte Lagen Sandsäcke** (`_bag()`) entlang der Front und
       der beiden Flanken (ca. 1,0–1,15 m Gesamthöhe, passend zur
       Wannenhöhe eines mittleren Panzers bzw. zum Schutzschild einer
       Haubitze).
     - Außenseite: schräg angeschüttete, leicht unregelmäßige **Erdböschung**
       (`SPOIL`) bis zum umliegenden Gelände (`sim_host.height_at`), die sich
       weich in das Terrain einfügt.
     - **Beschädigter Zustand (`damaged == true`):** Bei Artillerietreffern
       (über das bestehende `sim_host.damaged_works()`) brechen einzelne obere
       Sandsäcke weg, versengte Stellen (`SCORCH`) und verstreute Säcke
       erscheinen an der Böschung (analog zu `nest()`/`trench()`).
2. **Sinnvolle Ausrichtung & Zusammenspiel mit eingegrabenen Einheiten (`works_view.gd`):**
   - `works_view.gd` bestimmt die Frontrichtung `facing` der Stellung
     deterministisch ohne Simulationsänderung:
     - Wenn in der Nähe eine eigene Grabenlinie liegt, zeigt die Front analog zu
       `_away_from_trench()` nach außen weg vom Graben;
     - sonst – falls eine eigene Einheit in der Stellung steht – initial nach
       deren Blickrichtung (oder deterministisch zur Kartenmitte, falls leer).
   - Prüfen, wie der bestehende `_draw_dug_in()`-Erdkragen (`_collars`) mit der
     neuen U-Stellung zusammenwirkt: doppelte/clippende Erdwälle vermeiden (wenn
     die Einheit bereits in einer `vehicle_pit` steht, reicht die neue Stellung
     bzw. ein dezenter innerer Erd-Ansatz statt eines zweiten Walls quer durch
     die Sandsäcke).
3. **Gelände- & Biom-Verträglichkeit:**
   - Auf flachem wie leicht geneigtem Gelände (`surface_at` +
     `sim_host.height_at`) keine schwebenden Sandsäcke oder Löcher; in Wald,
     Wüste und Winter stimmig (bestehende `TrenchBuilder`-Farbpalette prüfen).

## 3. Prüfen und zeigen

- **Vorher/Nachher-Bilder** (nah und aus Spielhöhe):
  - Leere `vehicle_pit` (Einfahrt hinten, Holzverschalung innen, Sandsackwall
    und Außenböschung klar lesbar).
  - Eingegrabener Panzer (z. B. `su_t34_76_1943` oder `de_pz4_h`) und ein
    Geschütz (z. B. `su_m30` / `de_pak40`) in der Stellung – Turm/Rohr ragen
    sauber über die Sandsäcke, kein Clipping mit dem alten Erdkragen.
  - Beschädigte Stellung nach Artillerietreffer.
- **Tests & Performance:**
  - `just check`, `just selftest` (inkl. Feldstellungs-/Pionier-Checks) grün;
    Golden Hashes unverändert.
  - Vertex-/Dreieckszahl einer `vehicle_pit` schlank halten (im Bereich von
    `nest()`/`trench()`).

## 4. Nicht Teil dieses Prompts

- Änderungen an Deckungswerten, Bauzeiten, Kosten oder Eingrab-Regeln für
  gezogene Geschütze in `crates/sim` / `data/works.toml`.
- Neue Karten oder Skizzen-Platzierungen (bestehende `[[field_work]]`-Stellungen
  profitieren automatisch).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/230-panzer-und-geschuetzstellungen-sandsaecke.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.

## Ergebnis (Nachtlauf 26.11.2026)

- **U-/Hufeisenstellung in `godot/scenes/world/trench_builder.gd` (`TrenchBuilder.pit`):**
  - Geschlossenes 360°-Erdoval durch eine hinten offene U-Stellung (`256°` umschlossen, Innenradien `2.95 × 3.65 m`, ~`4.0 m` breite offene Rückseite) mit gemeinsamem `TrenchBuilder.material()` (0 neue Materialien/Draw-Calls) ersetzt.
  - Wandaufbau: abgesenkter, dem ausgehobenen Gelände folgender dunkler Boden (`MUD`), innere 3-reihige Holzbohlen-Verschalung (`PLANK`/`PLANK_DARK`) mit Stützpfählen (`POST`), geschlossene Seitenwangen hinten, 3–4 versetzte Sandsacklagen (`~1.05–1.15 m` Fronthöhe mit 4. Lage im ±85°-Frontbogen), weich ins Terrain übergehende Erdböschung (`SPOIL` auf `sim_host.height_at`) sowie eine hinten ansteigende Einfahrtsrampe (`PATH`) mit 4 quergelagerten Holzbohlen.
  - Beschädigte Variante (`damaged == true`): deterministische Trefferlücke im Front-/Flankenbogen mit fehlenden oberen Sandsacklagen, geschwärzter Verschalung/Boden (`SCORCH`) und 7 verstreuten Sandsäcken.
  - Kompakte Geometrie: `1 119` Dreiecke intakt, `1 099` Dreiecke beschädigt (1 Draw-Call).
- **Ausrichtung & Kragen-Koordination (`godot/scenes/world/works_view.gd`):**
  - `_pit_facing()` bestimmt die Frontrichtung deterministisch: weg von einem nahen fertigen Graben (`≤ 45 m`), sonst initiale Blickrichtung einer darin stehenden Einheit (`sim_host.unit_facing_yaws()`), sonst zur Kartenmitte `(0, 0)`.
  - `_draw_dug_in()` überspringt den alten runden `DugInCollar`-Torus, sobald die eingegrabene Einheit in einer gebauten `vehicle_pit` (`_in_vehicle_pit()`, `≤ 4.5 m`) steht → kein Clipping mehr durch Sandsäcke oder Rückrampe (`COLLARS=0` bei eingegrabenem Panzer IV).
- **Prüfung:**
  - `godot/scenes/tools/positions_self_test.gd` prüft zusätzlich eine gebaute `vehicle_pit`.
  - Visuell geprüft mit leerer Stellung, eingegrabenem Panzer IV Ausf. H und beschädigter Stellung (`DUG_IN=[0]`, `COLLARS=0`, `DAMAGED=[2]`).
  - `just check` und `just selftest` vollständig grün; `crates/sim` und Golden Hashes unverändert.
- **Vorläufige Entscheidungen (Nachtlauf):**
  - Umschließungswinkel `256°` (`±128°`), Innenradien `2.95 × 3.65 m`, Graben-Suchradius `45 m` für `_pit_facing()`.
