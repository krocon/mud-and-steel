# Prompt 046 – Befehle und Formationen Stufe 2

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/CONTROLS.md`, `docs/ARCHITECTURE.md` (Auswahl, Befehlsketten aus 031, Wegfindung).
>
> **Konkretisiert** (siehe Abschnitt 4): Ausgangslage geprüft, Entscheidungen
> mit Begründung als Tabelle festgehalten.

## 1. Ziel

Steuergruppen, Wegpunkte, Patrouille und Formationsbewegung gibt es (009, 031). Es fehlen
die Befehle, mit denen man eine Truppe wirklich führt: vorrücken und dabei kämpfen, ein
Gebiet sichern, Formationen bewusst wählen und Panzer gezielt ausrichten.

## 2. Umfang

1. **Angriffsbewegung** (`AttackMove`): zum Ziel fahren, unterwegs auf Feinde in
   Reichweite reagieren, danach weiter. Standard für Infanterie, wählbar für Fahrzeuge.
2. **Gebiet sichern** (`Secure`): Gebiet (Kreis) halten, Feinde darin bekämpfen, nicht
   weiter als ein Radius verfolgen, danach zurück in Stellung.
3. **Formationen:** Linie, Keil, Kolonne, Breitkeil, aufgelockert; **Auto** wählt nach
   Gelände vor der Gruppe (Straße/Engstelle → Kolonne, offenes Feld → Keil/Linie, Wald →
   aufgelockert). Deterministisch in der Sim, Wahl im HUD (031).
4. **Ausrichtung per Ziehen:** Rechtsklick-Ziehen legt Zielpunkt und Blickrichtung fest;
   die Formation wird senkrecht dazu aufgestellt, Panzer zeigen die Frontpanzerung.
5. **Eigene Trupps:** prüfen, ob die Steuergruppen (031) als „Trupps“ mit Namen und
   gemeinsamer Formation reichen; sonst schlank ergänzen.
6. **KI** nutzt Angriffsbewegung und Ausrichtung zum Gegner.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für AttackMove, Secure, Auto-Formation, Ausrichtung;
   neue `Command`s hinten ans Enum; Golden-Hash begründet.
2. `just selftest`, `just lan-selftest` grün; `docs/CONTROLS.md` aktualisiert.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## 4. Konkretisierung (Ausgangslage und Entscheidungen)

**Ausgangslage im Code (main nach 031, 034, 036, 037, 038, 040):**

- `MoveTo` stellt eine Gruppe in ein quadratisches Raster um das Ziel
  (`formation_slots`), ein gemeinsamer Gruppenpfad (`request_group_path`),
  jede Einheit folgt ihm mit ihrem Versatz (`formation_route`).
- Einheiten mit „Feuer frei“ wählen selbst Ziele (`auto_target`) und
  schießen auch während der Fahrt; ein `Attack` fährt in Reichweite.
- 031: Wegpunktketten und Patrouille in `sim/orders.rs` (`OrderQueue` pro
  Einheit), HUD-Befehlsleiste mit Zielbefehlen (`command_mode`),
  Steuergruppen nur in Godot (`selection.gd`, 1…9).
- 038: `Sim` wird komplett per serde gespeichert; neue Felder sind
  automatisch im Spielstand, der Rules-Hash erkennt geänderte Layouts.
- KI (`crates/ai`) bewegt Gruppen mit `MoveTo` und greift mit `Attack` an.

**Entscheidungen:**

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Zwei neue Befehle am Enum-Ende: `FormationMove { units, target, formation, facing, engage }` und `Secure { units, center, radius_m, formation, facing }`. `MoveTo` bleibt unverändert. | Ein Befehl für alle Formationsbewegungen statt je einer für Bewegen/Angriffsbewegung; `MoveTo` bleibt für Wegpunkte, KI-Rückzug und alte Replays gleich (Golden-Hash unverändert). |
| 2 | Angriffsbewegung = `FormationMove` mit `engage`: `Never` (reine Bewegung), `Infantry` (Standard bei Rechtsklick: nur Trupps halten an), `All` (Angriffsbewegung für alle). | Erfüllt „Standard für Infanterie, wählbar für Fahrzeuge“ auch in gemischten Gruppen, ohne die Gruppe auf zwei Befehle (und zwei Formationen) aufzuteilen. |
| 3 | Eine Einheit mit Angriffsbewegung hält an, solange sie ein automatisches Ziel hat (Feind in Reichweite, gesehen, verwundbar), und fährt danach mit eigenem Pfad zu ihrem Platz weiter. „Feuer halten“ hält nie an. | Nutzt die vorhandene Zielwahl (017/018) statt einer zweiten; „in Reichweite“ steht so im Prompt. |
| 4 | `Secure`: Gruppe nimmt die Formation um den Mittelpunkt ein; jede Einheit greift (alle `retarget_ticks`) den nächsten Feind an, den ihr Spieler im Kreis sieht und den sie verwunden kann; verlässt das Ziel den Kreis, bricht sie ab und kehrt auf ihren Platz zurück. Radius 10–300 m, im HUD 80 m [provisional]. | „Nicht weiter als ein Radius verfolgen“: das Ziel muss im Kreis sein, die Einheit fährt nur bis in Waffenreichweite, bleibt also nahe am Kreis. Wiederverwendung von `Attack` (Suche, Sichtregeln). |
| 5 | Formationen: Linie (max. 8 je Glied), Keil (Spitze und zwei Glieder = 5, ein Panzerzug; weitere folgen in Gliedern zu 5 dahinter), Kolonne (ab 9 Einheiten zwei Reihen), Breitkeil (umgekehrter Keil), aufgelockert (versetzte Reihen, doppelter Abstand); Abstand wie bisher `formation_spacing` der größten Einheit [provisional]. | Einfache, ganzzahlige Formen; Werte sind Spielregeln, keine Katalogwerte. |
| 6 | **Auto** wird beim Befehl in der Sim aufgelöst (einmal, nicht unterwegs neu): 1. freie Breite quer zur Blickrichtung am Ziel < 3 Plätze (für die einschränkendste Größenklasse) → Kolonne; 2. ≥ 30 % der Zellen um das Ziel durch Objekte für Panzer gesperrt → aufgelockert; 3. Ziel auf Straße und ≥ 50 % des Wegs Straße → Kolonne; 4. sonst Keil (mit Fahrzeugen) bzw. Linie (nur Infanterie). Schwellen [provisional]. | Deterministisch, billig (wenige Stichproben), kein neuer Zustand. Unterwegs umformieren würde den Gruppenpfad (009) neu bauen und ist für Stufe 2 zu viel. |
| 7 | Plätze werden Glied für Glied von vorn vergeben: die vordersten Einheiten (entlang der Blickrichtung) ins vorderste Glied, im Glied von links nach rechts. | Unabhängig von der Reihenfolge im Befehl, wenig Kreuzen. |
| 8 | Ausrichtung: `facing` im Befehl (Standard: Fahrtrichtung); die Formation steht senkrecht dazu, und stehende Einheiten ohne Ziel drehen ihre Wanne dorthin (Panzer zeigen die Frontpanzerung; Trupps drehen ihre Formation). Rechtsklick-Ziehen in Godot liefert Zielpunkt (Drücken) und Richtung (Loslassen). | Drehen nur im Stand, damit Kampf (Turm/Kasematte) Vorrang hat. |
| 9 | Neuer Zustand `Unit::drill` (Blickrichtung, Aufgabe) in `sim/drill.rs`; gespeichert per serde (038), im State-Hash nur für Einheiten mit Aufgabe/Blickrichtung (hinten angehängt). | Spiele ohne die neuen Befehle behalten ihren Hash: Golden-Hash unverändert. |
| 10 | Andere Befehle (`MoveTo`, `Stop`, `Attack`, `Garrison`, `Patrol`, `Embark` …) beenden Aufgabe und Blickrichtung; ein Wegpunkt (Shift) beendet nur das Sichern, nach einer Angriffsbewegung geht die Kette danach weiter. | Wie die Ketten aus 031. |
| 11 | Eigene Trupps: **die Steuergruppen reichen**; jede Gruppe merkt sich in Godot ihre Formation (Abruf setzt die Formation der Leiste). Keine Namen [provisional]. | Kein neuer Sim-Zustand, die Formation reist ohnehin im Befehl mit. Namen bringen ohne Gruppenleiste wenig. |
| 12 | HUD: Formationswahl als erste zwei Zellen der Befehlsleiste (5 statt 4 Spalten, damit sie nicht höher wird; Taste `,` schaltet weiter), Zielbefehle „Advance“ = Angriffsbewegung (Taste Y) und „Secure“ (Taste I) [provisional]; M bleibt „reine Bewegung“ (auch Infanterie hält nicht an). Ziehen mit rechts zeigt einen Pfeil; Sicherungskreise und Angriffsbewegungswege zeigt das Wegpunkt-Overlay. | Freie Tasten, alle neu belegbar. Eine höhere Leiste hätte Klickpunkte des Selbsttests und der Karte verdeckt. |
| 13 | KI (Normal, Schwer): Vormarsch zu Sichtung/Gegnerbasis/Versorgungspunkt als `FormationMove` mit `engage = All`, Formation Auto, Blickrichtung zum Ziel; Sammelpunkt vor dem Angriff mit Blick zur Gegnerbasis. Leicht bleibt bei `MoveTo` [provisional]. | „KI nutzt Angriffsbewegung und Ausrichtung zum Gegner“; Leicht soll leicht bleiben. |

**Umsetzung und Erfahrungen:**

- Ein Keil aus 8 Panzern ohne Begrenzung war 120 m breit; die Selbsttest-
  Gruppe stand dadurch so, dass sie später alle Gegner abschoss (Sieg mitten
  im Selbsttest). Mit dem Zug-Keil (5) bleibt die Gruppe kompakt.
- Der Selbsttest der Befehle prüfte die Mitte der Formation als Mitte der
  Bounding-Box; Formationen sind jetzt über den Mittelwert zentriert (beim
  alten Raster identisch), die Prüfung nutzt den Mittelwert mit derselben
  Toleranz.
- Golden-Hash unverändert (`MoveTo` und KI-freie Szenarien unberührt).
