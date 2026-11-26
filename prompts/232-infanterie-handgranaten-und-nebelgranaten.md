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
