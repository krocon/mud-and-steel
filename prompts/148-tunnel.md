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
