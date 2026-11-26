# Prompt 234 – Rückzugsbefehl für Infanterietrupps

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/INFANTRY.md`, `docs/CONTROLS.md`, `docs/AI.md`,
> `docs/ARCHITECTURE.md` („Infantry“, „Tactics“), `data/tactics.toml`
> (`[suppression]`, `[rush]`, `[discipline]`, `[drop]`) sowie `data/infantry.toml`.
> Im Code:
> `crates/sim/src/command.rs`,
> `crates/sim/src/tactics.rs`,
> `crates/sim/src/sim/tactics.rs`,
> `crates/sim/src/sim/infantry.rs`,
> `crates/sim/src/sim/discipline.rs` (dort existiert bereits `unarmed_retreat`
> für unbewaffnete Fahrzeuge als Vergleich),
> `crates/bridge/src/`,
> `crates/ai/src/`,
> `godot/scenes/selection/selection.gd`,
> `godot/scenes/hud/command_bar.gd`,
> `godot/scenes/hud/state_marks.gd`,
> `godot/scenes/hud/hud_icons.gd`,
> `godot/scenes/world/infantry_view.gd`.
>
> **Stil-/Spielgefühl-Referenz (nur lokal ansehen, nichts übernehmen):**
> `.temp/vorlage/CMAF_1080 [CMAF_1080].mp4`, Sekunden `01:32`, `02:54–02:58`,
> `03:44–03:45` (eigene und gegnerische Infanterietrupps brechen den Kampf per
> Rückzugsbefehl ab, lösen sich im schnellen Lauf Richtung Hinterland/HQ und
> tragen über sich die klare Kennzeichnung `RETREAT`).
> **Keine Bilder, Namen oder Spielwerte aus dem Video ins Repo.**
> **Hinweis zu Golden Hashes:** Neuer Befehl `Retreat` und `[retreat]`-Regeln
> in `crates/sim` – bestehende Matches ohne diesen Befehl sollten bei
> unverändertem KI-Startverhalten geprüft werden; bei bewusster Änderung durch
> KI-Nutzung Golden Hashes dokumentiert aktualisieren.

## 1. Problem

In *Mud and Steel* gibt es bisher nur den automatischen Rückzug für
unbewaffnete Fahrzeuge (`unarmed_retreat` in `data/tactics.toml`), aber
**keinen Rückzugsbefehl für Infanterietrupps**:

- Geraten Schützen unter schweres MG- oder Artilleriefeuer, werden sie
  **niedergehalten** (`pinned_at = 60`): sie werfen sich hin (`prone`, 20 %
  Tempo) und verlieren zusätzlich bis zu 50 % Geschwindigkeit durch
  Unterdrückung.
- Gibt der Spieler einen normalen Bewegungsbefehl (`Rechtsklick` oder `M`),
  kriecht oder stolpert der dezimierte Trupp im Schneckentempo aus dem
  Gefahrenbereich und wird meist komplett aufgerieben – erfahrene
  Veteranentrupps lassen sich kaum retten.
- Auch für die Spieltiefe (Erhalt von Veteranen, Rückzug zu Sanitätern /
  Nachschub / HQ) fehlt eine klare, mit einem Tastendruck auslösbare
  Rückzugsmechanik.

## 2. Ziel

Ein sauberer, deterministischer **Rückzugsbefehl** für Infanterietrupps (und
optional leichte Fuß-Waffenteams):

1. **Regeln in `data/tactics.toml` (`[retreat]`) und `crates/sim`:**
   - Neuer Befehl `Command::Retreat { units }` in `crates/sim/src/command.rs`
     (vollständig im Lockstep-Protokoll & Replay; für Fahrzeuge ignoriert bzw.
     nur für Infanterietrupps wirksam).
   - **Zielpunkt des Rückzugs (deterministisch in `crates/sim`):**
     - Im Basisbau-Modus: Nähe des eigenen Hauptquartiers (`base_hq`) – falls
       zerstört, der nächste eigene Sammel-/Startpunkt.
     - In Gefechten / Missionen ohne eigenes HQ: der eigene Start-/Eintrittspunkt
       des Spielers auf der Karte (oder eine feste Rückzugsdistanz Richtung
       eigener Kartenseite, z. B. `fallback_mm = 150000`, auf erreichbarem
       Gelände).
     - Besetzte Gebäude werden sofort verlassen (`garrison = None`), ebenso
       Feuerteam-Sprünge (`fire_teams`) und Eskort-Kopplung (`escort`).
   - **Verhalten während des Rückzugs (`[retreat]` in `data/tactics.toml`, alle
     Werte `[provisional]`):**
     - Trupp richtet sich auf (`stand`), bricht jedes Feuer ab (schießt nicht
       zurück!) und läuft im schnellen Rückzugstempo (`speed_pct`, z. B. wie
       `rush` bzw. `130–150 %` nach Prompt 231, ohne durch `suppression` oder
       `pinned` ausgebremst zu werden und ohne `[drop]`-Hinwerfen).
     - Moderater Schutz vor sofortiger Auslöschung im Lauf (z. B.
       `hit_pct = 70–80 %` reduzierte Trefferwahrscheinlichkeit gegen direkte
       Kleinwaffen – Artillerie/HE und Panzer bleiben voll gefährlich!), damit
       der Rückzug eine echte Chance bietet, aber kein Unverwundbarkeits-Cheat
       ist.
     - Unterdrückung baut sich auf dem Rückweg schneller ab, sobald der Trupp
       außerhalb direkten Beschusses ist.
     - **Ende des Rückzugs:** Bei Ankunft am Zielpunkt (innerhalb
       `arrive_mm`, z. B. 15 m) endet der Rückzug automatisch und der Trupp ist
       wieder normal steuerbar. Prüfen und klar dokumentieren, ob ein neuer
       Befehl den Rückzug erst nach einer kurzen Mindestzeit (`lock_ms`, z. B.
       3 s) oder gar nicht abbrechen darf (klassisch verhindert eine Sperre bis
       zur Ankunft bzw. bis außerhalb von Beschuss Missbrauch als gratis
       Sprint-Knopf nach vorne!).
2. **Bridge & Darstellung (`crates/bridge`, `godot/`):**
   - `SimHost::unit_retreating()` (oder Bitflag in bestehender Taktik-Maske)
     liefert den Rückzugsstatus an Godot.
   - **Befehlsleiste & Taste (`command_bar.gd`, `settings.gd`, `docs/CONTROLS.md`):**
     Kontext-Kachel **„Rückzug“** für ausgewählte Infanterie mit umbelegbarer
     Taste (Achtung: `R` ist in `docs/CONTROLS.md` bereits `Attack` – freien
     Shortcut wählen, z. B. `Shift+R` oder `Backspace`, und sauber in
     `docs/CONTROLS.md` und `godot/locale/` eintragen).
   - **Markierung über dem Trupp (`state_marks.gd`, `hud_icons.gd`):**
     Eigener, klar lesbarer Status-Marker `retreat` (Pfeil zurück / gelb-orange
     Hervorhebung, sichtbar für eigene, verbündete **und** aufgeklärte
     gegnerische Trupps – wie `ENEMY_MARKS` in `state_marks.gd`!).
   - **Bewegungsanimation (`infantry_view.gd`):** Rückziehende Soldaten laufen
     stets im `run`-Clip, ohne stehenzubleiben oder anzulegen.
   - **Event-/Sound-Feedback:** Kurze Bestätigung (`ui_order` / optional
     dezenter Gefechtsruf).
3. **KI-Integration (`crates/ai`):**
   - Die KI zieht stark dezimierte (z. B. `<= 35 %` Mannstärke) oder
     festgenagelte, isolierte Infanterietrupps – insbesondere Veteranen – per
     `Command::Retreat` zurück, statt sie sinnlos aufreiben zu lassen.

## 3. Prüfen und zeigen

- **Simulations-Tests (`crates/sim`):**
  - Niedergehaltener Trupp (`suppression = 90`, `pinned`) löst sich bei
    `Command::Retreat` sofort, ignoriert Beschuss-Stopps, feuert nicht,
    verlässt ggf. Gebäude, erreicht das eigene HQ / Rückzugsziel und beendet
    den Zustand sauber.
  - Missbrauchstest: `Retreat` lässt sich nicht als kostenloser Vorwärts-Sprint
    missbrauchen (Ziel liegt strikt Richtung eigenem HQ/Hinterland).
  - Determinismus- und Replay-Test.
- **Sichtprüfung & Balance:**
  - Kurze Spielszene / Screenshots: niedergehaltener Trupp zieht sich unter
    Beschuss mit klar erkennbarem Rückzugssymbol Richtung HQ zurück; feindlicher
    Trupp zeigt ebenfalls das Rückzugssymbol.
  - `just ai-tournament` (prüfen, dass KI-Spiele stabil bleiben und Trupps
    sinnvoll überleben).
  - `just check`, `just selftest`, `just infantry-selftest` grün.
  - `docs/INFANTRY.md`, `docs/CONTROLS.md`, `docs/AI.md`, `docs/ARCHITECTURE.md`
    und `godot/locale/` (DE + EN) aktualisieren.

## 4. Nicht Teil dieses Prompts

- Automatisches Auffüllen verlorener Soldaten am HQ (Trupp-Verstärkung/
  Reinforce ist ggf. ein eigener Design-Prompt).
- Rückzugsbefehl für Panzer oder Geschütze (nur Infanterie).

## Abschluss

Nach Abschluss diese Datei mit `git mv prompts/234-infanterie-rueckzugsbefehl.md prompts/done/`
verschieben, Ergebnisabschnitt ergänzen und den Status in `prompts/README.md`
auf „erledigt“ setzen.
