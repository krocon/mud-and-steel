# Prompt 148 – Tunnel durch Felsmassive

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `docs/DETERMINISM.md`; in `docs/ARCHITECTURE.md` die Wegfindung (Flaggen,
> Größenklassen und besonders **„Decks“**: eine Zelle auf einer Brücke ist offen,
> was auch immer darunter liegt), Sicht (Sichtfelder, Budget, Zustände der
> Zellen), Geschossflug und Sprengwirkung, Wracks (sperren ihre Wegzellen),
> Luftunterstützung, Pioniere (`DemolishBridge` / `RepairBridge`) und
> „Destruction“; in `docs/MAPS.md` Kartenformat, „Bridges“ und die
> Skizzen-Tabellen (`[[crossing]]`, `[[road]]`, `[[rail]]`);
> `prompts/done/014-*` (Brücken) und den Prompt der Pioniere; im Code
> `crates/sim/src/map.rs`, `path.rs`, `sim/vision.rs`, die Brücken in Simulation
> und Bridge, `crates/cli/src/gen_sketch.rs`, `crates/editor`. Ist Prompt 115
> (lange Brücken, Unterführung unter Vorlandstücken) erledigt, auch dessen
> Ergebnis: Hat er zwei Ebenen übereinander gebracht, darauf aufbauen.
>
> **Anlass:** Der Nutzer will Gebirgskarten mit Tunneln (Prompts 149 und 150):
> „Tunnel könnte interessant sein“, „Tunnel durch Felsmassive“. Ein Tunnel ist der
> schnelle, gedeckte, enge Weg durch den Berg; die Passstraße darüber ist der
> langsame, offene.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe Bilder und Testläufe
> zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

Tunnel als Teil der Karte und als Spielregel: Einheiten fahren und gehen durch
einen Berg, sind darin von außen nicht zu sehen und nicht zu treffen, stehen in
einer Röhre hintereinander und können eingeschlossen werden. Pioniere sprengen
und räumen Portale. Alles deterministisch in `crates/sim`, Godot zeigt nur.

## 2. Der einfachste tragfähige Entwurf

Ein Tunnel ist das Gegenstück zur Brücke: **eine Linie von Zellen, die offen
ist, was auch immer darüber liegt.**

- **Kartenobjekt** (`Map::tunnels`, im Kartenformat neben den Brücken): zwei
  **Portale** (Position, Höhe), dazwischen eine gerade Strecke oder ein
  Linienzug aus wenigen geraden Stücken, eine **Breite** (Straßentunnel
  einspurig etwa 6 m; Bahntunnel etwa 8–9 m), eine Lastgrenze gibt es nicht,
  wohl aber eine **lichte Höhe und Breite**, an der zu große Fahrzeuge
  scheitern (Katalogmaße; Richtwert Straßentunnel 4,5 m hoch). Länge höchstens
  1500 m.
- **Eine Ebene je Ort:** Über einem Tunnel muss das Gelände **für alle
  gesperrt** sein (Fels, zu steil), außer auf den Vorplätzen der Portale. So
  steht nie eine Einheit über einer anderen, und Kollision, Sicht und Darstellung
  brauchen keine zweite Ebene. `map-check` und der Generator prüfen das. Hat 115
  schon zwei Ebenen gebracht, darf diese Einschränkung fallen; sonst bleibt sie
  und „Straße über dem Tunnel“ ist ein offener Punkt.
- **Wegfindung:** Zellen auf der Tunnellinie sind offen (Flaggen für Gelände,
  Steigung und Rand gelöscht wie bei Decks), kosten wie befestigte Straße, mit
  der Steigung zwischen den Portalen. Die Einheit hat im Tunnel die **Höhe der
  Tunnelsohle** (zwischen den Portalen interpoliert), nicht die des Bergs.
