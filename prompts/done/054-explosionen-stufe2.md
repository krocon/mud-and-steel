# Prompt 054 – Explosionen Stufe 2 und schwere Feuerschläge

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Effekte, indirektes Feuer, Licht).
>
> Konkretisiert in der ausführenden Session (siehe „Konkretisierung“).

## 1. Ziel

Große Explosionen sollen wuchtig aussehen und spielerisch wirken: Druckwelle, Blitz, der
den Boden beleuchtet, Munitionsexplosionen, die Soldaten umwerfen. Als spätes
Spielelement kommt ein schwerer Feuerschlag mit Vorwarnung, passend zur Epoche.

## 2. Umfang

1. **Druckwelle:** weiße, sich ausdehnende Kuppel/Ring bei großen Explosionen
   (Kondensation), Staubring am Boden, Bildschirmwackeln nach Entfernung.
2. **Blitzlicht:** kurzes Licht, das Gelände und Gebäude beleuchtet (bei Nacht aus 040
   besonders), Budget an Lichtquellen.
3. **Munitionsexplosion** eines Panzers: Turm fliegt ab (Darstellung), Druck wirft
   Infanterie im Radius um (Regel aus 048), Splitter.
4. **Schwerer Feuerschlag** [provisional, WW2-passend statt moderner Waffen]: z. B.
   Eisenbahngeschütz oder Raketenwerfer-Batterie, im späten Spiel freischaltbar,
   Abschussstellung auf der Karte, die der Gegner in einer Frist zerstören kann; farbiger
   Rauch markiert den Zielpunkt, Sirenen-/Warnton, dann ein Flächenschlag mit Kratern.
5. Alles über die Effekte aus 019 und Sound aus 029/053.

## 3. Akzeptanzkriterien

1. `just check` grün (Feuerschlag als Sim-Regel getestet); Golden-Hash begründet.
2. `just selftest` grün; Schlacht-Benchmark hält das Budget.

## Konkretisierung (Ausgangslage und Entscheidungen)

**Ausgangslage im Code:** `effects.gd` (019) zeichnet Einschläge, Explosionen mit
Feuerball, Licht (Pool `fx_lights`), Trümmern und Kamerawackeln; alle Treffer kommen
als `hit`-Ereignisse mit Kaliber. Granaten (032), Bomben (033, Kaliber
25 mm × ∛kg, also 15–19 cm), explodierende Lager (041, 12–15 cm) und Minen laufen
alle über `Sim::detonate`/Granaten-Explosion: Splitter, Krater (021), Brand,
Umwerfen (048). 048 entscheidet in `knockdown.rs`, ob die Munition eines zerstörten
Fahrzeugs hochgeht (40 %, 12 m Radius), meldet das aber der Darstellung nicht. 040
hat Nacht (`atmosphere.daytime`), 053 Explosionsklänge mit Nachhall nach Kaliber,
038 speichert jedes Feld von `Sim` per serde. Luftabrufe (033) sind
Spieler-Abrufe ohne Gebäude; 055 baut weitere Luftabrufe.

