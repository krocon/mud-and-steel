# Prompt 034 – Transport, Aufsitzen und Geschütze schleppen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Einheiten, Infanterie, Wegfindung, Kampf), `docs/INFANTRY.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

LKW und Schützenpanzerwagen fahren bisher leer herum, Pak und Feldgeschütze stehen fest.
Nach diesem Prompt sitzen Infanterietrupps auf und ab, gezogene Geschütze werden von
Zugmaschinen verlegt und abgeprotzt, Panzer können Infanterie aufsitzen lassen
(Tank-Desant, z. B. sowjetisch).

## 2. Umfang

1. **Sim:** Transportkapazität aus dem Katalog (Sitzplätze, Zugkraft), neue
   `Command`s `Embark`, `Disembark`, `Tow`, `Unlimber` (hinten ans Enum).
   Aufgesessene Einheiten bewegen sich mit dem Fahrzeug, sind unsichtbar/geschützt
   je nach Fahrzeug (offen vs. geschlossen), sterben ganz oder teilweise bei Zerstörung.
   Aufsitz-, Absitz- und Abprotzzeiten (Spielregeln mit Begründung, Zeitfaktor 2).
2. **Geschütze:** gezogene Geschütze sind ohne Zugmaschine nur langsam von der
   Bedienung verschiebbar; abgeprotzt schussbereit, aufgeprotzt nicht.
3. **Bedienung:** Rechtsklick mit Infanterie auf eigenes Fahrzeug = aufsitzen; Taste für
   Absitzen; Anzeige der Insassen (mit 031).