- **Einspurig:** Die Röhre ist so schmal, dass große Einheiten nicht aneinander
  vorbeikommen. Gegenverkehr staut sich (vorhandene Ausweich- und Wartelogik
  prüfen; ein fester Vorrang, z. B. wer zuerst eingefahren ist, falls sie sich
  sonst verklemmen; mit Test). Infanterie kommt an Fahrzeugen vorbei.
- **Wrack im Tunnel:** sperrt ihn wie jede Wegzelle. Räumen über die vorhandene
  Regel für Wracks (Schieben, Bergen, Pioniere), falls es eine gibt; sonst ist
  der Tunnel für Fahrzeuge zu, bis die Regel kommt (offener Punkt), Infanterie
  kommt vorbei.

## 3. Sicht und Feuer

- **Von außen unsichtbar:** Einheiten im Tunnel sieht nur, wer selbst im Tunnel
  ist oder vor einem Portal in der Achse steht (ein kurzer Blick hinein,
  Richtwert im Spiel 40–60 m). Tunnelzellen zählen für kein Sichtfeld von der
  Oberfläche.
- **Von innen blind:** Einheiten im Tunnel sehen nur die Röhre und, nahe am
  Portal, einen schmalen Fächer hinaus. Geteilte Sicht der Verbündeten gilt wie
  sonst.
- **Kein Feuer von oben:** Indirektes Feuer, Bomben, Luftangriffe und
  Sprengwirkung an der Oberfläche treffen im Tunnel nichts. Das ist der Schutz,
  den der Tunnel gibt.
- **Feuer in der Röhre:** Direktes Feuer längs der Achse wirkt normal; die
  Röhre ist eine Todeszone für die erste Einheit. Sprengwirkung im Tunnel wirkt
  auf alle Zellen der Röhre im Wirkradius (keine Verstärkung; die wäre eine neue
  Regel).
- **Beschuss der Portale:** Ein Portal ist ein Kartenobjekt aus Mauerwerk mit
  Deckung für Infanterie daneben; es ist durch Beschuss nicht zu zerstören
  (Fels), nur durch Sprengen (Abschnitt 4).

## 4. Sprengen, Verschütten, Räumen

- Neue Befehle nach dem Muster der Brücken: **`DemolishTunnel { units, tunnel,
  portal }`** und **`ClearTunnel { units, tunnel, portal }`**: Pioniere
  verschütten ein Portal (Ladung und Dauer wie bei einer Brücke) bzw. räumen es
  (deutlich länger).
- **Verschüttet:** Das Portal ist für alle zu. Einheiten im Tunnel bleiben
  unverletzt und fahren zum anderen Portal hinaus. Sind **beide** Portale zu,
  sind sie eingeschlossen: Sie leben, können nichts sehen und nichts tun, bis
  ein Portal geräumt ist (von außen, oder von innen durch eigene Pioniere).
- Einheiten, die beim Verschütten im Portalbereich stehen, nehmen Schaden wie
  durch die Ladung.
- Die Wegsuche rechnet nach jedem Verschütten und Räumen neu, wie bei Brücken.
- **KI:** nutzt Tunnel über die Wegsuche, schießt in die Röhre, wenn sie
  hineinsieht, sprengt und räumt aber nicht selbst (offener Punkt).

## 5. Darstellung (Godot)

- **Portal** als Modell (`env_tunnel_portal`: gemauerter Bogen mit Stützwänden,
  Straßen- und Bahnvariante, verschüttete Variante mit Schuttkegel), mit
  Vorschaubild und `just units-web-sync`; das Gelände am Portal wird vom
  Generator zu einem ebenen Vorplatz und einer senkrechten Stirnwand geformt.
- **Einheiten im Tunnel** werden nicht gezeichnet. Eigene und verbündete
  bekommen eine **Marke an der Oberfläche über ihrer Position** (dezentes
  Zeichen in Spielerfarbe, wandert mit) und bleiben über Marke, Gruppentasten
  und Minikarte anwählbar und befehlbar. Gegner im Tunnel haben keine Marke.
