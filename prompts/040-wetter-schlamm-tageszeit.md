# Prompt 040 – Wetter, Schlamm und Tageszeit

> In einer frischen Session im Repo-Ordner ausführen. Lies zuerst `CLAUDE.md`,
> `docs/ARCHITECTURE.md` (Licht/Biome, Gelände, Wegfindung, Nebel), `docs/MAPS.md`.
>
> **Entwurf:** vor dem Umsetzen konkretisieren: Ausgangslage im Code prüfen,
> Entscheidungen mit Begründung als Tabelle festhalten, Umfang schärfen.

## 1. Ziel

„Mud and Steel“ hat noch keinen Schlamm. Nach diesem Prompt gibt es Wetter (klar,
bedeckt, Regen, Schnee, Nebel) mit spielerischer Wirkung: Regen weicht Böden auf,
Kettenfahrzeuge kommen im Schlamm langsamer voran, Radfahrzeuge bleiben stecken,
Straßen werden wichtig; Nebel und Dämmerung verkürzen die Sicht.

## 2. Umfang

1. **Sim:** Wetterzustand pro Partie (fest gewählt oder als deterministischer Ablauf),
   Bodenfeuchte pro Geländeart und Zelle, Geschwindigkeitsfaktoren nach Fahrwerk
   (Rad, Halbkette, Kette) und Boden; Wegfindungskosten passen sich an (Teil-Neubau wie
   bei Zerstörung). Sichtweite nach Wetter und Tageszeit. Alles im Hash.
2. **Tageszeit:** Morgen, Tag, Abend, Nacht als Partie-Option (Nacht: Mündungsfeuer
   und Brände sichtbar, Leuchtgeschosse optional).
3. **Darstellung:** Regen/Schnee-Partikel, nasser Boden, Pfützen, Fahrspuren im Schlamm
   (Decals), Licht und Himmel aus 015 an Wetter und Tageszeit gekoppelt; Ambiente
   (030) mit Regen und Wind.
4. **Setup und Editor:** Wetter und Tageszeit im Spiel-Setup und als Karten-Standard.
5. **KI:** berücksichtigt Bodenkosten über die Wegfindung automatisch; prüfen.

## 3. Akzeptanzkriterien

1. `just check` grün; Sim-Tests: Schlamm bremst Rad stärker als Kette, Straße bleibt
   schnell, Sicht im Nebel kürzer, gleicher Hash; Golden-Hash bewusst aktualisiert
   oder Standardwetter „klar“ ändert ihn nicht (besser).
2. `just selftest` mit Regen- und Nachtpartie grün; Benchmark: Wetter kostet höchstens
   2 ms pro Frame.
3. Doku: `docs/MAPS.md`, `docs/ARCHITECTURE.md`, Regelwerte mit Begründung.

## Arbeitsweise

Wie in den vorigen Prompts: Deutsch mit mir, Englisch im Repo, kein Push ohne Freigabe,
keine Tests abschwächen, Commits pro Schritt. Vor Installationen und Downloads fragen.
Gemeinsame Dateien (`sim.rs`, `bridge/src/lib.rs`, `main.gd`, `self_test.gd`,
`justfile`, Doku) nur klein und additiv ändern, weil andere Prompts parallel laufen.
Ändert sich der Golden-Hash, den Grund im Commit nennen. Am Ende nach `prompts/done/`
verschieben und `prompts/README.md` aktualisieren. Abschlussbericht: was gebaut wurde,
wie ich es teste, Messwerte, was offen ist.

## Konkretisierung (Ausgangslage und Entscheidungen)

Ausgangslage im Code (nach Welle 11): Geländearten mit Geschwindigkeitsanteil
(`data/terrain.toml`, darunter `mud` und `snow` als feste Kartenfarben), Wegegitter
mit Kosten pro Profil (= Einheitentyp) und Teil-Neubau (012/020/021), Sicht pro
Kategorie über `range.sight` (018), Rauchwolken mit festem Wind aus
`data/indirect.toml` (032, „fest bis Wetter 040“), Feuer ohne Wind (041), Aufklärer
mit eigenem Sichtradius (033), Spielstände als serde-Snapshot des ganzen `Sim`
(038). In Godot: Biom-Presets für Sonne, Himmel, Dunst, Schneefall (015), Ambiente
mit vorbereitetem `rain`-Regler (030). Der Katalog kennt kein Fahrwerk; die
Kategorie verrät es fast immer (der M3A1 Scout Car ist als „halftrack“ eingeordnet,
fährt aber auf Rädern).