| Frage | Entscheidung | Grund |
|---|---|---|
| Was ist „große Explosion“? | Treffer ab 110 mm (12,2-cm-Haubitzen, Bomben, Raketen, Lager) und Munitionsexplosionen | Alle laufen schon über `hit`-Ereignisse; 10,5 cm (Standard-Haubitzen) bleiben normal, sonst wäre jede Batterie-Salve eine Kuppel. [provisional] |
| Druckwelle | Weiße Halbkugel (Rand hell, Mitte klar), 0,075 m/mm, 0,32 s, flacher Druckring bis 2×, Staubring in Bodenfarbe; dazu schwererer Feuerball und grau-braune Erdwolke | Kondensation an der Stoßfront wirkt nur kurz; größere Kuppeln (erster Versuch 0,13 m/mm) sahen in den Bildern wie Glasglocken aus. Neue Datei `blast_effects.gd`, `effects.gd` bleibt unverändert. |
| Blitzlicht | Eigener Pool 1/2/4 Lichter (niedrig/mittel/hoch), 30–66 m, 0,32 s; Nacht ×3 Stärke, ×1,45 Reichweite, Dämmerung ×1,7 | Budget wie bei 019, aber eigene große Lichter, damit Mündungsfeuer sie nicht verdrängen; ohne Schatten (Kosten). |
| Munitionsexplosion | Neues Ereignis `StrikeEvent::AmmoExplosion` aus `knockdown.rs` (eine Zeile); Darstellung: Turm-Knoten fliegt ballistisch ab (ohne Godot-Physik), landet, bleibt liegen (max. 16) | Die Regel aus 048 bleibt unverändert (Umwerfen, Splitter kommen schon aus der Sim); nur die Darstellung wusste nichts davon. |
| Schwerer Feuerschlag: Waffe | **Raketenwerfer-Batterie** statt Eisenbahngeschütz | Alle drei Fraktionen hatten sie (Nebelwerfer 41, BM-13, T34 Calliope); ein Eisenbahngeschütz braucht Gleise auf der Karte. [provisional] |
| Abschussstellung | Neues Basisgebäude `base_rocket_battery` (braucht Fahrzeugwerk, max. 1, 1500 TP); geliehenes Modell Munitionslager (041) mit prozeduralen Startgestellen | Ein Gebäude ist zerstörbar, auswählbar, bezahlbar, ohne neues Katalogmodell. Echtes Modell offen. [provisional] |
| Freischaltung, Preis | Ab 600 s Spielzeit, 60/250/30, Abklingzeit 300 s, ein Schlag zur Zeit | „Spätes Spiel“: KI-Partien auf `test_small` sind nach ~8 min entschieden, Menschen spielen länger. 900 s getestet: dann kam der Schlag nie vor. [provisional] |
| Frist und Warnung | 30 s: roter Rauch auf dem Ziel, Sirene für alle, Stellung wird allen Gegnern aufgedeckt (`Vision::reveal` wie Mündungsfeuer bei Nacht); Stellung zerstört oder verkauft → abgesagt, Preis verloren | Der Gegner muss die Stellung finden können, sonst ist die Frist wertlos. |
| Salve | 24 Raketen alle 0,25 s, gleichverteilt in 60 m, Flug Strecke/300 m/s (mind. 3 s); jede Rakete ist eine gewöhnliche Granate des Fraktionskalibers (15 cm / 132 mm / 114 mm) | Wiederverwendet Flug, Darstellung, Splitter, Krater, Brand, Umwerfen; keine neue Schadensregel. Kaliber echt (Quellen in `data/strike.toml`). |
| Befehl | `Command::HeavyStrike { target }` am Ende des Enums; kein Luftabruf | Absprache mit 055 (Luftabrufe dort); Protokollversion nicht erhöht (wie 046). |
| Golden-Hash | Ändert sich nur durch das neue Gebäude in den gehashten Wirtschaftsregeln; Schlag-Zustand wird erst nach dem ersten Abruf gehasht | Andere Partien behalten ihr Verhalten; geprüft mit der alten `economy.toml`. |
| KI | Normal/Hard bauen die Stellung kurz vor der Freischaltung, rufen auf die größte Gruppe (≥ 4), ein sichtbares Gebäude oder die gegnerische Basis, nie nahe eigener Einheiten; gegen feindliche Schläge räumen Einheiten das Gebiet, Hard greift die Stellung mit 4 Panzern an | KI ist normaler Spieler (Regel 5) und nutzt nur Befehle. |
| Klang | Sirene im Code erzeugt (auf- und absteigender Ton, nahtlose Schleife), Explosionen wie bisher aus 053 | Kein Download; eine echte Sirenenaufnahme ist offen. [provisional] |
| Nicht im Umfang | Nuklear/modern; Raketenschweif-Darstellung; eigene Wurfgeräusche | Nicht verlangt bzw. offen. |

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Ergebnis

Umgesetzt wie in der Konkretisierung; Details in `docs/ARCHITECTURE.md` „Heavy
strike“ und „Effects / Big blasts (prompt 054)“, `docs/GAME_DESIGN.md` „Heavy
strike“ und Gebäudetabelle, `docs/CONTROLS.md` „Heavy strike“, `docs/AI.md` „Heavy
strike“.

