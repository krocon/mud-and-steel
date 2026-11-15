# Prompt 017 – Kampf-Grundlagen

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md`, `docs/DETERMINISM.md`, `docs/SCALE.md` und
> `data/catalog/SCHEMA.md`.

## 1. Ziel

Einheiten kämpfen: Sie richten Turm oder Wanne auf ein Ziel, laden nach, feuern
Geschosse mit ballistischer Bahn, treffen Gelände, Objekte oder Einheiten, und ein
Treffer durchschlägt die Panzerung je nach Trefferfläche und Auftreffwinkel – oder
nicht. Einheiten ohne Lebenspunkte sind zerstört; Fahrzeuge bleiben als Wrack und
Hindernis liegen. Alles deterministisch in `crates/sim`, alle Einheitenwerte aus dem
Katalog, umgerechnet nur über `data/scale.toml`. Godot zeigt Leuchtspuren,
Lebensbalken und Wracks.

Stand vorher: keine Ausrichtung der Einheiten in der Sim (Godot dreht die Modelle
nach der Fahrtrichtung), keine Waffen- oder Panzerungswerte im `UnitType`,
Geschütze (`gun_*`) und Flugzeuge sind nicht spawnbar (keine Bodengeschwindigkeit).

## 2. Umfang

1. **Kampfregeln** in einer neuen Datei `data/combat.toml` (Ganzzahlen, eingebaut wie
   `terrain.toml`, Teil von `SimConfig` und Zustands-Hash). Sie enthält nur Spielregeln
   (Streuung, Schadensformel, Drehraten pro Kategorie …), keine Einheitenwerte.
2. **Waffen und Panzerung im `UnitType`**, abgeleitet aus dem Katalog:
   - Direktfeuer-Waffen = Katalogwaffen mit Kaliber, Kadenz, Mündungsgeschwindigkeit
     und Reichweite. Einträge der Kategorien `inf_mortar_*`, `mortar_heavy`,
     `gun_howitzer` und `self_propelled` bekommen in 017 keine Waffen (Steilfeuer ist
     ein späterer Prompt). Waffen ohne diese Daten (Handgranaten, Pistolen ohne
     Kadenz, MGs ohne Reichweite, Minen, Bomben) werden nicht benutzt.
   - Reichweite = `range_effective_m` × Direktfeuer-Faktor (1/4), Nachladezeit =
     60 / Kadenz ÷ Zeitfaktor, Mündungsgeschwindigkeit × Zeitfaktor.
   - Panzerung Wanne/Turm vorne/seitlich/hinten in mm und Neigung der Wannenfront aus
     dem Katalog; fehlende Seiten fallen auf den nächsten bekannten Wert zurück
     (hinten → seitlich → vorne, Turm → Wanne); Fahrzeuge ohne Panzerung haben 0.
   - Neues optionales Katalogfeld `gun_traverse_deg` (Seitenrichtbereich der
     Hauptwaffe in Kasematten-Fahrzeugen, gesamt). Ohne das Feld hat ein Fahrzeug
     einen drehbaren Turm (360°). Gefüllt für Jagdpanzer 38(t), StuG III G, SU-85,
     SU-76M, ISU-152, Wespe und M7 Priest, mit Quellen bzw. `uncertain`.
3. **Ausrichtung in der Sim:** Wannenrichtung (Einheitsvektor) folgt der Fahrtrichtung;
   Startrichtung zur Kartenmitte. Zielrichtung der Waffe (Turm) dreht mit der
   Drehrate aus `combat.toml` pro Kategorie (× Zeitfaktor). Kasematten-Fahrzeuge
   richten im Stand die Wanne aus und feuern nur innerhalb ihres Richtbereichs.
4. **Befehle:** `Attack { units, target }` (fährt in Reichweite und bekämpft die
   Einheit), `AttackGround { units, target }` (beschießt einen Bodenpunkt),
   `SetFireMode { units, mode }` mit `FireAtWill` (Standard) und `HoldFire`. `MoveTo`
   und `Stop` heben einen Angriffsbefehl auf; Einheiten feuern auch während der Fahrt
   selbständig (mit Streuungs-Malus).
5. **Zielauswahl** automatisch, nach festen Regeln, alle 10 Ticks oder sobald das
   Ziel ungültig ist: lebende fremde Einheiten in Reichweite der ersten Waffe, sortiert
   nach (Hauptwaffe kann durchschlagen, Ziel kann mich bekämpfen, Entfernung, Id).
6. **Geschosse als Sim-Objekte:** Position und Geschwindigkeit in 3D (`Fixed`), Bahn
   mit Schwerkraft (g × Zeitfaktor²), Rohrerhöhung so, dass der Zielpunkt getroffen
   würde; Streuung über den geseedeten RNG (Seite und Höhe). Pro Tick wird die Strecke
   gegen Höhenfeld (≤ 1 m Schritte), Kartenobjekte (Bäume, Gebäude, Mauern, Hecken bis
   zu ihrer Katalog-Höhe) und Einheiten (gedrehte Quader aus Katalogmaßen) geprüft;
   der früheste Treffer gilt. Eigenbeschuss ist möglich.
7. **Panzerung nach Trefferwinkel:** Die Eintrittsfläche des Quaders bestimmt
   vorne/seitlich/hinten/oben; der obere Teil der Höhe (Anteil aus `combat.toml`) ist
   Turm. Wirksame Dicke = Platte / (cos Auftreffwinkel × cos Plattenneigung), der Kosinus
   nach unten begrenzt.
8. **Durchschlag und Schaden:** kinetische Geschosse
   `P = Kaliber × (v / v_ref)^1,5` mit Abfall über die echte Entfernung, Hohlladungen
   (Mündungsgeschwindigkeit unter einer Schwelle) `P = Kaliber × Faktor`; ±Streuung
   über den RNG. Durchschlag gegen Fahrzeuge kostet `Kaliber² × Faktor` Punkte,
   Infanterie verliert `Kaliber × Faktor` Punkte pro Treffer; Lebenspunkte: Fahrzeuge
   fest, Infanterie `squad_size × Punkte pro Soldat`.
9. **Zerstörung:** Lebenspunkte 0 → zerstört. Zerstörte Einheiten bleiben in der
   Einheitenliste (Ids stabil), nehmen keine Befehle mehr an, bewegen sich nicht und
   feuern nicht. Fahrzeuge werden zum Wrack: Hindernis für Wegfindung
   (`set_path_obstacle`) und Geschosse. Abfrage `Sim::alive_units(player)` für die
   Siegbedingungen von Prompt 027.
10. **Ereignisse im Snapshot:** `Shot`, `Hit` (Einheit mit Durchschlag/Schaden,
    Gelände, Objekt) und `Destroyed` pro Tick, dazu die fliegenden Geschosse,
    Lebenspunkte, Wannen- und Turmrichtung je Einheit. Die Bridge sammelt Ereignisse
    über mehrere Ticks pro Frame, Godot holt sie ab (`take_combat_events`), damit
    Prompt 019 (Effekte) und Sound darauf aufbauen.
11. **Godot:** Leuchtspur pro Geschoss, Lebensbalken über beschädigten Einheiten,
    Wrack (abgedunkelt, nicht mehr wählbar), Turm dreht nach der Sim-Richtung (Knoten
    `Turret`), Wanne nach der Sim-Richtung. Rechtsklick auf eine fremde Einheit =
    Angriff, Strg+Rechtsklick = Bodenangriff, `F` schaltet Feuer frei/halten.
12. **Demo und Tests:** Das Demo-Szenario (acht Panzer IV gegen acht T-34, 120 m
    Abstand) kämpft sofort; der Golden-Hash enthält das Gefecht. Der Godot-Selbsttest
    startet mit `HoldFire` für alle (`SimHost.hold_fire_at_start`), damit die
    Bewegungstests stabil bleiben, und prüft am Ende einen Angriff bis zur Zerstörung.

## 3. Akzeptanzkriterien

1. `just check` grün. Golden-Hash bewusst aktualisiert, Grund im Commit.
2. Test „Panzer A zerstört Panzer B nach erwarteter Tickzahl“: Panzer IV gegen
   stehenden T-34 (Flanke) mit Streuung 0; die Tickzahl wird im Test aus Katalog und
   Regeln hergeleitet (Nachladezeit, Flugzeit, Treffer bis 0 Lebenspunkte).
3. Weitere Sim-Tests: Durchschlagswerte bekannter Kanonen, Front hält/Flanke nicht,
   Winkel erhöht die wirksame Dicke, Geschoss trifft Hügel statt Ziel, Wrack blockiert
   den Weg, `HoldFire`, `Attack` fährt in Reichweite, Befehle an Wracks werden
   ignoriert, gleiche Seeds ergeben gleiche Gefechte.
4. Loopback-Test mit Gefecht: das Demo über zwei Peers, mindestens eine Einheit wird
   zerstört, Hashes jedes Ticks gleich.
5. `just selftest` und `just lan-selftest` grün; `-- --demo` zeigt das Gefecht.
6. Doku: `docs/ARCHITECTURE.md` (Abschnitt Kampf, Befehle), `docs/DETERMINISM.md`
   (Golden-Hash-Historie), `docs/SCALE.md` (Kampfwerte), `data/catalog/SCHEMA.md`
   (`gun_traverse_deg`), `docs/UNIT_CATALOG.md` neu erzeugt.

## 4. Entscheidungen

| Frage | Entscheidung | Warum |
|---|---|---|
| Schadensmodell | Lebenspunkte + Durchschlag ja/nein; Schaden ∝ Kaliber² bei Fahrzeugen, ∝ Kaliber bei Infanterie | Einfach, nachvollziehbar, aus Katalogwerten; ein 7,5-cm-Durchschlag braucht zwei Treffer, MGs schaden Panzern nicht. Module, Besatzungsverluste und Brände sind spätere Verfeinerungen. |
| Durchschlag | `Kaliber × (v/520)^1,5`, −20 % pro km echter Entfernung | Katalog hat keine Durchschlagswerte, aber Kaliber und v₀; die Formel trifft veröffentlichte Werte (KwK 36 ≈ 160 mm, KwK 40 ≈ 130 mm auf 100 m, 0°) und ist ganzzahlig mit `isqrt` lösbar. Keine Werte aus anderen Spielen. |
| Trefferfläche | Eintrittsfläche des gedrehten Quaders statt Winkel-Sektoren | Ergibt Front/Flanke/Heck und Auftreffwinkel aus derselben Geometrie, ohne Sonderregeln. |
| Zielauswahl | Ein Ziel pro Einheit, feste Sortierung, Neuwahl alle 10 Ticks | Deterministisch, billig (O(n²) alle 0,5 s), keine Zufallswahl nötig. |
| Ballistik | Echte Geschosse pro Schuss, Kollision pro Tick-Strecke | Treffer gegen Hügel, Bäume und Wracks fallen von selbst ab; Effekte bekommen Flugbahnen. Bei ≤ 200 Einheiten unkritisch (räumliches Raster: Prompt 016). |
| Automatische Waffen | 25 % der Kadenz (Dauerfeuer) | Der Katalog führt die Kadenz automatischer Waffen zyklisch; die praktische Feuerrate liegt bei einem Viertel bis Sechstel. |
| Drehraten | Pro Kategorie in `combat.toml` | Der Katalog hat keine Turm-Drehgeschwindigkeit; eine Regel pro Kategorie ist ehrlich und balancierbar. |
| Geschütze, Flugzeuge, Steilfeuer | Nicht in 017 | Geschütze brauchen Aufprotzen/Stellungswechsel, Flugzeuge Anflüge, Steilfeuer eigene Zielmarkierung. |

## 5. Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne
Freigabe, keine Tests abschwächen, Commits pro Schritt, `just check` und
`just selftest` grün. Gemeinsame Dateien (`crates/bridge/src/lib.rs`,
`godot/scenes/main.gd`, `self_test.gd`, `justfile`, Doku) nur additiv ändern; die
Bridge-Funktionen kommen in ein eigenes Modul. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren.
