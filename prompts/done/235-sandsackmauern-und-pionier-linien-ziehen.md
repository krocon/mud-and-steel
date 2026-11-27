# Prompt 235 – Sandsackmauern und Linien-Bau per Ziehen für Pioniere

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md` („Engineers“), `docs/ARCHITECTURE.md` („Field works“,
> „Positions“), `data/works.toml`, `data/infantry.toml` (`fort_sandbag_position`)
> und `data/destruction.toml`.
> Hinweis: Prompt 230 verschönert ausschließlich das 3D-Modell der bestehenden
> `vehicle_pit` (Panzer-/Geschützstellung); dieser Prompt ergänzt dagegen eine
> **lineare Sandsackmauer für Infanterie** in `data/works.toml` sowie eine
> **flüssige Zieh-Bedienung** für alle linearen Pionierbauten.
> Im Code:
> `crates/sim/src/works.rs`,
> `crates/sim/src/sim/works.rs`,
> `crates/sim/src/command.rs`,
> `crates/bridge/src/`,
> `godot/scenes/selection/engineer_orders.gd`,
> `godot/scenes/selection/selection.gd`,
> `godot/scenes/world/works_view.gd`,
> `godot/scenes/hud/command_bar.gd`,
> `godot/scenes/hud/hud_icons.gd`.
>
> **Stil-/Spielgefühl-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `01:30–01:36` und
> `01:58–02:06` (Pioniere ziehen per gedrückter Maustaste eine grüne
> segmentierte Sandsack-Linie über Straße/Feld mit direkter Anzeige
> „Sandbags · 12 sections · 240 supply“ am Cursor; geplante Abschnitte liegen
> als klar lesbare helle Baustellen-Markierung am Boden und füllen sich
> abschnittsweise mit Sandsäcken).
> **Keine Bilder, Namen oder Assets aus dem Video ins Repo.**
> **Hinweis zu Golden Hashes:** Neues Pionier-Bauwerk `sandbags` in
> `data/works.toml` / `crates/sim` – `works.toml` ist Teil des State-Hash,
> daher werden sich die Golden Hashes ändern und müssen nach Prüfung bewusst
> aktualisiert und im Commit benannt werden.

## 1. Problem

1. **Keine linearen Sandsackmauern für Pioniere:**
   - Das Katalogobjekt `fort_sandbag_position` existiert bereits samt
     Deckungsregel (`pct = 80` in `data/infantry.toml`), aber Pioniere können
     in `data/works.toml` bisher **keine Sandsacklinien** errichten – nur
     zeitaufwendige Gräben, Stacheldraht, Spanische Reiter oder einzelne
     Schützenlöcher. Eine schnelle, günstige Infanterie-Brustwehr entlang einer
     Straße oder Heckenlücke fehlt.
2. **Umständliche 2-Klick-Linienbedienung (`engineer_orders.gd`):**
   - Aktuell erfordern alle linearen Pionierbauten (Draht, Reiter, Graben,
     Verbindungsgraben, Höcker) **zwei getrennte Linksklicks** (Startpunkt
     klicken, Maus bewegen, Endpunkt klicken).
   - Beim schnellen Gefechtsbau fühlt sich **Linksklick halten und ziehen**
     (mit Einzelklick als kurzer Standard-Abschnitt) deutlich direkter und
     intuitiver an; außerdem zeigt die aktuelle Vorschau in `works_view.gd`
     nur einen flachen gelben Streifen statt klar gegliederter Einzelabschnitte
     mit Gültig-/Ungültig-Farbe und gut lesbarem Cursor-Label.

## 2. Ziel

1. **Neues Pionier-Bauwerk `sandbags` in `data/works.toml` und `crates/sim`:**
   - Neuer Eintrag `[work.sandbags]` (Werte `[provisional]`):
     - Niedrige Kosten (z. B. `[0, 0, 5]` Mannstärke oder `[0, 0, 4]`),
       schnelle Bauzeit pro Abschnitt (z. B. `build_s = 5–6 s`),
       `clear_s = 4`, `piece_mm = 3000` (passend zur Breite von
       `fort_sandbag_position`).
     - Fertige Abschnitte platzieren `fort_sandbag_position` sauber ausgerichtet
       entlang der gezogenen Linie (quer zur Bau-Normale), geben sofort
       **schwere Infanterie-Deckung** (`pct = 80`, harmoniert direkt mit
       Prompt 233!) und blockieren Fahrzeuge bis zu einer moderaten
       Masse – **schwere Panzer überrollen/plätten Sandsäcke** (analog zu
       Lehmmauern / `crush` in `data/works.toml` bzw. `data/destruction.toml`),
       und Artillerietreffer zerstören einzelne Abschnitte.
     - Infanterie kann Sandsacklinien überspringen/passieren (blockiert keine
       eigenen Soldatenwege, sondern dient als Deckungslinie!).
   - In `command_bar.gd`, `engineer_orders.gd`, `hud_icons.gd` und
     `godot/locale/` als Pionier-Kachel **„Sandsäcke“** ergänzen.
2. **Flüssiges Ziehen für alle linearen Pionierbauten (`engineer_orders.gd`, `selection.gd`):**
   - Gilt einheitlich für `sandbags`, `wire`, `hedgehogs`, `trench`,
     `comm_trench` und `teeth` (Bunker bleibt Startpunkt + Blickrichtung):
     - **Linksklick drücken + ziehen + loslassen:** startet die Linie am
       Druckpunkt und schickt den Bauauftrag direkt beim Loslassen ab!
     - **Kurzer Einzelklick (Ziehstrecke < z. B. 1,5 m):** legt automatisch
       einen kurzen Standard-Abschnitt (z. B. 2–3 Stücke quer zur Blickrichtung
       der Pioniere oder Kamera) an – kein versehentliches Hängenbleiben im
       Wartemodus.
     - **Abbrechen:** Rechtsklick oder `Esc` während des Ziehens bricht sauber
       ab, ohne einen Auftrag zu senden; `Shift` gehalten erlaubt mehrere
       Linien hintereinander.
   - Sicherstellen, dass das Ziehen im Pionier-Modus **keinen** Auswahlrahmen
     in `selection.gd` auslöst.
3. **Präzise Abschnittsvorschau & Baustellen-Darstellung (`works_view.gd`):**
   - Während des Ziehens zeichnet `works_view.gd` die geplanten Stücke als
     **klar segmentierte Linie auf dem Boden** (entlang der Geländehöhe
     `sim_host.height_at`, über dem Nebel des Krieges):
     - **Grün** für gültige/bezahlbare Abschnitte, **Rot/Orange** bei
       unpassierbarem Gelände, zu steiler Böschung, Wasser oder unzureichenden
       Ressourcen (Prüfung kommt aus der Simulation!).
   - Direkt am Ende der gezogenen Linie steht ein sauber lesbares 2D-HUD-Label
     (statt nur winzigem 3D-Label): z. B.
     `„Sandsäcke · 8 Abschnitte · 40 Mann“` (lokalisiert, skaliert mit
     `hud_theme.gd`).
   - **Geplante / im Bau befindliche Abschnitte:** Eine gut sichtbare, aber
     stimmige Baustellen-Linie auf dem Boden, die sich **Abschnitt für
     Abschnitt** mit den fertigen `fort_sandbag_position`-Modellen füllt,
     während die Pioniere die Linie entlangarbeiten.
4. **KI-Hinweis (`crates/ai`):**
   - Kurz prüfen, ob Pionier-KI-Skripte unverändert weiterlaufen (optional:
     Pioniere legen an defensiven Schlüsselpunkten kurze Sandsacklinien an).

## 3. Prüfen und zeigen

- **Simulations-Tests (`crates/sim`):**
  - Pioniere bauen eine 3–5-teilige Sandsacklinie; fertige Stücke liefern
    Deckungsplätze (`fort_sandbag_position`), blockieren keine Infanterie-Wege,
    werden von einem schweren Panzer geplättet und von HE zerstört.
  - Kosten, Abbruch/Erstattung (`abandon_s`) und `max_sites` greifen wie bei
    anderen Pionierbauten.
  - Golden Hashes bewusst aktualisieren (durch neues `[work.sandbags]` in
    `works.toml`).
- **Sichtprüfung im Spiel (Screenshots / kurzer Clip):**
  - Ziehen einer Sandsack- und Stacheldraht-Linie mit segmentierter grüner/roter
    Vorschau und Cursor-Kostenanzeige.
  - Pioniere bauen die Linie Abschnitt für Abschnitt auf; Infanterie geht direkt
    dahinter in Deckung.
- **Tests & Doku:**
  - `just check`, `just selftest` grün (inkl. Selbsttest für Ziehen +
    Einzelklick-Kurzabschnitt + Abbruch per Rechtsklick).
  - `docs/CONTROLS.md`, `docs/ARCHITECTURE.md` und `godot/locale/` (DE + EN)
    aktualisieren.

## 4. Nicht Teil dieses Prompts

- Umbau der 3D-Geometrie von `vehicle_pit` (Prompt 230).
- Deckungs-Schildsymbole und Deckungs-Zielmodus (Prompt 233).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/235-sandsackmauern-und-pionier-linien-ziehen.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.

## Ergebnis (Nachtlauf 27.11.2026)

- **Simulation & Regeln (`data/works.toml`, `crates/sim`):**
  - Neues Pionier-Bauwerk `WorkKind::Sandbags` / `LayKind::Sandbags` (`code = 13`, `[work.sandbags]`: `cost = [0, 0, 5]`, `build_s = 5`, `clear_s = 4`, `piece_mm = 3000`; `[crush] sandbags_kg = 12000`, vorläufig (Nachtlauf)).
  - `PathGrid` erweitert um `work_crush_kg` und `Crushing::work_limits`: Infanterie (`SizeClass::Small`) passiert Sandsackmauern ungehindert, leichte Fahrzeuge `< 12 t` werden blockiert, mittlere/schwere Panzer `>= 12 t` können durchrouten und walzen berührte Sandsackabschnitte platt (und geben deren Zellen sofort wieder frei). HE-Granatkrater zerstören fertige Sandsackabschnitte im Explosionsradius; Pioniere können Sandsäcke räumen.
  - Vollständige Deckungs-Integration (`Sim::sandbag_spots`, `Sim::sandbag_cover_absorbs` mit 80 % wie `fort_sandbag_position`, `Sim::near_sandbag`, `Sim::nearest_sandbag_facing`) in `infantry.rs`, `combat.rs` und `cover_order.rs` (schwere Deckung Grad 2 inkl. Deckungsvorschau & grünem Deckungsschild).
  - Read-only `Sim::work_line_preview(player, kind, from, to)` ohne Zustands-/RNG-Mutation prüft Geländegültigkeit (`1`), `max_sites` & kumulative Ressourcen (`2`) und gültige/bezahlbare Abschnitte (`0`).
- **Bridge & Godot UI:**
  - `SimHost::order_works` um `"sandbags"` erweitert, `SimHost::work_line_preview` neu; Icon `work_sandbags` (`hud_icons.gd`) + Kachel in `command_bar.gd` (Seite `works`, Slot 8); `works_view.gd` setzt `fort_sandbag_position` (`Position`, +90° entlang des Segments) sowie eine Baustellen-Markierung (`SANDBAG_SITE`).
  - Flüssiges **Linksklick-Drücken-Ziehen-Loslassen** (`engineer_orders.gd`, `selection.gd`) für alle linearen Pionierbauten (`sandbags`, `wire`, `hedgehogs`, `trench`, `comm_trench`, `teeth`) ohne Kastenauswahl-Konflikt; kurzer Klick `< 1,5 m` legt ein 2-Abschnitt-Standardstück quer zur Pionier-/Kamerarichtung; Rechtsklick / `Esc` bricht Ziehen ab; `Umschalt` hält Modus aktiv.
  - Terrainfolgende segmentierte Grün/Amber/Rot-Vorschau + `HudTheme`-skaliertes 2D-Cursor-Label in `order_preview.gd`; DE/EN-Übersetzungen in `godot/locale/hud.csv` (1462 Keys, 0 Probleme); `docs/CONTROLS.md` & `docs/ARCHITECTURE.md` aktualisiert.
- **Prüfung:**
  - 4 neue Tests in `crates/sim/tests/sandbags.rs` + 12/12 in `works.rs` grün.
  - Golden Hashes bewusst aktualisiert wegen gehashter `[work.sandbags]`- und `crush.sandbags_kg`-Regeln in `WorksRules::hash_into`: `determinism.rs` `0x462d_25c7_d644_3026` (Demo weiterhin Tick 625), `economy.rs` `0x37de_a01e_1753_cb4e`.
  - `just check` und `just selftest` (inkl. `works_self_test`, `positions_self_test`, `flow_self_test`, Riders, Gun Crews, Boats) komplett grün.
- **Vorläufige Entscheidungen (Nachtlauf):** `[work.sandbags]` (`cost = [0, 0, 5]`, `build_s = 5`, `clear_s = 4`, `piece_mm = 3000`), `crush.sandbags_kg = 12000`, `SHORT_DRAG_M = 1.5 m` (2 Abschnitte Standard), noch keine Hotkeys für einzelne Pionierwerke. Offen: Ingame-Screenshot der gezogenen Sandsacklinie im Fenster.
