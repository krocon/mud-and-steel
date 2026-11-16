# Prompt 036 – Nachschub, Verbrauch und Reparatur

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/GAME_DESIGN.md`, `docs/ARCHITECTURE.md` (Basisbau, Kampf, Transport aus 034).
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

`GAME_DESIGN.md` hat Treibstoff, Munition und Material als Konten, aber noch keinen
Verbrauch an der Einheit, keine Versorgungs-LKW und keine Reparatur (alles unter
„Open“). Nach diesem Prompt muss eine Offensive versorgt werden: Einheiten verbrauchen
Munition und Treibstoff, Versorgungsfahrzeuge füllen auf, Werkstätten und
Bergefahrzeuge reparieren, bewegungsunfähige Panzer können geborgen werden.

## 2. Umfang

1. **Design zuerst:** mit mir klären, wie viel Logistik Spaß macht (Vorschlag: Vorrat
   pro Einheit, Nachschub im Radius von Versorgungs-LKW/Depots, kein Mikromanagement
   einzelner Granaten). Ergebnis in `GAME_DESIGN.md` als bestätigt markieren.
2. **Sim:** Vorräte pro Einheit, Verbrauch pro Schuss/Strecke, Versorgung im Radius,
   Reparatur über Zeit (Werkstatt, Bergepanzer, Pioniere), Gebäude reparieren und
   verkaufen, Baustellen abbrechen (alles aus „Open“).
3. **Bedienung und HUD:** Vorratsbalken in der Einheitenanzeige (031), Warnung bei
   knapper Munition, Versorgungsradius sichtbar.
4. **KI:** versorgt und repariert ihre Einheiten, zieht leere Einheiten zurück.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Verbrauch, Nachschub, Reparatur, Bergung;
   Golden-Hash bewusst aktualisiert.
2. `just selftest`, `just lan-selftest`, `just ai-tournament` grün; die KI gewinnt
   gegen `Passive` weiterhin zuverlässig.
3. `GAME_DESIGN.md` aktualisiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Der Nutzer war nicht erreichbar; die Designfrage (Schritt 1) wurde nach dem
Vorschlag des Prompts entschieden und in `docs/GAME_DESIGN.md` als
**[provisional]** statt „bestätigt“ markiert.

Ausgangslage im Code (nach 023, 031–034, 037, 038, 041): Konten mit Treibstoff,
Munition, Personal (Tausendstel, `PARTS`), Hauptquartier und Depots zahlen
Einkommen; Einheitenpreise aus Katalogformel. Keine Vorräte an der Einheit; der
Katalog hat `ammo_carried` pro Waffe (nicht überall) und `range_road_km`.
Fahrzeuge haben 1000 Trefferpunkte, ein 7,5-cm-Treffer kostet 562, 8,8 cm 774.
Es gibt keinen Bergepanzer im Katalog und keinen Zustand „bewegungsunfähig“.
Pioniere (`inf_engineer`) gibt es als Kategorie, aber 035 baut sie parallel.
LKW (034) transportieren und ziehen. 038 speichert den ganzen `Sim` per serde;
der Regel-Hash im Spielstand-Kopf erkennt geänderte Zustandslayouts. 037: Teams
mit `Teams::allied`.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Vorrat pro Einheit** in Prozent (intern Millionstel), Munition und Treibstoff; keine einzelnen Granaten, keine Munitionsarten, kein Laden von LKW. **[provisional]** | Vorschlag des Prompts; kein Mikromanagement, eine Zahl pro Vorrat für HUD und KI. |
| 2 | **Munition = Salven der Hauptwaffe** (erste Waffe; bei Mörsern/Haubitzen/Sfl die indirekte): `ammo_carried / count` aus dem Katalog, sonst Vorgabe je Gruppe (Infanterie 150, Fahrzeug 100, Geschütz 60), begrenzt auf 12…2000. Panzer IV: 87 Schuss. Leer: **keine** Waffe feuert (auch MG). Befestigungen, Flak-Stellungen, Gebäude haben keinen Vorrat. **[provisional]** | Reale Werte, wo der Katalog sie hat; Nebenwaffen einzeln zu zählen wäre Mikromanagement. Stellungen werden von der Basis versorgt gedacht; sie können nicht zum Nachschub fahren. |
| 3 | **Treibstoff** nur für Fahrzeuge: Straßenreichweite × neuer Faktor `[range] fuel = 1/20` in `data/scale.toml` (Panzer IV 10,5 km, Tiger 9,75 km); Verbrauch nur nach Strecke, gelände-unabhängig. Leer: Fahrzeug bleibt stehen (auch Warteschlange/Patrouille enden). **[provisional]** | Reale 200 km liefen auf 4-km-Karten nie leer; die Umrechnung gehört laut CLAUDE.md nach `scale.toml`. Geländeabhängigkeit wäre mehr Regeln ohne Spielgewinn. |
| 4 | **Bewegungsunfähig:** Fahrzeuge mit ≤ 35 % Trefferpunkten fahren nicht (z. B. nach einem 8,8-cm-Treffer, zwei 5,7-cm-Treffern), schießen aber weiter, bis sie über die Schwelle repariert sind. **[provisional]** | Der Prompt verlangt Bergung bewegungsunfähiger Panzer; die Sim kannte den Zustand nicht. 35 % statt 25 %, weil ein 7,5-cm-Treffer schon 56 % kostet und 25 % fast nie erreicht würde. |
| 5 | **Nachschub:** fertiges HQ und Depots (60 m) sowie **jeder LKW** (40 m) versorgen Einheiten des eigenen Spielers **und der Verbündeten** mit 5 % pro Spielsekunde (Munition und Treibstoff gleichzeitig). LKW haben keinen eigenen Vorrat. Bezahlt vom **Empfänger**: eine volle Ladung kostet 50 % des Munitions- (Treibstoff-)Preises der Einheit; ohne Geld kein Nachschub. **[provisional]** | Konten bleiben die einzige Ressourcenquelle (kein zweites Lager-System); Verbündete helfen sich (037 hat geteilte Sicht, gemeinsames Versorgen passt), wer verbraucht, zahlt. Eigener LKW-Vorrat wäre Mikromanagement. |
| 6 | **Reparatur** (gratis, nur Zeit): Fahrzeugwerk 2 %/s im Umkreis von 50 m, LKW 1 %/s in 25 m, Pioniere (`inf_engineer`) 1 %/s in 20 m; nur Fahrzeuge und gezogene Geschütze, nicht Infanterie. Der beste Reparierer in Reichweite zählt. **[provisional]** | „Werkstatt, Bergepanzer, Pioniere“: Werkstatt = Fahrzeugwerk; ein Bergepanzer fehlt im Katalog (Modell, Quellen nötig) – LKW dienen als Feldwerkstatt; Pioniere über die Kategorie, sobald 035 sie baut. Infanterie-Ersatz wäre ein eigenes Thema. |
| 7 | **Bergung = Reparatur über die Schwelle:** ein bewegungsunfähiges Fahrzeug wird vor Ort repariert, bis es wieder fährt; kein Abschleppen von Panzern. **[provisional]** | Abschleppen bräuchte Zugmaschinen für 30–60 t (fehlen im Katalog) und Umbau von 034; vor Ort reparieren ist das, was Bergetrupps meist taten. |
| 8 | **Gebäude reparieren:** Befehl `Repair { building }` startet/stoppt die Reparatur eines eigenen fertigen, beschädigten Gebäudes: 2 %/s, eine volle Reparatur kostet 50 % des Gebäudepreises, laufend bezahlt; endet bei voller Gesundheit. **[provisional]** | Ein Befehl, den Mensch und KI gleich nutzen; Kosten verhindern Gratis-Unverwundbarkeit. |
| 9 | **Verkaufen / Baustelle abbrechen:** Befehl `Sell { building }`: fertig 50 % zurück, Baustelle 75 % zurück, Produktionsschlange voll zurück. Das Gebäude verschwindet (0 Trefferpunkte, Boden frei, nicht mehr im Snapshot, kein Trümmer). Zählt bei der Siegbedingung wie ein verlorenes Gebäude. **[provisional]** | Ein Befehl für beides („Open“ aus 023); Einheiten werden nie aus der Liste entfernt (Ids, Sicht-Arrays), daher „verschwinden“ über den Snapshot. |
| 10 | **Befehle** `Repair` und `Sell` am **Ende** des `Command`-Enums. Zustand (Vorräte jeder Einheit, Reparaturaufträge, verkaufte Gebäude) im Hash und über serde im Spielstand (038); kein Format-Versionssprung nötig (der Regel-Hash erkennt alte Stände). Regeln in neuem `data/logistics.toml` (wie `transport.toml`, nicht in `SimConfig`). | Parallele Prompts (035, 040) hängen ebenfalls an; additive Änderungen. |
| 11 | **HUD:** Einheitenpanel zeigt Munition/Treibstoff als Balken und Zeile mit Warnung („low ammunition“, „out of fuel“, „immobilized“); Kacheln der Mehrfachauswahl bekommen einen Munitionsbalken; Versorgungs-/Reparaturradius ausgewählter Quellen als Ring auf dem Boden; Gebäudeleiste mit „Repair“ und „Sell“/„Cancel“. | Prompt Punkt 3, auf vorhandenen 031-Bausteinen. |
| 12 | **KI:** Einheiten mit ≤ 25 % Munition, ≤ 20 % Treibstoff oder (Fahrzeuge, fahrbereit) ≤ 50 % Trefferpunkten ziehen sich zur nächsten eigenen Quelle zurück (Versorger bzw. Reparierer) und kehren voll zurück; beschädigte Gebäude (< 70 %) werden repariert, wenn die Kasse reicht. Kein Verkaufen. **[provisional]** | „Versorgt und repariert, zieht leere Einheiten zurück“ mit normalen Befehlen. |

## Ergebnis

Umgesetzt wie oben; das ganze Design ist **[provisional]** (Nutzer nicht erreichbar).
Details in `docs/GAME_DESIGN.md` „Supply and repair“, `docs/ARCHITECTURE.md`
„Supply and repair“, `docs/AI.md` „Supply and repair“, `docs/SCALE.md` (Treibstoff 1/20).

- **Sim:** `data/logistics.toml`, `crates/sim/src/logistics.rs`,
  `crates/sim/src/sim/logistics.rs`; `[range] fuel` in `data/scale.toml`; Befehle
  `Repair` und `Sell` am Enum-Ende; Vorräte, Reparaturaufträge und verkaufte Gebäude im
  Hash und im Spielstand (038). Kleine additive Stellen in `sim.rs`, `sim/combat.rs`,
  `sim/indirect.rs`, `sim/economy.rs` (Kontenhilfen, `remove_structure`), `sim/orders.rs`.
- **Tests:** `crates/sim/tests/logistics.rs` (87 Schuss Panzer IV, danach schweigt jede
  Waffe; Nachschub durch LKW für eigene und verbündete Einheiten gegen Bezahlung, nicht
  für Feinde und nicht außer Reichweite; Treibstoff nach Strecke und Stehenbleiben;
  Bewegungsunfähigkeit nach 8,8-cm-Treffer; Bergung durch LKW-Reparatur; Gebäude
  reparieren und verkaufen; Baustelle abbrechen; Determinismus),
  `crates/net/tests/snapshot_logistics.rs` (Spielstand mitten im Nachschub),
  `crates/ai/tests/ai.rs::ai_sends_empty_units_to_resupply`, Selbsttest-Teil
  `godot/scenes/tools/supply_self_test.gd` und Repair/Sell-Prüfung im Ablauftest.
- **Golden-Hash** bewusst aktualisiert: `0x0b1d_7230_9f70_e885` → `0x1971_0235_5cc0_0b3e`
  (Regeln und Vorräte im Hash, Verbrauch); Demo-Schlacht unverändert (Tick 467,
  Spieler 0 gewinnt). Basisbau-Golden-Hash `0xcdc0_5864_0c6f_5ac8` → `0xe435_5d73_1d25_d25a`.
- **Messwerte:** `just ai-tournament` (20 Spiele je Paarung, Schlacht): Hard/Normal/Easy
  gegen Passive je 100 %; Basisbau 30000 Ticks, 10 Spiele: Normal und Hard gegen Passive
  je 100 %. `just check`, `just selftest`, `just lan-selftest`, `just editor-selftest`,
  `just validate-assets` grün.

Offen: Bestätigung aller Werte und des Designs durch den Nutzer; kein Bergepanzer im
Katalog, kein Abschleppen von Panzern; KI schickt keine LKW zu liegengebliebenen
Einheiten; Infanterie wird nicht aufgefüllt (Ersatz von Soldaten); LKW ohne eigenen
Vorrat; Verkaufen hinterlässt keine Abbau-Animation; Ringe und Warnungen nur headless
geprüft (kein gerendertes Bild); Pioniere reparieren erst, wenn 035 sie baubar macht.