- **Sim:** `data/strike.toml`, `crates/sim/src/strike.rs`, `crates/sim/src/sim/strike.rs`,
  `Command::HeavyStrike` (letzte Variante), Basisgebäude `base_rocket_battery`,
  `Sim::launch_shell` (Raketen als Granaten), `StrikeEvent::AmmoExplosion` aus
  `knockdown.rs`. Tests: `crates/sim/tests/strike.rs` (Freischaltung, Warnung, Aufdecken,
  Salve im Radius mit Kratern und Schaden, Absage beim Verlust der Stellung, Kopie in der
  Frist läuft identisch, Munitionsexplosionen werden gemeldet).
- **KI:** `crates/ai/src/strike.rs`, Test `crates/ai/tests/strike.rs` (ruft den Schlag,
  der Gegner räumt das markierte Gebiet: 3 → 0 Einheiten).
- **Bridge:** `strike_funcs.rs` (`call_strike`, `strike_option`, `strikes`,
  `take_strike_events`).
- **Godot:** `world/blast_effects.gd` (Kuppel, Druck- und Staubring, Feuerball, Erdwolke,
  Blitzlicht mit Budget und Nachtfaktor, Turm fliegt ab), `world/strike_view.gd` (roter
  Rauch, Gebietsring, erzeugte Sirene, Countdown, Markierung der feindlichen Stellung,
  Leiste „Rockets“ mit Shift+6, Startgestelle), Vorschaubild des Gebäudes, Selbsttest-Check,
  `just blast-shots` (feste Ansichten Tag/Nacht) und `just strike-shots` (echter Schlag im
  Basis-Gefecht, schlägt fehl, wenn Rauch, Sirene, Countdown oder Start fehlen: PASSED).
- **Bilder:** `build/explosions/` (u. a. `blast_day_04.png`, `blast_village_night_00/01.png`
  vorher/nachher, `ammo_explosion_08/90.png`, `strike_day_salvo_15_00.png`,
  `strike_live_marked_00.png`, `strike_live_salvo_60.png`).

**Messwerte:** `just check` grün, `just selftest` und `just lan-selftest` grün. Golden-Hash
nach dem Merge mit main `0x7681_c049_8fc9_19b3` (vorher `0x2022_32ed_b313_f95a`),
Wirtschaft `0x656a_58c4_9caa_aed9`; einzige Ursache ist das neue Gebäude in den gehashten
Wirtschaftsregeln (geprüft mit der `economy.toml` von main). Schlacht-Benchmark
(`just bench`, Rechner durch parallele Sessions stark belastet): Zweig 8,6 ms Mittel,
p99 41 ms; main im selben Zeitraum 22,0 ms / 129 ms; gleiche Pfadknoten und Überlebende,
der Schlag kostet ohne Abruf nichts Messbares (Phase Combat 0,23 ms). Die Budgets
(5 ms Mittel) lassen sich nur auf einem ruhigen Rechner prüfen. KI-Turnier (8 Spiele):
Matrix-Hash `0xc44b_22e4_f4dd_002d`; Basis hard gegen normal mit 20 000 Ticks: 62 % / 25 %,
mittlere Dauer 557 s, also erreichen manche Partien die Freischaltung.

**Vorläufig [provisional]:** alle Zahlen in `data/strike.toml` (Freischaltung 600 s,
Preis, Frist 30 s, 24 Raketen, 60 m), das Gebäude (Werte, geliehenes Lagermodell mit
prozeduralen Gestellen), Aufdecken der Stellung für alle Gegner, roter Rauch, Sirene,
Taste Shift+6, Schwelle 110 mm und Größen der Druckwelle, Licht-Budget 1/2/4 und
Nachtfaktor 3.

**Offen:** echtes Modell der Raketenstellung; Raketen-Darstellung (Feuerschweif,
Startgeräusch) statt Granaten-Körper; eine aufgenommene Sirene; die Trefferringe eigener
Granaten (artillery_view) sind bei 24 Raketen unruhig; Bildschirmwackeln nach Entfernung
existierte schon (019) und wurde nur verstärkt; Protokollversion nicht erhöht (wie 046).
