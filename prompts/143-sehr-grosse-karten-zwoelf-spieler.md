# Prompt 143 – Sehr große Karten und zwölf Spieler

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`, dann
> `docs/TEAMS.md` ganz (Teams, Plätze, Lobby: bisher höchstens 6 Plätze),
> `prompts/done/037-teams-ki-slots.md`, in `docs/MAPS.md` das Kartenformat
> (Raster 2 m, „Measurements (`test_large`, 4 km × 4 km)“, Grenze der Skizzen bei
> 4 km), `docs/PERFORMANCE.md` ganz (Budgets, `just bench`, `just bench-view`),
> `docs/DETERMINISM.md`, `docs/LAN.md`, `docs/ONLINE.md`, `docs/AI.md`; im Code
> `crates/sim/src/map.rs`, `path.rs`, `positions.rs`, `sim/vision.rs`
> (`MAX_PLAYERS`), `crates/net` (Lobby-Protokoll), `crates/cli/src/gen_sketch.rs`,
> `crates/editor`, die Lobby- und Menüszenen in `godot/`.
>
> **Anlass:** Prompt 145 will eine Stadtkarte für **zwölf Spieler** bauen, größer
> als alles bisher. Heute enden Karten bei 4 km Kantenlänge und die Lobby bei
> sechs Plätzen. Dieser Prompt klärt mit Messungen, wie groß eine Karte sein
> darf und hebt die Grenzen an, soweit die Budgets es tragen.
>
> **Dieser Prompt läuft in drei Stufen.** Nach jeder Stufe die Messwerte als
> Tabelle zeigen und ohne Halt weiterarbeiten.

## 1. Ziel

- Karten bis **6 km × 4 km** (24 km², das Anderthalbfache von heute), wenn die
  Messungen es tragen; sonst die größte Größe, die im Budget bleibt, mit
  Begründung.
- **Zwölf Spieler** in einer Partie: Simulation, Lobby (Einzelspieler, LAN,
  Steam), Farben, Anzeige, KI, Turnier.
- **Mehr als zwei Teams** wählbar (z. B. 4 × 3 oder 3 × 4), soweit
  `SimConfig::teams` und die Siegregel das schon tragen; nur Lücken in Lobby und
  Anzeige schließen.

## 2. Stufe 1 – Messen, bevor etwas gebaut wird

Eine Testkarte `test_huge` (6 km × 4 km, erzeugt wie `test_large`, zwölf Starts
in zwei Teams) und eine dichte Variante (etwa 2500 Gebäude wie `river_city`,
hochgerechnet auf die Fläche). Dafür die Größengrenze in einem Arbeitsstand
anheben. Messen und mit `test_large` vergleichen:

| Größe | Was |
|---|---|
| Speicher | Höhen- und Geländeraster, Weggitter je Bewegungsklasse, Sicht je Spieler (zwölf Spieler!), Objektindex; Gesamtspeicher des Spiels |
| Ladezeit | Karte laden, Gelände-Mesh, Import |
| Tick | Zeit je Tick bei 0, 500, 1500, 3000 Einheiten, verteilt auf zwölf Spieler; schlechtester Tick |
| Wegsuche | längster Weg quer über die Karte, je Bewegungsklasse; Neuberechnung nach gesprengter Brücke |
| Sicht | Zeit für die Sichtberechnung von zwölf Spielern mit geteilter Sicht |
| Bildrate | jede Ansicht, über Stadt und Wald, mit Gefecht (`just bench-view`) |
| Netz | Größe der Befehle und Prüfsummen je Tick bei zwölf Spielern (Lockstep), Zeit bis zum Start in der LAN-Lobby |
| Minikarte und Nebel | Auflösung und Zeit bei 6 km |
| Gespeicherte Spiele und Wiederholungen | Größe, Ladezeit |

Ergebnis als Tabelle in `docs/PERFORMANCE.md`, mit dem Urteil: Welche Größe und
welche Einheitenzahl bleiben im Budget? Wo sind die drei größten Kosten?

## 3. Stufe 2 – Grenzen anheben

Nur was die Messung verlangt, in dieser Reihenfolge und so einfach wie möglich:

- **Kartengröße:** Grenze in Skizzen-Generator, Karteneditor, `map-check`,
  Kartenformat und Simulation auf die getragene Größe; Stellen mit fest
  eingebauten 4 km finden (auch Festkomma-Wertebereiche: Abstände und Quadrate
  von Abständen bei 6 km × 4 km dürfen in `sim::Fixed` nicht überlaufen; mit
  Test an den Kartenecken).
- **Engpässe** aus der Messung beheben, wenn es ohne Regeländerung geht (z. B.
  Sichtraster je Team statt je Spieler bei geteilter Sicht, Weggitter erst bei
  Bedarf je Klasse, gröbere Minikarte). Keine Threads in `crates/sim`, keine
  Floats, keine Iteration über `HashMap`.
- **Zwölf Plätze:** Lobby-Protokoll und Oberfläche (Einzelspieler, LAN, Steam)
  von 6 auf 12 Plätze, Platz *i* bleibt Spieler *i*; ältere Karten und
  gespeicherte Spiele laden wie bisher. Protokollversion erhöhen, wenn sich das
  Format ändert (`docs/LAN.md`).
- **Zwölf Farben**, klar unterscheidbar auf Gelände, Minikarte und im Nebel,
  auch für Farbfehlsichtige (Paar aus Farbe und Zeichen bzw. Nummer am Marker);
  Vorschlag als Bild, vorläufig entscheiden.
- **Teams:** In der Lobby je Platz ein Team von 1 bis 6 wählbar; Vorgabe aus
  `[[start]]` der Karte. Siegregel, geteilte Sicht, Pings, Punktetafel und
  Ergebnisbildschirm mit drei und vier Teams prüfen.
- **Anzeige:** Spielerliste, Punktetafel, Rohstoffanzeige der Verbündeten,
  Ergebnisbildschirm für zwölf Einträge.
- **KI:** zwölf Computerspieler laufen im Zeitbudget (Denkzeit je Tick über die
  Spieler verteilen, falls nötig); `just ai-tournament` mit mehr als zwei
  Spielern je Seite.

**Golden Hash:** Bestehende Karten und Partien müssen Bit für Bit gleich laufen.
Ändert sich der Hash, Ursache finden; nur wenn eine Regel sich gewollt ändert
(das ist hier nicht vorgesehen), neu setzen und im Commit nennen.

## 4. Stufe 3 – Prüfen und Abschluss

- Tests: Partie mit zwölf Spielern in zwei, drei und vier Teams bis zum Sieg;
  Determinismus (zweimal gleicher Lauf, gleicher Hash) auf `test_huge`;
  Festkomma an den Kartenecken; Laden alter Karten und Spielstände.
- `just lan-selftest` mit mehr als zwei Instanzen (so viele, wie der Rechner
  trägt; mindestens vier), `just selftest`, `just bench`, `just bench-view` auf
  `test_huge`.
- `docs/TEAMS.md`, `docs/MAPS.md` (neue Grenze, Tabelle der Größen),
  `docs/PERFORMANCE.md`, `docs/LAN.md`, `docs/ONLINE.md`, `docs/CONTROLS.md`
  (falls sich die Anzeige ändert). `just check`.
- **Empfehlung für Prompt 145** festhalten: Kartengröße, Zahl der Gebäude,
  Obergrenze für Einheiten je Spieler bei zwölf Spielern.

## 5. Nicht Teil dieses Prompts

- Die Karte selbst (Prompt 145) und ihre Bauten (Prompt 144).
- Karten über 6 km × 4 km, nachladbare Kartenteile, feineres oder gröberes
  Raster als 2 m.
- Mehr als zwölf Spieler, Zuschauerplätze, Beitritt in laufende Partien.
- Online-Tests über Steam mit zwölf echten Spielern (nur was ohne zweite
  Maschine prüfbar ist; der Rest als offener Punkt).

## Entscheidungen

Was die Session selbst entscheidet, hier als Tabelle (vorläufig), mit Begründung.
