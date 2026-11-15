# Prompt 023 – Basisbau, Ressourcen und Produktion

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/GAME_DESIGN.md` (das Spieldesign dieses Prompts), `docs/ARCHITECTURE.md`
> (Abschnitte Combat, Vision, Destruction, Victory conditions, Map editor),
> `docs/DETERMINISM.md`, `docs/AI.md` und `docs/MAPS.md`.

## 1. Ziel

Spieler bauen eine Basis, halten Versorgungspunkte und produzieren Einheiten:
drei Ressourcen (Treibstoff, Munition, Personal), vier Basisgebäude
(Hauptquartier, Kaserne, Versorgungsdepot, Fahrzeugwerk), Befestigungen aus dem
Katalog (MG-Bunker, Pak-Stellung). Alles in `crates/sim`, deterministisch; Godot
zeigt Bau-Vorschau, Baustellen, Produktion und Ressourcen nur an. Die KI baut und
produziert mit denselben Befehlen.

Die Designentscheidungen in `docs/GAME_DESIGN.md` sind **vorläufig, vom Nutzer
zu bestätigen**; der Nutzer hat sie bewusst dem Agenten überlassen. Alle Werte
stehen in `data/economy.toml`.

## 2. Leitentscheidung: Gebäude sind Einheiten

Gebäude, Baustellen und Befestigungen sind **unbewegliche Einheiten** der
Simulation (Geschwindigkeit 0). Dadurch funktionieren ohne Umbau:

- Kampf (017): Trefferbox aus Gebäudemaßen, Panzerung auf allen Seiten gleich,
  Trefferpunkte aus `economy.toml`; Gebäude werden Ziel des automatischen
  Zielwählens; Bunker und Pak schießen mit ihren Katalogwaffen.
- Sicht (018), Zerstörung (020: zerstörte Gebäude bleiben als Ruine liegen und
  blockieren ihre Pfadzellen wie Wracks), Siegbedingungen (027: wer noch ein
  Gebäude hat, lebt), KI-Ansicht (024).
- Bauplätze blockieren die Pfadzellen über `Sim::set_path_obstacle` (012).

Bewegungsbefehle (`MoveTo`, `Attack`, `AttackGround`, `Garrison`, `SetStance`)
ignorieren unbewegliche Einheiten. Die Wirtschaftsregeln sind wie
`data/destruction.toml` eingebaut (`EconomyRules::builtin`) und gehashed;
`SimConfig` bleibt unverändert (keine Konflikte mit parallelen Prompts, kein
neues Replay-Format für die Konfiguration).

## 3. Umfang

### 3.1 Regeln und Daten (`crates/sim/src/economy.rs`, `data/economy.toml`)

1. `EconomyRules` streng geparst (`deny_unknown_fields`, nur Ganzzahlen):
   Startkonto, Bau-Regeln (max. Baustellen, Start-HP, Steigung, Abstand),
   Produktion (Warteschlange, Einheitenlimit, Sammelpunkt), Versorgungspunkte
   (Marker-Präfix, Radius, Einnahmezeit, Einkommen), Kostenformel, Gebäude,
   Befestigungen, Produktionslisten pro Fraktion.
2. `UnitType` für Basisgebäude (`UnitType::structure`) und für Befestigungen aus
   dem Katalog (`UnitType::fixed_from_catalog`, Geschwindigkeit 0, Panzerung aus
   den Regeln, wenn angegeben).
3. Kostenformel aus Katalogwerten (Masse, Besatzung/Truppgröße, Waffen der Sim);
   `cli economy-table` gibt die Preise aller produzierbaren Einheiten aus.
4. Szenario: `economy = true` und Gruppen mit `role = "building"`
   (`data/scenarios/skirmish.toml`: pro Spieler ein HQ und wenige Starteinheiten).
   Mit `economy = true` hängt `parse_scenario_with_factions` alle Gebäude- und
   Befestigungstypen und die Produktionslisten der beteiligten Fraktionen an
   `unit_types` an.

### 3.2 Simulation (`crates/sim/src/sim/economy.rs`)

1. **Konten** pro Spieler in Tausendsteln (kein Rundungsverlust), Einkommen jede
   Sekunde Spielzeit aus HQ, Depots und gehaltenen Versorgungspunkten.
2. **Versorgungspunkte** aus Kartenmarkern `supply…`: Einnahme durch Einheiten
   (keine Gebäude) im Radius ohne Gegner; feindliche Punkte erst neutralisieren.
   `cli gen-map` erzeugt drei Punkte (Dorf, Hügel, Furt); `test_small` und
   `test_large` bekommen sie im `map.toml` (gleich wie der Generator).
3. **Befehle** (ans Ende von `Command`, ältere Replays bleiben lesbar):
   `Build { unit_type, position }`, `Produce { building, unit_type }`,
   `CancelProduction { building }`, `SetRallyPoint { building, point }`.
4. **Bauplatz-Prüfung** `Sim::placement(player, unit_type, position)` mit Grund
   (`PlaceError`): Typ baubar, Voraussetzungen, Anzahl, Kosten, Baustellenlimit,
   Bauradius eigener fertiger Gebäude, in der Karte, Pfadzellen für große
   Einheiten befahrbar (kein Wasser, keine Objekte, keine Hindernisse),
   Steigung, keine Einheit im Fußabdruck plus Abstand. Dieselbe Funktion nutzt
   die Bau-Vorschau.
5. **Baustelle**: Kosten bei Platzierung, Einheit sofort mit 10 % HP, linearer
   Zuwachs bis fertig; feuert, produziert und zahlt erst fertig.
6. **Produktion**: Warteschlange pro Gebäude, Kosten beim Einreihen,
   Abbrechen erstattet voll; fertige Einheit erscheint an einem freien Platz
   neben dem Gebäude (feste Suchreihenfolge) und fährt zum Sammelpunkt.
   Einheiten zur Laufzeit erzeugen (`Sim::spawn_unit`): Kampfzustand, Trupp,
   Sicht-Arrays wachsen mit; IDs bleiben aufsteigend.
7. Zustand im State-Hash; Abfragen für Bridge und KI: `account`, `income`,
   `structures`, `supply_points`, `build_options`, `production_options`,
   `find_site`.

### 3.3 KI (`crates/ai`)

1. `PlayerView` bekommt Konto, eigene Gebäude (Typ, fertig, Warteschlange),
   Versorgungspunkte (öffentlich) und freie Bauplätze für die aktuell baubaren
   Typen (`Sim::find_site`).
2. Neues Modul `economy.rs`: feste Bauordnung, Warteschlangen füllen nach einer
   Mischung je Schwierigkeit, Sammelpunkt an der eigenen Basis.
3. Armee wartet bis zur Angriffswelle (Easy 8, Normal 6, Hard 5 Einheiten) bei
   der Basis bzw. nimmt nahe Versorgungspunkte ein; Gegner in Sicht werden wie
   bisher bekämpft. Gebäude sind keine Gruppenmitglieder.
4. `just ai-tournament --mode base`: KI gegen KI im Basisbau-Szenario.

### 3.4 Bridge und Godot

1. `crates/bridge/src/economy_funcs.rs`: Konto und Einkommen, Bauoptionen,
   `placement_check`, `place_building`, Produktionsoptionen und Warteschlange
   eines Gebäudes, `produce`, `cancel_production`, `set_rally_point`,
   Versorgungspunkte, Bauzustand der Gebäude; `SimHost.mode`
   (`"battle"`/`"base"`) wählt das Szenario für `start_match`; `speed` als
   Zeitfaktor für Selbsttests.
2. `scenes/world/base_view.gd`: Modelle der Gebäude (geliehene Häuser aus 007
   mit Schadensvarianten aus 020), Baustelle (Modell wächst mit dem
   Fortschritt), Versorgungspunkte als Fahnen in Besitzerfarbe.
3. `scenes/world/base_hud.gd`: Ressourcenleiste oben, Baumenü (Kosten,
   gesperrt mit Grund), Bau-Vorschau als Geisterkörper grün/rot an der Maus,
   Produktionsleiste des gewählten Gebäudes mit Warteschlange und Fortschritt,
   Rechtsklick mit gewähltem Gebäude setzt den Sammelpunkt.
4. Hauptmenü-Setup: Spielmodus „Base building“ (Standard) / „Battle“.

## 4. Akzeptanzkriterien

1. `just check` grün. Golden-Hash bewusst aktualisiert (Versorgungspunkte auf
   `test_small` werden in der Demo eingenommen, Wirtschaftszustand im Hash),
   Grund im Commit und in `crates/sim/tests/determinism.rs`.
2. Tests: Regeln parsen und Kostenformel; Bauplatzprüfung (Wasser, Steigung,
   Radius, Einheiten, Kosten); Bau → Fertigstellung → Produktion → Einheit am
   Sammelpunkt; Abbrechen erstattet; zerstörtes Gebäude verliert seine
   Warteschlange; Versorgungspunkt wird eingenommen und zahlt.
   **Produktionskette auf zwei Peers** im Loopback-Lockstep mit Hash-Vergleich
   jedes Ticks (`crates/net/tests/economy.rs`) und eigenem Golden-Hash des
   Basisbau-Szenarios.
3. KI: Test, dass Normal in einer Basisbau-Partie Gebäude errichtet und
   Einheiten produziert; Turnier im Modus `base` läuft.
4. Alle Selbsttests grün: `just selftest` (Flow-Test mit neuem Basisbau-Match),
   `just lan-selftest`, `just editor-selftest`, `just gallery-check`, Demo-Lauf
   (`just run -- --demo` bzw. `demo_reel.gd`).
5. Doku: `docs/GAME_DESIGN.md`, `docs/ARCHITECTURE.md` (Economy-Abschnitt,
   Befehlstabelle), `docs/AI.md`, `docs/MAPS.md` (Versorgungspunkte),
   `docs/DETERMINISM.md` (Golden-Hash-Historie).

## 5. Arbeitsweise

Deutsch mit dem Nutzer, Englisch im Repo, kein Push, keine Installation ohne
Freigabe, keine Tests abschwächen. Parallel laufen 016 (Performance), 021
(Gelände) und 026 (Steam): Änderungen an gemeinsamen Dateien (`sim.rs`,
`main.gd`, `selection.gd`, `self_test.gd`, `match_setup.gd`) klein und additiv
halten, neue Logik in eigene Dateien. Commits pro Schritt. Am Ende den aktuellen
lokalen `main` übernehmen, Konflikte lösen, diese Datei nach `prompts/done/`
verschieben, `prompts/README.md` auf „erledigt“ setzen.
