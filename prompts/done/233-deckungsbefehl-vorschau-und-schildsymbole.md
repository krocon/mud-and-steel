# Prompt 233 – Gezielter Deckungsbefehl, Deckungsvorschau und Deckungsschilde

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/CONTROLS.md`, `docs/ARCHITECTURE.md`
> („Infantry“, „Selection“, „HUD“) sowie `data/infantry.toml` (`[cover]`) und
> `data/works.toml` (`[dig]`, `[positions]`).
> Im Code:
> `crates/sim/src/sim/infantry.rs` (Deckungsplatz-Suche `cover_spot`,
> `cover_spots_near`, `assign_cover`),
> `crates/sim/src/command.rs`,
> `crates/bridge/src/` (wie `formation_preview` an Godot gereicht wird),
> `godot/scenes/selection/selection.gd`,
> `godot/scenes/hud/order_preview.gd`,
> `godot/scenes/hud/command_bar.gd`,
> `godot/scenes/hud/state_marks.gd`,
> `godot/scenes/hud/hud_icons.gd`,
> `godot/scenes/hud/relation_colors.gd`,
> `godot/scenes/world/combat_view.gd`.
>
> **Stil-/Spielgefühl-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `01:58–02:06` und
> `02:34–02:48` (Deckungs-Modus zeigt beim Überfahren von Sandsäcken, Mauern
> und Hecken grüne/gelbe Einzelpunkte pro Soldat mit kleiner Blickrichtungs-
> Markierung genau auf der gewählten Seite + „Take cover here“; besetzte Trupps
> tragen ein grünes/gelbes Deckungsschild neben ihrem Balken).
> **Keine Bilder, Namen oder Assets aus dem Video ins Repo.**
> **Hinweis zu Golden Hashes:** Neuer Befehl `TakeCover` in `crates/sim` –
> bestehende Replays ohne diesen Befehl bleiben unverändert; prüfen, ob Golden
> Hashes bit-identisch bleiben (sonst bewusst dokumentieren).

## 1. Problem

In *Mud and Steel* existiert bereits eine detaillierte Simulationslogik für
Deckung (`sim/infantry.rs`: feste Hindernisse, weiche Deckung in
`data/infantry.toml`, Krater, Gräben), aber sie ist für den Spieler im Gefecht
**kaum sichtbar und nicht direkt steuerbar**:

1. **Keine Deckungsvorschau:** Beim Bewegen oder Halten der rechten Maustaste
   sieht man nur einen pauschalen Trupp-Kreis (`order_preview.gd`), aber nicht,
   **wo** die einzelnen Soldaten hinter einer Mauer, Hecke, Sandsackstellung
   oder in einem Graben Platz finden und **wie gut** die Deckung dort ist.
2. **Kein gezielter Deckungsbefehl:** Will man einen Trupp exakt auf die
   *richtige Seite* einer Mauer oder Hecke legen (z. B. geschützt vor Feuer aus
   Norden), muss man per Rechtsklick-Ziehen raten. Eine schnelle Taste für
   „nächste Deckung aufsuchen“ fehlt ebenfalls.
3. **Kein sofort lesbarer Deckungsstatus über dem Trupp:** Ohne den Trupp
   anzuklicken und ins Detail zu schauen, erkennt man aus Spielhöhe nicht auf
   einen Blick, ob ein eigener oder feindlicher Trupp in schwerer Deckung
   (Mauer, Sandsack, Graben), leichter Deckung (Hecke, Busch, Krater) oder
   völlig offen steht.

## 2. Ziel

Deckung wird zu einem direkt lesbaren und präzise steuerbaren Kernwerkzeug der
Infanterietaktik – bei strikter **Single-Source-of-Truth** in `crates/sim`
(keine doppelte Deckungs-Geometrie in GDScript!):

1. **Simulations-Vorschau & Deckungsgrad in `crates/sim` + `crates/bridge`:**
   - Die bestehende Deckungsplatz-Logik in `crates/sim/src/sim/infantry.rs`
     wird als deterministische Vorschau-Abfrage verfügbar gemacht:
     `Sim::cover_preview(units, target, facing)` (analog zu
     `SimHost::formation_preview` in Prompt 099).
   - Rückgabe pro Soldatenplatz: Weltposition `(x, z)`, Blickrichtung
     `(fx, fz)` (weg vom Hindernis Richtung Feindseite) und **Deckungsstufe**:
     - `0` = offen / keine Deckung,
     - `1` = **leichte Deckung** (weiche Deckung `< 60 %`, z. B. Busch,
       Hecke, Baumstumpf, flacher Krater, Holzstapel),
     - `2` = **schwere Deckung** (feste Hindernisse wie Stein-/Lehmmauer,
       Sandsäcke `>= 60 %`, Graben, Schützenloch, MG-Nest, Unterstand, Ruine).
   - **Deckungsgrad pro Trupp im Snapshot (`SimHost::unit_cover_grade`):**
     Liefert für jeden Trupp kompakt `0 / 1 / 2` (z. B. Mehrheits- oder
     Durchschnitts-Deckungsstufe der lebenden Soldaten an Deckungsplätzen;
     besetzte Gebäude = `2` bzw. eigenes Gebäude-Symbol).
2. **Gezielter Deckungsbefehl (`TakeCover`) in `crates/sim`:**
   - Neuer deterministischer Befehl `Command::TakeCover { units, target, facing }`
     (im Lockstep-Protokoll & Replay serialisiert):
     - Der Trupp rückt zum gewählten Deckungsobjekt (Mauer, Hecke, Sandsack,
       Graben, Krater) vor und verteilt seine Soldaten gezielt auf die
       berechneten Deckungsplätze **auf der angeklickten Seite** mit Front über
       die Deckung hinweg.
     - **Schnellfunktion „Nächste Deckung“:** Erneutes Drücken der Deckungstaste
       (oder Klick auf den eigenen Standort) sucht deterministisch in der
       Simulation die nächste geeignete Deckung im Umfeld (z. B. 25 m) und
       schickt den Trupp direkt dorthin.
   - Bonus-Komfort: Auch beim normalen **Rechtsklick-Halten / -Ziehen** mit
     reiner Infanterie-Auswahl in der Nähe einer Mauer, Hecke, Sandsacklinie
     oder eines Grabens zeigt `order_preview.gd` direkt die tatsächlichen
     Soldaten-Deckungspunkte an, sobald `cover_preview` dort Deckungsplätze
     findet!
3. **Interaktive Deckungsvorschau in Godot (`selection.gd`, `order_preview.gd`, `command_bar.gd`):**
   - Neue Kontext-Kachel **„Deckung“** in der Befehlsleiste für Infanterie mit
     umbelegbarer Taste (prüfen, welche Taste frei ist, z. B. `Shift+V` oder
     Kontext-Taste – Konflikte mit `C` Kamera folgen und `Z/X/V/B` Haltungen in
     `docs/CONTROLS.md` vermeiden!).
   - Im Deckungs-Zielmodus (und beim Rechtsklick-Halten über Deckung):
     - Jeder berechnete Soldatenplatz erscheint als klarer runder Punkt mit
       kurzem Richtungsstrich in Schussrichtung: **grün** für schwere Deckung,
       **gelb/ocker** für leichte Deckung (Farben über `relation_colors.gd` /
       Farbsehschwächen-Paletten abstimmbar!).
     - Ein knapper Hinweis direkt am Ziel (`„Hier Deckung nehmen“` bzw.
       `„Schwere Deckung“` / `„Leichte Deckung“`, lokalisiert) bestätigt die
       Auswahl.
     - Bei Wahl der anderen Seite einer Mauer/Sandsacklinie springen die Punkte
       sofort sichtbar auf diese Seite um.
4. **Deckungsschild-Symbole über Trupps (`state_marks.gd` / `combat_view.gd`):**
   - Kompakter **Deckungsschild** (grün = schwere Deckung, gelb = leichte
     Deckung) sauber neben dem Lebensbalken eines Trupps (nicht mit dem
     2-Slot-Limit kritischer Warnungen wie `burning`/`suppressed` konkurrierend,
     sondern als dezentes eigenes Mini-Schild direkt am Balkenende).
   - Sichtbar für eigene, verbündete und **aufgeklärte feindliche Trupps**
     (streng Nebel-des-Krieges-treu!).
   - Respektiert bestehende Einstellungen (`state_marks`, `health_bars`, `F1`
     HUD aus, Fotomodus, Trailer-Shots ohne HUD).

## 3. Prüfen und zeigen

- **Simulations-Tests (`crates/sim`):**
  - `cover_preview` und `TakeCover` liefern bit-identische Plätze auf beiden
    Seiten einer Mauer/Hecke/Sandsacklinie; schwere vs. leichte Deckung
    korrekt klassifiziert; „Nächste Deckung“ findet deterministisch Deckung;
    `unit_cover_grade` wechselt von `0` → `1`/`2` bei Ankunft.
- **Sichtprüfung im Spiel (Screenshots in `build/cover-preview/`):**
  - Deckungsvorschau an einer Hecke (gelb), einer Steinmauer / Sandsacklinie
    (grün) und einem Graben – jeweils von beiden Seiten.
  - Trupp in schwerer und leichter Deckung mit sichtbarem Schildsymbol neben
    dem Balken (auch im Farbsehschwächen-Modus prüfen).
- **Performance & Tests:**
  - `cover_preview` nur bei Cursorbewegung > `REFETCH_M` abrufen (wie
    `formation_preview` in `order_preview.gd`); `just bench-view` ohne
    Regression.
  - `just check`, `just selftest`, `just infantry-selftest` grün.
  - `docs/CONTROLS.md`, `docs/INFANTRY.md`, `docs/ARCHITECTURE.md` und
    `godot/locale/` aktualisieren.

## 4. Nicht Teil dieses Prompts

- Neubau von Sandsacklinien durch Pioniere (Prompt 235).
- Infanterie-Handgranaten (Prompt 232) oder Rückzugsbefehl (Prompt 234).
- Änderungen an bestehenden Deckungs-Prozentwerten in `data/infantry.toml`.

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/233-deckungsbefehl-vorschau-und-schildsymbole.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.

## Ergebnis (Nachtlauf 27.11.2026)

- **Simulation & Single Source of Truth (`crates/sim/src/command.rs`, `crates/sim/src/sim/cover_order.rs`, `infantry.rs`, `drill.rs`):**
  - Neuer Befehl `Command::TakeCover { units, target, facing }` + `Sim::cover_preview` und `Sim::unit_cover_grade` (`0` offen, `1` leichte Deckung `< 60 %` / Hecken / Zäune / Krater, `2` schwere Deckung Mauern / Sandsäcke `>= 60 %` / Gräben / Schützenlöcher / MG-Nester / Unterstände / besetzte Gebäude).
  - `cover_preview` und `order_take_cover` teilen dieselben Hilfsfunktionen (`spots_behind`, `crater_spots`, `trench_spots`, `choose_spots_for`) und drehen die Deckungs-Blickrichtung automatisch über das nächste Hindernis hinweg (wechselt beim Überfahren auf die andere Mauerseite sofort um).
  - Klick auf den eigenen Standort (≤ 2 m) bzw. zweiter Druck der Deckungstaste löst deterministisch `find_nearest_cover` (25 m, bevorzugt schwere vor leichter Deckung) aus; `turn_to_facing` setzt `spots_at = None`, damit Soldaten exakt auf die vorgezeigten Deckungsplätze einrücken.
  - **Golden Hashes unverändert** (`determinism.rs` & `economy.rs` bit-identisch).
- **Bridge & Godot (`crates/bridge/src/infantry_funcs.rs`, `godot/scenes/hud/`, `selection.gd`, `settings.gd`):**
  - Bridge: `take_cover`, `cover_preview` (5 Floats je Soldatenplatz), `unit_cover_grades` (strikt nebelgefiltert aus `self.current`).
  - Kontext-Kachel „Deckung“ + umbelegbare Taste `Shift+V` (`order_cover`, Doppeldruck = nächste Deckung); `order_preview.gd` zeichnet Soldaten-Deckungspunkte mit Blickrichtungsstrich und Label („Schwere Deckung“ / „Leichte Deckung“ / „Keine Deckung hier“), gedrosselt über `REFETCH_M`, auch beim Rechtsklick-Halten mit reiner Infanterie über Deckung.
  - Kompakte palettenfähige Deckungsschilde (`RelationColors.cover_heavy` / `cover_light`) links neben dem Lebensbalken in `state_marks.gd` für eigene, verbündete und aufgeklärte gegnerische Trupps (konkurriert nicht mit den 2 Warn-Slots über dem Balken; respektiert `state_marks`, Nebel, `F1`, Fotomodus).
  - DE/EN-Übersetzungen (`hud.csv`, `settings.csv`) und Doku (`INFANTRY.md`, `CONTROLS.md`, `ARCHITECTURE.md`) aktualisiert.
- **Prüfung:**
  - `crates/sim/tests/cover_order.rs` (3 Tests: Seitenwechsel & schwere vs. leichte Deckung, `TakeCover` entspricht `cover_preview` & `unit_cover_grade` `0 → 2`, Doppeldruck „Nächste Deckung“ & Determinismus).
  - `just check` (inkl. `l10n_check` 1452 Keys / 0 Probleme), `just infantry-selftest` und `just selftest` komplett grün.
- **Vorläufig (Nachtlauf):** Taste `Shift+V`, 25-m-Suchradius für „Nächste Deckung“, Schwellenwert `>= 60 %` für schwere Deckung, Schild-Position links neben dem Balken. Offen: visuelle Screenshots in `build/cover-preview/` im Fenster.