| # | Entscheidung | Begründung |
|---|---|---|
| 1 | **Ein Wetter und eine Tageszeit pro Partie** (`WeatherSetup` in `SimConfig`: klar/bedeckt/Regen/Schnee/Nebel × Morgen/Tag/Abend/Nacht). Kein Wetterwechsel während der Partie; der Boden ändert sich aber (Regen weicht ihn in Stufen auf). **[vorläufig]** | Spec erlaubt „fest gewählt“; Sichtweiten bleiben pro Partie fest (keine Neuberechnung aller Sichtfelder), der Regen wirkt trotzdem als Ablauf. |
| 2 | Regeln in eigener Datei **`data/weather.toml`**, von `Sim::new` geladen (wie 021/032/041). Der Standard **klar/Tag** muss überall 100 % haben (Parser prüft das) und der Wetterzustand geht nur dann in den Hash ein, wenn er nicht Standard ist. | Golden-Hash bleibt unverändert (bevorzugt laut Auftrag); Regeln bleiben balancierbar. |
| 3 | **Bodenweichheit** als ein Wert pro Partie (0..100 %, Art: trocken/nass/Schnee), wirksam je **Geländeart** (`[[ground.terrain]]`: Feldweg 100 %, Gras 70 %, Straße 0 % nass / 30 % Schnee …). Keine Feuchte pro Zelle. **[vorläufig]** | Pro Zelle bräuchte eigenen Zustand und Godot-Daten; die Geländeart trägt den spielerischen Kern (Straßen werden wichtig). Krater machen ohnehin schon Schlamm-Zellen (021). |
| 4 | **Fahrwerk** (Fuß, Rad, Halbkette, Kette) aus der **Katalog-Kategorie** plus Id-Ausnahmen in `data/weather.toml` (M3A1 → Rad); gezogene Geschütze = Rad. Tempo = Geländetempo × (100 % − Verlust(Fahrwerk) × Weichheit(Gelände) × Bodenwert), mindestens 8 %. Verluste bei voller Nässe: Rad 80, Halbkette 50, Kette 35, Fuß 30 %. Räder „bleiben stecken“ als Kriechtempo (Rad im durchweichten Schlamm: 40 % × 20 % = 8 % des Geländetempos), kein Festfahren mit Bergen. **[vorläufig]** | Kein neues Katalogfeld nötig (Katalog braucht Quellen, das hier sind Spielregeln); Kriechen statt Festsitzen braucht keine Bergungs-Mechanik. |
| 5 | **Wegfindung:** jedes Profil bekommt die Bodenfaktoren pro Geländeart (`PathGrid::set_ground_pct`), die Zellkosten folgen. Beim Start alles auf einmal; bei jeder Regen-Stufe werden die Kosten **8 Zeilen pro Tick** neu berechnet (4-km-Karte: 63 Ticks, gemessen ≤ 1,6 ms pro Tick). Passierbarkeit ändert sich nicht, Komponenten bleiben. Bestehende Routen bleiben, neue Pfade nutzen die neuen Kosten. | „Teil-Neubau wie bei Zerstörung“ ohne Lastspitze; alle Einheiten neu planen zu lassen würde die Pfad-Warteschlange fluten. |
| 6 | **Regen:** Boden startet halb aufgeweicht (50 %) und wird alle 120 s Spielzeit um 10 % weicher bis 100 %. Schnee: geschlossene Decke (100 %) von Anfang an. Nebel: feuchter Boden 20 %. Bedeckt/klar: trocken. **[vorläufig]** | Regen ist der Ablauf; nach 10 min ist alles durchweicht. |
| 7 | **Sicht:** alle Sichtweiten (Einheiten und Aufklärer aus 033) × Wetter × Tageszeit: Regen 70, Schnee 60, Nebel 35, bedeckt 90; Morgen/Abend 80, Nacht 45 %. Umsetzung über den Sichtfaktor `sight_ratio` der Sichtregeln (`SimConfig::effective_vision`), die KI (024) nutzt dieselben Werte. **[vorläufig]** | Ein Eingriff, alle Sichtradien und die KI folgen; keine neuen Sichtfeld-Tabellen. |
| 8 | **Nacht: Mündungsfeuer.** Wer direkt schießt, ist 2 s Spielzeit für jeden Spieler sichtbar, der eine Einheit im Umkreis von 2000 m real (500 m Spiel) hat, ohne Sichtlinie. Brände leuchten nachts (Darstellung von 041). Leuchtgeschosse: offen. **[vorläufig]** | Spec „Mündungsfeuer sichtbar“ als Spielregel: Feuer verrät, man kann zurückschießen. |
| 9 | **Wind:** Wetter skaliert den Wind aus `data/indirect.toml` (Regen 150, Nebel 20 %); Rauch driftet entsprechend. **Feuer (041):** nasses Holz fängt schwerer (`fire_catch_pct`, Regen 40 %), und die Ausbreitung bekommt einen Windbonus (±50 % × cos(Winkel) × Windstärke). Der Windbonus gilt auch bei klarem Wetter; die Demo-Schlacht und ihr Hash ändern sich dadurch nicht (geprüft). | Auftrag: Wind für Feuer, wenn einfach – ein Faktor auf den vorhandenen Würfel, keine neuen Zufallszüge. |
| 10 | **Spielstand (038):** `WeatherState` (Bodenwert, nächste Stufe, laufende Neuberechnung, Mündungsfeuer) und die Bodenfaktoren im Wegegitter sind Felder von `Sim` und damit im Snapshot. Replay-Format 8, Protokoll 4 (`SimConfig` hat ein neues Feld). | Vorgabe; neue Felder sind automatisch im Snapshot. |
| 11 | **Setup und Editor:** Karten-Standard in `map.toml` (`weather`, `daytime`, nicht im Karten-Hash, wie `biome`), Editor-Felder; Spiel-Setup mit „Kartenstandard“ + Auswahl; `--weather=`/`--daytime=` für Direktstarts. LAN/Online nutzen den Kartenstandard. **[vorläufig]** | Kleine, additive Änderung; Lobby-Option bräuchte Protokoll- und Lobby-UI-Änderungen (offen). |
| 12 | **Darstellung:** `world/weather_look.gd` passt das Biom-Preset an (Sonne nach Tageszeit, Himmel/Licht bei Bedeckung/Regen, dichter Dunst bei Nebel, Schneefall bei Schnee) und baut Regen-Partikel um den Kamerafokus; das Terrain bekommt ein Uniform `wetness` (dunkler, glänzender Boden, Pfützen in Senken, additiv im Shader). Ambiente: `rain`-Regler. Fahrspuren im Schlamm: offen. **[vorläufig]** | Kleine Eingriffe in `atmosphere.gd` und den Terrain-Shader (057 arbeitet am Gras). |
| 13 | **KI:** keine eigene Wetterlogik; sie bekommt die Bodenkosten über die Wegfindung und die kürzere Sicht über `effective_vision`. Turnier läuft weiter. | Spec: „prüfen“. |
