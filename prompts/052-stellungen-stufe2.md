# Prompt 052 – Stellungen Stufe 2: Gräben, Sperren, Tarnung

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Pioniere aus 035, Krater, Infanterie), `docs/MAPS.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

035 bringt Minen, Draht, Gräben und Pionierbefehle. Dieser Prompt macht Stellungen zum
Kernstück der Verteidigung: Gräben sehen von Hand gegraben aus, Soldaten kämpfen von der
Grabenwand, Sperren werden als Linie gezogen, Panzer graben sich ein, Tarnnetze
verbergen Stellungen vor Luftaufklärung.

## 2. Umfang

1. **Gräben:** unregelmäßige, handgegrabene Optik mit Brustwehr; Zickzack-Verlauf; als
   Linie zeichnen wie in 035; Soldaten besetzen Feuerplätze an der Wand und an Ecken.
2. **Sperren als Linie zeichnen:** Stacheldraht, Panzersperren (Höcker, Igel) entlang
   einer gezogenen Linie, Kosten pro Meter.
3. **Panzer eingraben:** Pioniere heben eine Fahrzeugstellung aus; nur Turm sichtbar
   und treffbar (Trefferzonen aus 017 anpassen), Fahrzeug kann nicht fahren, bis es
   ausfährt.
4. **Tarnnetze:** über Stellungen/Fahrzeugen; verbergen vor Luftaufklärung (033) und
   verringern Sichtweite des Gegners, solange nicht gefeuert wird.
5. **Bunker und MG-Nester** am Grabenrand als baubare Stellungen (Modelle aus 041).
6. **KI** baut Stellungen an Engstellen, wenn sie verteidigt.
7. **Grabenstellung wie im Referenzbild des Nutzers** (nicht im Repo, nur Wirkung):
   ein Zickzack-Graben mit Schulterwehr aus Erde, Sandsäcken und Holzverschalung,
   Schützenständen in den Knicken, Querriegeln, Verbindungsgräben nach hinten und
   Unterständen; als Stützpunkt um einen Versorgungspunkt (Flagge) gelegt.
   - **Vorgefertigte Grabenstellungen** als Kartenobjekte (Editor 028), damit Karten
     schon zu Beginn befestigte Punkte haben; Bau durch Pioniere wie in 035.
   - Gräben schneiden sichtbar ins Gelände (Höhenfeld wie Krater aus 021 oder als
     eigenes Mesh mit Boden-Ausschnitt); Infanterie darin ist nur von oben/nah treffbar.
   - Darstellung: aufgeworfene Erde am Rand, Trampelpfade, zerschossene Stellen.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests für Eingraben (nur Turm trifft), Tarnnetz und
   Sperrlinien; Golden-Hash begründet.
2. `just selftest`, `just editor-selftest`, `just gallery-check` grün.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach 035, 038, 041, 033, 022, 017): Pioniere legen über
`Lay`/`Dig` Linien in Stücken (Draht und Graben 4 m, Igel alle 3 m) als Baustellen an
(`crate::works`, `crate::sim::works`, `data/works.toml`); Linien zieht man schon mit zwei
Klicks. Gräben und Schützenlöcher senken über die Krater-Maschinerie das Höhenfeld (2-m-
Raster) um nur 0,5 m, weil sonst die Wegzelle für Infanterie zu steil wird; gegrabene
Stützstellen geben stehenden Trupps Deckung (60/75/85 %), auch gegen Splitter.
`WorkKind` ist zum Anhängen vorbereitet. Bunker gibt es zweifach: als Kartenobjekt
`fort_bunker_mg` (besetzbar wie ein Haus, 022) und als Befestigungen des Basisbaus
(`de_bunker_tobruk`, `su_bunker_dzot`, `us_bunker_mg`, kämpfen mit Katalogwaffen, nur im
Bauradius). Trefferzonen (017): Turm ab `turret_height_pct` der Höhe, sonst Wanne.
Sicht (018) ist zellbasiert, Luftaufklärung (033) fügt Sichtfelder ohne Einheit hinzu,
Mündungsfeuer verrät Schützen (040). Der Spielstand (038) speichert den ganzen `Sim` per
serde. Karten haben benannte Marker (`[[marker]]`, Versorgungspunkte heißen `supply_*`),
der Editor (028) erhält sie, hat aber kein Werkzeug dafür. Die vorhandenen
Grabenmodelle (`fort_trench_section`) liegen unter der Geländeoberfläche und sind im Spiel
unsichtbar, weil das Gelände nicht ausgeschnitten wird.

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | Alles bleibt in der Feldbefestigungs-Schicht aus 035: neue Arten werden an `WorkKind`, `LayKind`, `DigKind` angehängt, **keine neuen `Command`-Varianten**. Neu: `Dig`: Verbindungsgraben (`CommTrench`), Fahrzeugstellung (`VehiclePit`), MG-Nest (`MgNest`), Unterstand (`Dugout`); `Lay`: Tarnnetz (`CamoNet`), Bunker (`Bunker`), Höckerhindernis (`Teeth`). | Befehle, Baustellen, Kosten, Abbruch, Spielstand und Wiederholungen funktionieren unverändert; angehängte Enum-Varianten halten alte Replays lesbar. |
| 2 | Neuer Zustand (`Positions`: eingegrabene Fahrzeuge, Feuerzeitpunkte unter Netzen, Stützstellen-Klassen, ausgebaute Stützpunkte) liegt in `Works` mit `#[serde(default)]`; neue Regeln in `[positions]` von `data/works.toml`. Beides geht erst in den State-Hash, sobald etwas aus 052 benutzt wurde. | Spielstände (038) enthalten es automatisch; Partien ohne Stellungen (Demo-Schlacht) behalten ihren Hash, parallele Prompts bekommen keine Golden-Hash-Konflikte. |
| 3 | **Grabentiefe:** das Höhenfeld bleibt bei 0,5 m (Raster 2 m, Steigungsgrenze). Die „echte“ Tiefe zeigt Godot: ein prozedurales Grabennetz mit Bretterverschalung, Laufrost auf -0,3 m und Brustwehr aus Erde/Sandsäcken bis +0,7 m; sichtbar ≈ 1–1,1 m tief. **[vorläufig]** | Ein feineres Höhenfeld oder ein Gelände-Ausschnitt im Shader wäre ein großer Umbau (Raster, Wegzellen, Sicht); so passen Sim (Deckung) und Bild zusammen, Soldaten stehen bis zur Brust im Graben. |
| 4 | **Zickzack:** ein Graben `Trench` wird nicht mehr gerade, sondern abwechselnd ±1,5 m quer zur Linie versetzt in 4-m-Stücke zerlegt (Knick = Schützenstand). `CommTrench` bleibt gerade. **[vorläufig: Amplitude]** | Handgegrabene Optik und Splitterschutz durch Knicke; jedes Stück bleibt eine 4-m-Baustelle wie in 035. |
| 5 | **Feuerplätze:** fertige Gräben und MG-Nester liefern Deckungsplätze für die Deckungswahl aus 022: alle Knicke (bevorzugt, zuerst) und alle 2 m entlang der Wand, 0,3 m zur Blickrichtung des Trupps versetzt. Stehende Trupps besetzen sie wie Kraterplätze. | „Soldaten besetzen Feuerplätze an der Wand und an Ecken“ ohne neue Infanterielogik (eine Zeile in `choose_spots`). |
| 6 | **Nur von oben/nah treffbar:** die Grabendeckung (Brustwehr) fängt direkte Schüsse nur, wenn der Schütze weiter als `close_mm` (25 m) entfernt ist und das Geschoss flacher als 30° einfällt; Splitter von oben und Nahschüsse treffen normal. **[vorläufig]** | Die Wirkung aus dem Prompt mit der vorhandenen Deckungsprobe; gilt nur für von Pionieren gegrabene Stützstellen, Krater bleiben wie in 021. |
| 7 | **MG-Nest** (gegraben, Sandsackring) und **Unterstand** (gegraben, Holzdecke): eigene Deckungswerte je Stützstelle (Nest 85/90/95 %, Unterstand 95 % in jeder Haltung), Unterstände schützen damit vor Artillerie. Beide nur am Grabenrand (≤ 8 m von einem fertigen eigenen Graben). **[vorläufig]** | Teil des Referenzbildes (Schützenstände, Unterstände); Deckung über dieselbe Probe wie Gräben. |
| 8 | **Bunker** (`Lay`/`Bunker`, Punkt, Richtung = zweiter Punkt): nur am Grabenrand; nach der Bauzeit entsteht die Basisbau-Befestigung der Fraktion (`de_bunker_tobruk`, `su_bunker_dzot`, `us_bunker_mg`, Modelle aus 023/041) als fertiges Bauwerk des Spielers; ohne Bauradius, ohne Kaserne. Fehlt der Typ in der Partie, wird nichts angelegt. **[vorläufig: Kosten 0/80/60, 60 s]** | „Bunker am Grabenrand als baubare Stellungen“ mit vorhandenen Einheiten, die schon feuern, Treffer nehmen und im Spielstand sind. |
| 9 | **Fahrzeugstellung** (`Dig`/`VehiclePit`, Punkt): ein Fahrzeug des Teams, das `enter_s` (8 s) ohne Fahrbefehl darin steht, ist **eingegraben**: nur der Teil über `turret_height_pct` (Turm, bei Sturmgeschützen der Aufbau) ist treffbar; Treffer darunter schlagen in die Brustwehr (Bodentreffer); gegnerische Schützen zielen auf den Turm. Ein Fahrbefehl ist das Ausfahren: es endet sofort, wieder eingraben dauert `enter_s`. **[vorläufig: kein Verzögern beim Ausfahren]** | Treffer über die Trefferzonen aus 017; „kann nicht fahren, bis es ausfährt“ ohne Geschwindigkeiten umzuschalten, die Befehlsprüfungen (`mobile_indices`) sonst verwerfen würden. |
| 10 | **Tarnnetz** (`Lay`/`CamoNet`, Punkt, Radius 5 m): eigene/verbündete Einheiten darunter, die stehen und seit `fire_reveal_s` (20 s) nicht gefeuert haben, sieht der Gegner nur mit einer Bodeneinheit innerhalb `spot_mm` (60 m) — Luftaufklärung und weiter entfernte Beobachter sehen sie nicht. Fertige Stellungen unter einem gegnerischen Netz (und das Netz selbst) liefert `works_for` dem Gegner ebenso nur in dieser Nähe. **[vorläufig]** | „Verbergen vor Luftaufklärung und verringern Sichtweite, solange nicht gefeuert wird“ als Nachbearbeitung der Sichtmaske, ohne das Sichtgitter zu ändern. |
| 11 | **Sperrlinien:** Draht und Igel gab es als Linie; neu Höckerhindernis (`Teeth`, Betonhöcker alle 1,5 m, sperrt Panzer, nicht plattwalzbar, Räumen 30 s). Godot zeigt beim Ziehen eine Vorschau mit Länge und Gesamtkosten (Kosten pro Stück × Stückzahl, pro Meter angezeigt). | „Sperren als Linie zeichnen, Kosten pro Meter“; Höcker gehören zu „Panzersperren (Höcker, Igel)“. |
| 12 | **Vorgefertigte Stützpunkte** als Kartenmarker `strongpoint_<name>` (keine neuen Kartenobjekte, kein neues Kartenformat): beim ersten Tick baut die Sim um den Marker einen **neutralen** fertigen Stützpunkt: Zickzack-Grabenring (Radius 16 m, acht Seiten), vier MG-Nester in den Diagonalen, zwei Verbindungsgräben zu zwei Unterständen in der Mitte, Drahtring (Radius 30 m) mit zwei Lücken. Liegt ein Versorgungspunkt (Flagge) am Marker, umschließt ihn der Ring. Der Editor bekommt ein Werkzeug „Strongpoint“ (Marker setzen/entfernen, Umriss als Vorschau). **[vorläufig: Maße, neutral]** | Kartenmarker hält Kartenformat, Karten-Hash-Regeln und Katalog unverändert; neutral, weil Kartenobjekte keinen Besitzer haben (wie die Minenfelder aus 035). Rundumverteidigung braucht keine Richtung. |
| 13 | **KI** (Normal, Hard), wenn sie verteidigt (sie hat einen Pioniertrupp und ihre Engstellen aus 035): nach den Minen gräbt sie an der ersten Engstelle eine Grabenlinie quer zum Weg zum Gegner, zieht Draht 12 m davor, setzt ein MG-Nest und ein Tarnnetz; höchstens eine Stellung je Engstelle, zwei insgesamt. **[vorläufig]** | „KI baut Stellungen an Engstellen“ nur über normale Befehle und den `PlayerView`. |
| 14 | **Modelle:** Tarnnetz (`fort_camo_net`), Höcker (`fort_dragon_teeth`) und Unterstand-Eingang (`fort_dugout`) als neue neutrale Katalogeinträge mit Quellen und Blender-Bauplänen (Env-Kit); Graben, Verbindungsgraben, MG-Nest-Ring und Fahrzeugstellung prozedural in Godot (sie folgen beliebigen Linien). Sandsack-Stellung, Igel, Draht, Bunker: vorhandene Modelle. | Gräben folgen frei gezogenen Linien, feste Modelle passen nicht; die drei festen Objekte brauchen echte Maße (`docs/SOURCES.md`). |
| 15 | **Darstellung:** aufgeworfene Erde an der Brustwehr, Laufroste, Bretter- und Faschinenverschalung, Sandsäcke an den Feuerplätzen und Knicken, Trampelpfade hinter den Gräben (dunkler Streifen), zerschossene Stellen: Krater in ≤ 3 m reißen die Brustwehr eines Stücks auf (verstreute Säcke). Bildnachweis: `just position-shots build/positions`. | Wirkung des Referenzbildes ohne es nachzubauen. |