4. **KI:** fährt Infanterie mit LKW an die Front, verlegt Geschütze.
5. **Darstellung:** Soldaten auf offenen Fahrzeugen sichtbar sitzend, Geschütz an der
   Anhängerkupplung (Sockel aus dem Baukasten prüfen/ergänzen).

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Auf- und Absitzen, Verluste bei Zerstörung, Schleppen
   und Abprotzen, deterministischer Hash; Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest`, `just gallery-check` grün.
3. Doku: `docs/ARCHITECTURE.md`, `docs/INFANTRY.md`.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach 031, 032, 038): LKW und Schützenpanzerwagen sind
normale Fahrzeuge ohne Ladefunktion; gezogene Geschütze (`Group::Gun`) sind nicht
beweglich (`UnitType::from_catalog` liefert `None`) und kommen nur als
„Pak-Stellung“ vor, eine Befestigung aus 023 (`UnitType::fixed_from_catalog`,
Tempo 0). 032 kennt eine Aufbauzeit nach jeder Bewegung (`setup_ms`), die bei
gezogenen Geschützen bisher nie greift. 038 speichert den ganzen `Sim` per serde;
neue Felder sind automatisch im Spielstand. 031 hat Befehlsleiste, Einheitenpanel
und Tasten als Input-Actions. Der Katalog nennt Sitzplätze nur im Freitext
(`note`), keine Zuglast. Die Modelle haben `Socket_Passenger_NN` (LKW,
Schützenpanzer) und `Socket_Tow` (alle Fahrzeuge und Geschütze), Panzer haben
keine Aufsitzer-Sockel.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Neues Katalogfeld **`passengers`** (Sitzplätze für Truppen neben der Besatzung) für LKW und Schützenpanzer: Sd.Kfz. 251 10, M3 12, M3A1 6, ZiS-5 25 (aus den vorhandenen Quellen/Notizen); Opel Blitz und GMC CCKW je 16 als markierte Schätzung (`uncertain`). | Prompt verlangt Kapazität aus dem Katalog; Werte stehen schon in den Notizen, zwei sind ehrlich als Schätzung markiert (offline keine Quelle). |
| 2 | **Zugkraft** ohne neues Katalogfeld: ein Fahrzeug zieht ein Geschütz, wenn `power_hp ≥ 8 PS/t × (Masse Zugmaschine + Geschütz)` (Regel `min_hp_per_tonne`). Beim Ziehen fährt es mit `Masse / (Masse + Geschützmasse)` seines Tempos. **[vorläufig]** | Leistung und Massen stehen schon belegt im Katalog; eine erfundene „Anhängelast“ je Fahrzeug wäre nicht belegbar. 8 PS/t lässt jeden LKW/Schützenpanzer die Pak und leichte Haubitze seiner Fraktion ziehen, aber keine 8,8-cm-/90-mm-Flak (deren Zugmaschinen fehlen im Katalog). |
| 3 | Regeln in neuem **`data/transport.toml`** (von `Sim::new` geladen, im Hash, nicht in `SimConfig`): pro Träger-Kategorie Sitze (Panzer: `riders`), offen/geschlossen, Auf-/Absitzzeit, Verlustchancen, ob er zieht; pro Geschütz-Kategorie Auf-/Abprotzzeit und Schiebetempo. | Wie `data/indirect.toml`: kein neues Config-/Replay-Format, balancierbar. |
| 4 | **Zeiten** (real, Zeitfaktor 2 gilt): Aufsitzen LKW 20 s, Schützenpanzer 12 s, Panzer 10 s; Absitzen 10/8/4 s; Auf-/Abprotzen Pak 30 s, Haubitze 40 s, leichte Flak 30 s, schwere Flak 120 s, schwerer Mörser 20 s. Haubitzen brauchen danach zusätzlich ihre Aufbauzeit aus 032. **[vorläufig]** | Größenordnung realer Drills; Timer laufen nur, solange das Fahrzeug steht. |
| 5 | **Tank-Desant für alle Fraktionen:** leichte Panzer 6, mittlere und schwere 10 Aufsitzer. **[vorläufig]** | Historisch vor allem sowjetisch, aber auch bei US/DE üblich; eine Fraktionsregel wäre Balancing, nicht Simulation. |
| 6 | **Passagiere bleiben Einheiten** (gleiche Id) mit Verknüpfung `Aboard(Fahrzeug)`: sie stehen in der Fahrzeugmitte, sind für den Gegner unsichtbar, nicht treffbar (nicht im Einheitenraster), schießen nicht und nehmen nur `Disembark` an. Kein Sitz wird gezählt außer lebenden Soldaten; ein Trupp steigt nur ganz ein. | Keine Umbauten an Ids, Sicht- und Kampf-Arrays pro Einheitenindex; Wiederverwendung der Garnisonslogik-Ideen aus 022. |
| 7 | **Verluste:** verliert der Träger Trefferpunkte (auch Splitter), stirbt jeder Passagier mit `hit_pct` (LKW 25 %, SPW 15 %, Panzer 40 %); Treffer ohne Schaden (Abpraller) treffen Aufsitzer auf Panzern mit 25 %. Bei Zerstörung stirbt jeder Passagier mit 60/50/70 %, die Überlebenden springen sofort ab. **[vorläufig]** | „Ganz oder teilweise“ wie gefordert, deterministisch über `sim::Rng`; offen/geschlossen wirkt über die Prozentsätze (alle Träger im Katalog sind offen). |
| 8 | **Gezogene Geschütze** werden beweglich: Bedienung schiebt sie mit 2 km/h (Pak), 1 km/h (Haubitze) real. Angehängt (`Towed(Zugmaschine)`) folgen sie wie ein Anhänger an der Kupplung (Mündung nach hinten), sind sichtbar und treffbar, schießen nicht; beim Abprotzen dreht die Bedienung sie in Fahrtrichtung. Geschütze feuern nur stehend und nicht während Auf-/Abprotzen. Fällt die Zugmaschine aus, wird das Geschütz abgehängt und ist erst nach der Abprotzzeit bereit. | „Ohne Zugmaschine nur langsam verschiebbar; abgeprotzt schussbereit, aufgeprotzt nicht.“ Gleiche Id, keine Sonderwege in Sicht und Kampf. |
| 9 | Die **Pak-Stellung** (Befestigung aus 023) entfällt; stattdessen produziert das Fahrzeugwerk LKW, Pak und leichte Haubitze jeder Fraktion (`data/economy.toml`, am Listenende). **[vorläufig]** | Gleiche Katalog-Id als feste und als bewegliche Einheit ginge nicht (Typen über Namen). 023 nannte die Stellung ausdrücklich eine Übergangslösung, „bis Geschütze bewegt werden“. |
| 10 | **Befehle** (am Enum-Ende): `Embark { units, carrier }`, `Disembark { units }` (Fahrzeuge entladen alle, Passagiere steigen einzeln aus), `Tow { units, gun }` (das nächste freie, starke genug Fahrzeug der Auswahl), `Unlimber { units }` (Fahrzeuge oder Geschütze). Bewegungs-, Angriffs- und Stoppbefehle brechen Anmarsch und Anfahrt ab, nicht laufendes Absitzen/Abprotzen. | Klein und additiv; dieselbe Logik wie `Garrison`. |
| 11 | **Bedienung:** Rechtsklick mit Infanterie auf eigenes Fahrzeug mit Plätzen = aufsitzen; Rechtsklick mit Fahrzeugen auf ein eigenes Geschütz = anhängen; **J** absitzen, **K** abprotzen (Input-Actions, umbelegbar), Schaltflächen in der Befehlsleiste; das Einheitenpanel zeigt Insassen „9/10“ und das gezogene Geschütz. Passagiere sind nicht auswählbar. | Wie gefordert; J/K waren frei. |
| 12 | **Darstellung:** aufgesessene Soldaten sitzen (Pose „kneel“, Uniform der Fraktion) an den `Socket_Passenger_NN` offener Fahrzeuge; Panzer haben keine solchen Sockel, dort verteilt Godot die Aufsitzer auf dem hinteren Deck (Bounding-Box des Modells). Das Geschütz steht in der Sim an der Kupplung (Katalogmaße 1:1). Kein Blender-Lauf. **[vorläufig: Aufsitzer-Sockel für Panzer im Baukasten ergänzen]** | Sockel für LKW/SPW und `Socket_Tow` sind vorhanden; die Panzer-Modelle neu zu bauen wäre ein großer Asset-Schritt ohne Sim-Nutzen. |
| 13 | **KI:** eigene Rollen „Transport“ (LKW und SPW) und „Geschütz“. Wartende Infanterie nahe der Basis steigt in freie Transporter, die fahren zur Infanterie-Gruppe bzw. zum Ziel und laden 120 m vor bekannten Feinden ab. Geschütze ohne Zugmaschine lassen sich von einem freien Fahrzeug abholen und hinter die Front bringen, dort abprotzen. Normal und Hard bauen LKW und Pak in ihren Mix ein. | „Fährt Infanterie mit LKW an die Front, verlegt Geschütze“, über normale Befehle. |