- **Tunnelverlauf** als feine gestrichelte Linie auf Gelände und Minikarte, wenn
  eine eigene Einheit darin ist oder der Spieler ein Portal anwählt bzw. einen
  Befehl gibt (Wegvorschau läuft sichtbar durch den Berg).
- **Ton:** Motoren im Tunnel gedämpft, Hall am Portal, wenn es einfach ist.
- Staub am Portal bei Durchfahrt und beim Verschütten.

## 6. Karten und Werkzeuge

- **Skizzen-Tabelle `[[tunnel]]`** (in `docs/MAPS.md`): `from`, `to` (und
  Zwischenpunkte), `kind` (`road` / `rail`); schließt an `[[road]]` bzw.
  `[[rail]]` an, formt die Vorplätze, setzt die Portale, prüft „eine Ebene je
  Ort“ und meldet verständlich, wenn Gelände über dem Tunnel begehbar wäre.
- `just map-check` und `just map-reach` kennen Tunnel (Erreichbarkeit mit
  offenen und mit verschütteten Tunneln getrennt ausgeben).
- **Karteneditor:** Tunnel anzeigen und beim Speichern erhalten; Setzen und
  Ändern im Editor nur, wenn es mit dem Brückenwerkzeug fast umsonst ist, sonst
  offener Punkt.
- **Testkarte `tunnel_test`** (etwa 800 m × 600 m): ein Felsriegel mit einem
  Straßentunnel, einer Passstraße in Serpentinen darüber hinweg an anderer
  Stelle und einem Bahntunnel.

## 7. Determinismus und Hash

- Alles in Festkomma, kein Zufall außer `sim::Rng`, keine Iteration über
  `HashMap`. Zustand der Tunnel (offen / verschüttet je Portal) im Zustandshash.
- **Karten ohne Tunnel müssen Bit für Bit gleich laufen:** Der Tunnelzustand
  geht nur in den Hash ein, wenn die Karte Tunnel hat (wie bei den Teams). Der
  Golden Hash in `crates/sim/tests/determinism.rs` bleibt; ändert er sich,
  Ursache finden.
- Gespeicherte Spiele und Wiederholungen: Format erweitern, alte Dateien laden
  wie bisher. Lockstep: neue Befehle im Protokoll, Protokollversion erhöhen.

## 8. Stufen

**Stufe 1 – Weg und Karte:** Kartenformat, Wegfindung, Höhe im Tunnel,
`[[tunnel]]`, `tunnel_test`, Portalmodell, Marke an der Oberfläche; ein Panzer
fährt durch.

**Stufe 2 – Sicht, Feuer, Enge:** Abschnitt 3, Einspurigkeit, Wrack; Tests:
unsichtbar von außen, kein Schaden durch Artillerie und Bomben von oben, Feuer
längs der Röhre trifft, Gegenverkehr löst sich auf, Wrack sperrt Fahrzeuge,
Infanterie kommt vorbei, zu großes Fahrzeug wird abgewiesen und plant über den
Pass.

**Stufe 3 – Sprengen und Abschluss:** Abschnitt 4, KI nutzt den Tunnel
(`ai-tournament` auf `tunnel_test`), Messung: Aufbau des Weggitters und
Sichtkosten mit und ohne Tunnel (`docs/PERFORMANCE.md`), `just selftest`,
`just lan-selftest` (Befehle über das Netz, gleiche Hashes), Hilfe und
`docs/CONTROLS.md` (Pionierbefehle), `docs/ARCHITECTURE.md` (eigener Abschnitt
„Tunnels“), `docs/MAPS.md`, `docs/ASSETS.md`, `docs/AI.md`. `just check`.

## 9. Nicht Teil dieses Prompts

- Zwei Ebenen übereinander, wenn 115 sie nicht gebracht hat (Straße über dem
  Tunnel, Kreuzungen im Berg).
- Tunnel bauen im Spiel, Stollen und Bunkeranlagen mit Räumen, Gabelungen im
  Berg.
