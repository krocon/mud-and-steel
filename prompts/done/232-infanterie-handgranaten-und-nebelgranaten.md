# Prompt 232 – Infanterie-Handgranaten und Nebelhandgranaten

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/CONTROLS.md`, `docs/SOURCES.md`,
> `docs/ARCHITECTURE.md` („Indirect fire and smoke“, „Infantry“) sowie
> `data/infantry.toml`, `data/indirect.toml`, `data/logistics.toml`,
> `godot/assets/models/infantry/animations.json` (die Clips `throw_grenade`
> und `throw_grenade_kneel` existieren bereits für alle Soldaten!).
> Im Code:
> `crates/sim/src/command.rs`,
> `crates/sim/src/sim/infantry.rs`,
> `crates/sim/src/sim/indirect.rs`,
> `crates/sim/src/sim/combat.rs`,
> `crates/sim/src/sim/logistics.rs`,
> `crates/bridge/src/`,
> `crates/ai/src/`,
> `godot/scenes/selection/selection.gd`,
> `godot/scenes/hud/command_bar.gd`,
> `godot/scenes/hud/order_preview.gd`,
> `godot/scenes/hud/range_rings.gd`,
> `godot/scenes/world/infantry_view.gd`,
> `godot/scenes/world/artillery_view.gd`,
> `godot/scenes/world/effects.gd`.
>
> **Stil-/Spielgefühl-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `02:42–04:14`
> (Schützentrupps werfen Nebelgranaten, um feindliche MG-Bunker zu blenden,
> rücken unter Nebelschutz vor und schalten Bunker sowie verschanzte Gegner mit
> Splitterhandgranaten aus; Zielkreis + Wurfreichweiten-Ring + automatisches
> Vorrücken bei Klick außerhalb der Wurfreichweite).
> **Keine Bilder, Namen oder Spielwerte aus dem Video ins Repo.**
> **Hinweis zu Golden Hashes:** Neue Spielmechanik in `crates/sim` – Golden
> Hashes prüfen (für Szenarien ohne Granatwurfbefehle idealerweise unverändert,
> ansonsten nach Prüfung bewusst aktualisieren und im Commit benennen).

## 1. Problem

Infanterie hat bei uns bisher **weder Splitterhandgranaten noch
Nebelhandgranaten** – Nebel gibt es nur über Mörser und Artillerie (`G`/`T`),
und gegen Bunker, besetzte Häuser oder Grabenstellungen sind reine
Schützengruppen ohne Panzer- oder Artillerieunterstützung weitgehend machtlos.

Dabei sind die Grundlagen im Projekt bereits weitgehend vorhanden:
- Jeder Soldat besitzt bereits die fertigen Animationsclips `throw_grenade` und
  `throw_grenade_kneel` (`godot/assets/models/infantry/animations.json`).
- Sichtblockierende Rauchwolken (`indirect.rs` + `artillery_view.gd` nach
  Prompt 200) und Splitter-/Druckwellenregeln (`data/indirect.toml`,
  `data/infantry.toml`) existieren bereits.

## 2. Ziel

Ein taktisches, deterministisches Handgranaten- und Nebelgranaten-System für
Infanterietrupps mit klarer, direkter Bedienung:

1. **Regeln & Vorrat in `data/infantry.toml` (`[grenades]`) und `crates/sim`:**
   - **Wer trägt Granaten:** Gefechts-Infanterie (z. B. `inf_rifle`, `inf_smg`,
     `inf_engineer`, `inf_recon`, `inf_command`, Fallschirmjäger) erhält
     einen begrenzten Vorrat an **Splitterhandgranaten** (z. B. 2–3 Würfe pro
     Trupp) und **Nebelhandgranaten** (z. B. 1–2 Würfe pro Trupp). Schwere
     Waffenteams (`inf_hmg`, `inf_mortar_*`), Sanitäter und Scharfschützen
     bleiben ohne (über Kategorieliste in `data/infantry.toml` konfigurierbar).
   - **Historische Einordnung (`docs/SOURCES.md`):** In Doku/Tooltips die
     fraktionsüblichen Typen benennen (z. B. DE Stielhandgranate 24 /
     Nebelhandgranate 39, US Mk 2 / M18 Smoke, SU F-1 / RDG-1, UK Mills
     No. 36M / No. 77) – Spielwerte als einheitliche, balancierte Regelwerte in
     `data/infantry.toml` (`[provisional]`).
   - **Nachschub & Abklingzeit:** Verbrauchte Granaten werden über bestehende
     Versorgungs-Lkw / Munitionsdepots (`logistics.rs`) automatisch
     aufgefüllt; kurze Abklingzeit zwischen zwei Würfen pro Trupp (z. B.
     6–10 s Spielzeit), damit kein sofortiges Dauer-Spamming entsteht.
   - **Wurfreichweite & Ballistik:**
     - Realistische, spielbare Wurfreichweite (z. B. ~20–25 m im Spiel –
       kompakt genug für echten Nahkampf, größer als `direct_fire_min_m = 15 m`).
     - Kurze Flugzeit im hohen Bogen (~1,0–1,4 s Spielzeit) plus kurze
       Zündverzögerung (~0,5 s nach Aufschlag) bei Splittergranaten, sodass
       aufmerksame Spieler/Einheiten im offenen Feld noch reagieren können.
   - **Wirkung:**
     - **Splitterhandgranate:** Hoher Schaden & starke Unterdrückung in kleinem
       Radius (~5–6 m), besonders wirksam gegen **Gräben, Deckungsstellungen,
       besetzte Gebäude** (reduzierte Gebäudedeckung im Nahwurf), **Bunker**
       (kann MG-Bunker bei 1–2 gezielten Treffern ausschalten) sowie offene
       Geschützbedienungen/Halbketten; wirkungslos gegen geschlossene
       Panzerung.
     - **Nebelhandgranate:** Erzeugt eine kompakte, dichte, sichtblockierende
       Nebelwolke (z. B. Radius ~12–15 m, Dauer ~18–25 s Spielzeit) über die
       bestehende `Smoke`-Mechanik in `indirect.rs` – blendet Bunker, MGs und
       Panzer genau lang genug für einen Flanken- oder Sturmangriff.
2. **Automatisches Vorrücken außerhalb der Wurfreichweite (`crates/sim`):**
   - Neue deterministische Befehle `ThrowGrenade { units, target }` und
     `ThrowSmokeGrenade { units, target }` in `crates/sim/src/command.rs`
     (vollständig im Lockstep-Protokoll und in Replays serialisiert).
   - Liegt das angeklickte Ziel **außerhalb der Wurfreichweite**, läuft der
     Trupp automatisch bis an die Wurfreichweite heran, wirft die Granate
     (der nächste lebende Soldat wirft, inkl. Event für die Darstellung) und
     nimmt danach wieder normale Deckung/Kampfhaltung ein.
   - Bei Mehrfachauswahl wirft nur **ein** geeigneter Trupp pro Klick (der
     nächste mit Vorrat & bereitem Cooldown), damit nicht versehentlich 5
     Trupps gleichzeitig ihre gesamte Munition auf denselben Punkt werfen
     (Shift+Klick erlaubt gezielt mehrere Würfe).
3. **Bedienung, Zielvorschau & Animation in Godot:**
   - **Befehlsleiste & Tasten (`command_bar.gd`, `settings.gd`, `docs/CONTROLS.md`):**
     Zwei Kontext-Kacheln für Infanterie mit Vorrat: **Granate** und
     **Nebelgranate** (mit Restanzahl-Zähler auf der Kachel, Abkling-Sektor
     und umbelegbaren Tasten; bestehende Belegungen `G`/`T` für
     Artillerie-Sperrfeuer/-Nebel sauber abstimmen, sodass sie kontextabhängig
     oder konfliktfrei funktionieren).
   - **Zielmodus (`selection.gd`, `order_preview.gd`):**
     - Zeigt den **Wurfreichweiten-Ring** um die ausgewählten Trupps.
     - Zeigt am Mauszeiger einen klaren **Wirkungs-/Nebelradius-Kreis**
       (orange/gelb für Granate, hellweiß für Nebel).
     - Liegt der Mauszeiger außerhalb der Wurfreichweite, wechselt der
       Zielkreis dezent die Farbe und zeigt direkt am Kreis den kurzen Hinweis
       `„Außer Reichweite – Trupp rückt vor“` (lokalisiert in `godot/locale/`).
     - Rechtsklick oder `Esc` bricht den Zielmodus ab.
   - **Darstellung (`infantry_view.gd`, `effects.gd`, `artillery_view.gd`, `battle_sound.gd`):**
     - Der werfende Soldat spielt `throw_grenade` bzw. `throw_grenade_kneel`.
     - Sichtbare, kleine fliegende Granate im Bogen (mit dezentem Funken-/
       Rauchfaden bei der Nebelgranate), passende Explosion/Splitterwolke bzw.
       dichte weiße Nebelwolke nach Prompt 200 und passende Sound-Ereignisse.
4. **KI-Nutzung (`crates/ai`):**
   - Die KI setzt Nebelhandgranaten vor befestigten MG-Nestern/Bunkern oder
     unter starkem Sperrfeuer ein und wirft Splitterhandgranaten gegen
     besetzte Häuser, Bunker und nahe Gegner in Deckung.

## 3. Prüfen und zeigen

- **Simulations- & Integrationstests (`crates/sim`):**
  - Deterministischer Test: Wurf innerhalb Reichweite, automatisches Vorrücken
    außerhalb Reichweite, Vorratsverbrauch & Nachschub via Versorgungs-Lkw,
    Cooldown, Nebel blockiert Sichtlinie eines Bunkers, Splittergranate schadet
    Bunker/Hausbesatzung, Replay-Roundtrip & Determinismus.
- **Sichtprüfung im Spiel (Vorher/Nachher-Bilder / kurzer Clip):**
  - Zielmodus mit Wurfreichweiten-Ring, Wirkungskreis und „Trupp rückt vor“-
    Hinweis.
  - Schützentrupp wirft Nebel vor einen Bunker (`throw_grenade`-Animation
    sichtbar!), rückt im Nebel vor und zerstört den Bunker per Handgranate.
- **Tests & Doku:**
  - `just check`, `just selftest`, `just infantry-selftest` grün.
  - `just ai-tournament` kurz prüfen (keine einseitige Fraktionsverschiebung).
  - `docs/INFANTRY.md`, `docs/CONTROLS.md`, `docs/ARCHITECTURE.md` und
    `godot/locale/` (DE + EN) aktualisieren.

## 4. Nicht Teil dieses Prompts

- Panzerabwehr-Haftladungen / Geballte Ladungen gegen schwere Panzer (eigener
  Schritt bei Bedarf).
- Änderungen am bestehenden Artillerie-Nebel (`indirect.toml` für Mörser/
  Haubitzen bleibt unberührt).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/232-infanterie-handgranaten-und-nebelgranaten.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.

## Ergebnis (Nachtlauf 27.11.2026)

- **Regeln & Simulation (`data/infantry.toml`, `crates/sim/src/infantry.rs`, `crates/sim/src/command.rs`, `crates/sim/src/sim/grenades.rs`):**
  - `[grenades]` in `data/infantry.toml`: `categories = ["inf_rifle", "inf_lmg", "inf_engineer", "inf_recon", "inf_command", "inf_flamethrower"]`, `frag_stock = 3`, `smoke_stock = 2`, `range_mm = 24000`, `cooldown_ms = 16000` (8 s Spielzeit), `flight_ms = 2200` (1,1 s), `fuse_ms = 1000` (0,5 s), `arc_apex_mm = 5000`, `frag_radius_mm = 6000`, `frag_damage = 140`, `frag_bunker_damage = 1400` (2 Treffer zerstören 2500-HP-Bunker), `frag_cover_pct = 40`, `frag_building_exposure_pct = 75`, `smoke_radius_mm = 14000`, `smoke_lifetime_ms = 44000` (22 s).
  - Neue Befehle `Command::ThrowGrenade { units, target }` und `Command::ThrowSmokeGrenade { units, target }`; pro Klick wirft genau **ein** geeigneter Trupp (nächster bereiter Trupp ohne offenen Wurfauftrag; Shift+Klick für weitere Würfe); außerhalb Reichweite verlässt der Trupp Gebäude und rückt automatisch in Wurfreichweite vor.
  - Nebelgranaten nutzen `indirect::spawn_smoke_cloud`; Nachschub über Versorgungs-Lkw/Depots in `sim/logistics.rs`; `soldier_flags::THROWING = 32` in `sim/tactics.rs`.
- **Bridge, Godot & KI:**
  - Bridge: `throw_grenade`, `unit_grenades`, `grenade_params`, `grenade_positions`, `grenade_previous_positions`, `grenade_smoke`.
  - Godot: Kontext-Kacheln „Handgranate“ (`Shift+H`, Kontext-Fallback `G` ohne Artillerie) und „Nebelgranate“ (`Shift+T`, Kontext-Fallback `T` ohne Artillerie) mit Restanzahl-Badge & Abkling-Sektor (`command_bar.gd`, `hud_tile.gd`, `hud_icons.gd`, `settings.gd`, `selection.gd`), Zielvorschau mit Wurfreichweiten-Ring, Wirkungskreis & „Außer Reichweite – Trupp rückt vor“-Hinweis (`order_preview.gd`), `throw_grenade`/`throw_grenade_kneel` (`infantry_view.gd`), fliegende Granaten im Bogen (`artillery_view.gd`), DE/EN-Übersetzungen (`hud.csv`, `settings.csv`), Doku (`INFANTRY.md`, `CONTROLS.md`, `ARCHITECTURE.md`).
  - KI (`crates/ai/src/posture.rs`, `view.rs`, `strike.rs`): Normal/Hard wirft Splittergranaten auf Bunker, besetzte Häuser, sMGs und nahe Infanterie (mit `HoldFire`-Schutz, 8 m Eigenabstand) und Nebel unter starkem Beschuss / vor Bunkern & sMGs (ohne eigene Sturmpanzer/Unterstützungsfahrzeuge zu blenden); nebenbei robuste Nachführung bei Raketenschlag-Evakuierung in `strike.rs`.
- **Prüfung:**
  - `crates/sim/tests/grenades.rs` (4 Tests: Wurf in Reichweite & 1-pro-Klick, Vorrücken & Lkw-Nachschub, Nebel blendet Bunker & 2 Splitter zerstören Bunker, Hausbesatzung getroffen & geschlossener Panzer immun & Determinismus/Snapshot-Roundtrip).
  - Golden Hashes bewusst aktualisiert durch gehashtes `[grenades]` in `InfantryRules::hash_into`: `determinism.rs` `0xbf08_5391_a81d_e818` (Demo unverändert Tick 625), `economy.rs` `0x3e27_7592_07e0_f924`.
  - `just check`, `just infantry-selftest` (86 Soldaten, 62 Clips), `just selftest` (alle 15 Missionen/Tutorials + Replay/Riders/Gun-Crew/Boats) und `just ai-tournament` (`0xa8e5e9ce8d0449c9`, Hard vs. Normal 47/52 %, AI 1–3 µs/Tick) grün.
- **Vorläufig (Nachtlauf):** `[grenades]`-Werte (`frag_stock = 3`, `smoke_stock = 2`, `24 m`, `16 s` Cooldown, `1400` Bunkerschaden), Tastenbelegung `Shift+H` / `Shift+T` (+ Kontext-Fallback `G`/`T`), KI-Schwellen. Offen: visuelle Ingame-Screenshots im Fenster.