- Züge, Licht und Scheinwerfer, Rauchgas, verstärkte Sprengwirkung in der Röhre.
- Die Karten selbst (Prompts 149 und 150).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.

| Frage | Entscheidung (vorläufig, Nachtlauf) | Begründung |
|---|---|---|
| Ebenenmodell | Eine Ebene je Ort (wie Prompt 115); Gelände über Tunnel-Innenzellen muss für alle unpassierbar sein | Kein 2-Ebenen-System in 115 vorhanden; `gen-sketch` und `map-check` erzwingen das zuverlässig ohne Kollisions-/Sicht-Duplizierung. |
| Standardmaße & Lichtraumprofil | Straße `width_mm = 6000`, Lichtraum `4500 × 4500 mm`; Bahn `width_mm = 8000`, Lichtraum `5500 × 5500 mm`; Länge `<= 1500 m` | Entspricht den Richtwerten aus Abschnitt 2; überoptionale Felder pro `[[tunnel]]` überschreibbar. |
| Einspur-Vorrang & Gegenverkehr | Einfahr-Wartepflicht vor Portalen bei Gegenverkehr im Tunnel (Gleichstand: niedrigere `UnitId`); bei Frontalbegegnung in der Röhre weicht die Seite näher am Eingangsportal deterministisch auf den Vorplatz zurück und fährt danach selbständig weiter | Verhindert Verklemmen vollständig, ohne neue Zustandsvariablen auf `Unit` einzuführen (0 Hash-Einfluss auf Karten ohne Tunnel). |
| Blick ins / aus dem Portal | `PORTAL_LOOK_M = 55 m`, Achsenkegel `dot >= 0,9` (~25°), Portal-Vorzone `PORTAL_APRON_M = 4 m`, Einsturzradius `PORTAL_COLLAPSE_M = 6 m` | Liegt mittig im geforderten Bereich 40–60 m; erlaubt direktes Feuer am Portal und schirmt das Innere sauber ab. |
| Portal-Mauerwerk & Deckung | `env_tunnel_portal` im Katalog (`neutral.toml`), direkt aus `Map::tunnels()` in `tunnel_view.gd` gerendert; Mauerwerks-Deckung neben den Portalen (`Map::near_tunnel_portal`, 80 % / schwere Deckung) | Hält `data/objects.toml` unverändert, sodass alle bestehenden Golden Hashes Bit für Bit identisch bleiben. |
| Sprengen / Räumen | Pionier-Brückenmodus (`order_bridge_work`) prüft nach Brücken automatisch das nächste Tunnelportal (`order_tunnel_work`); Kosten/Dauer wie `bridge.demolish_*` (30 s) bzw. `bridge.repair_*` (90 s, 3× länger); funktioniert auch von innen für eingeschlossene Pioniere | Spart eine redundante Schaltfläche in der Pionierleiste und nutzt bestehende `works.toml`-Regeln ohne Hash-Änderung. |

## Ergebnis (Nachtlauf 27.11.2026)

- **Simulation (`crates/sim`):**
  - Neues Modul `crates/sim/src/tunnels.rs` (`Tunnel`, `TunnelEntry`, `TunnelKind::{Road, Rail}`) sowie `[[tunnel]]` in `Map` (`map.rs`), `MapDelta::tunnel_portals` (`snapshot.rs`), `Command::{DemolishTunnel, ClearTunnel}` (`command.rs`) und `PROTOCOL_VERSION = 10` (`crates/net/src/lobby/mod.rs`).
  - Wegfindung (`path.rs`): offene Tunnelzellen unter unpassierbarem Fels mit interpolierter Sohlenhöhe und konstanter Portalsteigung, Lichtraumprofil über `Profile::tunnel_tier` (zu große Fahrzeuge weichen über Passstraßen aus), Wracks sperren `SizeClass::Large` während `SizeClass::Small` (Infanterie) vorbeikommt.
  - Sicht, Schutz & Feuer (`sim/vision.rs`, `indirect.rs`, `air.rs`, `combat.rs`, `cover_order.rs`): Einheiten im Tunnel werfen kein Oberflächen-Sichtfeld, sind von außen unsichtbar (außer über Portal-Achsenkegel `55 m`) und innen blind außer längs der Röhre; Artillerie, Bomben und Luftangriffe schlagen auf `Map::surface_height_at` (Berggipfel) ein und verursachen im Tunnel 0 Schaden; direktes Feuer und Explosionen in derselben Röhre wirken normal; Portal-Flügelmauern geben Infanterie schwere Deckung.
  - Einspur-Verkehr & Pioniere (`sim.rs`, `sim/works.rs`): deterministische Portal-Wartelogik & Rücksetz-Auflösung bei Gegenverkehr; `DemolishTunnel` verschüttet Portale inkl. Sprengladung auf Sohlenhöhe, `ClearTunnel` räumt Portale von außen oder innen.
  - Tests: `crates/sim/tests/tunnels.rs` (9/9 grün); **Golden Hashes in `determinism.rs` und `economy.rs` unverändert**.
- **Generator, Werkzeuge & Testkarte:**
  - `crates/mapgen/src/gen_sketch/`: `[[tunnel]]` mit optionalem `ridge_m`/`ridge_width_m`, ebenen Portal-Vorplätzen und automatischer Prüfung der Eine-Ebene-je-Ort-Regel.
  - `data/map_sketches/tunnel_test.toml` & `data/maps/tunnel_test` (800 × 600 m, zentraler Straßentunnel, nördliche Serpentinen-Passstraße, südlicher Bahntunnel): `map-check` 0 Fehler/0 Warnungen (`0x778d8df059f7bbfc`, 67 ms Release), `map-reach` gibt Erreichbarkeit offen & vollständig verschüttet getrennt aus (alle Starts/Versorgungspunkte bleiben über den Pass in `main`).
  - `crates/editor/`: erhält `[[tunnel]]` beim Laden/Speichern und prüft Eine-Ebene-je-Ort in `validate.rs`.
- **Modell & Darstellung (`tools/blender`, `crates/bridge`, `godot`):**
  - `@env_part("tunnel_portal")` in `tools/blender/kit/buildings.py` + `env_tunnel_portal.toml` + `data/catalog/neutral.toml` (`Road`, `Rail`, `RoadCollapsed`, `RailCollapsed`, `408/800 tris`, `8.00 × 11.50 × 7.80 m`), Thumbnail gerendert, `just units-web-sync` & `just catalog-doc` ausgeführt.
  - `crates/bridge/src/works_funcs.rs` (`tunnels`, `tunnel_portals_open`, `units_in_tunnel`, `surface_height_at`, `order_tunnel_work` + Fallback in `order_bridge_work`) und `godot/scenes/world/tunnel_view.gd` + `main.gd` + `minimap.gd`: Portalmodelle mit Einsturzwechsel & Staub, Ausblenden der 3D-Modelle im Tunnel, Spielerfarben-Oberflächenmarken über eigenen/verbündeten Einheiten, gestrichelte Verlaufslinie auf Gelände und Minikarte.
- **Prüfung & Doku:** `just check`, `just selftest`, `just lan-selftest`, `just editor-selftest`, `just glb-check` (467 GLBs), `just l10n-check` (1481 Keys), `ai-tournament --map tunnel_test --games 2` (0,3 s, 0 Timeouts) alle grün; `docs/{ARCHITECTURE,MAPS,ASSETS,CONTROLS,PERFORMANCE,AI}.md` aktualisiert.
- **Offen:** Zweite Ebene (Straße über Tunnel), Wrack-Räumung im Tunnel, Tunnel-Werkzeug im Karteneditor, KI-Sprengung/-Räumung von Tunnelportalen, gedämpfter Tunnel-Motorenton.
